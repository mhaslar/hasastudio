//! Read actual native/decoder properties and preserve every picture's PTS/hash.
use anyhow::{Context, Result};
use rezie_media::{DecodeMode, FileDecoder};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let mut args = std::env::args().skip(1);
    let mut input = None;
    let mut output = None;
    let mut mode = DecodeMode::Software;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = Some(PathBuf::from(args.next().context("--input needs a file")?)),
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().context("--output needs a new JSON path")?,
                ))
            }
            "--mode" => {
                mode = match args.next().as_deref() {
                    Some("auto") => DecodeMode::Auto,
                    Some("software") => DecodeMode::Software,
                    Some("hardware") => DecodeMode::RequireHardware,
                    _ => anyhow::bail!("--mode must be auto, software or hardware"),
                }
            }
            _ => anyhow::bail!("unknown argument '{arg}'"),
        }
    }
    let output = output.context("--output is required; existing evidence is never overwritten")?;
    anyhow::ensure!(
        !output.exists(),
        "output '{}' already exists",
        output.display()
    );
    let native = rezie_media::initialize()?;
    ffmpeg_next::util::log::set_level(ffmpeg_next::util::log::Level::Verbose);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = root.join("tests/assets/phase-1/decode");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(fixtures.join("manifest.json"))?)?;
    let files = if let Some(input) = input {
        vec![input]
    } else {
        manifest["files"]
            .as_array()
            .context("fixture manifest files")?
            .iter()
            .map(|f| {
                f["file"]
                    .as_str()
                    .map(|s| fixtures.join(s))
                    .context("fixture name")
            })
            .collect::<Result<Vec<_>>>()?
    };
    let mut cases = Vec::new();
    let mut passed = true;
    for path in files {
        let input_hash = format!("{:x}", Sha256::digest(fs::read(&path)?));
        let result = (|| -> Result<serde_json::Value> {
            let mut decoder = FileDecoder::open(&path, mode)?;
            let mut pictures = Vec::new();
            while let Some(picture) = decoder.next_picture()? {
                pictures.push(picture.inspect()?);
            }
            anyhow::ensure!(!pictures.is_empty(), "no decoded pictures");
            let reference = manifest["files"]
                .as_array()
                .context("fixture files")?
                .iter()
                .find(|f| f["sha256"].as_str() == Some(&input_hash));
            let mut matches_oracle = None;
            if let Some(reference) = reference {
                let oracle = &reference["pictures"];
                let observed: Vec<_> = pictures.iter().map(|p| json!({
                    "pts":p.pts,"time_base":p.time_base,"component_sha256":p.component_sha256,
                    "component_depth":p.component_depth,"dimensions":p.dimensions
                })).collect();
                matches_oracle = Some(serde_json::to_value(observed)? == *oracle);
            }
            let success = matches_oracle != Some(false)
                && (mode != DecodeMode::RequireHardware
                    || decoder.status().hardware_frame_context_observed);
            Ok(
                json!({"input":path.file_name().map(|s| s.to_string_lossy()),"input_sha256":input_hash,"status":decoder.status(),
                "pictures":pictures,"matches_independent_fixture_oracle":matches_oracle,"passed":success}),
            )
        })();
        match result {
            Ok(case) => {
                passed &= case["passed"] == true;
                cases.push(case);
            }
            Err(error) => {
                passed = false;
                cases.push(json!({"input":path.file_name().map(|s| s.to_string_lossy()),"input_sha256":input_hash,"error":format!("{error:#}"),"passed":false}));
            }
        }
    }
    let source = Command::new("git")
        .current_dir(&root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    let report = json!({"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,
        "mode":format!("{mode:?}"),"git_revision":String::from_utf8_lossy(&source.stdout).trim(),
        "build_native":{"path":env!("REZIE_BUILD_AVCODEC_PATH"),"version":env!("REZIE_BUILD_AVCODEC_VERSION"),
            "configuration":env!("REZIE_BUILD_AVCODEC_CONFIGURATION"),"licence":env!("REZIE_BUILD_AVCODEC_LICENCE")},
        "runtime_native":{"version":native.version,"configuration":native.configuration,"licence":native.licence},
        "scope":"File decode correctness; no GPU upload, preview, NDI or performance gate evaluated",
        "cases":cases,"passed":passed});
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    tracing::info!(passed, output=%output.display(), "decoder report written");
    anyhow::ensure!(
        passed,
        "one or more decoder cases failed; see '{}'",
        output.display()
    );
    Ok(())
}
