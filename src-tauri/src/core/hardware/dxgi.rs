use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct DxgiSelectedAdapter {
    pub name: String,
    pub dedicated_vram_mb: u64,
    pub shared_memory_mb: u64,
    pub current_vram_usage_mb: u64,
    pub luid_low: u32,
    pub luid_high: i32,
}

#[cfg(windows)]
pub fn detect_best_dxgi_adapter() -> AppResult<DxgiSelectedAdapter> {
    use windows::core::Interface;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter3, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
        DXGI_ERROR_NOT_FOUND, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
    };

    let factory: IDXGIFactory1 = unsafe {
        CreateDXGIFactory1().map_err(|e| AppError::InternalError(format!("CreateDXGIFactory1: {e}")))?
    };

    let mut best: Option<(DxgiSelectedAdapter, windows::Win32::Graphics::Dxgi::IDXGIAdapter1)> = None;
    let mut index = 0u32;

    loop {
        match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => {
                let desc = unsafe {
                    adapter
                        .GetDesc1()
                        .map_err(|e| AppError::InternalError(format!("GetDesc1({index}): {e}")))?
                };
                let is_software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;
                if !is_software {
                    let end = desc
                        .Description
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(desc.Description.len());
                    let name = String::from_utf16_lossy(&desc.Description[..end]);
                    let dedicated_mb = (desc.DedicatedVideoMemory as u64) / (1024 * 1024);
                    let shared_mb = (desc.SharedSystemMemory as u64) / (1024 * 1024);

                    let candidate = DxgiSelectedAdapter {
                        name,
                        dedicated_vram_mb: dedicated_mb,
                        shared_memory_mb: shared_mb,
                        current_vram_usage_mb: 0,
                        luid_low: desc.AdapterLuid.LowPart,
                        luid_high: desc.AdapterLuid.HighPart,
                    };

                    let replace = match &best {
                        None => true,
                        Some((prev, _)) => candidate.dedicated_vram_mb > prev.dedicated_vram_mb,
                    };
                    if replace {
                        best = Some((candidate, adapter));
                    }
                }
                index += 1;
            }
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(e) => {
                return Err(AppError::InternalError(format!(
                    "EnumAdapters1({index}): {e}"
                )));
            }
        }
    }

    let (mut selected, adapter) = best.ok_or_else(|| {
        AppError::NotFound("No DXGI GPU adapter detected on this device".to_string())
    })?;

    if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
        let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
        if unsafe {
            adapter3.QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
        }
        .is_ok()
        {
            selected.current_vram_usage_mb = info.CurrentUsage / (1024 * 1024);
        }
    }

    Ok(selected)
}

#[cfg(windows)]
pub fn query_live_vram_mb() -> Option<(u64, u64)> {
    if let Some((used, total)) = crate::core::hardware::vulkan::probe_nvml_memory_mb() {
        return Some((used, total));
    }
    let adapter = detect_best_dxgi_adapter().ok()?;
    Some((adapter.current_vram_usage_mb, adapter.dedicated_vram_mb))
}

#[cfg(windows)]
pub fn query_system_ram_gb() -> AppResult<f64> {
    use std::mem::size_of;
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut mem = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        GlobalMemoryStatusEx(&mut mem)
            .map_err(|e| AppError::InternalError(format!("GlobalMemoryStatusEx: {e}")))?;
    }
    let gb = mem.ullTotalPhys as f64 / (1024.0 * 1024.0 * 1024.0);
    Ok((gb * 100.0).round() / 100.0)
}

#[cfg(not(windows))]
pub fn detect_best_dxgi_adapter() -> AppResult<DxgiSelectedAdapter> {
    Err(AppError::InternalError("DXGI is only supported on Windows".to_string()))
}

#[cfg(not(windows))]
pub fn query_live_vram_mb() -> Option<(u64, u64)> {
    None
}

#[cfg(not(windows))]
pub fn query_system_ram_gb() -> AppResult<f64> {
    Ok(0.0)
}
