use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
mod metric;
const WIDTH: u32 = 257;
const HEIGHT: u32 = 65;
const CASES: [&str; 5] = ["black", "white", "colour", "translucent", "transparent"];
const INPUT_HASH: &str = "47f24b377b54f5ea21902be325f7121f26ea3bcef70b9357e255a38e70dd3dad";
const REF_DIR: &str = "tests/golden/phase-1/colour-alpha";
const MEAN_LIMIT: f64 = 1.0;
const MAX_LIMIT: f64 = 3.0;
const ALPHA_LIMIT: f64 = 0.002;
const LINEAR_LIMIT: f64 = 0.002;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn unique_id() -> Result<String> {
    Ok(format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        std::process::id()
    ))
}
fn input_pixels() -> Vec<u8> {
    let mut bytes = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let ramp = x.min(255) as u8;
            let pixel = match y / 8 {
                0 => [255, 255, 255, 128],
                1 => [0, 0, 0, 128],
                2 => [255, 0, 255, 0],
                3 => [ramp, 255 - ramp, 73, 255],
                4 => [255, 255, 255, ramp],
                5 => [231, 73, 19, ramp],
                6 => [ramp, ramp, ramp, 128],
                7 => [ramp, 0, 255 - ramp, 1],
                _ => [10, 11, 12, 255],
            };
            bytes.extend(pixel);
        }
    }
    bytes
}
fn write_png(path: &Path, bytes: &[u8], depth: png::BitDepth) -> Result<()> {
    let mut encoder = png::Encoder::new(BufWriter::new(fs::File::create(path)?), WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(depth);
    encoder.set_source_srgb(png::SrgbRenderingIntent::RelativeColorimetric);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(bytes)?;
    writer.finish()?;
    Ok(())
}
fn read_png(path: &Path, depth: png::BitDepth) -> Result<Vec<u8>> {
    let mut decoder = png::Decoder::new(BufReader::new(
        fs::File::open(path).with_context(|| format!("open {}", path.display()))?,
    ));
    decoder.set_limits(png::Limits {
        bytes: 4 * 1024 * 1024,
    });
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    anyhow::ensure!(
        info.width == WIDTH
            && info.height == HEIGHT
            && info.color_type == png::ColorType::Rgba
            && info.bit_depth == depth
            && info.srgb.is_some(),
        "{}: expected {WIDTH}x{HEIGHT} sRGB RGBA {depth:?}",
        path.display()
    );
    let mut bytes = vec![
        0;
        reader
            .output_buffer_size()
            .context("PNG decoded size overflow")?
    ];
    let info = reader.next_frame(&mut bytes)?;
    bytes.truncate(info.buffer_size());
    Ok(bytes)
}
fn pixels16(path: &Path) -> Result<Vec<[u16; 4]>> {
    Ok(read_png(path, png::BitDepth::Sixteen)?
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| std::array::from_fn(|c| u16::from_be_bytes([b[c * 2], b[c * 2 + 1]])))
        .collect())
}
pub(super) fn gen_assets() -> Result<()> {
    let directory = super::root().join("tests/assets/phase-1");
    fs::create_dir_all(&directory)?;
    let bytes = input_pixels();
    anyhow::ensure!(
        hash(&bytes) == INPUT_HASH,
        "generated input differs from approved scene; review required"
    );
    write_png(
        &directory.join("input-alpha.png"),
        &bytes,
        png::BitDepth::Eight,
    )?;
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(
            &json!({"phase":1,"width":WIDTH,"height":HEIGHT,"rgba_sha256":INPUT_HASH,"generator":"xtask::golden::input_pixels","cases":CASES}),
        )?,
    )?;
    tracing::info!("generated exact Phase 1 alpha/colour input from code");
    Ok(())
}
fn validate_references(root: &Path) -> Result<Value> {
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join(REF_DIR).join("approved.json"))?)?;
    anyhow::ensure!(
        manifest["status"] == "approved" && manifest["approved_proposal_commit"] == "c9397f0",
        "missing approved reference provenance"
    );
    let files = manifest["files"]
        .as_array()
        .context("reference file inventory missing")?;
    anyhow::ensure!(files.len() == 10, "expected ten approved reference files");
    for name in CASES {
        for extension in ["png", "rgba16f.le"] {
            let path = format!("{REF_DIR}/{name}.{extension}");
            let entries: Vec<_> = files.iter().filter(|f| f["destination"] == path).collect();
            anyhow::ensure!(
                entries.len() == 1,
                "reference inventory must name {path} exactly once"
            );
            let bytes = fs::read(root.join(&path))
                .with_context(|| format!("read approved reference {path}"))?;
            anyhow::ensure!(entries[0]["bytes"] == bytes.len() && entries[0]["sha256"] == hash(&bytes), "approved reference {path} was changed; restore approved bytes or obtain a reviewed update");
        }
    }
    Ok(manifest)
}
#[derive(Serialize)]
struct Comparison {
    pixels: usize,
    // Order is appearance over linear black, then linear white.
    mean_delta_e00: [f64; 2],
    max_delta_e00: [f64; 2],
    mean_alpha_error: f64,
    max_alpha_error: f64,
    max_raw_linear_error: f64,
    passed: bool,
    #[serde(skip)]
    colour_diff: Vec<u8>,
    #[serde(skip)]
    alpha_diff: Vec<u8>,
}
fn bounds(mean: [f64; 2], max: [f64; 2], alpha: f64, raw: f64) -> bool {
    mean.into_iter().all(|x| x < MEAN_LIMIT)
        && max.into_iter().all(|x| x < MAX_LIMIT)
        && alpha <= ALPHA_LIMIT
        && raw <= LINEAR_LIMIT
}
fn half(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let e = (bits >> 10) & 31;
    let f = f64::from(bits & 1023);
    match e {
        0 => sign * f * 2_f64.powi(-24),
        31 => f64::NAN,
        _ => sign * (1.0 + f / 1024.0) * 2_f64.powi(i32::from(e) - 15),
    }
}
fn raw_error(reference: &[u8], actual: &[u8]) -> Result<f64> {
    anyhow::ensure!(
        reference.len() == (WIDTH * HEIGHT * 8) as usize && actual.len() == reference.len(),
        "raw linear sample count mismatch"
    );
    let mut max = 0_f64;
    for (a, b) in reference
        .as_chunks::<2>()
        .0
        .iter()
        .zip(actual.as_chunks::<2>().0)
    {
        let a = half(u16::from_le_bytes(*a));
        let b = half(u16::from_le_bytes(*b));
        anyhow::ensure!(
            a.is_finite() && b.is_finite(),
            "nonfinite raw linear sample"
        );
        max = max.max((a - b).abs());
    }
    Ok(max)
}
fn compare(reference: &[[u16; 4]], actual: &[[u16; 4]], raw: f64) -> Result<Comparison> {
    anyhow::ensure!(
        !reference.is_empty() && reference.len() == actual.len(),
        "PNG pixel count mismatch"
    );
    let mut result = Comparison {
        pixels: reference.len(),
        mean_delta_e00: [0.; 2],
        max_delta_e00: [0.; 2],
        mean_alpha_error: 0.,
        max_alpha_error: 0.,
        max_raw_linear_error: raw,
        passed: false,
        colour_diff: Vec::new(),
        alpha_diff: Vec::new(),
    };
    for (&reference, &actual) in reference.iter().zip(actual) {
        let mut worst = 0_f64;
        for (i, background) in [0.0, 1.0].into_iter().enumerate() {
            let delta = metric::delta_e(
                metric::lab(reference, background),
                metric::lab(actual, background),
            );
            result.mean_delta_e00[i] += delta;
            result.max_delta_e00[i] = result.max_delta_e00[i].max(delta);
            worst = worst.max(delta);
        }
        let alpha = f64::from(reference[3].abs_diff(actual[3])) / 65535.0;
        result.mean_alpha_error += alpha;
        result.max_alpha_error = result.max_alpha_error.max(alpha);
        let heat = ((worst / MAX_LIMIT).min(1.0) * 65535.0).round() as u16;
        let alpha_heat = ((alpha / ALPHA_LIMIT).min(1.0) * 65535.0).round() as u16;
        for v in [heat, 0, 0, 65535] {
            result.colour_diff.extend(v.to_be_bytes());
        }
        for v in [alpha_heat, alpha_heat, alpha_heat, 65535] {
            result.alpha_diff.extend(v.to_be_bytes());
        }
    }
    for mean in &mut result.mean_delta_e00 {
        *mean /= result.pixels as f64;
    }
    result.mean_alpha_error /= result.pixels as f64;
    result.passed = bounds(
        result.mean_delta_e00,
        result.max_delta_e00,
        result.max_alpha_error,
        raw,
    );
    Ok(result)
}
fn failure_images(
    directory: &Path,
    name: &str,
    actual: &Path,
    comparison: &Comparison,
) -> Result<()> {
    fs::create_dir_all(directory)?;
    fs::copy(actual, directory.join(format!("{name}-actual.png")))?;
    write_png(
        &directory.join(format!("{name}-difference.png")),
        &comparison.colour_diff,
        png::BitDepth::Sixteen,
    )?;
    write_png(
        &directory.join(format!("{name}-alpha-difference.png")),
        &comparison.alpha_diff,
        png::BitDepth::Sixteen,
    )?;
    Ok(())
}
pub(super) fn run(development: bool, output: Option<PathBuf>) -> Result<()> {
    let root = super::root();
    let references = validate_references(&root)?;
    let host = if development {
        json!({"os":std::env::consts::OS,"architecture":std::env::consts::ARCH})
    } else {
        super::reference::capture()?
    };
    let input = root.join("tests/assets/phase-1/input-alpha.png");
    anyhow::ensure!(
        hash(&read_png(&input, png::BitDepth::Eight).context("run cargo xtask gen-assets first")?)
            == INPUT_HASH,
        "generated input does not match approved scene"
    );
    let id = unique_id()?;
    let output = output
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("target/golden-runs").join(&id));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&output).with_context(|| {
        format!(
            "golden output must be a NEW directory: {}",
            output.display()
        )
    })?;
    let render = output.join("render");
    let failures = root.join("target/golden-failures").join(&id);
    let result = (|| -> Result<Value> {
        // Build/execute off the composite thread; no performance claims for this command.
        let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(&root)
            .args([
                "run",
                "--locked",
                "-p",
                "rezie-gpu",
                "--bin",
                "rezie-colour-check",
                "--",
                "--output",
            ])
            .arg(&render)
            .arg("--input")
            .arg(&input)
            .status()
            .context("execute native golden renderer")?;
        let renderer: Value = serde_json::from_slice(
            &fs::read(render.join("report.json")).context("renderer did not produce a report")?,
        )?;
        anyhow::ensure!(
            renderer["schema_version"] == 2 && renderer["input_rgba_sha256"] == INPUT_HASH,
            "unexpected renderer schema/input"
        );
        if !development {
            anyhow::ensure!(
                renderer["os"] == "windows"
                    && renderer["backend"] == "Dx12"
                    && renderer["adapter"] == "AMD Radeon RX 6800 XT",
                "renderer did not use reference D3D12 adapter"
            );
        }
        let mut comparisons = Vec::new();
        let mut passed = status.success() && renderer["passed"] == true;
        for name in CASES {
            let actual_path = render.join(format!("{name}.png"));
            let raw = raw_error(
                &fs::read(root.join(REF_DIR).join(format!("{name}.rgba16f.le")))?,
                &fs::read(render.join(format!("{name}.rgba16f.le")))?,
            )?;
            let comparison = compare(
                &pixels16(&root.join(REF_DIR).join(format!("{name}.png")))?,
                &pixels16(&actual_path)?,
                raw,
            )?;
            // Also preserve images if the renderer's independent numerical gate failed.
            if !comparison.passed || !status.success() || renderer["passed"] != true {
                failure_images(&failures, name, &actual_path, &comparison)?;
                fs::copy(
                    render.join(format!("{name}.rgba16f.le")),
                    failures.join(format!("{name}-actual.rgba16f.le")),
                )?;
            }
            passed &= comparison.passed;
            comparisons.push(json!({"name":name,"comparison":comparison}));
        }
        Ok(
            json!({"passed":passed,"normative_reference_result":!development,"mode":if development {"development"} else {"reference"},
            "metric":"CIEDE2000, unit factors, CIELAB D65; appearance over linear black and white",
            "background_order":["black","white"],"mean_delta_e_strictly_less_than":MEAN_LIMIT,"max_delta_e_strictly_less_than":MAX_LIMIT,
            "alpha_max_error_limit":ALPHA_LIMIT,"raw_linear_max_error_limit":LINEAR_LIMIT,
            "host":host,"renderer":renderer,"comparisons":comparisons,"approved_proposal_commit":references["approved_proposal_commit"],
            "reference_manifest_sha256":hash(&fs::read(root.join(REF_DIR).join("approved.json"))?),
            "harness_source_sha256":hash(include_bytes!("golden.rs")),"metric_source_sha256":hash(include_bytes!("golden/metric.rs")),
            "failure_directory":if passed {Value::Null} else {json!(failures)},"golden_references_updated":false,"phase_gate_passed":false}),
        )
    })();
    match result {
        Ok(report) => {
            fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
            anyhow::ensure!(
                report["passed"] == true,
                "golden comparison failed; see {} and {}",
                output.display(),
                failures.display()
            );
            tracing::info!(path = %output.display(), normative = !development, "five colour/alpha goldens passed; Phase 1 remains open");
            Ok(())
        }
        Err(error) => {
            fs::create_dir_all(&failures)?;
            // A malformed/missing readback cannot have a valid difference image; preserve what exists.
            for name in CASES {
                for extension in ["png", "rgba16f.le"] {
                    let path = render.join(format!("{name}.{extension}"));
                    if path.is_file() {
                        fs::copy(path, failures.join(format!("{name}-actual.{extension}")))?;
                    }
                }
            }
            let report = json!({"passed":false,"normative_reference_result":!development,"error":format!("{error:#}"),"failure_directory":failures});
            fs::write(
                output.join("report.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
            fs::write(
                failures.join("error.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_input_and_installed_references_match_approval() {
        assert_eq!(hash(&input_pixels()), INPUT_HASH);
        validate_references(&super::super::root()).unwrap();
    }
    #[test]
    fn wrong_colour_alpha_only_and_raw_regressions_fail() {
        let expected = [[65535; 4]];
        assert!(compare(&expected, &expected, 0.).unwrap().passed);
        assert!(!compare(&expected, &[[0, 0, 0, 65535]], 0.).unwrap().passed);
        assert!(
            !compare(&expected, &[[65535, 65535, 65535, 0]], 0.)
                .unwrap()
                .passed
        );
        assert!(!compare(&expected, &expected, 0.003).unwrap().passed);
        let alpha_only = compare(&expected, &[[65535, 65535, 65535, 65335]], 0.).unwrap();
        assert!(alpha_only.mean_delta_e00.into_iter().all(|d| d < 1.0));
        assert!(alpha_only.max_delta_e00.into_iter().all(|d| d < 3.0));
        assert!(alpha_only.max_alpha_error > ALPHA_LIMIT && !alpha_only.passed);
        assert!(!bounds([1., 0.], [0.; 2], 0., 0.));
        assert!(!bounds([0.; 2], [0., 3.], 0., 0.));
        assert!(bounds([0.999; 2], [2.999; 2], 0.002, 0.002));
    }
    #[test]
    fn tampered_reference_fails_before_rendering() {
        let root = std::env::temp_dir().join(format!("rezie-reference-{}", unique_id().unwrap()));
        let source = super::super::root().join(REF_DIR);
        let dest = root.join(REF_DIR);
        fs::create_dir_all(&dest).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let path = entry.unwrap().path();
            fs::copy(&path, dest.join(path.file_name().unwrap())).unwrap();
        }
        let path = dest.join("black.png");
        let mut data = fs::read(&path).unwrap();
        data[20] ^= 1;
        fs::write(path, data).unwrap();
        assert!(validate_references(&root)
            .unwrap_err()
            .to_string()
            .contains("was changed"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failure_artifacts_keep_png16_and_make_alpha_visible() {
        let temp = std::env::temp_dir().join(format!("rezie-golden-{}", unique_id().unwrap()));
        fs::create_dir(&temp).unwrap();
        let reference = vec![[65535; 4]; (WIDTH * HEIGHT) as usize];
        let actual = vec![[12345_u16, 23456, 34567, 0]; reference.len()];
        let data: Vec<u8> = actual
            .iter()
            .flat_map(|p| p.iter().flat_map(|v| v.to_be_bytes()))
            .collect();
        let path = temp.join("actual.png");
        write_png(&path, &data, png::BitDepth::Sixteen).unwrap();
        assert_eq!(pixels16(&path).unwrap(), actual);
        let c = compare(&reference, &actual, 0.).unwrap();
        failure_images(&temp, "alpha", &path, &c).unwrap();
        assert_eq!(
            fs::read(temp.join("alpha-actual.png")).unwrap(),
            fs::read(path).unwrap()
        );
        assert!(pixels16(&temp.join("alpha-alpha-difference.png"))
            .unwrap()
            .iter()
            .all(|p| *p == [65535; 4]));
        assert!(temp.join("alpha-difference.png").is_file());
        fs::remove_dir_all(temp).unwrap();
    }
}
