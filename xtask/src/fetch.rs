use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
    process::Command,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub from_phase: u8,
    pub platform: String,
    pub url: String,
    pub sha256: String,
    pub filename: String,
    pub license: String,
}

impl Dependency {
    fn validate(&self) -> Result<()> {
        let name = self.name.to_ascii_lowercase();
        anyhow::ensure!(
            !name.contains("ndi"),
            "NDI SDK must never be fetched; install it manually after licence acceptance"
        );
        anyhow::ensure!(
            !name.contains("cef") || self.from_phase >= 10,
            "CEF cannot be fetched before Phase 10"
        );
        anyhow::ensure!(
            self.from_phase <= 11,
            "invalid consuming phase for '{}'",
            self.name
        );
        anyhow::ensure!(
            self.sha256.len() == 64 && self.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid SHA-256 for '{}'",
            self.name
        );
        anyhow::ensure!(
            self.url.starts_with("https://"),
            "dependency '{}' requires HTTPS",
            self.name
        );
        let mut components = Path::new(&self.filename).components();
        anyhow::ensure!(
            matches!(components.next(), Some(Component::Normal(_)))
                && components.next().is_none()
                && !self.filename.contains(['\\', ':']),
            "unsafe cache filename '{}'",
            self.filename
        );
        anyhow::ensure!(
            !self.version.is_empty() && !self.license.is_empty(),
            "version and licence required for '{}'",
            self.name
        );
        Ok(())
    }
}

pub fn eligible(manifest: &Manifest, phase: u8, platform: &str) -> Result<Vec<Dependency>> {
    let mut result = Vec::new();
    let mut names = std::collections::HashSet::new();
    for dependency in &manifest.dependencies {
        dependency.validate()?;
        anyhow::ensure!(
            names.insert(&dependency.filename),
            "duplicate dependency cache filename '{}'",
            dependency.filename
        );
        if dependency.from_phase <= phase
            && (dependency.platform == "all" || dependency.platform == platform)
        {
            result.push(dependency.clone());
        }
    }
    Ok(result)
}

pub fn verify(path: &Path, expected: &str) -> Result<()> {
    let mut input =
        fs::File::open(path).with_context(|| format!("open dependency '{}'", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hash.finalize());
    anyhow::ensure!(
        actual.eq_ignore_ascii_case(expected),
        "SHA-256 mismatch for '{}': expected {expected}, got {actual}",
        path.display()
    );
    Ok(())
}

pub fn fetch(dependency: &Dependency, directory: &Path) -> Result<()> {
    dependency.validate()?;
    fs::create_dir_all(directory)?;
    let destination = directory.join(&dependency.filename);
    if destination.exists() {
        verify(&destination, &dependency.sha256)?;
        tracing::info!(dependency = %dependency.name, "verified cached dependency");
        return Ok(());
    }
    let partial = directory.join(format!(
        "{}.{}.partial",
        dependency.filename,
        std::process::id()
    ));
    let result = (|| {
        let status = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" })
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "15",
                "--max-time",
                "600",
                "--retry",
                "2",
                "--output",
            ])
            .arg(&partial)
            .arg(&dependency.url)
            .status()
            .context("run HTTPS dependency downloader (curl)")?;
        anyhow::ensure!(
            status.success(),
            "download '{}' from '{}' failed with {status}",
            dependency.name,
            dependency.url
        );
        verify(&partial, &dependency.sha256)?;
        fs::rename(&partial, &destination)?;
        tracing::info!(dependency = %dependency.name, version = %dependency.version, "downloaded and hash-verified dependency");
        Ok(())
    })();
    if partial.exists() {
        let _ = fs::remove_file(&partial);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> Manifest {
        serde_json::from_str(include_str!("../dependencies.json")).unwrap()
    }
    #[test]
    fn phase_zero_selects_real_consumed_dependency_and_never_ffmpeg() {
        let selected = eligible(&manifest(), 0, "windows-x86_64").unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "crossbeam-channel");
        assert_eq!(eligible(&manifest(), 1, "windows-x86_64").unwrap().len(), 2);
        assert_eq!(eligible(&manifest(), 1, "macos-aarch64").unwrap().len(), 1);
    }
    #[test]
    fn ndi_and_early_cef_cannot_bypass_manifest_policy() {
        let mut m = manifest();
        m.dependencies[0].name = "ndi-sdk".into();
        assert!(eligible(&m, 11, "all").is_err());
        m.dependencies[0].name = "cef".into();
        assert!(eligible(&m, 0, "all").is_err());
        m.dependencies[0].from_phase = 10;
        assert!(eligible(&m, 0, "all").unwrap().is_empty());
    }
    #[test]
    fn tampering_is_rejected_and_no_unsafe_filename_is_accepted() {
        let directory =
            std::env::temp_dir().join(format!("rezie-fetch-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let file = directory.join("corrupt");
        fs::write(&file, b"tampered").unwrap();
        assert!(verify(&file, &manifest().dependencies[0].sha256).is_err());
        let mut d = manifest().dependencies[0].clone();
        d.filename = "../escape".into();
        assert!(d.validate().is_err());
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn real_dependency_is_present_and_verified_after_fetch_deps() {
        let d = &manifest().dependencies[0];
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../.deps")
            .join(&d.filename);
        verify(&path, &d.sha256).expect("run cargo xtask fetch-deps before workspace tests");
    }
}
