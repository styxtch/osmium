use std::path::Path;

#[cfg(windows)]
pub fn probe_vulkan_loader_version() -> Option<String> {
    type VkEnumerateInstanceVersionFn = unsafe extern "system" fn(*mut u32) -> i32;

    let lib = unsafe { libloading::Library::new("vulkan-1.dll") }.ok()?;
    let func = unsafe {
        lib.get::<VkEnumerateInstanceVersionFn>(b"vkEnumerateInstanceVersion\0")
    };

    match func {
        Ok(enumerate_fn) => {
            let mut api_version: u32 = 0;
            let res = unsafe { enumerate_fn(&mut api_version) };
            if res == 0 {
                let major = (api_version >> 22) & 0x7f;
                let minor = (api_version >> 12) & 0x3ff;
                Some(format!("{major}.{minor}"))
            } else {
                None
            }
        }
        Err(_) => Some("1.0".to_string()),
    }
}

#[cfg(not(windows))]
pub fn probe_vulkan_loader_version() -> Option<String> {
    None
}

#[cfg(windows)]
pub fn detect_vulkan_device_id(bin_dir: Option<&Path>, dxgi_gpu_name: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = bin_dir?.join("llama-server.exe");
    if !exe.exists() {
        return None;
    }

    let output = std::process::Command::new(exe)
        .arg("--list-devices")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_llama_devices_and_match(&combined, dxgi_gpu_name)
}

#[cfg(not(windows))]
pub fn detect_vulkan_device_id(_bin_dir: Option<&Path>, _dxgi_gpu_name: &str) -> Option<String> {
    None
}

/// Parses `llama-server --list-devices` lines (e.g. `Vulkan0: NVIDIA GeForce RTX 3060 (...)`)
/// and matches the discrete DXGI GPU name to prevent hybrid-graphics iGPU spillover.
pub fn parse_llama_devices_and_match(output: &str, dxgi_gpu_name: &str) -> Option<String> {
    let mut devices: Vec<(String, String)> = Vec::new();

    for raw_line in output.lines() {
        let line = raw_line.trim();
        let Some((id_part, rest)) = line.split_once(':') else {
            continue;
        };
        let dev_id = id_part.trim();
        if !dev_id.starts_with("Vulkan") {
            continue;
        }
        let dev_name = rest
            .split('(')
            .next()
            .unwrap_or(rest)
            .trim()
            .to_string();
        devices.push((dev_id.to_string(), dev_name));
    }

    if devices.is_empty() {
        return None;
    }

    let target = normalize_gpu_name(dxgi_gpu_name);
    if !target.is_empty() {
        if let Some((id, _)) = devices.iter().find(|(_, name)| {
            let cand = normalize_gpu_name(name);
            cand == target || cand.contains(&target) || target.contains(&cand)
        }) {
            return Some(id.clone());
        }

        // Token-overlap fallback for minor naming variations between DXGI and Vulkan drivers.
        let target_tokens: Vec<&str> = target.split_whitespace().collect();
        if let Some((id, _)) = devices
            .iter()
            .max_by_key(|(_, name)| {
                let cand = normalize_gpu_name(name);
                target_tokens
                    .iter()
                    .filter(|tok| tok.len() >= 3 && cand.contains(**tok))
                    .count()
            })
            .filter(|(_, name)| {
                let cand = normalize_gpu_name(name);
                target_tokens
                    .iter()
                    .filter(|tok| tok.len() >= 3 && cand.contains(**tok))
                    .count()
                    >= 2
            })
        {
            return Some(id.clone());
        }
    }

    Some(devices[0].0.clone())
}

fn normalize_gpu_name(name: &str) -> String {
    name.to_lowercase()
        .replace("(tm)", "")
        .replace("(r)", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Default)]
pub struct NvmlSnapshot {
    pub thermal_c: Option<u32>,
    pub clock_mhz: Option<u32>,
    pub vram_used_mb: Option<u64>,
    pub vram_total_mb: Option<u64>,
}

#[cfg(windows)]
pub fn probe_nvml_snapshot() -> NvmlSnapshot {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct NvmlMemory {
        total: u64,
        free: u64,
        used: u64,
    }

    type NvmlInitFn = unsafe extern "system" fn() -> i32;
    type NvmlShutdownFn = unsafe extern "system" fn() -> i32;
    type NvmlGetHandleFn = unsafe extern "system" fn(u32, *mut *mut c_void) -> i32;
    type NvmlGetTempFn = unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> i32;
    type NvmlGetClockFn = unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> i32;
    type NvmlGetMemFn = unsafe extern "system" fn(*mut c_void, *mut NvmlMemory) -> i32;

    let Ok(lib) = (unsafe { libloading::Library::new("nvml.dll") }) else {
        return NvmlSnapshot::default();
    };

    unsafe {
        let Ok(init) = lib.get::<NvmlInitFn>(b"nvmlInit_v2\0") else {
            return NvmlSnapshot::default();
        };
        if init() != 0 {
            return NvmlSnapshot::default();
        }

        let mut snap = NvmlSnapshot::default();

        if let Ok(get_handle) = lib.get::<NvmlGetHandleFn>(b"nvmlDeviceGetHandleByIndex_v2\0") {
            let mut device: *mut c_void = std::ptr::null_mut();
            if get_handle(0, &mut device) == 0 && !device.is_null() {
                if let Ok(get_temp) = lib.get::<NvmlGetTempFn>(b"nvmlDeviceGetTemperature\0") {
                    let mut val: u32 = 0;
                    if get_temp(device, 0, &mut val) == 0 {
                        snap.thermal_c = Some(val);
                    }
                }
                if let Ok(get_clock) = lib.get::<NvmlGetClockFn>(b"nvmlDeviceGetClockInfo\0") {
                    let mut val: u32 = 0;
                    if get_clock(device, 0, &mut val) == 0 {
                        snap.clock_mhz = Some(val);
                    }
                }
                if let Ok(get_mem) = lib.get::<NvmlGetMemFn>(b"nvmlDeviceGetMemoryInfo\0") {
                    let mut mem = NvmlMemory::default();
                    if get_mem(device, &mut mem) == 0 {
                        snap.vram_used_mb = Some(mem.used / (1024 * 1024));
                        snap.vram_total_mb = Some(mem.total / (1024 * 1024));
                    }
                }
            }
        }

        if let Ok(shutdown) = lib.get::<NvmlShutdownFn>(b"nvmlShutdown\0") {
            let _ = shutdown();
        }

        snap
    }
}

#[cfg(windows)]
pub fn probe_nvml_memory_mb() -> Option<(u64, u64)> {
    let snap = probe_nvml_snapshot();
    match (snap.vram_used_mb, snap.vram_total_mb) {
        (Some(used), Some(total)) if total > 0 => Some((used, total)),
        _ => None,
    }
}

#[cfg(not(windows))]
pub fn probe_nvml_snapshot() -> NvmlSnapshot {
    NvmlSnapshot::default()
}

#[cfg(not(windows))]
pub fn probe_nvml_memory_mb() -> Option<(u64, u64)> {
    None
}
