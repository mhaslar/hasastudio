//! Platform-independent production data and programme timeline arithmetic.
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod clock;
mod domain;

pub use clock::*;
pub use domain::*;
