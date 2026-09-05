//! Measured clock/dispatch acceptance shared by headless CLI, xtask and integration tests.
use crate::{Engine, EngineConfig};
use rezie_core::{ClockStats, FrameRate, OutputId, SinkStats};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Machine-labelled observations; fps is diagnostic, drift is normative.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClockReport {
    /// Target platform OS.
    pub os: String,
    /// Target architecture.
    pub architecture: String,
    /// Requested programme duration in seconds.
    pub duration_seconds: u64,
    /// Expected and verified tick count, including index zero.
    pub expected_ticks: u64,
    /// Received ticks with contiguous indices and exact PTS.
    pub received_ticks: u64,
    /// Actual clock observations.
    pub clock: ClockStats,
    /// Independently observed sink counters.
    pub sinks: Vec<SinkStats>,
    /// Diagnostic throughput; this is not an alternative drift pass condition.
    pub observed_fps: f64,
    /// Strict final drift bound and complete dispatch check.
    pub passed: bool,
}

/// Run an actual 50 fps engine with one draining and one deliberately stalled sink.
pub fn run(seconds: u64) -> anyhow::Result<ClockReport> {
    anyhow::ensure!(
        (1..=86_400).contains(&seconds),
        "clock measurement must last 1–86400 seconds"
    );
    let rate = FrameRate::default();
    let expected = seconds * 50 + 1;
    let (mut engine, mut sinks) = Engine::start(EngineConfig {
        rate,
        sinks: vec![(OutputId(0), 256), (OutputId(1), 2)],
        frame_count: Some(expected),
    })?;
    let start = Instant::now();
    let mut received = 0_u64;
    let mut valid = true;
    loop {
        while let Some(frame) = sinks[0].pop() {
            valid &= frame.index == received && frame.pts == rate.pts(frame.index)?;
            received += 1;
        }
        if engine.clock_finished() {
            // done is release-published after dispatch; drain any final tick missed by the first pop.
            while let Some(frame) = sinks[0].pop() {
                valid &= frame.index == received && frame.pts == rate.pts(frame.index)?;
                received += 1;
            }
            break;
        }
        anyhow::ensure!(
            start.elapsed() < Duration::from_secs(seconds + 30),
            "clock run exceeded its wall-time deadline"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    engine.shutdown()?;
    let clock = engine.clock_stats();
    let stats: Vec<_> = sinks.iter().map(|s| s.stats()).collect();
    let passed = valid
        && !engine.clock_failed()
        && received == expected
        && clock.emitted == expected
        && clock.final_lateness_ns < 20_000_000
        && stats[0].dropped == 0
        && stats[1].dropped == expected - 2
        && clock.last_frame.is_some_and(|frame| {
            frame.index == expected - 1 && frame.pts == Duration::from_secs(seconds)
        });
    let observed_fps =
        (expected - 1) as f64 / (seconds as f64 + clock.final_lateness_ns as f64 / 1e9);
    Ok(ClockReport {
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        duration_seconds: seconds,
        expected_ticks: expected,
        received_ticks: received,
        clock,
        sinks: stats,
        observed_fps,
        passed,
    })
}
