//! Keeps every program the app starts, and everything those programs start,
//! in one Windows "job object" so they can all be stopped together.
//!
//! yt-dlp.exe is really two processes (a small launcher and the Python program
//! it unpacks), and it starts ffmpeg and node by itself. Stopping only the
//! process the app started left the others running. Cancel ends the whole job,
//! and closing the job (when the app is done with the program) ends anything
//! that is still left over.

use std::process::Child;

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    pub struct Job(HANDLE);

    // The handle is only used through the Win32 calls below, which may be made
    // from any thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn for_child(child: &Child) -> Option<Job> {
            unsafe {
                let handle = CreateJobObjectW(None, PCWSTR::null()).ok()?;
                let job = Job(handle);
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .ok()?;
                AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle() as *mut c_void)).ok()?;
                Some(job)
            }
        }

        pub fn kill(&self) {
            unsafe {
                let _ = TerminateJobObject(self.0, 1);
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::process::Child;

    pub struct Job;

    impl Job {
        pub fn for_child(_child: &Child) -> Option<Job> {
            None
        }

        pub fn kill(&self) {}
    }
}

pub use imp::Job;

/// A program the app is running, with the job that holds it and its children.
pub struct Running {
    pub child: Child,
    pub job: Option<Job>,
}

impl Running {
    pub fn new(child: Child) -> Self {
        let job = Job::for_child(&child);
        Self { child, job }
    }

    /// Stops the program and everything it started.
    pub fn kill_tree(&mut self) {
        match &self.job {
            Some(job) => job.kill(),
            None => taskkill_tree(self.child.id()),
        }
        let _ = self.child.kill();
    }
}

/// Fallback when the job could not be set up (for example if Windows refused
/// to nest it): ask Windows' own taskkill to end the process and its children.
/// It is started and not waited for, so the window never waits on it.
fn taskkill_tree(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn();
    }
    #[cfg(not(windows))]
    let _ = pid;
}
