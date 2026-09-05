use crate::{SchedulingPolicy, SchedulingReport, ThreadBudget};
use std::{io, time::Duration};

// SDK mach/mach_time.h: two uint32_t fields; conversion is immutable per host.
#[repr(C)]
struct MachTimebase {
    numer: u32,
    denom: u32,
}
// SAFETY: this signature matches mach_timebase_info_data_t / kern_return_t.
// libc deprecates its copy of this declaration, so keep the small ABI boundary
// here rather than suppressing deprecation across the scheduling module.
#[link(name = "System")]
unsafe extern "C" {
    fn mach_timebase_info(info: *mut MachTimebase) -> i32;
}

pub(super) struct Guard {
    thread: libc::thread_act_t,
    previous: libc::thread_time_constraint_policy,
    previous_was_default: bool,
    previous_extended: libc::thread_extended_policy,
    changed: bool,
}

impl Guard {
    pub(super) fn configure(
        budget: ThreadBudget,
        report: &mut SchedulingReport,
    ) -> io::Result<Self> {
        // SAFETY: pthread_self identifies this live calling thread; the Mach port
        // returned by pthread_mach_thread_np is borrowed, not a new owned send right.
        let thread = unsafe { libc::pthread_mach_thread_np(libc::pthread_self()) };
        let mut previous = libc::thread_time_constraint_policy {
            period: 0,
            computation: 0,
            constraint: 0,
            preemptible: 0,
        };
        let mut count = libc::THREAD_TIME_CONSTRAINT_POLICY_COUNT;
        let mut get_default = 0;
        // SAFETY: the initialized structure has the SDK-defined layout and count,
        // all output pointers remain valid for this synchronous call.
        let result = unsafe {
            libc::thread_policy_get(
                thread,
                libc::THREAD_TIME_CONSTRAINT_POLICY as libc::thread_policy_flavor_t,
                (&mut previous as *mut libc::thread_time_constraint_policy).cast(),
                &mut count,
                &mut get_default,
            )
        };
        if result != 0 {
            return Err(io::Error::other(format!(
                "read prior Mach time-constraint policy: kern_return_t={result}"
            )));
        }
        let mut previous_extended = libc::thread_extended_policy { timeshare: 1 };
        let mut extended_count = libc::THREAD_EXTENDED_POLICY_COUNT;
        let mut extended_default = 0;
        // SAFETY: valid calling-thread port and writable SDK-layout policy/count fields.
        let result = unsafe {
            libc::thread_policy_get(
                thread,
                libc::THREAD_EXTENDED_POLICY as libc::thread_policy_flavor_t,
                (&mut previous_extended as *mut libc::thread_extended_policy).cast(),
                &mut extended_count,
                &mut extended_default,
            )
        };
        if result != 0 {
            return Err(io::Error::other(format!(
                "read prior Mach extended policy: kern_return_t={result}"
            )));
        }
        let mut guard = Self {
            thread,
            previous,
            previous_was_default: get_default != 0,
            previous_extended,
            changed: false,
        };
        let mut timebase = MachTimebase { numer: 0, denom: 0 };
        // SAFETY: timebase points to initialized writable SDK-sized storage.
        let result = unsafe { mach_timebase_info(&mut timebase) };
        if result != 0 || timebase.numer == 0 || timebase.denom == 0 {
            return Err(io::Error::other("invalid Mach absolute-time conversion"));
        }
        let ticks = |duration: Duration| -> io::Result<u32> {
            u32::try_from(
                (duration.as_nanos() * u128::from(timebase.denom))
                    .div_ceil(u128::from(timebase.numer)),
            )
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Mach thread budget exceeds u32 clock units",
                )
            })
        };
        let mut desired = libc::thread_time_constraint_policy {
            period: ticks(budget.period)?,
            computation: ticks(budget.computation)?,
            constraint: ticks(budget.constraint)?,
            preemptible: 1,
        };
        // SAFETY: the current-thread port is live; the four-field policy has the
        // exact count required by Mach and is read only during this synchronous call.
        let result = unsafe {
            libc::thread_policy_set(
                thread,
                libc::THREAD_TIME_CONSTRAINT_POLICY as libc::thread_policy_flavor_t,
                (&mut desired as *mut libc::thread_time_constraint_policy).cast(),
                libc::THREAD_TIME_CONSTRAINT_POLICY_COUNT,
            )
        };
        if result != 0 {
            report.realtime_error = Some(result);
            return Ok(guard);
        }
        guard.changed = true;
        let mut actual = libc::thread_time_constraint_policy {
            period: 0,
            computation: 0,
            constraint: 0,
            preemptible: 0,
        };
        let mut actual_count = libc::THREAD_TIME_CONSTRAINT_POLICY_COUNT;
        let mut actual_default = 0;
        // SAFETY: valid SDK-sized output storage, count and calling-thread port.
        let result = unsafe {
            libc::thread_policy_get(
                thread,
                libc::THREAD_TIME_CONSTRAINT_POLICY as libc::thread_policy_flavor_t,
                (&mut actual as *mut libc::thread_time_constraint_policy).cast(),
                &mut actual_count,
                &mut actual_default,
            )
        };
        if result == 0
            && actual_default == 0
            && actual.period == desired.period
            && actual.computation == desired.computation
            && actual.constraint == desired.constraint
        {
            report.policy = SchedulingPolicy::MachTimeConstraint;
            report.realtime = true;
        } else {
            report.realtime_error = Some(if result != 0 {
                result
            } else {
                libc::KERN_FAILURE
            });
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
        if !self.changed {
            return;
        }
        // SAFETY: the non-Send public guard is dropped on its creating live
        // thread. Prior policies were captured with these exact SDK layouts.
        unsafe {
            if self.previous_was_default {
                libc::thread_policy_set(
                    self.thread,
                    libc::THREAD_EXTENDED_POLICY as libc::thread_policy_flavor_t,
                    (&mut self.previous_extended as *mut libc::thread_extended_policy).cast(),
                    libc::THREAD_EXTENDED_POLICY_COUNT,
                );
            } else {
                libc::thread_policy_set(
                    self.thread,
                    libc::THREAD_TIME_CONSTRAINT_POLICY as libc::thread_policy_flavor_t,
                    (&mut self.previous as *mut libc::thread_time_constraint_policy).cast(),
                    libc::THREAD_TIME_CONSTRAINT_POLICY_COUNT,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> (u32, u32, u32, i32, i32) {
        // SAFETY: this test snapshots its own live thread with SDK-sized writable buffers.
        unsafe {
            let thread = libc::pthread_mach_thread_np(libc::pthread_self());
            let mut value = libc::thread_time_constraint_policy {
                period: 0,
                computation: 0,
                constraint: 0,
                preemptible: 0,
            };
            let mut count = libc::THREAD_TIME_CONSTRAINT_POLICY_COUNT;
            let mut default = 0;
            assert_eq!(
                libc::thread_policy_get(
                    thread,
                    libc::THREAD_TIME_CONSTRAINT_POLICY as libc::thread_policy_flavor_t,
                    (&mut value as *mut libc::thread_time_constraint_policy).cast(),
                    &mut count,
                    &mut default
                ),
                0
            );
            (
                value.period,
                value.computation,
                value.constraint,
                value.preemptible,
                default,
            )
        }
    }
    #[test]
    fn mach_policy_is_restored_on_unwind() {
        std::thread::spawn(|| {
            let before = policy();
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
            assert_eq!(policy(), before);
        })
        .join()
        .unwrap();
    }
}
