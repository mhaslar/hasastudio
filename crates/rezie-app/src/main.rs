//! Empty Foundation GUI shell; all production state belongs to the engine.
#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use anyhow::{Context, Result};
use eframe::egui;
use rezie_engine::{Engine, EngineConfig, TickConsumer};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct FoundationApp {
    engine: Engine,
    client: rezie_api::Client,
    sinks: Vec<TickConsumer>,
    smoke: bool,
    updated: Arc<AtomicBool>,
}

impl eframe::App for FoundationApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        for sink in &mut self.sinks {
            while sink.pop().is_some() {}
        }
        let state = self.client.snapshot();
        egui::CentralPanel::default().show(ctx, |_ui| {});
        // Smoke success requires an actual GUI update with a live engine tick.
        if state.clock.emitted > 0 && state.running {
            self.updated.store(true, Ordering::Release);
            if self.smoke {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

impl Drop for FoundationApp {
    fn drop(&mut self) {
        if let Err(error) = self.engine.shutdown() {
            tracing::error!(%error, "engine shutdown failed");
        }
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let smoke_marker = match args.next().as_deref() {
        None => None,
        Some("--smoke-test") => Some(PathBuf::from(
            args.next().context("--smoke-test requires a marker path")?,
        )),
        Some(arg) => anyhow::bail!("unknown argument '{arg}'"),
    };
    anyhow::ensure!(args.next().is_none(), "unexpected application arguments");
    let _logs = rezie_engine::logging::init(&std::env::temp_dir().join("rezie-logs"))?;
    let (engine, sinks) = Engine::start(EngineConfig::default())?;
    let client = engine.client();
    let updated = Arc::new(AtomicBool::new(false));
    let app = FoundationApp {
        engine,
        client,
        sinks,
        smoke: smoke_marker.is_some(),
        updated: updated.clone(),
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "HasaStudio",
        options,
        Box::new(move |_cc| Ok(Box::new(app))),
    )
    .map_err(|error| anyhow::anyhow!("start HasaStudio window: {error}"))?;
    if let Some(marker) = smoke_marker {
        anyhow::ensure!(
            updated.load(Ordering::Acquire),
            "GUI closed before rendering with a ticking engine"
        );
        std::fs::write(&marker, "GUI updated with a live engine tick\n")
            .with_context(|| format!("write smoke marker '{}'", marker.display()))?;
    }
    Ok(())
}
