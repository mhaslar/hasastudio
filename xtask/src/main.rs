//! Phase-aware development, verification and packaging tasks.
#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

mod fetch;
mod reference;
mod sweep;

const ASSET_MANIFEST: &str = "{\n  \"phase\": 0,\n  \"pixel_assets\": [],\n  \"reason\": \"Phase 0 dispatches FrameTime only; GPU pixels begin in Phase 1 (ADR 0001).\"\n}\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("execute {command:?}"))?;
    anyhow::ensure!(status.success(), "command {command:?} failed with {status}");
    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    run(
        Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(root())
            .args(args),
    )
}

fn phase() -> Result<u8> {
    let text = fs::read_to_string(root().join("AGENTS.md"))?;
    let marker = text
        .lines()
        .find_map(|line| line.strip_prefix("> **Phase: "))
        .context("missing current phase marker")?;
    marker
        .split_whitespace()
        .next()
        .context("missing phase number")?
        .parse()
        .context("invalid current phase")
}

fn fetch_deps() -> Result<()> {
    let manifest: fetch::Manifest = serde_json::from_str(include_str!("../dependencies.json"))?;
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    for dependency in fetch::eligible(&manifest, phase()?, &platform)? {
        fetch::fetch(&dependency, &root().join(".deps"))?;
    }
    Ok(())
}

fn golden() -> Result<()> {
    anyhow::ensure!(
        phase()? == 0,
        "golden path inventory must be implemented for the new phase"
    );
    let manifest = fs::read_to_string(root().join("tests/assets/phase-0.json"))
        .context("run cargo xtask gen-assets first")?;
    anyhow::ensure!(
        manifest == ASSET_MANIFEST,
        "Phase 0 asset manifest differs from approved tick-only scope"
    );
    let golden_dir = root().join("tests/golden");
    for entry in fs::read_dir(golden_dir)? {
        let path = entry?.path();
        anyhow::ensure!(
            path.file_name().is_some_and(|name| name == "README.md"),
            "unexpected Phase 0 golden reference '{}': pixels belong to Phase 1",
            path.display()
        );
    }
    anyhow::ensure!(
        !root().join("crates/rezie-gpu").exists(),
        "GPU crate exists: compositor golden coverage must be implemented"
    );
    tracing::info!(
        compositor_paths = 0,
        frame_comparisons = 0,
        "Phase 0 golden inventory verified; no pixels exist in this phase"
    );
    Ok(())
}

fn build_headless() -> Result<()> {
    // Build first, then leave the machine quiet before starting measurement.
    // The measured executable is release-built, not this development xtask.
    cargo(&[
        "build",
        "--release",
        "--locked",
        "-p",
        "rezie-engine",
        "--bin",
        "rezie-headless",
    ])
}

fn settle() {
    tracing::info!(
        "all builds finished; settling for 15 seconds; keep this machine otherwise idle"
    );
    std::thread::sleep(Duration::from_secs(15));
}

fn measure(seconds: u64, path: &Path, latency: bool, slack: Option<u64>) -> Result<()> {
    build_headless()?;
    if latency {
        settle();
    }
    run_measurement(
        seconds,
        path,
        if latency { Some("--latency") } else { None },
        slack,
    )
}

fn run_measurement(
    seconds: u64,
    path: &Path,
    mode: Option<&str>,
    slack: Option<u64>,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let binary = root().join("target/release").join(if cfg!(windows) {
        "rezie-headless.exe"
    } else {
        "rezie-headless"
    });
    let mut command = Command::new(binary);
    command
        .current_dir(root())
        .arg("--clock-seconds")
        .arg(seconds.to_string())
        .arg("--report")
        .arg(path);
    if let Some(mode) = mode {
        command.arg(mode);
    }
    if let Some(slack) = slack {
        command.arg("--slack-us").arg(slack.to_string());
    }
    run(&mut command)
}

fn dist() -> Result<PathBuf> {
    cargo(&[
        "build",
        "--release",
        "--locked",
        "-p",
        "rezie-app",
        "-p",
        "rezie-engine",
        "--bins",
    ])?;
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let output = root().join("dist").join(platform);
    let binary = if cfg!(windows) {
        "rezie-app.exe"
    } else {
        "rezie-app"
    };
    let headless = if cfg!(windows) {
        "rezie-headless.exe"
    } else {
        "rezie-headless"
    };
    let release = root().join("target/release");
    fs::create_dir_all(&output)?;
    let destination = if cfg!(target_os = "macos") {
        let contents = output.join("HasaStudio.app/Contents");
        fs::create_dir_all(contents.join("MacOS"))?;
        fs::write(contents.join("Info.plist"), "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>rezie-app</string><key>CFBundleIdentifier</key><string>studio.hasa.rezie</string><key>CFBundleName</key><string>HasaStudio</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleVersion</key><string>0.1.0</string><key>NSHighResolutionCapable</key><true/></dict></plist>\n")?;
        contents.join("MacOS").join(binary)
    } else {
        output.join(binary)
    };
    fs::copy(release.join(binary), &destination)?;
    fs::copy(release.join(headless), output.join(headless))?;
    if cfg!(target_os = "linux") {
        fs::write(output.join("HasaStudio.desktop"), "[Desktop Entry]\nType=Application\nName=HasaStudio\nExec=rezie-app\nTerminal=false\nCategories=AudioVideo;Video;\n")?;
    }
    fs::write(output.join("README.txt"), "HasaStudio Phase 0 — empty application shell.\nLaunch the application to open its window. No media SDKs are required.\nThis development bundle is not a Phase 11 installer. Linux requires a desktop session, Vulkan driver and standard X11/Wayland libraries.\n")?;
    tracing::info!(path = %destination.display(), "application bundle built");
    Ok(destination)
}

