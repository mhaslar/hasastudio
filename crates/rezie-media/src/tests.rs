use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[test]
fn actual_loaded_library_is_approved() {
    let native = initialize().unwrap();
    assert_eq!(native.version >> 16, 61);
    assert!(!native.configuration.contains("--enable-gpl"));
    assert!(!native.configuration.contains("--enable-nonfree"));
    assert!(native.licence.starts_with("LGPL version "));
}

#[test]
fn software_decode_matches_independent_pixels_pts_and_drains_eof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/assets/phase-1/decode");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for case in manifest["files"].as_array().unwrap() {
        let path = root.join(case["file"].as_str().unwrap());
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            case["sha256"]
        );
        let mut decoder = FileDecoder::open(&path, DecodeMode::Software).unwrap();
        let mut pictures = Vec::new();
        while let Some(picture) = decoder.next_picture().unwrap() {
            assert_eq!(picture.index, pictures.len() as u64);
            let record = picture.inspect().unwrap();
            pictures.push(json!({"pts":record.pts,"time_base":record.time_base,
                "component_sha256":record.component_sha256,"component_depth":record.component_depth,
                "dimensions":record.dimensions}));
        }
        assert_eq!(json!(pictures), case["pictures"], "{}", path.display());
        assert!(decoder.next_picture().unwrap().is_none());
        assert!(!decoder.status().hardware_frame_context_observed);
        assert_eq!(decoder.status().hardware_device, None);
        if case["file"].as_str().unwrap().starts_with("av1") {
            assert_eq!(decoder.status().decoder, "libdav1d");
        }
    }
}

#[test]
fn invalid_input_paths_are_errors_not_panics() {
    for path in ["missing-rezie-media-input.mp4", "embedded\0nul.mp4"] {
        assert!(FileDecoder::open(path, DecodeMode::Software).is_err());
    }
    let malformed = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    assert!(FileDecoder::open(malformed, DecodeMode::Software).is_err());
}
