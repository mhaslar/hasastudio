//! End-to-end Foundation behavior through the real engine and both transports.
use futures_util::{SinkExt, StreamExt};
use rezie_api::{Command, Event, Request, WebSocketServer};
use rezie_engine::{Engine, EngineConfig};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn websocket_mutation_rejection_and_in_process_state_agree() {
    let (mut engine, _sinks) = Engine::start(correctness_config()).unwrap();
    let client = engine.client();
    let server = WebSocketServer::bind("127.0.0.1:0".parse().unwrap(), client.clone())
        .await
        .unwrap();
    let (mut socket, _) = connect_async(format!("ws://{}", server.address()))
        .await
        .unwrap();
    let request = Request {
        id: 42,
        command: Command::SetProjectName {
            name: "Evening programme".into(),
        },
    };
    socket
        .send(Message::Text(
            serde_json::to_string(&request).unwrap().into(),
        ))
        .await
        .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let event: Event = serde_json::from_str(reply.to_text().unwrap()).unwrap();
    match event {
        Event::Applied { id, state } => {
            assert_eq!(id, 42);
            assert_eq!(state.project.name, "Evening programme");
            assert_eq!(state.revision, 1);
        }
        other => panic!("unexpected event {other:?}"),
    }
    assert_eq!(client.snapshot().project.name, "Evening programme");

    let rejected = client
        .request_async(Request {
            id: 43,
            command: Command::SetProjectName { name: "\n".into() },
        })
        .await
        .unwrap();
    assert!(matches!(rejected, Event::Rejected { id: Some(43), .. }));
    assert_eq!(client.snapshot().revision, 1);
    assert_eq!(client.snapshot().project.name, "Evening programme");

    socket
        .send(Message::Text("{malformed".into()))
        .await
        .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(
        serde_json::from_str::<Event>(reply.to_text().unwrap()).unwrap(),
        Event::Rejected { id: None, .. }
    ));
    socket
        .send(Message::Text(
            r#"{"id":44,"command":{"type":"GetState"}}"#.into(),
        ))
        .await
        .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(
        serde_json::from_str::<Event>(reply.to_text().unwrap()).unwrap(),
        Event::State { id: 44, .. }
    ));
    server.shutdown().await.unwrap();
    engine.shutdown().unwrap();
    assert!(!client.snapshot().running);
}

#[tokio::test]
async fn listener_rejects_non_loopback_and_shutdown_stops_clock() {
    let (mut engine, _sinks) = Engine::start(correctness_config()).unwrap();
    let client = engine.client();
    assert!(
        WebSocketServer::bind("0.0.0.0:0".parse().unwrap(), client.clone())
            .await
            .is_err()
    );
    let response = client
        .request_async(Request {
            id: 1,
            command: Command::Shutdown,
        })
        .await
        .unwrap();
    assert!(matches!(response, Event::Applied { .. }));
    engine.shutdown().unwrap();
    assert!(engine.clock_finished());
    assert!(!client.snapshot().running);
    assert!(client
        .submit(Request {
            id: 2,
            command: Command::GetState
        })
        .is_err());
}

#[test]
fn clock_runs_and_a_stalled_sink_cannot_interrupt_dispatch() {
    let report = rezie_engine::benchmark::run_with_slack(
        2,
        rezie_engine::benchmark::MeasurementMode::Correctness,
        Some(0),
    )
    .unwrap();
    assert!(report.passed, "{report:#?}");
    assert_eq!(report.received_ticks, 101);
    assert_eq!(report.latency_passed, None);
    assert_eq!(report.sinks[1].dropped, 99);
}

#[test]
#[ignore = "idle local/reference latency gate only; hosted CI runs correctness"]
fn ten_minute_clock_drift_is_strictly_under_one_frame() {
    let report =
        rezie_engine::benchmark::run(600, rezie_engine::benchmark::MeasurementMode::IdleLatency)
            .unwrap();
    assert!(report.passed, "{report:#?}");
    assert_eq!(report.received_ticks, 30_001);
    assert!(report.clock.final_lateness_ns < 20_000_000);
    assert!(report.lateness.max_ns < 20_000_000);
    assert!(report.lateness.p99_9_ns < 5_000_000);
}

