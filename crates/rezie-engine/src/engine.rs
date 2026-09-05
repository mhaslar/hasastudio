use crate::{tick_sink, TickConsumer, TickProducer};
use arc_swap::ArcSwap;
use rezie_api::{Client, Command, Envelope, Event};
use rezie_core::{ClockStats, EngineState, FrameRate, FrameTime, OutputId, Project};
use rezie_rt::{RealtimeThread, SchedulingReport, ThreadBudget};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Foundation startup settings; no runtime output subsystem is enabled.
pub struct EngineConfig {
    /// Programme rate.
    pub rate: FrameRate,
    /// Independent tick queues, identified by sink ID and capacity.
    pub sinks: Vec<(OutputId, usize)>,
    /// Optional finite run, including tick zero; used by acceptance tests.
    pub frame_count: Option<u64>,
    /// Exact diagnostic slack override; None uses the platform default.
    pub clock_slack: Option<Duration>,
    /// Measure finishing-spin CPU cost; diagnostic runs only.
    pub profile_clock: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            rate: FrameRate::default(),
            sinks: vec![(OutputId(0), 8)],
            frame_count: None,
            clock_slack: None,
            profile_clock: false,
        }
    }
}

/// Actionable startup or shutdown failure.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Invalid configuration.
    #[error("invalid engine configuration: {0}")]
    Configuration(String),
    /// OS thread startup failed.
    #[error("failed to start engine thread: {0}")]
    Thread(#[from] std::io::Error),
    /// Control transport setup failed.
    #[error(transparent)]
    Api(#[from] rezie_api::ApiError),
    /// A thread panicked; all other threads are still joined.
    #[error("engine thread panicked during shutdown")]
    Panic,
}

#[derive(Default)]
struct Telemetry {
    samples: Box<[AtomicU64]>,
    sequence: AtomicU64,
    emitted: AtomicU64,
    max_lateness: AtomicU64,
    lateness: AtomicU64,
    misses: AtomicU64,
    done: AtomicBool,
    failed: AtomicBool,
    spin_cpu: AtomicU64,
    spin_wall: AtomicU64,
    spin_entries: AtomicU64,
    thread_cpu: AtomicU64,
    thread_wall: AtomicU64,
    profiled: AtomicBool,
}

struct ClockExit<'a>(&'a Telemetry);

impl Drop for ClockExit<'_> {
    fn drop(&mut self) {
        // Also unblock control shutdown if an upstream dependency unexpectedly unwinds.
        if !self.0.done.load(Ordering::Acquire) {
            self.0.failed.store(true, Ordering::Release);
            self.0.done.store(true, Ordering::Release);
        }
    }
}

impl Telemetry {
    fn record(&self, lateness: u64, period: u64) {
        // Sequential consistency makes the read-side sequence check a coherent snapshot.
        self.sequence.fetch_add(1, Ordering::SeqCst);
        let index = self.emitted.fetch_add(1, Ordering::SeqCst);
        if let Some(sample) = self.samples.get(index as usize) {
            sample.store(lateness, Ordering::Relaxed);
        }
        self.max_lateness.fetch_max(lateness, Ordering::SeqCst);
        self.lateness.store(lateness, Ordering::SeqCst);
        if lateness >= period {
            self.misses.fetch_add(1, Ordering::SeqCst);
        }
        self.sequence.fetch_add(1, Ordering::SeqCst);
    }

    fn snapshot(&self, rate: FrameRate) -> ClockStats {
        loop {
            let before = self.sequence.load(Ordering::SeqCst);
            if !before.is_multiple_of(2) {
                std::hint::spin_loop();
                continue;
            }
            let emitted = self.emitted.load(Ordering::SeqCst);
            let stats = ClockStats {
                last_frame: emitted
                    .checked_sub(1)
                    .and_then(|index| rate.pts(index).ok().map(|pts| FrameTime { index, pts })),
                emitted,
                max_lateness_ns: self.max_lateness.load(Ordering::SeqCst),
                final_lateness_ns: self.lateness.load(Ordering::SeqCst),
                deadline_misses: self.misses.load(Ordering::SeqCst),
            };
            if before == self.sequence.load(Ordering::SeqCst) {
                return stats;
            }
        }
    }
}