fn smoke(binary: &Path) -> Result<()> {
    let marker = root().join("target/dist-smoke.txt");
    if marker.exists() {
        fs::remove_file(&marker)?;
    }
    let mut child = Command::new(binary)
        .arg("--smoke-test")
        .arg(&marker)
        .current_dir(root())
        .spawn()
        .with_context(|| format!("launch packaged application '{}'", binary.display()))?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            anyhow::ensure!(status.success(), "packaged GUI exited with {status}");
            break;
        }
        if start.elapsed() > Duration::from_secs(45) {
            child.kill()?;
            let _ = child.wait();
            anyhow::bail!("packaged GUI did not complete an update within 45 seconds");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    anyhow::ensure!(
        fs::read_to_string(&marker).context("GUI did not write smoke-test evidence")?
            == "GUI updated with a live engine tick\n",
        "invalid GUI smoke evidence"
    );
    tracing::info!(binary = %binary.display(), "packaged GUI rendered with a live engine tick");
    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .context("usage: cargo xtask fetch-deps|gen-assets|golden|bench|soak|dist|ci")?;
    match command.as_str() {
        "fetch-deps" => {
            anyhow::ensure!(
                args.next().is_none(),
                "fetch-deps uses the current phase; no overrides"
            );
            fetch_deps()?;
        }
        "gen-assets" => {
            anyhow::ensure!(args.next().is_none(), "gen-assets takes no arguments");
            anyhow::ensure!(
                phase()? == 0,
                "new phase requires its media asset generator"
            );
            fs::create_dir_all(root().join("tests/assets"))?;
            fs::write(root().join("tests/assets/phase-0.json"), ASSET_MANIFEST)?;
        }
        "golden" => {
            anyhow::ensure!(args.next().is_none(), "Phase 0 has no reference updates; golden --update requires human review in a pixel-producing phase");
            golden()?;
        }
        "clock-check" => {
            anyhow::ensure!(args.next().is_none(), "clock-check takes no arguments");
            measure(
                10,
                &root().join("target/clock-correctness.json"),
                false,
                Some(0),
            )?;
        }
        "bench" => {
            let mut slack = None;
            let mut path = root().join(format!(
                "docs/benchmarks/phase-0-idle-{}-{}.json",
                std::env::consts::OS,
                std::env::consts::ARCH
            ));
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--slack-us" => {
                        slack = Some(
                            args.next()
                                .context("--slack-us requires microseconds")?
                                .parse::<u64>()?,
                        );
                    }
                    "--output" => {
                        path = root().join(args.next().context("--output requires a JSON path")?);
                    }
                    other => anyhow::bail!("unexpected bench option '{other}'"),
                }
            }
            anyhow::ensure!(
                slack.is_none_or(|s| s <= 5000),
                "slack must be 0–5000 microseconds"
            );
            anyhow::ensure!(
                path.extension().is_some_and(|x| x == "json"),
                "bench output must have a .json extension"
            );
            anyhow::ensure!(
                !path.exists() && !path.with_extension("host.json").exists(),
                "preserve existing evidence; select a fresh --output JSON path"
            );
            let host = reference::capture()?;
            let metadata = serde_json::json!({"host": host, "source": sweep::source_metadata()?, "slack_override_us": slack});
            fs::write(
                path.with_extension("host.json"),
                serde_json::to_string_pretty(&metadata)?,
            )?;
            measure(600, &path, true, slack)?;
        }
        "clock-sweep" => sweep::run(args.collect())?,
        "soak" => {
            let _host = reference::capture()?;
            let minutes = match args.next().as_deref() {
                None => 30,
                Some("--minutes") => args
                    .next()
                    .context("--minutes requires a value")?
                    .parse::<u64>()?,
                Some(other) => anyhow::bail!("unexpected soak option '{other}'"),
            };
            anyhow::ensure!(args.next().is_none(), "unexpected soak arguments");
            measure(
                minutes.checked_mul(60).context("soak duration overflow")?,
                &root().join("target/soak.json"),
                false,
                None,
            )?;
        }
        "dist" => {
            let check = match args.next().as_deref() {
                None => false,
                Some("--smoke") => true,
                Some(other) => anyhow::bail!("unexpected dist option '{other}'"),
            };
            anyhow::ensure!(args.next().is_none(), "unexpected dist arguments");
            let binary = dist()?;
            if check {
                smoke(&binary)?;
            }
        }
        "ci" => {
            anyhow::ensure!(args.next().is_none(), "ci takes no arguments");
            fetch_deps()?;
            cargo(&["xtask", "gen-assets"])?;
            cargo(&["fmt", "--all", "--", "--check"])?;
            cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ])?;
            cargo(&["nextest", "run", "--workspace", "--locked"])?;
            cargo(&["test", "--workspace", "--doc", "--locked"])?;
            golden()?;
        }
        _ => anyhow::bail!("unknown xtask '{command}'"),
    }
    Ok(())
}
