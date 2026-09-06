//! Native-GPU numerical diagnostic; outputs are not approved golden references.
#![forbid(unsafe_code)]
use anyhow::{Context, Result};
use rezie_gpu::{FrameKey, FramePool, GpuContext, COLOUR_SHADER};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

const WIDTH: u32 = 257;
const HEIGHT: u32 = 65;
const LINEAR_LIMIT: f64 = 0.002;
const PNG_EGRESS_LIMIT: u16 = 2;

fn linear(v: u8) -> f64 {
    let v = f64::from(v) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn expected(fg: [u8; 4], bg: [u8; 4]) -> [f64; 4] {
    let a = f64::from(fg[3]) / 255.0;
    let b = f64::from(bg[3]) / 255.0;
    [
        linear(fg[0]) * a + linear(bg[0]) * b * (1.0 - a),
        linear(fg[1]) * a + linear(bg[1]) * b * (1.0 - a),
        linear(fg[2]) * a + linear(bg[2]) * b * (1.0 - a),
        a + b * (1.0 - a),
    ]
}
fn exported(p: [f64; 4]) -> [u16; 4] {
    let mut out = [0; 4];
    for i in 0..3 {
        let v = if p[3] > 0.0 { p[i] / p[3] } else { 0.0 };
        let srgb = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        out[i] = (srgb.clamp(0.0, 1.0) * 65535.0).round() as u16;
    }
    out[3] = (p[3].clamp(0.0, 1.0) * 65535.0).round() as u16;
    out
}

fn fixture() -> Vec<u8> {
    let mut bytes = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let ramp = x.min(255) as u8;
            let pixel = match y / 8 {
                0 => [255, 255, 255, 128],
                1 => [0, 0, 0, 128],
                2 => [255, 0, 255, 0], // hidden colour must contribute nothing
                3 => [ramp, 255 - ramp, 73, 255],
                4 => [255, 255, 255, ramp],
                5 => [231, 73, 19, ramp],
                6 => [ramp, ramp, ramp, 128],
                7 => [ramp, 0, 255 - ramp, 1],
                _ => [10, 11, 12, 255], // straddle the sRGB transfer breakpoint
            };
            bytes.extend_from_slice(&pixel);
        }
    }
    bytes
}

