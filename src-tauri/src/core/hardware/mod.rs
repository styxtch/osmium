pub mod disk;
pub mod dxgi;
pub mod types;
pub mod vulkan;

use crate::core::hardware::types::{HardwareProfile, HardwareTier};
use crate::error::AppResult;
use std::path::Path;

pub const THRESHOLD_OPTIMAL_VRAM_MB: u64 = 7680; // 7.5 GB
pub const THRESHOLD_BALANCED_VRAM_MB: u64 = 5632; // 5.5 GB
pub const THRESHOLD_HIGH_CTX_VRAM_MB: u64 = 10752; // 10.5 GB

pub fn classify_tier(dedicated_vram_mb: u64, vulkan_supported: bool) -> HardwareTier {
    if !vulkan_supported {
        return HardwareTier::FallbackCpu;
    }
    if dedicated_vram_mb >= THRESHOLD_OPTIMAL_VRAM_MB {
        HardwareTier::Optimal8b
    } else if dedicated_vram_mb >= THRESHOLD_BALANCED_VRAM_MB {
        HardwareTier::Balanced4b
    } else {
        HardwareTier::FallbackCpu
    }
}

pub fn recommended_ctx_size(dedicated_vram_mb: u64, vulkan_supported: bool) -> u32 {
    if !vulkan_supported {
        return 2048;
    }
    if dedicated_vram_mb >= THRESHOLD_HIGH_CTX_VRAM_MB {
        8192
    } else if dedicated_vram_mb >= THRESHOLD_BALANCED_VRAM_MB {
        4096
    } else {
        2048
    }
}

/// Estimates partial GPU offload layers for < 5.5 GB GPUs.
/// Returns `0` when fewer than 4 layers fit to avoid excessive PCIe transfer overhead.
pub fn estimate_partial_offload_layers(
    dedicated_vram_mb: u64,
    model_size_bytes: u64,
    block_count: Option<u32>,
) -> u32 {
    if dedicated_vram_mb >= THRESHOLD_BALANCED_VRAM_MB {
        return 99;
    }
    let Some(blocks) = block_count.filter(|&b| b > 0) else {
        return 0;
    };
    let model_mb = (model_size_bytes as f64) / (1024.0 * 1024.0);
    if model_mb <= 0.0 {
        return 0;
    }
    let usable_vram_mb = dedicated_vram_mb.saturating_sub(768) as f64;
    if usable_vram_mb <= 0.0 {
        return 0;
    }
    let ratio = (usable_vram_mb / (model_mb * 1.15)).clamp(0.0, 1.0);
    let layers = (ratio * f64::from(blocks)).floor() as u32;
    if layers < 4 {
        0
    } else {
        layers.min(blocks)
    }
}

pub fn scan_hardware(bin_dir: Option<&Path>) -> AppResult<HardwareProfile> {
    let adapter = dxgi::detect_best_dxgi_adapter()?;
    let system_ram_gb = dxgi::query_system_ram_gb()?;
    let vulkan_version = vulkan::probe_vulkan_loader_version();
    let vulkan_device_id = if vulkan_version.is_some() {
        vulkan::detect_vulkan_device_id(bin_dir, &adapter.name)
    } else {
        None
    };

    let vulkan_supported = vulkan_version.is_some() && (bin_dir.is_none() || vulkan_device_id.is_some());
    let recommended_tier = classify_tier(adapter.dedicated_vram_mb, vulkan_supported);
    let nvml = vulkan::probe_nvml_snapshot();

    Ok(HardwareProfile {
        gpu_name: adapter.name,
        dedicated_vram_mb: nvml.vram_total_mb.unwrap_or(adapter.dedicated_vram_mb),
        shared_memory_mb: adapter.shared_memory_mb,
        current_vram_usage_mb: nvml.vram_used_mb.unwrap_or(adapter.current_vram_usage_mb),
        system_ram_gb,
        vulkan_supported,
        vulkan_version,
        vulkan_device_id,
        flash_attention_supported: vulkan_supported,
        recommended_tier,
        thermal_c: nvml.thermal_c,
        clock_mhz: nvml.clock_mhz,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn hardware_tier_and_ctx_thresholds() {
        assert_eq!(classify_tier(12115, true), HardwareTier::Optimal8b);
        assert_eq!(classify_tier(7680, true), HardwareTier::Optimal8b);
        assert_eq!(classify_tier(7679, true), HardwareTier::Balanced4b);
        assert_eq!(classify_tier(5632, true), HardwareTier::Balanced4b);
        assert_eq!(classify_tier(4096, true), HardwareTier::FallbackCpu);
        assert_eq!(classify_tier(12115, false), HardwareTier::FallbackCpu);

        assert_eq!(recommended_ctx_size(12115, true), 8192);
        assert_eq!(recommended_ctx_size(8192, true), 4096);
        assert_eq!(recommended_ctx_size(6144, true), 4096);
        assert_eq!(recommended_ctx_size(4096, true), 2048);
        assert_eq!(recommended_ctx_size(12115, false), 2048);
    }

    #[test]
    fn partial_offload_estimation_handles_edges() {
        // 6GB+ gets full offload
        assert_eq!(estimate_partial_offload_layers(6144, 2_500_000_000, Some(36)), 99);
        // 4GB GPU (4096 MB) with 2.4 GiB 4B model & 36 blocks: usable = 3328 MB -> full/nearly full layers
        let layers_4gb = estimate_partial_offload_layers(4096, 2_497_280_256, Some(36));
        assert!(layers_4gb >= 30 && layers_4gb <= 36);
        // 1GB GPU -> fewer than 4 layers -> falls back to 0 (CPU only)
        assert_eq!(estimate_partial_offload_layers(1024, 2_497_280_256, Some(36)), 0);
    }

    #[test]
    fn hybrid_graphics_vulkan_device_matching() {
        let output = "\
Available devices:
  Vulkan0: Intel(R) UHD Graphics (16384 MiB, 16000 MiB free)
  Vulkan1: NVIDIA GeForce RTX 4060 Laptop GPU (8188 MiB, 7900 MiB free)
";
        let matched = vulkan::parse_llama_devices_and_match(
            output,
            "NVIDIA GeForce RTX 4060 Laptop GPU",
        );
        assert_eq!(matched.as_deref(), Some("Vulkan1"));
    }

    #[test]
    fn disk_space_and_protected_path_validation() {
        let temp_dir = std::env::temp_dir().join(format!("osmium_disk_test_{}", uuid::Uuid::new_v4()));
        let info = disk::check_disk_space(&temp_dir).expect("temp dir disk space check");
        assert!(info.free_bytes > 0);
        assert!(info.writable);
        let _ = std::fs::remove_dir_all(&temp_dir);

        let forbidden = PathBuf::from("C:\\Windows\\Temp\\osmium_test");
        assert!(disk::validate_user_dir(&forbidden).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn live_scan_hardware_succeeds() {
        let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("bin");
        let profile = scan_hardware(Some(&bin_dir)).expect("scan_hardware should succeed");
        println!("[Hardware Test] Profile: {:?}", profile);
        assert!(!profile.gpu_name.is_empty());
        assert!(profile.system_ram_gb > 0.0);
        assert!(profile.vulkan_device_id.is_some());
        if profile.thermal_c.is_some() {
            assert!(
                profile.current_vram_usage_mb > 0,
                "NVML GPU-wide VRAM usage should report > 0 MB"
            );
        }
    }
}
