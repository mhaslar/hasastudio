//! Shared, fail-closed policy for actual libavcodec properties.

/// FFmpeg 7.x's libavcodec ABI major, not the FFmpeg product version.
pub const CODEC_MAJOR: u32 = 61;

/// Reject unapproved ABI or licence/configuration before native media use.
pub fn validate(version: u32, configuration: &str, licence: &str) -> Result<(), String> {
    let major = version >> 16;
    let forbidden =
        configuration.contains("--enable-gpl") || configuration.contains("--enable-nonfree");
    if major != CODEC_MAJOR || forbidden || !licence.starts_with("LGPL version ") {
        return Err(format!(
            "libavcodec rejected: actual {}.{}.{} (expected major {CODEC_MAJOR}); \
             licence {licence:?}; configuration {configuration:?}. \
             Rezie requires dynamically linked LGPL FFmpeg 7.x without \
             --enable-gpl or --enable-nonfree. Run cargo xtask native-deps and \
             activate .deps/native-env.sh (Unix) or .deps/native-env.ps1 (Windows).",
            major,
            (version >> 8) & 255,
            version & 255,
        ));
    }
    Ok(())
}
