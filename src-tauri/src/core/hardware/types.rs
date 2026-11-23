use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareTier {
    #[serde(rename = "optimal_8b")]
    Optimal8b,
    #[serde(rename = "balanced_4b")]
    Balanced4b,
    #[serde(rename = "fallback_cpu")]
    FallbackCpu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub gpu_name: String,
    pub dedicated_vram_mb: u64,
    pub shared_memory_mb: u64,
    pub current_vram_usage_mb: u64,
    pub system_ram_gb: f64,
    pub vulkan_supported: bool,
    pub vulkan_version: Option<String>,
    pub vulkan_device_id: Option<String>,
    pub flash_attention_supported: bool,
    pub recommended_tier: HardwareTier,
    pub thermal_c: Option<u32>,
    pub clock_mhz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskSpaceInfo {
    pub free_bytes: u64,
    pub writable: bool,
}