fn write_png(path: &Path, bytes: &[u8], depth: png::BitDepth) -> Result<()> {
    let file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .with_context(|| format!("create PNG {}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(depth);
    encoder.set_source_srgb(png::SrgbRenderingIntent::RelativeColorimetric);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(bytes)?;
    writer.finish()?;
    Ok(())
}
fn read_fixture(path: &Path) -> Result<Vec<u8>> {
    let mut reader = png::Decoder::new(BufReader::new(fs::File::open(path)?)).read_info()?;
    let mut bytes = vec![
        0;
        reader
            .output_buffer_size()
            .context("PNG output size overflow")?
    ];
    let info = reader.next_frame(&mut bytes)?;
    anyhow::ensure!(
        info.width == WIDTH
            && info.height == HEIGHT
            && info.color_type == png::ColorType::Rgba
            && info.bit_depth == png::BitDepth::Eight
            && reader.info().srgb.is_some(),
        "fixture must be 257x65 RGBA8 sRGB"
    );
    bytes.truncate(info.buffer_size());
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .map_err(|e| anyhow::anyhow!("initialize colour-check logging: {e}"))?;
    let mut args = std::env::args().skip(1);
    let output = match args.next().as_deref() {
        Some("--output") => {
            PathBuf::from(args.next().context("--output requires a new directory")?)
        }
        _ => anyhow::bail!("usage: rezie-colour-check --output <new-directory>"),
    };
    anyhow::ensure!(args.next().is_none(), "unexpected colour-check arguments");
    // Atomic create, not exists-then-write: never overwrite earlier reports or candidates.
    fs::create_dir(&output).with_context(|| {
        format!(
            "create NEW output directory {} (parent must exist)",
            output.display()
        )
    })?;
    let gpu = GpuContext::request().await?;
    let adapter = gpu.adapter.get_info();
    anyhow::ensure!(
        adapter.device_type != wgpu::DeviceType::Cpu,
        "native-GPU diagnostic requires hardware; found {}",
        adapter.name
    );
    let source_path = output.join("input-alpha.png");
    write_png(&source_path, &fixture(), png::BitDepth::Eight)?;
    let source = read_fixture(&source_path)?;
    let mut pool = FramePool::new(gpu.device.clone(), 16 * 1024 * 1024);
    let mut cases = Vec::new();
    let mut passed = true;
    for (name, background) in [
        ("black", [0, 0, 0, 255]),
        ("white", [255, 255, 255, 255]),
        ("colour", [31, 153, 219, 255]),
        ("translucent", [31, 153, 219, 96]),
        ("transparent", [255, 0, 255, 0]),
    ] {
        let result = pool
            .check_colour(
                &gpu.queue,
                FrameKey::new(WIDTH, HEIGHT)?,
                &source,
                background,
            )
            .await?;
        anyhow::ensure!(
            result.png_rgba16_be.len() == source.len() * 2
                && result.linear_rgba_f16_le.len() == source.len() * 2
                && result.linear_rgba.len() * 4 == source.len(),
            "readback length mismatch"
        );
        let mut max_linear = 0_f64;
        let mut max_png_egress = 0_u16;
        let mut max_png_ideal = 0_u16;
        let mut bad_pixels = 0_u64;
        let mut first_failure = None;
        for (index, pixel) in source.as_chunks::<4>().0.iter().enumerate() {
            let target = expected([pixel[0], pixel[1], pixel[2], pixel[3]], background);
            let target_png = exported(target);
            let observed = result.linear_rgba[index].map(f64::from);
            let egress_expected = exported(observed);
            let actual_png: [u16; 4] = std::array::from_fn(|c| {
                let offset = (index * 4 + c) * 2;
                u16::from_be_bytes([
                    result.png_rgba16_be[offset],
                    result.png_rgba16_be[offset + 1],
                ])
            });
            let mut bad = false;
            for c in 0..4 {
                let actual = f64::from(result.linear_rgba[index][c]);
                let error = (actual - target[c]).abs();
                let egress_error = actual_png[c].abs_diff(egress_expected[c]);
                let ideal_error = actual_png[c].abs_diff(target_png[c]);
                max_linear = max_linear.max(error);
                max_png_egress = max_png_egress.max(egress_error);
                max_png_ideal = max_png_ideal.max(ideal_error);
                bad |=
                    !actual.is_finite() || error > LINEAR_LIMIT || egress_error > PNG_EGRESS_LIMIT;
            }
            if bad {
                bad_pixels += 1;
                if first_failure.is_none() {
                    first_failure = Some(
                        serde_json::json!({"x": index % WIDTH as usize, "y": index / WIDTH as usize,
                        "expected_linear": target, "actual_linear": result.linear_rgba[index],
                        "ideal_png16": target_png, "egress_expected_png16": egress_expected, "actual_png16": actual_png}),
                    );
                }
            }
        }
        let png_path = output.join(format!("{name}.png"));
        write_png(&png_path, &result.png_rgba16_be, png::BitDepth::Sixteen)?;
        let raw_name = format!("{name}.rgba16f.le");
        fs::write(output.join(&raw_name), &result.linear_rgba_f16_le)?;
        passed &= bad_pixels == 0;
        cases.push(serde_json::json!({"name": name, "background_srgb_rgba": background,
            "pixels_checked": result.linear_rgba.len(), "max_linear_absolute_error": max_linear,
            "max_png16_egress_code_value_error": max_png_egress,
            "max_png16_vs_ideal_code_value_error": max_png_ideal, "failing_pixels": bad_pixels, "first_failure": first_failure,
            "png_rgba16_be_sha256": hash(&result.png_rgba16_be),
            "png_file_sha256": hash(&fs::read(&png_path)?),
            "linear_readback": {"path": raw_name, "sha256": hash(&result.linear_rgba_f16_le),
                "bytes": result.linear_rgba_f16_le.len(), "pixels": result.linear_rgba.len()}}));
    }
    let report = serde_json::json!({
        "scope": "Phase 1 native-GPU numerical PNG alpha diagnostic; not approved goldens or phase acceptance",
        "passed": passed, "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "adapter": adapter.name, "backend": format!("{:?}", adapter.backend), "driver": adapter.driver,
        "driver_info": adapter.driver_info, "width": WIDTH, "height": HEIGHT,
        "working_format": "Rgba16Float linear BT.709 premultiplied", "workgroup_size": [8, 8, 1],
        "schema_version": 2, "input_bit_depth": 8, "output_bit_depth": 16,
        "linear_absolute_error_limit": LINEAR_LIMIT, "png16_egress_code_value_error_limit": PNG_EGRESS_LIMIT,
        "linear_readback_layout": {"format": "IEEE 754 binary16", "byte_order": "little-endian",
            "channels": "RGBA", "order": "row-major, top-to-bottom, left-to-right", "bytes_per_pixel": 8,
            "row_stride_bytes": WIDTH * 8, "padding_bytes": 0},
        "shader_sha256": hash(COLOUR_SHADER.as_bytes()),
        "probe_source_sha256": hash(include_bytes!("../pool/colour.rs")),
        "checker_source_sha256": hash(include_bytes!("rezie-colour-check.rs")),
        "input_rgba_sha256": hash(&source), "cases": cases,
        "golden_references_updated": false, "five_minute_allocation_criterion_evaluated": false,
        "performance_measured": false,
    });
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    anyhow::ensure!(
        passed,
        "colour diagnostic failed; inspect {}/report.json and PNGs",
        output.display()
    );
    tracing::info!(adapter = %adapter.name, path = %output.display(), "colour/alpha numerical diagnostic passed; not golden approval");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oracle_has_analytical_linear_midpoint_and_alpha_cases() {
        let half_white = expected([255, 255, 255, 128], [0, 0, 0, 255]);
        assert_eq!(
            half_white,
            [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0]
        );
        assert_eq!(exported(half_white), [48276, 48276, 48276, 65535]);
        // A change too small for RGBA8 must survive this diagnostic's export.
        assert_ne!(
            exported([0.5, 0.5, 0.5, 1.0]),
            exported([0.5001, 0.5001, 0.5001, 1.0])
        );
        assert_eq!(expected([255, 0, 255, 0], [0, 0, 0, 0]), [0.0; 4]);
        assert_eq!(exported([0.0; 4]), [0; 4]);
        let p = expected([255, 0, 0, 128], [0, 0, 255, 128]);
        assert!((p[3] - (1.0 - (127.0 / 255.0_f64).powi(2))).abs() < 1e-12);
        assert_eq!(
            exported(expected([31, 153, 219, 255], [0; 4])),
            [31 * 257, 153 * 257, 219 * 257, 65535]
        );
    }
}
