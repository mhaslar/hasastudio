use crate::{tick_sink, TickConsumer, TickProducer};
use arc_swap::ArcSwap;
use rezie_api::{Client, Command, Envelope, Event};
use rezie_core::{ClockStats, EngineState, FrameRate, FrameTime, OutputId, Project};
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
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            rate: FrameRate::default(),
            sinks: vec![(OutputId(0), 8)],
            frame_count: None,
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
    sequence: AtomicU64,
    emitted: AtomicU64,
    max_lateness: AtomicU64,
    lateness: AtomicU64,
    misses: AtomicU64,
    done: AtomicBool,
    failed: AtomicBool,
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
        self.emitted.fetch_add(1, Ordering::SeqCst);
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
            if before % 2 != 0 {
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
        FrameRate::new(config.rate.numerator(), config.rate.denominator())
            .map_err(|e| EngineError::Configuration(e.to_string()))?;
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
        let telemetry = Arc::new(Telemetry::default());
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
                clock_loop(
                    rate,
                    config.frame_count,
                    &clock_stop,
                    &clock_telemetry,
                    &mut producers,
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
        Ok((
            Self {
                client,
                stop,
                telemetry,
                rate,
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
        loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let remaining = deadline - now;
            if remaining > Duration::from_micros(300) {
                thread::sleep(
                    (remaining - Duration::from_micros(200)).min(Duration::from_millis(10)),
                );
            } else {
                std::hint::spin_loop();
            }
        }
        if stop.load(Ordering::Acquire) {
            break;
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
