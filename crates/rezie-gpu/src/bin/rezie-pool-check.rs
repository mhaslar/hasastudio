//! Real-device pool ownership check; no compositor or performance acceptance claim.
#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use rezie_core::FrameTime;
use rezie_gpu::{FrameKey, FramePool, GpuContext, GpuError, WORKING_FORMAT};
use std::{path::PathBuf, time::Duration};

fn time(index: u64) -> FrameTime {
    FrameTime {
        index,
        pts: Duration::from_millis(index * 20),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .map_err(|e| anyhow::anyhow!("initialize pool-check logging: {e}"))?;
    let mut args = std::env::args().skip(1);
    let output = match args.next().as_deref() {
        Some("--output") => PathBuf::from(args.next().context("--output requires a path")?),
        _ => anyhow::bail!("usage: rezie-pool-check --output <report.json>"),
    };
    anyhow::ensure!(args.next().is_none(), "unexpected pool-check arguments");
    let context = GpuContext::request().await?;
    let adapter = context.adapter.get_info();
    anyhow::ensure!(
        adapter.device_type != wgpu::DeviceType::Cpu,
        "pool hardware check requires a real GPU, found {}",
        adapter.name
    );
    let mut pool = FramePool::new(context.device.clone(), 1024 * 1024);
    let key = FrameKey::new(64, 64)?;
    let reader = pool.reserve(key, 2).await?;
    let first = reader
        .try_acquire(time(0))?
        .context("first frame missing")?;
    let second = reader
        .try_acquire(time(1))?
        .context("second frame missing")?;
    anyhow::ensure!(
        reader.try_acquire(time(2))?.is_none(),
        "exhaustion must not grow or block"
    );
    anyhow::ensure!(
        first.texture().format() == WORKING_FORMAT && first.key() == key,
        "wrong GPU format/key"
    );
    let shared = first.try_clone()?;
    drop(first);
    anyhow::ensure!(reader.available() == 0, "shared frame was recycled early");
    anyhow::ensure!(shared.time() == time(0), "sharing lost programme time");
    drop(shared);
    anyhow::ensure!(reader.available() == 1, "last release did not return slot");
    drop(second);
    let before = pool.allocations();
    let mut workers = Vec::new();
    for _ in 0..2 {
        let reader = reader.clone();
        workers.push(std::thread::spawn(move || -> Result<()> {
            for index in 0..10_000 {
                let frame = reader
                    .try_acquire(time(index))?
                    .context("two-worker pool lost capacity")?;
                let shared = frame.try_clone()?;
                drop(frame);
                anyhow::ensure!(shared.time() == time(index), "reused frame has stale time");
                drop(shared);
            }
            Ok(())
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("pool worker panicked"))??;
    }
    anyhow::ensure!(
        reader.available() == 2 && pool.allocations() == before,
        "steady reuse leaked slots or allocated resources"
    );
    let retained = reader
        .try_acquire(time(20_001))?
        .context("retained frame missing")?;
    let grown = pool.reserve(key, 4).await?;
    anyhow::ensure!(
        retained.texture().width() == 64 && retained.time() == time(20_001),
        "growth invalidated old lease"
    );
    anyhow::ensure!(
        reader.capacity() == 2 && grown.capacity() == 4,
        "growth corrupted reader generations"
    );
    let after = pool.allocations();
    anyhow::ensure!(
        after.textures == 6 && after.views == 6,
        "unexpected resource creation calls"
    );
    let rejected = pool.reserve(FrameKey::new(1024, 1024)?, 1).await;
    anyhow::ensure!(
        matches!(rejected, Err(GpuError::Budget { .. })),
        "byte budget was not enforced"
    );
    drop(retained);
    drop(reader);
    pool.collect_retired();
    let report = serde_json::json!({
        "scope": "Phase 1 real-GPU ownership correctness; not a performance or phase-gate result",
        "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "adapter": adapter.name, "backend": format!("{:?}", adapter.backend),
        "driver": adapter.driver, "driver_info": adapter.driver_info,
        "working_format": format!("{:?}", WORKING_FORMAT),
        "worker_threads": 2, "acquire_share_release_cycles": 20_000,
        "initial_texture_calls": before.textures, "initial_view_calls": before.views,
        "steady_state_additional_texture_calls": 0, "steady_state_additional_view_calls": 0,
        "after_growth_texture_calls": after.textures, "after_growth_view_calls": after.views,
        "exhaustion_checked": true, "shared_lease_retained": true,
        "old_lease_survived_growth": true, "byte_budget_checked": true,
        "five_minute_reference_criterion_evaluated": false,
        "compositor_or_golden_frames_evaluated": false,
    });
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&output, serde_json::to_string_pretty(&report)?)?;
    tracing::info!(path = %output.display(), adapter = %adapter.name, "GPU pool ownership check passed; not phase acceptance");
    drop(grown);
    drop(pool);
    Ok(())
}
