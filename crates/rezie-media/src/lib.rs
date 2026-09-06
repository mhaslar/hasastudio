//! File decode on its owning worker, with fail-closed native dependency policy.
#![warn(missing_docs)]

mod decode;
mod inspect;
pub use inspect::PictureRecord;
mod policy;
pub use decode::{DecodeMode, DecodeStatus, DecodedPicture, FileDecoder};
mod runtime_policy;

pub use runtime_policy::NativeLibrary;

/// Check the process-linked library before calling any media initialization API.
pub fn initialize() -> Result<NativeLibrary, MediaError> {
    let library = runtime_policy::check().map_err(MediaError::NativePolicy)?;
    ffmpeg_next::init().map_err(|e| MediaError::NativePolicy(format!("initialize FFmpeg: {e}")))?;
    Ok(library)
}

/// Specific, actionable native or media failure.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// A forbidden build or incompatible ABI was found.
    #[error("{0}")]
    NativePolicy(String),
    /// An input cannot be opened or decoded.
    #[error("input '{path}': {operation}: {detail}")]
    Input {
        /// User-selected path.
        path: String,
        /// Failed operation.
        operation: &'static str,
        /// Native or validation error.
        detail: String,
    },
}

#[cfg(test)]
mod tests;
