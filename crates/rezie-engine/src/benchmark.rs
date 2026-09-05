//! Separate deterministic tick correctness from idle-machine scheduling acceptance.
use crate::{Engine, EngineConfig};
use rezie_core::{ClockStats, FrameRate, OutputId, SinkStats};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Hosted correctness must not assert shared-runner scheduling latency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementMode {
    /// Check tick counts, ordering, PTS and sink isolation only.
    Correctness,
    /// Additionally enforce the human-approved idle latency bounds.
    IdleLatency,
    /// Sweep diagnostics: full lateness and spin CPU cost, with no acceptance claim.
    Calibration,
}

/// All per-tick samples and nearest-rank percentile observations, in nanoseconds.
#[derive(Debug, Serialize, Deserialize)]
pub struct LatenessDistribution {
    /// One value per tick, in index order; no histogram truncation or sample loss.
    pub samples_ns: Vec<u64>,
    /// Nearest-rank median.
    pub p50_ns: u64,
    /// Nearest-rank 99th percentile.
    pub p99_ns: u64,
    /// Nearest-rank 99.9th percentile.
    pub p99_9_ns: u64,
    /// Largest sample over the entire run.
    pub max_ns: u64,
}

impl LatenessDistribution {
    fn from_samples(samples_ns: Vec<u64>) -> anyhow::Result<Self> {
        anyhow::ensure!(!samples_ns.is_empty(), "clock produced no lateness samples");
        let mut sorted = samples_ns.clone();
        sorted.sort_unstable();
        let rank = |permille: usize| sorted[(sorted.len() * permille).div_ceil(1000) - 1];
        Ok(Self {
            p50_ns: rank(500),
            p99_ns: rank(990),
            p99_9_ns: rank(999),
            max_ns: rank(1000),
            samples_ns,
        })
    }

    fn meets_latency_bounds(&self, frame_interval: Duration, final_lateness_ns: u64) -> bool {
        u128::from(final_lateness_ns) < frame_interval.as_nanos()
            && u128::from(self.max_ns) < frame_interval.as_nanos()
            && self.p99_9_ns < 5_000_000
    }
}

/// Raw evidence and independently evaluated correctness/latency outcomes.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClockReport {
    /// Target operating system.
    pub os: String,
    /// Target architecture.
    pub architecture: String,
    /// Deliberately selected evaluation mode.
    pub mode: MeasurementMode,
    /// Seconds of programme time measured.
    pub duration_seconds: u64,
    /// Expected count includes index zero.
    pub expected_ticks: u64,
    /// Observed consumer count.
    pub received_ticks: u64,
    /// Missing, duplicated or out-of-order tick observations.
    pub index_errors: u64,
    /// Ticks whose PTS disagreed with their exact rational deadline.
    pub pts_errors: u64,
    /// Actual clock telemetry.
    pub clock: ClockStats,
    /// Native policy and timer/nice results reported by the measured thread.
    pub scheduling: rezie_rt::SchedulingReport,
    /// Requested override; None means the compiled platform default was used.
    pub slack_override_us: Option<u64>,
    /// Calibration-only CPU profiling; absent in normal correctness/acceptance runs.
    pub wait_profile: Option<rezie_rt::WaitProfile>,
    /// Every sample and its percentile summary.
    pub lateness: LatenessDistribution,
    /// Independent sink counters after shutdown.
    pub sinks: Vec<SinkStats>,
    /// All correctness requirements passed.
    pub correctness_passed: bool,
    /// None in hosted correctness mode; latency is never silently asserted there.
    pub latency_passed: Option<bool>,
    /// Complete result for the requested mode; pilot durations are not ten-minute acceptance.
    pub passed: bool,
}

/// Run the actual engine; use IdleLatency only on an otherwise idle local/reference machine.
pub fn run(seconds: u64, mode: MeasurementMode) -> anyhow::Result<ClockReport> {
    run_with_slack(seconds, mode, None)
}

