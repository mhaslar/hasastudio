//! Headless Foundation engine: authoritative control, clock ticks and bounded dispatch.
#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod benchmark;
mod engine;
pub mod logging;
mod sink;
pub use engine::{Engine, EngineConfig, EngineError};
pub use sink::{tick_sink, TickConsumer, TickProducer};
