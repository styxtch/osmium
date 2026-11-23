use crate::error::{AppError, AppResult};
use std::ffi::c_void;

#[cfg(windows)]
pub struct Win32JobObject {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for Win32JobObject {}
#[cfg(windows)]
unsafe impl Sync for Win32JobObject {}

#[cfg(windows)]
impl Win32JobObject {
    pub fn new() -> AppResult<Self> {
        use std::mem::size_of;
        use windows::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        let handle = unsafe { CreateJobObjectW(None, None) }
            .map_err(|e| AppError::InternalError(format!("CreateJobObjectW: {e}")))?;

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        if let Err(e) = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
            return Err(AppError::InternalError(format!(
                "SetInformationJobObject: {e}"
            )));
        }

        Ok(Self { handle })
    }

    pub fn assign_process_handle(&self, raw_handle: *mut c_void) -> AppResult<()> {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::JobObjects::AssignProcessToJobObject;

        unsafe { AssignProcessToJobObject(self.handle, HANDLE(raw_handle)) }
            .map_err(|e| AppError::InternalError(format!("AssignProcessToJobObject: {e}")))
    }
}

#[cfg(windows)]
impl Drop for Win32JobObject {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(not(windows))]
pub struct Win32JobObject;

#[cfg(not(windows))]
impl Win32JobObject {
    pub fn new() -> AppResult<Self> {
        Ok(Self)
    }

    pub fn assign_process_handle(&self, _raw_handle: *mut c_void) -> AppResult<()> {
        Ok(())
    }
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::WaitForSingleObject;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    #[test]
    fn job_object_terminates_child_when_dropped() {
        let job = Win32JobObject::new().expect("create Win32JobObject");
        let mut child = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn child");

        let proc_handle = child.as_raw_handle();
        job.assign_process_handle(proc_handle)
            .expect("assign child to job object");

        let before = unsafe { WaitForSingleObject(HANDLE(proc_handle as _), 50) };
        assert_eq!(before, WAIT_TIMEOUT);

        drop(job);

        let after = unsafe { WaitForSingleObject(HANDLE(proc_handle as _), 1500) };
        let _ = child.wait();
        assert_eq!(after, WAIT_OBJECT_0);
    }
}