/// Measure an explicit calibration/acceptance slack without changing the compiled default.
pub fn run_with_slack(
    seconds: u64,
    mode: MeasurementMode,
    slack_us: Option<u64>,
) -> anyhow::Result<ClockReport> {
    anyhow::ensure!(
        slack_us.is_none_or(|s| (0..=5000).contains(&s)),
        "slack override must be 0–5000 microseconds"
    );
    anyhow::ensure!(
        (1..=86_400).contains(&seconds),
        "clock measurement must last 1–86400 seconds"
    );
    let rate = FrameRate::default();
    let expected = seconds * 50 + 1;
    // The correctness consumer can retain the entire finite run if a noisy CI
    // runner deschedules the harness. This prevents a latency gate by accident.
    let (mut engine, mut sinks) = Engine::start(EngineConfig {
        rate,
        sinks: vec![(OutputId(0), expected as usize), (OutputId(1), 2)],
        frame_count: Some(expected),
        clock_slack: slack_us.map(Duration::from_micros),
        profile_clock: mode == MeasurementMode::Calibration,
    })?;
    let start = Instant::now();
    let mut received = 0_u64;
    let mut index_errors = 0_u64;
    let mut pts_errors = 0_u64;
    loop {
        let finished = engine.clock_finished();
        while let Some(frame) = sinks[0].pop() {
            index_errors += u64::from(frame.index != received);
            pts_errors += u64::from(frame.pts != rate.pts(frame.index)?);
            received += 1;
        }
        if finished {
            break;
        }
        anyhow::ensure!(
            start.elapsed() < Duration::from_secs(seconds + 120),
            "clock run exceeded its correctness liveness deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    engine.shutdown()?;
    let clock = engine.clock_stats();
    let lateness = LatenessDistribution::from_samples(engine.lateness_samples()?)?;
    let stats: Vec<_> = sinks.iter().map(|s| s.stats()).collect();
    let correctness_passed = index_errors == 0
        && pts_errors == 0
        && !engine.clock_failed()
        && received == expected
        && clock.emitted == expected
        && lateness.samples_ns.len() == expected as usize
        && stats[0].dropped == 0
        && stats[1].dropped == expected - 2
        && lateness.max_ns == clock.max_lateness_ns
        && clock.last_frame.is_some_and(|frame| {
            frame.index == expected - 1 && frame.pts == Duration::from_secs(seconds)
        });
    let scheduling = engine.scheduling_report();
    let wait_profile = engine.wait_profile();
    let correctness_passed =
        correctness_passed && (mode != MeasurementMode::Calibration || wait_profile.is_some());
    let frame_interval = rate.pts(1)?;
    let latency_passed = (mode == MeasurementMode::IdleLatency).then(|| {
        scheduling.correctly_prioritized()
            && lateness.meets_latency_bounds(frame_interval, clock.final_lateness_ns)
    });
    Ok(ClockReport {
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        mode,
        duration_seconds: seconds,
        expected_ticks: expected,
        received_ticks: received,
        index_errors,
        pts_errors,
        clock,
        scheduling,
        slack_override_us: slack_us,
        wait_profile,
        lateness,
        sinks: stats,
        correctness_passed,
        passed: correctness_passed && latency_passed.unwrap_or(true),
        latency_passed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_distribution_uses_nearest_rank_and_keeps_tick_order() {
        let samples: Vec<u64> = (1..=1000).rev().collect();
        let d = LatenessDistribution::from_samples(samples.clone()).unwrap();
        assert_eq!(d.samples_ns, samples);
        assert_eq!(
            (d.p50_ns, d.p99_ns, d.p99_9_ns, d.max_ns),
            (500, 990, 999, 1000)
        );
    }
    #[test]
    fn one_earlier_stall_fails_even_when_final_tick_is_on_time() {
        let mut samples = vec![1000; 30_001];
        samples[2] = 139_018_291;
        let d = LatenessDistribution::from_samples(samples).unwrap();
        assert!(!d.meets_latency_bounds(Duration::from_millis(20), 0));
        let d = LatenessDistribution::from_samples(vec![5_000_000; 1000]).unwrap();
        assert!(!d.meets_latency_bounds(Duration::from_millis(20), 0));
        let d = LatenessDistribution::from_samples(vec![4_999_999; 1000]).unwrap();
        assert!(d.meets_latency_bounds(Duration::from_millis(20), 0));
        assert!(!d.meets_latency_bounds(Duration::from_millis(20), 20_000_000));
    }
}
