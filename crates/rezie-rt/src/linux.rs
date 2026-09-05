use crate::{SchedulingPolicy, SchedulingReport, ThreadBudget};
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};

pub(super) struct Guard {
    timer: OwnedFd,
    prior_policy: i32,
    prior_param: libc::sched_param,
    prior_nice: i32,
    changed_policy: bool,
    changed_nice: bool,
}

fn nice_value() -> io::Result<i32> {
    // SAFETY: errno is thread-local; getpriority with pid=0 targets this thread.
    // Clearing errno distinguishes a valid nice value of -1 from an error.
    unsafe {
        *libc::__errno_location() = 0;
        let value = libc::getpriority(libc::PRIO_PROCESS, 0);
        let errno = *libc::__errno_location();
        if errno == 0 {
            Ok(value)
        } else {
            Err(io::Error::from_raw_os_error(errno))
        }
    }
}
impl Guard {
    pub(super) fn configure(_: ThreadBudget, report: &mut SchedulingReport) -> io::Result<Self> {
        // SAFETY: monotonic timerfd with close-on-exec returns a fresh owned fd.
        let fd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fd is a successful fresh descriptor; ownership transfers exactly once.
        let timer = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut prior_param = libc::sched_param { sched_priority: 0 };
        // SAFETY: pid=0 targets this thread; prior_param is writable SDK storage.
        let prior_policy = unsafe { libc::sched_getscheduler(0) };
        if prior_policy < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: initialized writable sched_param and current-thread target.
        if unsafe { libc::sched_getparam(0, &mut prior_param) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let prior_nice = nice_value()?;
        let mut guard = Self {
            timer,
            prior_policy,
            prior_param,
            prior_nice,
            changed_policy: false,
            changed_nice: false,
        };
        let parameter = libc::sched_param { sched_priority: 10 };
        // SAFETY: this thread is the only target; the parameter's FIFO priority is valid.
        if unsafe { libc::sched_setscheduler(0, libc::SCHED_FIFO, &parameter) } == 0 {
            guard.changed_policy = true;
            report.policy = SchedulingPolicy::SchedFifo;
            report.realtime = true;
            report.fifo_priority = Some(10);
        } else {
            report.realtime_error = io::Error::last_os_error().raw_os_error();
            report.policy = SchedulingPolicy::TimerFdFallback;
            if prior_nice > -10 {
                // SAFETY: adjusts only the calling thread's nice value; privileges are checked by the OS.
                if unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -10) } == 0 {
                    guard.changed_nice = true;
                } else {
                    report.nice_error = io::Error::last_os_error().raw_os_error();
                }
            }
        }
        report.nice = Some(nice_value()?);
        Ok(guard)
    }

    pub(super) fn sleep(&mut self, duration: Duration) -> io::Result<()> {
        let mut now = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: now is writable timespec storage and CLOCK_MONOTONIC is supported.
        if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let nanos = now.tv_nsec as u64 + u64::from(duration.subsec_nanos());
        let target = libc::timespec {
            tv_sec: now.tv_sec + duration.as_secs() as i64 + (nanos / 1_000_000_000) as i64,
            tv_nsec: (nanos % 1_000_000_000) as i64,
        };
        let setting = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: target,
        };
        // SAFETY: timer is live and owned; setting is a valid one-shot absolute
        // monotonic deadline, and null old_value requests no previous setting.
        if unsafe {
            libc::timerfd_settime(
                self.timer.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME,
                &setting,
                std::ptr::null_mut(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut expirations = 0_u64;
        loop {
            // SAFETY: the timerfd read writes exactly one u64 into valid storage.
            // This is the explicitly prescribed timer wait, not unrelated sink I/O.
            let count = unsafe {
                libc::read(
                    self.timer.as_raw_fd(),
                    (&mut expirations as *mut u64).cast(),
                    size_of::<u64>(),
                )
            };
            if count == size_of::<u64>() as isize {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            if count < 0 && error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(if count < 0 {
                error
            } else {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "short timerfd expiration read",
                )
            });
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: the public guard is thread-affine; restore only successfully
        // changed settings captured from this same live thread. OwnedFd closes the timer.
        unsafe {
            if self.changed_policy {
                libc::sched_setscheduler(0, self.prior_policy, &self.prior_param);
            }
            if self.changed_nice {
                libc::setpriority(libc::PRIO_PROCESS, 0, self.prior_nice);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linux_policy_and_nice_are_restored_on_unwind() {
        std::thread::spawn(|| {
            // SAFETY: sched_getscheduler reads only this live calling thread's policy.
            let before_policy = unsafe { libc::sched_getscheduler(0) };
            let before_nice = nice_value().unwrap();
            let unwind = std::panic::catch_unwind(|| {
                let _guard = crate::RealtimeThread::configure(ThreadBudget {
                    period: Duration::from_millis(20),
                    computation: Duration::from_millis(2),
                    constraint: Duration::from_millis(3),
                })
                .unwrap();
                panic!("exercise RAII restoration");
            });
            assert!(unwind.is_err());
            // SAFETY: same current-thread query after the guard's destructor ran.
            assert_eq!(unsafe { libc::sched_getscheduler(0) }, before_policy);
            assert_eq!(nice_value().unwrap(), before_nice);
        })
        .join()
        .unwrap();
    }
}
