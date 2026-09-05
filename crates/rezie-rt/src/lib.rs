//! Safe, thread-affine realtime scheduling and deadline waiting; no domain semantics.
#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use serde::{Deserialize, Serialize};
use std::{
    io,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
compile_error!("rezie-rt supports macOS, Windows and Linux only");

/// Finishing slack selected for the scheduling correction, in OS-independent units.
/// Retained after the idle 60 s Apple M4 pilot at 50 Hz with Mach RT priority:
/// p99.9 lateness 18.375 us, maximum 19 us (ADR 0018). This calibrates the
/// combined priority/wait strategy; it does not isolate slack's contribution.
pub const FINISHING_SLACK: Duration = Duration::from_micros(1500);

/// A caller's periodic execution budget, independent of its work type.
#[derive(Debug, Clone, Copy)]
pub struct ThreadBudget {
    /// Typical interval between activations.
    pub period: Duration,
    /// CPU budget, including the finishing spin.
    pub computation: Duration,
    /// Maximum desired time to finish the computation after activation.
    pub constraint: Duration,
}

/// Scheduling actually achieved on the calling thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// Mach time-constraint policy was applied and read back.
    MachTimeConstraint,
    /// Windows multimedia scheduler Pro Audio task.
    MmcssProAudio,
    /// Linux FIFO policy at the reported priority.
    SchedFifo,
    /// Linux monotonic timerfd fallback; inspect nice/error fields.
    TimerFdFallback,
    /// Requested priority was unavailable; this is not a successful RT setup.
    Unavailable,
}

/// Startup evidence suitable for logging from a caller's control/startup thread.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchedulingReport {
    /// Effective scheduling path.
    pub policy: SchedulingPolicy,
    /// Whether native realtime elevation succeeded.
    pub realtime: bool,
    /// OS error from rejected realtime setup, if any.
    pub realtime_error: Option<i32>,
    /// Effective FIFO priority, when applicable.
    pub fifo_priority: Option<i32>,
    /// Effective Linux nice value (negative means elevated priority).
    pub nice: Option<i32>,
    /// OS error if Linux denied the requested nice elevation.
    pub nice_error: Option<i32>,
    /// Windows timer period actually acquired.
    pub timer_resolution_ms: Option<u32>,
    /// Error from Windows timer resolution setup.
    pub timer_error: Option<u32>,
    /// Configured finishing slack in nanoseconds.
    pub finishing_slack_ns: u64,
    /// Requested period in nanoseconds.
    pub period_ns: u64,
    /// Requested CPU budget in nanoseconds.
    pub computation_ns: u64,
    /// Requested completion constraint in nanoseconds.
    pub constraint_ns: u64,
}

impl SchedulingReport {
    fn new(budget: ThreadBudget) -> Self {
        Self {
            policy: SchedulingPolicy::Unavailable,
            realtime: false,
            realtime_error: None,
            fifo_priority: None,
            nice: None,
            nice_error: None,
            timer_resolution_ms: None,
            timer_error: None,
            finishing_slack_ns: FINISHING_SLACK.min(budget.computation * 3 / 4).as_nanos() as u64,
            period_ns: budget.period.as_nanos() as u64,
            computation_ns: budget.computation.as_nanos() as u64,
            constraint_ns: budget.constraint.as_nanos() as u64,
        }
    }

    /// True only if the requested native priority (or elevated Linux fallback) was achieved.
    pub fn correctly_prioritized(&self) -> bool {
        match self.policy {
            SchedulingPolicy::MachTimeConstraint | SchedulingPolicy::SchedFifo => self.realtime,
            SchedulingPolicy::MmcssProAudio => self.realtime && self.timer_resolution_ms == Some(1),
            SchedulingPolicy::TimerFdFallback => self.nice.is_some_and(|n| n < 0),
            SchedulingPolicy::Unavailable => false,
        }
    }
}

/// Owns this thread's scheduling changes and restores them on drop, including unwind.
/// This guard is deliberately neither Send nor Sync.
///
/// ```compile_fail
/// fn cannot_transfer(guard: rezie_rt::RealtimeThread) {
///     std::thread::spawn(move || drop(guard));
/// }
/// ```
pub struct RealtimeThread {
    native: platform::Guard,
    report: SchedulingReport,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl RealtimeThread {
    /// Configure the current thread. Policy denial is reported, not hidden.
    /// Failure to establish resources needed for waiting is returned as an error.
    pub fn configure(budget: ThreadBudget) -> io::Result<Self> {
        if budget.period > Duration::from_secs(1)
            || budget.computation.is_zero()
            || budget.computation >= budget.constraint
            || budget.constraint > budget.period
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "realtime budget requires 0 < computation < constraint <= period <= 1 second",
            ));
        }
        let mut report = SchedulingReport::new(budget);
        let native = platform::Guard::configure(budget, &mut report)?;
        Ok(Self {
            native,
            report,
            _thread_affinity: PhantomData,
        })
    }

    /// What the OS actually accepted; the caller may log this before starting work.
    pub fn report(&self) -> SchedulingReport {
        self.report
    }

    /// Wait to an absolute caller-owned deadline with a bounded finishing spin.
    /// Returns false when cancellation was observed. Does not allocate or log.
    pub fn wait_until(&mut self, deadline: Instant, cancelled: &AtomicBool) -> io::Result<bool> {
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Ok(false);
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(true);
            }
            let remaining = deadline - now;
            let slack = Duration::from_nanos(self.report.finishing_slack_ns);
            if remaining > slack {
                self.native
                    .sleep((remaining - slack).min(Duration::from_millis(20)))?;
            } else {
                std::hint::spin_loop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadlines_and_cancellation_have_no_shared_runner_latency_assertions() {
        let mut guard = RealtimeThread::configure(ThreadBudget {
            period: Duration::from_millis(20),
            computation: Duration::from_millis(2),
            constraint: Duration::from_millis(3),
        })
        .unwrap();
        let cancellation = AtomicBool::new(false);
        let deadline = Instant::now() + Duration::from_millis(5);
        assert!(guard.wait_until(deadline, &cancellation).unwrap());
        assert!(Instant::now() >= deadline);
        cancellation.store(true, Ordering::Release);
        assert!(!guard
            .wait_until(Instant::now() + Duration::from_secs(1), &cancellation)
            .unwrap());
    }
}
