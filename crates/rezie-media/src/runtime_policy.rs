//! Queries process-linked symbols, independently of the build-time probe.
use std::ffi::{c_char, CStr};

extern "C" {
    fn avcodec_version() -> u32;
    fn avcodec_configuration() -> *const c_char;
    fn avcodec_license() -> *const c_char;
}

/// Properties copied from the library loaded into this process.
#[derive(Debug, Clone)]
pub struct NativeLibrary {
    /// libavcodec's packed major/minor/micro version.
    pub version: u32,
    /// Actual build configuration, not a filename or external executable.
    pub configuration: String,
    /// Actual licence version.
    pub licence: String,
}

/// Mandatory check before codec initialization or engine thread startup.
pub fn check() -> Result<NativeLibrary, String> {
    // SAFETY: FFmpeg exports these no-argument ABI functions on supported majors.
    // Returned strings are library-owned, NUL-terminated and live while loaded.
    // No FFmpeg structs are touched before the ABI major has been checked.
    let (version, configuration, licence) = unsafe {
        let configuration = avcodec_configuration();
        let licence = avcodec_license();
        if configuration.is_null() || licence.is_null() {
            return Err("libavcodec rejected: null configuration/licence string".into());
        }
        (
            avcodec_version(),
            CStr::from_ptr(configuration).to_string_lossy().into_owned(),
            CStr::from_ptr(licence).to_string_lossy().into_owned(),
        )
    };
    super::policy::validate(version, &configuration, &licence)?;
    Ok(NativeLibrary {
        version,
        configuration,
        licence,
    })
}
