use anyhow::{Context, Result};
use rezie_engine::benchmark::ClockReport;
use serde::Serialize;
use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
struct Point {
    slack_us: u64,
    priority_confirmed: bool,
    p50_ns: u64,
    p99_ns: u64,
    p99_9_ns: u64,
    max_ns: u64,
    spin_cpu_ns: u64,
    spin_cpu_percent_one_core: f64,
    thread_cpu_ns: u64,
    thread_cpu_percent_one_core: f64,
    spin_wall_ns: u64,
    spin_entries: u64,
    report: String,
}

fn output(command: &mut Command) -> Result<String> {
    let result = command
        .output()
        .with_context(|| format!("execute {command:?}"))?;
    anyhow::ensure!(
        result.status.success(),
        "metadata command {command:?} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(String::from_utf8(result.stdout)?.trim().to_owned())
}

pub(super) fn source_metadata() -> Result<serde_json::Value> {
    let cargo =
        std::env::var_os("CARGO").context("CARGO is required; invoke through cargo xtask")?;
    let rustc = Path::new(&cargo).with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    Ok(serde_json::json!({
        "rustc_vV": output(Command::new(rustc).arg("-vV"))?,
        "git_revision": output(Command::new("git").current_dir(super::root()).args(["rev-parse", "HEAD"]))?,
        "git_worktree": output(Command::new("git").current_dir(super::root()).args(["status", "--porcelain"]))?,
        "captured_unix_seconds": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    }))
}

pub(super) fn run(arguments: Vec<String>) -> Result<()> {
    let mut seconds = 60;
    let mut slacks = vec![1500, 0, 5000, 500, 3000, 1000];
    let mut directory = super::root().join(format!(
        "docs/benchmarks/phase-0-slack-sweep-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    let mut args = arguments.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--slacks-us" => {
                slacks = args
                    .next()
                    .context("--slacks-us requires comma-separated microseconds")?
                    .split(',')
                    .map(str::parse)
                    .collect::<std::result::Result<Vec<u64>, _>>()?
            }
            "--seconds" => {
                seconds = args
                    .next()
                    .context("--seconds requires a value")?
                    .parse::<u64>()?
            }
            "--output" => directory = args.next().context("--output requires a directory")?.into(),
            _ => anyhow::bail!("unknown clock-sweep argument '{arg}'"),
        }
    }
    anyhow::ensure!(
        (1..=600).contains(&seconds),
        "sweep duration must be 1–600 seconds per value"
    );
    anyhow::ensure!(
        (2..=32).contains(&slacks.len())
            && slacks
                .iter()
                .enumerate()
                .all(|(i, s)| *s <= 5000 && !slacks[..i].contains(s)),
        "provide 2–32 unique slack values from 0–5000 microseconds"
    );
    // Avoid overwriting the very comparison evidence this command is meant to preserve.
    anyhow::ensure!(
        !directory.exists(),
        "sweep directory '{}' already exists; choose a fresh --output directory",
        directory.display()
    );
    let host = if seconds < 60 {
        serde_json::json!({"kind": "functional smoke only; not calibration evidence"})
    } else if cfg!(windows) {
        super::reference::capture()?
    } else {
        anyhow::ensure!(
            cfg!(target_os = "macos"),
            "the approved calibration targets are Windows reference and M4"
        );
        let cpu = output(Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]))?;
        anyhow::ensure!(
            cpu.contains("Apple M4"),
            "Mac calibration requires M4; found {cpu}"
        );
        serde_json::json!({"cpu": cpu, "os": output(&mut Command::new("sw_vers"))?, "role": "development, never production"})
    };
    super::build_headless()?;
    fs::create_dir_all(&directory)?;
    let metadata = serde_json::json!({
        "kind": if seconds < 60 { "functional-smoke-not-calibration" } else { "slack-calibration-not-acceptance" },
        "host": host,
        "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "seconds_per_candidate": seconds,
        "candidate_order_us": slacks,
        "started_unix_seconds": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "source": source_metadata()?,
        "cpu_clock": if cfg!(windows) { "GetThreadTimes: kernel + user, 100 ns units; OS accounting granularity applies" } else { "clock_gettime(CLOCK_THREAD_CPUTIME_ID)" },
        "instrumentation": "two CPU-time queries per finishing-spin segment; total clock-thread CPU also recorded; disabled for ten-minute acceptance",
        "native_budget": "computation = max(2 ms, slack + 500 us), constraint = computation + 1 ms at 50 Hz; actual policy/budgets in every report",
        "selection": "none; inspect the curve and choose the smallest slack comfortably above degradation"
    });
    fs::write(
        directory.join("metadata.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    super::settle();
    let mut points = Vec::new();
    for slack in slacks {
        let name = format!("slack-{slack}-us.json");
        let path = directory.join(&name);
        super::run_measurement(seconds, &path, Some("--calibrate"), Some(slack))?;
        let report: ClockReport = serde_json::from_slice(&fs::read(&path)?)?;
        anyhow::ensure!(
            report.scheduling.finishing_slack_ns == slack * 1000,
            "requested and applied slack disagree"
        );
        let profile = report
            .wait_profile
            .context("calibration did not record CPU-time cost")?;
        anyhow::ensure!(
            profile.thread_wall_ns > 0,
            "calibration wall interval is zero"
        );
        let percent = |ns| ns as f64 / profile.thread_wall_ns as f64 * 100.0;
        let point = Point {
            slack_us: slack,
            priority_confirmed: report.scheduling.correctly_prioritized(),
            p50_ns: report.lateness.p50_ns,
            p99_ns: report.lateness.p99_ns,
            p99_9_ns: report.lateness.p99_9_ns,
            max_ns: report.lateness.max_ns,
            spin_cpu_ns: profile.spin_cpu_ns,
            spin_cpu_percent_one_core: percent(profile.spin_cpu_ns),
            thread_cpu_ns: profile.thread_cpu_ns,
            thread_cpu_percent_one_core: percent(profile.thread_cpu_ns),
            spin_wall_ns: profile.spin_wall_ns,
            spin_entries: profile.spin_entries,
            report: name,
        };
        tracing::info!(slack_us = slack, p50_ns = point.p50_ns, p99_ns = point.p99_ns,
            p99_9_ns = point.p99_9_ns, max_ns = point.max_ns,
            spin_cpu_percent_one_core = point.spin_cpu_percent_one_core,
            scheduling = ?report.scheduling, "slack trial recorded (not acceptance)");
        points.push(point);
        write_summary(&directory, &points, seconds)?;
        std::thread::sleep(Duration::from_secs(2));
    }
    tracing::info!(path = %directory.display(), "sweep complete; inspect curve.svg, summary.csv and all raw trial reports; no slack chosen automatically");
    Ok(())
}

fn write_summary(directory: &Path, points: &[Point], seconds: u64) -> Result<()> {
    fs::write(
        directory.join("summary.json"),
        serde_json::to_string_pretty(points)?,
    )?;
    let mut csv = String::from("slack_us,p50_ns,p99_ns,p99_9_ns,max_ns,spin_cpu_ns,spin_cpu_percent_one_core,thread_cpu_ns,thread_cpu_percent_one_core,spin_wall_ns,spin_entries,report,priority_confirmed\n");
    for p in points {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{:.6},{},{:.6},{},{},{},{}\n",
            p.slack_us,
            p.p50_ns,
            p.p99_ns,
            p.p99_9_ns,
            p.max_ns,
            p.spin_cpu_ns,
            p.spin_cpu_percent_one_core,
            p.thread_cpu_ns,
            p.thread_cpu_percent_one_core,
            p.spin_wall_ns,
            p.spin_entries,
            p.report,
            p.priority_confirmed
        ));
    }
    fs::write(directory.join("summary.csv"), csv)?;
    let mut ordered: Vec<_> = points.iter().collect();
    ordered.sort_by_key(|p| p.slack_us);
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 960 620"><rect width="960" height="620" fill="white"/><g font-family="sans-serif" font-size="14" fill="#222"><text x="60" y="28">Slack sweep — diagnostic measurements, no automatic selection</text>"##,
    );
    svg.push_str(&format!(
        r##"<text x="60" y="44" font-size="11">{} / {} — {} s per value — {}</text>"##,
        std::env::consts::OS,
        std::env::consts::ARCH,
        seconds,
        if seconds < 60 {
            "functional check only, not calibration evidence"
        } else {
            "calibration only, not production acceptance"
        }
    ));
    type Series = (&'static str, &'static str, Vec<f64>);
    if points.iter().any(|p| !p.priority_confirmed) {
        svg.push_str(r##"<text x="60" y="615" fill="#c64141">Priority is unconfirmed for some trials. Inspect priority_confirmed and raw policy reports before comparison.</text>"##);
    }
    let panels: [(&str, f64, Vec<Series>); 2] = [
        (
            "Lateness (ms)",
            75.0,
            vec![
                (
                    "p50",
                    "#748094",
                    ordered.iter().map(|p| p.p50_ns as f64 / 1e6).collect(),
                ),
                (
                    "p99",
                    "#3478b8",
                    ordered.iter().map(|p| p.p99_ns as f64 / 1e6).collect(),
                ),
                (
                    "p99.9",
                    "#8f55ad",
                    ordered.iter().map(|p| p.p99_9_ns as f64 / 1e6).collect(),
                ),
                (
                    "max",
                    "#c64141",
                    ordered.iter().map(|p| p.max_ns as f64 / 1e6).collect(),
                ),
            ],
        ),
        (
            "CPU cost (% of one core)",
            355.0,
            vec![
                (
                    "spin CPU",
                    "#23854e",
                    ordered
                        .iter()
                        .map(|p| p.spin_cpu_percent_one_core)
                        .collect(),
                ),
                (
                    "whole thread CPU",
                    "#bd791d",
                    ordered
                        .iter()
                        .map(|p| p.thread_cpu_percent_one_core)
                        .collect(),
                ),
            ],
        ),
    ];
    for (title, top, series) in panels {
        let max = series
            .iter()
            .flat_map(|(_, _, values)| values)
            .copied()
            .fold(0.001_f64, f64::max)
            * 1.1;
        svg.push_str(&format!(r##"<text x="60" y="{}">{title}</text><path d="M 80 {top} v 185 h 690" fill="none" stroke="#222"/>"##, top - 12.0));
        for tick in 0..=4 {
            let y = top + 185.0 - tick as f64 * 185.0 / 4.0;
            svg.push_str(&format!(
                r##"<text x="12" y="{}">{:.3}</text><path d="M 80 {y} h 690" stroke="#ddd"/>"##,
                y + 4.0,
                tick as f64 * max / 4.0
            ));
        }
        for (index, (name, color, values)) in series.iter().enumerate() {
            let mut line = String::new();
            for (p, value) in ordered.iter().zip(values) {
                let x = 80.0 + p.slack_us as f64 / 5000.0 * 690.0;
                let y = top + 185.0 - value / max * 185.0;
                line.push_str(&format!("{x:.2},{y:.2} "));
                svg.push_str(&format!(
                    r##"<circle cx="{x}" cy="{y}" r="3" fill="{color}"/>"##
                ));
            }
            svg.push_str(&format!(r##"<polyline points="{line}" fill="none" stroke="{color}" stroke-width="2"/><text x="790" y="{}" fill="{color}">{name}</text>"##, top + 20.0 + index as f64 * 22.0));
        }
        for p in &ordered {
            svg.push_str(&format!(
                r##"<text x="{}" y="{}">{:.1}</text>"##,
                74.0 + p.slack_us as f64 / 5000.0 * 690.0,
                top + 205.0,
                p.slack_us as f64 / 1000.0
            ));
        }
        svg.push_str(&format!(
            r##"<text x="345" y="{}">Finishing slack (ms)</text>"##,
            top + 232.0
        ));
    }
    svg.push_str("</g></svg>\n");
    fs::write(directory.join("curve.svg"), svg)?;
    Ok(())
}
