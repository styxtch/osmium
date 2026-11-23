//! Phase 0 Windows Technical Spike:
//! 1. DXGI adapter enumeration + System RAM detection (`GlobalMemoryStatusEx`)
//! 2. Dynamic Vulkan loader (`vulkan-1.dll`) + `llama-server.exe --list-devices`
//! 3. Win32 Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) for 0 zombie process guarantee

#[derive(Debug, Clone)]
pub struct DxgiAdapterInfo {
    pub index: u32,
    pub name: String,
    pub luid_low: u32,
    pub luid_high: i32,
    pub dedicated_vram_bytes: u64,
    pub shared_system_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct HardwareSpikeReport {
    pub adapters: Vec<DxgiAdapterInfo>,
    pub total_system_ram_bytes: u64,
    pub vulkan_version: Option<(u32, u32, u32)>,
}

#[cfg(windows)]
pub fn probe_dxgi_and_ram() -> Result<(Vec<DxgiAdapterInfo>, u64), String> {
    use std::mem::size_of;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND,
    };
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut mem_status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        GlobalMemoryStatusEx(&mut mem_status)
            .map_err(|e| format!("GlobalMemoryStatusEx failed: {e}"))?;
    }

    let factory: IDXGIFactory1 = unsafe {
        CreateDXGIFactory1().map_err(|e| format!("CreateDXGIFactory1 failed: {e}"))?
    };

    let mut adapters = Vec::new();
    let mut index = 0u32;

    loop {
        match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => {
                let desc = unsafe {
                    adapter
                        .GetDesc1()
                        .map_err(|e| format!("GetDesc1 failed for adapter {index}: {e}"))?
                };
                let is_software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;
                if !is_software {
                    let end = desc
                        .Description
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(desc.Description.len());
                    let name = String::from_utf16_lossy(&desc.Description[..end]);
                    adapters.push(DxgiAdapterInfo {
                        index,
                        name,
                        luid_low: desc.AdapterLuid.LowPart,
                        luid_high: desc.AdapterLuid.HighPart,
                        dedicated_vram_bytes: desc.DedicatedVideoMemory as u64,
                        shared_system_bytes: desc.SharedSystemMemory as u64,
                    });
                }
                index += 1;
            }
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(e) => return Err(format!("EnumAdapters1({index}) failed: {e}")),
        }
    }

    Ok((adapters, mem_status.ullTotalPhys))
}

#[cfg(windows)]
pub fn probe_vulkan_version() -> Option<(u32, u32, u32)> {
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
                let patch = api_version & 0xfff;
                Some((major, minor, patch))
            } else {
                None
            }
        }
        Err(_) => Some((1, 0, 0)),
    }
}

#[cfg(windows)]
pub fn run_llama_list_devices(exe_path: &std::path::Path) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = std::process::Command::new(exe_path)
        .arg("--list-devices")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to spawn llama-server.exe: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "llama-server --list-devices exited with {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(format!("{stdout}\n{stderr}"))
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::WaitForSingleObject;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    #[test]
    fn spike_test_1_dxgi_and_system_ram() {
        let (adapters, ram_bytes) =
            probe_dxgi_and_ram().expect("DXGI + GlobalMemoryStatusEx probe should succeed");
        let ram_gb = ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        println!("[Spike Test 1] System RAM: {:.2} GB ({} bytes)", ram_gb, ram_bytes);
        println!("[Spike Test 1] Non-software DXGI Adapters ({}):", adapters.len());
        for a in &adapters {
            println!(
                "  - [{}] {} | Dedicated VRAM: {} MB | Shared: {} MB | LUID: ({:#x}, {:#x})",
                a.index,
                a.name,
                a.dedicated_vram_bytes / (1024 * 1024),
                a.shared_system_bytes / (1024 * 1024),
                a.luid_high,
                a.luid_low
            );
        }
        assert!(ram_bytes > 0, "System RAM must be > 0");
        assert!(
            !adapters.is_empty(),
            "Expected at least one non-software DXGI adapter"
        );
    }

    #[test]
    fn spike_test_2_vulkan_and_llama_list_devices() {
        let vk_ver = probe_vulkan_version();
        println!("[Spike Test 2] Vulkan loader version: {:?}", vk_ver);

        let bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("bin")
            .join("llama-server.exe");
        assert!(
            bin_path.exists(),
            "Bundled llama-server.exe must exist at {}",
            bin_path.display()
        );

        let devices_output = run_llama_list_devices(&bin_path)
            .expect("llama-server.exe --list-devices should succeed without missing DLL errors");
        println!(
            "[Spike Test 2] llama-server --list-devices output:\n{}",
            devices_output.trim()
        );
    }

    #[test]
    fn spike_test_3_win32_job_object_kills_child_on_close() {
        let job = unsafe { CreateJobObjectW(None, None) }
            .expect("CreateJobObjectW should succeed");

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .expect("SetInformationJobObject should succeed");

        let mut child = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("Failed to spawn long-lived child process");

        let proc_handle = HANDLE(child.as_raw_handle() as _);
        unsafe { AssignProcessToJobObject(job, proc_handle) }
            .expect("AssignProcessToJobObject should succeed");

        let wait_before = unsafe { WaitForSingleObject(proc_handle, 50) };
        assert_eq!(
            wait_before, WAIT_TIMEOUT,
            "Child process should still be alive before closing Job Object"
        );

        unsafe { CloseHandle(job) }.expect("CloseHandle(job) should succeed");

        let wait_after = unsafe { WaitForSingleObject(proc_handle, 1500) };
        let _ = child.wait();

        println!(
            "[Spike Test 3] WaitForSingleObject after closing Job Object: {:?}",
            wait_after
        );
        assert_eq!(
            wait_after, WAIT_OBJECT_0,
            "Child process must terminate immediately when Job Object closes"
        );
    }
}
