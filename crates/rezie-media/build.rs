//! Fail the build when its actual selected libavcodec violates policy.
#[path = "src/policy.rs"]
mod policy;

use std::{env, ffi::CStr, path::PathBuf};

fn check() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("HOST")? != env::var("TARGET")? {
        return Err("native libavcodec validation requires a native build; cross compilation cannot run the target guard".into());
    }
    for key in [
        "FFMPEG_DIR",
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_LIBDIR",
        "PKG_CONFIG",
        "VCPKG_ROOT",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/policy.rs");
    let directory = if cfg!(windows) {
        PathBuf::from(env::var_os("FFMPEG_DIR").ok_or(
            "FFMPEG_DIR is required; run cargo xtask native-deps, then . .\\.deps\\native-env.ps1",
        )?)
        .join("bin")
    } else {
        if env::var_os("FFMPEG_DIR").is_some() || env::var_os("VCPKG_ROOT").is_some() {
            return Err(
                "Unix FFmpeg must resolve through pkg-config, not FFMPEG_DIR/VCPKG_ROOT".into(),
            );
        }
        let pc = PathBuf::from(pkg_config::get_variable("libavcodec", "pcfiledir")?)
            .join("libavcodec.pc");
        println!("cargo:rerun-if-changed={}", pc.display());
        PathBuf::from(pkg_config::get_variable("libavcodec", "libdir")?)
    };
    println!("cargo:rerun-if-changed={}", directory.display());
    let path = directory
        .join(if cfg!(windows) {
            "avcodec-61.dll"
        } else if cfg!(target_os = "macos") {
            "libavcodec.dylib"
        } else {
            "libavcodec.so"
        })
        .canonicalize()?;
    println!("cargo:rerun-if-changed={}", path.display());
    // SAFETY: the user-approved native dependency is loaded for inspection only.
    // Its handle outlives every symbol/string access; no frame ABI is used here.
    let library: libloading::Library = unsafe {
        #[cfg(windows)]
        {
            libloading::os::windows::Library::load_with_flags(&path, 0x100 | 0x1000)?.into()
        }
        #[cfg(not(windows))]
        {
            libloading::Library::new(&path)?
        }
    };
    // SAFETY: these are FFmpeg's stable exported function signatures. Strings
    // are copied while the library handle remains alive, with null checks.
    let (version, configuration, licence) = unsafe {
        let version: libloading::Symbol<unsafe extern "C" fn() -> u32> =
            library.get(b"avcodec_version\0")?;
        type Text = unsafe extern "C" fn() -> *const std::ffi::c_char;
        let configuration: libloading::Symbol<Text> = library.get(b"avcodec_configuration\0")?;
        let licence: libloading::Symbol<Text> = library.get(b"avcodec_license\0")?;
        let (config, licence) = (configuration(), licence());
        if config.is_null() || licence.is_null() {
            return Err("null native licence/configuration".into());
        }
        (
            version(),
            CStr::from_ptr(config).to_string_lossy().into_owned(),
            CStr::from_ptr(licence).to_string_lossy().into_owned(),
        )
    };
    policy::validate(version, &configuration, &licence)
        .map_err(|e| format!("build-time probe of '{}': {e}", path.display()))?;
    println!(
        "cargo:rustc-env=REZIE_BUILD_AVCODEC_PATH={}",
        path.display()
    );
    println!("cargo:rustc-env=REZIE_BUILD_AVCODEC_VERSION={version}");
    println!("cargo:rustc-env=REZIE_BUILD_AVCODEC_CONFIGURATION={configuration}");
    println!("cargo:rustc-env=REZIE_BUILD_AVCODEC_LICENCE={licence}");
    Ok(())
}

fn main() {
    if let Err(error) = check() {
        eprintln!("Rezie native dependency validation FAILED: {error}");
        std::process::exit(1);
    }
}
