use crate::{SchedulingPolicy, SchedulingReport, ThreadBudget};
use std::{ffi::c_void, io, time::Duration};

// SAFETY: signatures and calling convention match the Win32 avrt.h declarations.
#[link(name = "avrt")]
unsafe extern "system" {
    fn AvSetMmThreadCharacteristicsW(task_name: *const u16, task_index: *mut u32) -> *mut c_void;
    fn AvRevertMmThreadCharacteristics(handle: *mut c_void) -> i32;
}
// SAFETY: UINT parameters/results match the Win32 timeapi.h ABI.
#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(period: u32) -> u32;
    fn timeEndPeriod(period: u32) -> u32;
}

pub(super) struct Guard {
    handle: *mut c_void,
    timer_acquired: bool,
}
impl Guard {
    pub(super) fn configure(_: ThreadBudget, report: &mut SchedulingReport) -> io::Result<Self> {
        // SAFETY: timeBeginPeriod accepts this documented 1 ms resolution request.
        let timer = unsafe { timeBeginPeriod(1) };
        let mut guard = Self {
            handle: std::ptr::null_mut(),
            timer_acquired: timer == 0,
        };
        if timer == 0 {
            report.timer_resolution_ms = Some(1);
        } else {
            report.timer_error = Some(timer);
        }
        let task: [u16; 10] = [80, 114, 111, 32, 65, 117, 100, 105, 111, 0];
        let mut index = 0;
        // SAFETY: task is a NUL-terminated UTF-16 "Pro Audio" string; index is
        // writable for the call. The returned handle is retained on this thread.
        guard.handle = unsafe { AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index) };
        if guard.handle.is_null() {
            report.realtime_error = io::Error::last_os_error().raw_os_error();
        } else {
            report.policy = SchedulingPolicy::MmcssProAudio;
            report.realtime = true;
        }
        Ok(guard)
    }
    pub(super) fn sleep(&mut self, duration: Duration) -> io::Result<()> {
        std::thread::sleep(duration);
        Ok(())
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: only successful acquisitions are released, once, on the
        // acquiring thread. Revert restores pre-MMCSS thread characteristics;
        // matching timeEndPeriod releases only our timer-resolution request.
        unsafe {
            if !self.handle.is_null() {
                AvRevertMmThreadCharacteristics(self.handle);
            }
            if self.timer_acquired {
                timeEndPeriod(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // SAFETY: these signatures match Win32 processthreadsapi.h.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn GetThreadPriority(thread: *mut c_void) -> i32;
    }
    #[test]
    fn windows_priority_is_restored_on_unwind() {
        std::thread::spawn(|| {
            // SAFETY: GetCurrentThread returns a pseudo-handle valid on this calling thread.
            let before = unsafe { GetThreadPriority(GetCurrentThread()) };
            let unwind = std::panic::catch_unwind(|| {
                let _guard = crate::RealtimeThread::configure_wait(
                    ThreadBudget {
                        period: Duration::from_millis(20),
                        computation: Duration::from_millis(2),
                        constraint: Duration::from_millis(3),
                    },
                    Duration::ZERO,
                    false,
                )
                .unwrap();
                panic!("exercise RAII restoration");
            });
            assert!(unwind.is_err());
            // SAFETY: same current-thread pseudo-handle queried after cleanup.
            assert_eq!(unsafe { GetThreadPriority(GetCurrentThread()) }, before);
        })
        .join()
        .unwrap();
    }
}
