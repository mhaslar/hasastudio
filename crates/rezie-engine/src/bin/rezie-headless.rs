//! Headless Foundation engine entry point for the integration harness.
use anyhow::{Context, Result};
use rezie_api::WebSocketServer;
use rezie_engine::{benchmark, logging, Engine, EngineConfig};
use std::{net::SocketAddr, path::Path, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let _logs = logging::init(Path::new(".logs"))?;
    let mut args = std::env::args().skip(1);
    let mut address: SocketAddr = "127.0.0.1:9800".parse()?;
    let mut clock_seconds = None;
    let mut report = None;
    let mut ready_file = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ws" => {
                address = args
                    .next()
                    .context("--ws requires an address")?
                    .parse()
                    .context("invalid --ws address")?
            }
            "--clock-seconds" => {
                clock_seconds = Some(
                    args.next()
                        .context("--clock-seconds requires seconds")?
                        .parse::<u64>()?,
                )
            }
            "--report" => report = Some(args.next().context("--report requires a path")?),
            "--ready-file" => {
                ready_file = Some(args.next().context("--ready-file requires a path")?)
            }
            _ => anyhow::bail!("unknown argument '{arg}'"),
        }
    }
    if let Some(seconds) = clock_seconds {
        anyhow::ensure!(ready_file.is_none(), "--ready-file requires WebSocket mode");
        let result = tokio::task::spawn_blocking(move || benchmark::run(seconds)).await??;
        if let Some(path) = report {
            std::fs::write(&path, serde_json::to_string_pretty(&result)?)
                .with_context(|| format!("write clock report '{path}'"))?;
        }
        tracing::info!(
            passed = result.passed,
            ticks = result.received_ticks,
            drift_ns = result.clock.final_lateness_ns,
            max_lateness_ns = result.clock.max_lateness_ns,
            "clock measurement complete"
        );
        anyhow::ensure!(result.passed, "clock/dispatch acceptance failed");
        return Ok(());
    }
    anyhow::ensure!(report.is_none(), "--report requires --clock-seconds");
    let (mut engine, mut sinks) = Engine::start(EngineConfig::default())?;
    let server = WebSocketServer::bind(address, engine.client()).await?;
    if let Some(path) = ready_file {
        std::fs::write(&path, server.address().to_string())
            .with_context(|| format!("write harness address '{path}'"))?;
    }
    tracing::info!(address = %server.address(), "headless engine listening");
    let signal = tokio::signal::ctrl_c();
    tokio::pin!(signal);
    loop {
        tokio::select! {
            result = &mut signal => { result?; break; }
            _ = tokio::time::sleep(Duration::from_millis(2)) => {
                for sink in &mut sinks { while sink.pop().is_some() {} }
                if !engine.client().snapshot().running { break; }
            }
        }
    }
    server.shutdown().await?;
    engine.shutdown()?;
    Ok(())
}
