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
mod cpu_time;

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

/// M4: 500 µs used 2.338% of one core for spin with p99.9 19.291 µs;
/// larger values gave no clear latency benefit in the six-value sweep (ADRs 0021–0022).
#[cfg(target_os = "macos")]
pub const FINISHING_SLACK: Option<Duration> = Some(Duration::from_micros(500));
/// RX 6800 XT / i5-14600K: 1 ms is the smallest tested low-tail slack;
/// v2 ten-minute max/p99.9 were 128.5/25.5 µs (ADRs 0021, 0028).
#[cfg(target_os = "windows")]
pub const FINISHING_SLACK: Option<Duration> = Some(Duration::from_micros(1000));
/// Linux has correctness coverage but no calibrated operating value.
#[cfg(target_os = "linux")]
pub const FINISHING_SLACK: Option<Duration> = None;

/// Return this platform's measured default, or explain the missing calibration.
pub fn calibrated_slack() -> io::Result<Duration> {
    FINISHING_SLACK.ok_or_else(|| io::Error::new(
        io::ErrorKind::NotFound,
        format!("no calibrated realtime slack for {}; run cargo xtask clock-sweep and record the reviewed platform value (ADR 0022); diagnostics require an explicit slack override", std::env::consts::OS),
    ))
}

/// Optional OS CPU accounting, subject to the platform counter's resolution.
/// Windows GetThreadTimes can quantize short segments to zero; its 100 ns
/// units do not imply 100 ns accuracy. Do not equate these CPU columns with
/// Unix thread-clock accuracy or substitute spin wall time (ADR 0027).
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct WaitProfile {
    /// Sum of OS CPU-counter deltas around finishing-spin segments, including query overhead.
    /// Zero may mean unresolved accounting, not absence of spinning.
    pub spin_cpu_ns: u64,
    /// Wall nanoseconds in finishing-spin segments; may include descheduling.
    pub spin_wall_ns: u64,
    /// Number of measured finishing-spin segments.
    pub spin_entries: u64,
    /// Total thread CPU time since profiling started, including dispatch and instrumentation.
    pub thread_cpu_ns: u64,
    /// Wall duration of the whole profiled interval, denominator for one-core CPU percentages.
    pub thread_wall_ns: u64,
}

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
    fn new(budget: ThreadBudget, slack: Duration) -> Self {
        Self {
            policy: SchedulingPolicy::Unavailable,
            realtime: false,
            realtime_error: None,
            fifo_priority: None,
            nice: None,
            nice_error: None,
            timer_resolution_ms: None,
            timer_error: None,
            finishing_slack_ns: slack.as_nanos() as u64,
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
    profile: Option<(u64, Instant, WaitProfile)>,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl RealtimeThread {
    /// Configure the current thread. Policy denial is reported, not hidden.
    /// Failure to establish resources needed for waiting is returned as an error.
    pub fn configure(budget: ThreadBudget) -> io::Result<Self> {
        Self::configure_wait(
            budget,
            calibrated_slack()?.min(budget.computation * 3 / 4),
            false,
        )
    }

    /// Configure an exact finishing slack and optional CPU profiling for calibration.
    /// Profiling adds CPU-clock queries around spin segments; normal callers leave it disabled.
    pub fn configure_wait(
        budget: ThreadBudget,
        slack: Duration,
        profile: bool,
    ) -> io::Result<Self> {
        if budget.period > Duration::from_secs(1)
            || budget.computation.is_zero()
            || budget.computation >= budget.constraint
            || budget.constraint > budget.period
            || slack >= budget.computation
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "realtime budget requires slack < computation < constraint <= period <= 1 second and computation > 0",
            ));
        }
        let mut report = SchedulingReport::new(budget, slack);
        let native = platform::Guard::configure(budget, &mut report)?;
        let profile = if profile {
            Some((cpu_time::current()?, Instant::now(), WaitProfile::default()))
        } else {
            None
        };
        Ok(Self {
            native,
            report,
            profile,
            _thread_affinity: PhantomData,
        })
    }

    /// What the OS actually accepted; the caller may log this before starting work.
    pub fn report(&self) -> SchedulingReport {
        self.report
    }

    /// Finish diagnostic accounting on the owning thread after its work stops.
    pub fn finish_profile(&self) -> io::Result<Option<WaitProfile>> {
        self.profile
            .map(|(start, wall_start, mut value)| {
                value.thread_cpu_ns = cpu_time::current()?.saturating_sub(start);
                value.thread_wall_ns = wall_start.elapsed().as_nanos() as u64;
                Ok(value)
            })
            .transpose()
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
            } else if self.profile.is_some() {
                let cpu_start = cpu_time::current()?;
                let wall_start = Instant::now();
                let reached = loop {
                    if cancelled.load(Ordering::Acquire) {
                        break false;
                    }
                    if Instant::now() >= deadline {
                        break true;
                    }
                    std::hint::spin_loop();
                };
                let wall = wall_start.elapsed().as_nanos() as u64;
                let cpu = cpu_time::current()?.saturating_sub(cpu_start);
                if let Some((_, _, profile)) = &mut self.profile {
                    profile.spin_cpu_ns = profile.spin_cpu_ns.saturating_add(cpu);
                    profile.spin_wall_ns = profile.spin_wall_ns.saturating_add(wall);
                    profile.spin_entries += 1;
                }
                return Ok(reached);
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
        let mut guard = RealtimeThread::configure_wait(
            ThreadBudget {
                period: Duration::from_millis(20),
                computation: Duration::from_millis(2),
                constraint: Duration::from_millis(3),
            },
            Duration::ZERO,
            false,
        )
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
