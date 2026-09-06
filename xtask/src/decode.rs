use anyhow::{Context, Result};
use serde_json::json;
use std::{fs, path::PathBuf, process::Command};

pub fn run(arguments: Vec<String>) -> Result<()> {
    let mut output = None;
    let mut forwarded = Vec::new();
    let mut args = arguments.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().context("--output requires a new directory")?,
                ))
            }
            "--input" | "--mode" => {
                let value = args.next().context("decode option needs a value")?;
                forwarded.extend([arg, value]);
            }
            _ => anyhow::bail!("unknown decode-check argument '{arg}'"),
        }
    }
    let directory = output.context("decode-check requires --output <new-directory>")?;
    anyhow::ensure!(
        !directory.exists(),
        "output '{}' already exists; choose a new directory",
        directory.display()
    );
    super::cargo(&[
        "build",
        "--locked",
        "-p",
        "rezie-media",
        "--bin",
        "rezie-decode-check",
    ])?;
    if let Some(parent) = directory.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&directory)?; // Atomic ownership: another writer cannot reuse our output directory.
    let report_path = directory.join("report.json");
    let binary = super::root().join("target/debug").join(if cfg!(windows) {
        "rezie-decode-check.exe"
    } else {
        "rezie-decode-check"
    });
    let result = Command::new(binary)
        .args(forwarded)
        .arg("--output")
        .arg(&report_path)
        .output()?;
    fs::write(directory.join("native.log"), &result.stderr)?;
    fs::write(directory.join("stdout.log"), &result.stdout)?;
    let log = String::from_utf8_lossy(&result.stderr);
    let mut versions = Vec::new();
    for line in log.lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        for pair in words.windows(2) {
            if pair[0] == "libdav1d" && pair[1].bytes().next().is_some_and(|b| b.is_ascii_digit()) {
                let version = pair[1].to_owned();
                if !versions.contains(&version) {
                    versions.push(version);
                }
            }
        }
    }
    let mut report = if report_path.exists() {
        serde_json::from_slice::<serde_json::Value>(&fs::read(&report_path)?)?
    } else {
        json!({"passed":false,"error":"decoder exited before producing its report; see native.log"})
    };
    report["dav1d_runtime_versions_from_native_log"] = json!(versions);
    report["native_log"] = json!("native.log");
    if std::env::var("REZIE_DISABLE_HW_DECODE").as_deref() == Ok("1") {
        let verified = report["cases"].as_array().is_some_and(|cases| {
            !cases.is_empty()
                && cases.iter().all(|c| {
                    c["passed"] == true
                        && c["status"]["environment_disabled_hardware"] == true
                        && c["status"]["hardware_device"].is_null()
                        && c["status"]["hardware_frame_context_observed"] == false
                })
        });
        report["software_override_verified"] = json!(verified);
        if !verified {
            report["passed"] = json!(false);
        }
    }
    #[cfg(windows)]
    {
        report["reference_host"] = match super::reference::capture() {
            Ok(host) => host,
            Err(error) => json!({"reference_match":false,"detail":format!("{error:#}")}),
        };
    }
    #[cfg(target_os = "macos")]
    {
        let cpu = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()?;
        let os = Command::new("sw_vers").output()?;
        anyhow::ensure!(
            cpu.status.success() && os.status.success(),
            "capture Mac host metadata"
        );
        report["host"] = json!({"cpu":String::from_utf8_lossy(&cpu.stdout).trim(),
            "os":String::from_utf8_lossy(&os.stdout).trim(),"role":"development correctness, not production performance"});
    }
    report["source"] = super::sweep::source_metadata()?;
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    anyhow::ensure!(
        result.status.success() && report["passed"] == true,
        "decode check failed; inspect '{}' and native.log",
        report_path.display()
    );
    tracing::info!(path=%report_path.display(), "decode check passed; native logs and per-picture evidence retained");
    Ok(())
}