/// Owns all engine threads and joins them on explicit shutdown or drop.
pub struct Engine {
    client: Client,
    stop: Arc<AtomicBool>,
    telemetry: Arc<Telemetry>,
    rate: FrameRate,
    scheduling: SchedulingReport,
    clock: Option<JoinHandle<()>>,
    control: Option<JoinHandle<()>>,
}

impl Engine {
    /// Allocate queues and state, then start dedicated control and clock threads.
    pub fn start(config: EngineConfig) -> Result<(Self, Vec<TickConsumer>), EngineError> {
        if config.frame_count == Some(0) || config.sinks.is_empty() || config.sinks.len() > 8 {
            return Err(EngineError::Configuration(
                "require 1–8 tick sinks and a positive frame count".into(),
            ));
        }
        if config.frame_count.is_some_and(|count| count > 5_000_000) {
            return Err(EngineError::Configuration(
                "finite clock runs support at most 5,000,000 preallocated samples".into(),
            ));
        }
        FrameRate::new(config.rate.numerator(), config.rate.denominator())
            .map_err(|e| EngineError::Configuration(e.to_string()))?;
        let period = config
            .rate
            .pts(1)
            .map_err(|e| EngineError::Configuration(e.to_string()))?;
        let slack = config
            .clock_slack
            .unwrap_or_else(|| rezie_rt::FINISHING_SLACK.min(period * 3 / 16));
        if slack >= period {
            return Err(EngineError::Configuration(
                "clock slack must be below the programme period".into(),
            ));
        }
        // Retain the proven 2 ms minimum for low-slack trials: a 0.5 ms Mach
        // budget did not read back as requested in the functional sweep (ADR 0021).
        let computation = (slack + Duration::from_micros(500))
            .max(Duration::from_millis(2))
            .min(period / 3);
        let budget = ThreadBudget {
            period,
            computation,
            constraint: (computation + Duration::from_millis(1)).min(period / 2),
        };
        if slack >= computation {
            return Err(EngineError::Configuration(format!(
                "clock slack {slack:?} exceeds the {period:?} period's CPU budget"
            )));
        }
        for (i, (id, capacity)) in config.sinks.iter().enumerate() {
            if *capacity > 1_000_000 || config.sinks[..i].iter().any(|(other, _)| other == id) {
                return Err(EngineError::Configuration(format!(
                    "sink {} has excessive capacity or a duplicate ID",
                    id.0
                )));
            }
        }
        let mut producers = Vec::new();
        let mut consumers = Vec::new();
        let mut observers = Vec::new();
        for (id, capacity) in config.sinks {
            let (producer, consumer) = tick_sink(id, capacity)?;
            observers.push(producer.observer());
            producers.push(producer);
            consumers.push(consumer);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let telemetry = Arc::new(Telemetry {
            samples: (0..config.frame_count.unwrap_or(0))
                .map(|_| AtomicU64::new(0))
                .collect(),
            ..Telemetry::default()
        });
        let mut project = Project::default();
        project.settings.format.fps = config.rate;
        let state = EngineState {
            project,
            revision: 0,
            running: true,
            clock: telemetry.snapshot(config.rate),
            sinks: observers.iter().map(TickConsumer::stats).collect(),
        };
        let snapshots = Arc::new(ArcSwap::from_pointee(state.clone()));
        let (client, commands) = rezie_api::channel(snapshots.clone(), 64)?;
        let control_stop = stop.clone();
        let control_telemetry = telemetry.clone();
        let rate = config.rate;
        let control = thread::Builder::new()
            .name("rezie-control".into())
            .spawn(move || {
                let _span = tracing::info_span!("engine_control").entered();
                control_loop(
                    state,
                    snapshots,
                    commands,
                    control_stop,
                    control_telemetry,
                    observers,
                    rate,
                );
            })?;
        let clock_stop = stop.clone();
        let clock_telemetry = telemetry.clone();
        let (configured, configuration) = crossbeam_channel::bounded(1);
        // Create the span on the startup thread. The real-time loop never logs or enters spans.
        let clock_span = tracing::info_span!(
            "programme_clock",
            numerator = rate.numerator(),
            denominator = rate.denominator()
        );
        let clock = match thread::Builder::new()
            .name("rezie-clock".into())
            .spawn(move || {
                let _completion = ClockExit(&clock_telemetry);
                let _span = clock_span.entered();
                let mut realtime =
                    match RealtimeThread::configure_wait(budget, slack, config.profile_clock) {
                        Ok(realtime) => realtime,
                        Err(error) => {
                            let _ = configured.try_send(Err(error));
                            return;
                        }
                    };
                let _ = configured.try_send(Ok(realtime.report()));
                clock_loop(
                    rate,
                    config.frame_count,
                    &clock_stop,
                    &clock_telemetry,
                    &mut producers,
                    &mut realtime,
                );
            }) {
            Ok(clock) => clock,
            Err(error) => {
                telemetry.done.store(true, Ordering::Release);
                stop.store(true, Ordering::Release);
                let _ = control.join();
                return Err(error.into());
            }
        };
        let scheduling = match configuration.recv() {
            Ok(Ok(report)) => report,
            result => {
                stop.store(true, Ordering::Release);
                let _ = clock.join();
                let _ = control.join();
                return Err(EngineError::Configuration(format!(
                    "realtime thread initialization failed: {result:?}"
                )));
            }
        };
        if !scheduling.realtime {
            tracing::warn!(?scheduling, "RT scheduling unavailable; inspect effective timer/nice fallback and OS error codes");
        }
        Ok((
            Self {
                client,
                stop,
                telemetry,
                rate,
                scheduling,
                clock: Some(clock),
                control: Some(control),
            },
            consumers,
        ))
    }

    /// Connect a GUI or harness to the same authoritative command bus.
    pub fn client(&self) -> Client {
        self.client.clone()
    }
    /// Whether the finite clock run ended or shutdown was requested.
    pub fn clock_finished(&self) -> bool {
        self.telemetry.done.load(Ordering::Acquire)
    }
    /// Whether timestamp arithmetic failed, rather than a clean finite completion.
    pub fn clock_failed(&self) -> bool {
        self.telemetry.failed.load(Ordering::Acquire)
    }
    /// Observe the real clock counters directly, without snapshot publication delay.
    pub fn clock_stats(&self) -> ClockStats {
        self.telemetry.snapshot(self.rate)
    }

    /// Completed diagnostic CPU accounting, never a wall-time estimate of CPU cost.
    pub fn wait_profile(&self) -> Option<rezie_rt::WaitProfile> {
        if !self.clock_finished() || !self.telemetry.profiled.load(Ordering::Acquire) {
            return None;
        }
        Some(rezie_rt::WaitProfile {
            spin_cpu_ns: self.telemetry.spin_cpu.load(Ordering::Relaxed),
            spin_wall_ns: self.telemetry.spin_wall.load(Ordering::Relaxed),
            spin_entries: self.telemetry.spin_entries.load(Ordering::Relaxed),
            thread_cpu_ns: self.telemetry.thread_cpu.load(Ordering::Relaxed),
            thread_wall_ns: self.telemetry.thread_wall.load(Ordering::Relaxed),
        })
    }
    /// Native scheduling achieved on the clock thread, reported before its first tick.
    pub fn scheduling_report(&self) -> SchedulingReport {
        self.scheduling
    }
    /// Copy every measured tick after a finite run ends, in tick-index order.
    /// Infinite production runs do not allocate a sample buffer.
    pub fn lateness_samples(&self) -> Result<Vec<u64>, EngineError> {
        if !self.clock_finished() {
            return Err(EngineError::Configuration(
                "lateness samples are available only after the clock ends".into(),
            ));
        }
        let count = self.telemetry.emitted.load(Ordering::Acquire) as usize;
        Ok(self
            .telemetry
            .samples
            .iter()
            .take(count)
            .map(|s| s.load(Ordering::Relaxed))
            .collect())
    }
    /// Request termination and join both OS threads.
    pub fn shutdown(&mut self) -> Result<(), EngineError> {
        self.stop.store(true, Ordering::Release);
        let mut panicked = false;
        if let Some(clock) = self.clock.take() {
            panicked |= clock.join().is_err();
        }
        if let Some(control) = self.control.take() {
            panicked |= control.join().is_err();
        }
        if panicked {
            Err(EngineError::Panic)
        } else {
            Ok(())
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn clock_loop(
    rate: FrameRate,
    count: Option<u64>,
    stop: &AtomicBool,
    telemetry: &Telemetry,
    sinks: &mut [TickProducer],
    realtime: &mut RealtimeThread,
) {
    let origin = Instant::now();
    let mut index = 0_u64;
    let period = rate
        .pts(1)
        .map(|d| d.as_nanos().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(u64::MAX);
    while !stop.load(Ordering::Acquire) && count.is_none_or(|limit| index < limit) {
        let Ok(pts) = rate.pts(index) else {
            telemetry.failed.store(true, Ordering::Release);
            break;
        };
        let Some(deadline) = origin.checked_add(pts) else {
            telemetry.failed.store(true, Ordering::Release);
            break;
        };
        match realtime.wait_until(deadline, stop) {
            Ok(true) => {}
            Ok(false) => break,
            Err(_) => {
                telemetry.failed.store(true, Ordering::Release);
                break;
            }
        }
        let frame = FrameTime { index, pts };
        for sink in sinks.iter_mut() {
            sink.dispatch(frame);
        }
        let lateness = Instant::now()
            .saturating_duration_since(deadline)
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        telemetry.record(lateness, period);
        let Some(next) = index.checked_add(1) else {
            telemetry.failed.store(true, Ordering::Release);
            break;
        };
        index = next;
    }
    match realtime.finish_profile() {
        Ok(Some(profile)) => {
            telemetry
                .spin_cpu
                .store(profile.spin_cpu_ns, Ordering::Relaxed);
            telemetry
                .spin_wall
                .store(profile.spin_wall_ns, Ordering::Relaxed);
            telemetry
                .spin_entries
                .store(profile.spin_entries, Ordering::Relaxed);
            telemetry
                .thread_cpu
                .store(profile.thread_cpu_ns, Ordering::Relaxed);
            telemetry
                .thread_wall
                .store(profile.thread_wall_ns, Ordering::Relaxed);
            telemetry.profiled.store(true, Ordering::Release);
        }
        Ok(None) => {}
        Err(_) => {
            telemetry.failed.store(true, Ordering::Release);
        }
    }
    telemetry.done.store(true, Ordering::Release);
}

fn control_loop(
    mut state: EngineState,
    snapshots: Arc<ArcSwap<EngineState>>,
    commands: crossbeam_channel::Receiver<Envelope>,
    stop: Arc<AtomicBool>,
    telemetry: Arc<Telemetry>,
    sinks: Vec<TickConsumer>,
    rate: FrameRate,
) {
    while !stop.load(Ordering::Acquire) {
        if let Ok(envelope) = commands.recv_timeout(Duration::from_millis(10)) {
            let id = envelope.request.id;
            let rejection = match envelope.request.command {
                Command::GetState => None,
                Command::SetProjectName { ref name } => {
                    if name.trim().is_empty()
                        || name.len() > 128
                        || name.chars().any(char::is_control)
                    {
                        Some("project name must be nonempty, at most 128 UTF-8 bytes and contain no control characters".to_owned())
                    } else {
                        state.project.name.clone_from(name);
                        state.revision += 1;
                        None
                    }
                }
                Command::Shutdown => {
                    state.running = false;
                    state.revision += 1;
                    stop.store(true, Ordering::Release);
                    None
                }
            };
            state.clock = telemetry.snapshot(rate);
            state.sinks = sinks.iter().map(TickConsumer::stats).collect();
            let snapshot = Arc::new(state.clone());
            snapshots.store(snapshot.clone());
            let event = if let Some(reason) = rejection {
                Event::Rejected {
                    id: Some(id),
                    reason,
                }
            } else if matches!(envelope.request.command, Command::GetState) {
                Event::State {
                    id,
                    state: snapshot,
                }
            } else {
                Event::Applied {
                    id,
                    state: snapshot,
                }
            };
            let _ = envelope.reply.try_send(event);
        }
        state.clock = telemetry.snapshot(rate);
        state.sinks = sinks.iter().map(TickConsumer::stats).collect();
        snapshots.store(Arc::new(state.clone()));
    }
    // Stop requested: wait outside the clock thread so the final snapshot contains final counters.
    while !telemetry.done.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(1));
    }
    state.running = false;
    state.clock = telemetry.snapshot(rate);
    state.sinks = sinks.iter().map(TickConsumer::stats).collect();
    snapshots.store(Arc::new(state));
}