#[test]
fn calibration_retains_samples_and_actual_cpu_cost_without_a_latency_gate() {
    let report = rezie_engine::benchmark::run_with_slack(
        1,
        rezie_engine::benchmark::MeasurementMode::Calibration,
        Some(500),
    )
    .unwrap();
    assert!(report.correctness_passed);
    assert_eq!(report.latency_passed, None);
    assert_eq!(report.scheduling.finishing_slack_ns, 500_000);
    assert_eq!(report.lateness.samples_ns.len(), 51);
    let encoded = serde_json::to_value(&report).unwrap();
    let observed = encoded["observed_ticks"].as_array().unwrap();
    assert_eq!(observed.len(), 51);
    for (index, frame) in observed.iter().enumerate() {
        assert_eq!(frame["index"].as_u64().unwrap(), index as u64);
        let pts = frame["pts"]["secs"].as_u64().unwrap() * 1_000_000_000
            + frame["pts"]["nanos"].as_u64().unwrap();
        assert_eq!(pts, index as u64 * 20_000_000);
    }
    // Keep an erroneous observation verbatim: serialization is evidence, not repair.
    let mut unusual = report.observed_ticks[0];
    unusual.index = 7;
    unusual.pts = Duration::from_nanos(123);
    let mut report = report;
    report.observed_ticks[0] = unusual;
    let encoded = serde_json::to_value(&report).unwrap();
    assert_eq!(encoded["observed_ticks"][0]["index"], 7);
    assert_eq!(encoded["observed_ticks"][0]["pts"]["nanos"], 123);
    let cost = report.wait_profile.unwrap();
    assert!(cost.thread_wall_ns > 0);
    assert!(cost.thread_cpu_ns >= cost.spin_cpu_ns);
    // No minimum spin count: a descheduled hosted runner may miss every spin window.
    let invalid = Engine::start(EngineConfig {
        clock_slack: Some(Duration::MAX),
        ..EngineConfig::default()
    });
    assert!(invalid.is_err());
}

struct HeadlessProcess {
    child: std::process::Child,
    directory: std::path::PathBuf,
}

impl Drop for HeadlessProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn headless_binary_serves_websocket_and_flushes_shutdown_reply() {
    let directory = std::env::temp_dir().join(format!("rezie-process-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let ready = directory.join("ready.txt");
    if ready.exists() {
        std::fs::remove_file(&ready).unwrap();
    }
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_rezie-headless"))
        .args(["--ws", "127.0.0.1:0", "--slack-us", "0", "--ready-file"])
        .arg(&ready)
        .current_dir(&directory)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut process = HeadlessProcess { child, directory };
    let address = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(&ready) {
                if let Ok(address) = text.parse::<std::net::SocketAddr>() {
                    break address;
                }
            }
            assert!(
                process.child.try_wait().unwrap().is_none(),
                "headless process exited before binding"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let (mut socket, _) = connect_async(format!("ws://{address}")).await.unwrap();
    for (id, command) in [(1, Command::GetState), (2, Command::Shutdown)] {
        socket
            .send(Message::Text(
                serde_json::to_string(&Request { id, command })
                    .unwrap()
                    .into(),
            ))
            .await
            .unwrap();
        let reply = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let event: Event = serde_json::from_str(reply.to_text().unwrap()).unwrap();
        match event {
            Event::State { id: 1, state } => assert!(state.running),
            Event::Applied { id: 2, state } => assert!(!state.running),
            other => panic!("unexpected subprocess reply {other:?}"),
        }
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(status) = process.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn bounded_command_bus_reports_backpressure_without_losing_accepted_work() {
    let (mut engine, _) = Engine::start(correctness_config()).unwrap();
    let state = std::sync::Arc::new(arc_swap::ArcSwap::from(engine.client().snapshot()));
    let (client, receiver) = rezie_api::channel(state, 1).unwrap();
    let first = client
        .submit(Request {
            id: 1,
            command: Command::GetState,
        })
        .unwrap();
    assert!(matches!(
        client.submit(Request {
            id: 2,
            command: Command::GetState
        }),
        Err(rezie_api::ApiError::Busy)
    ));
    let accepted = receiver.try_recv().unwrap();
    assert_eq!(accepted.request.id, 1);
    accepted
        .reply
        .try_send(Event::State {
            id: 1,
            state: client.snapshot(),
        })
        .unwrap();
    assert!(matches!(
        first.recv_timeout(Duration::from_secs(1)).unwrap(),
        Event::State { id: 1, .. }
    ));
    drop(receiver);
    assert!(matches!(
        client.submit(Request {
            id: 3,
            command: Command::GetState
        }),
        Err(rezie_api::ApiError::Closed)
    ));
    engine.shutdown().unwrap();
}

// Correctness fixtures select sleep-only explicitly; they do not calibrate a platform.
fn correctness_config() -> EngineConfig {
    EngineConfig {
        clock_slack: Some(Duration::ZERO),
        ..EngineConfig::default()
    }
}

#[test]
fn normal_start_requires_a_calibrated_platform_value() {
    let started = Engine::start(EngineConfig::default());
    if cfg!(target_os = "macos") {
        let (mut engine, _) = started.unwrap();
        assert_eq!(engine.scheduling_report().finishing_slack_ns, 500_000);
        engine.shutdown().unwrap();
    } else {
        let error = started.err().expect("uncalibrated startup must fail");
        assert!(error.to_string().contains("no calibrated realtime slack"));
        assert!(error.to_string().contains(std::env::consts::OS));
    }
}
