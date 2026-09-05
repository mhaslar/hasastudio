//! Shared commands/events and bounded in-process and loopback WebSocket transports.
#![warn(missing_docs)]

use arc_swap::ArcSwap;
use crossbeam_channel::{bounded, Receiver, Sender};
use rezie_core::EngineState;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

mod websocket;
pub use websocket::WebSocketServer;

/// Foundation commands describe absolute intent, never optimistic state deltas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Command {
    /// Request current authoritative state.
    GetState,
    /// Replace the project label.
    SetProjectName {
        /// Nonempty label, at most 128 UTF-8 bytes, without control characters.
        name: String,
    },
    /// Stop the clock and engine.
    Shutdown,
}

/// Correlation envelope shared by both transports.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Client-selected correlation ID.
    pub id: u64,
    /// Complete operator intent.
    pub command: Command,
}

/// What the engine actually did, or why it refused a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// A requested current state.
    State {
        /// Correlated request ID.
        id: u64,
        /// Authoritative state.
        state: Arc<EngineState>,
    },
    /// An accepted mutation and its resulting state.
    Applied {
        /// Correlated request ID.
        id: u64,
        /// State after the command.
        state: Arc<EngineState>,
    },
    /// No mutation occurred.
    Rejected {
        /// Absent only if malformed input had no usable request ID.
        id: Option<u64>,
        /// Specific rejection reason.
        reason: String,
    },
    /// Transport could not confirm an engine outcome; query state when indeterminate.
    TransportError {
        /// Correlated request ID.
        id: u64,
        /// Specific transport failure.
        reason: String,
        /// True when failure does not prove that the command was unapplied.
        may_have_applied: bool,
    },
}

/// A transport failure, distinct from an engine rejection event.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Control backpressure is explicit and bounded.
    #[error("engine command queue is full; command was not submitted")]
    Busy,
    /// Engine or request channel has closed.
    #[error("engine control connection is closed")]
    Closed,
    /// A deadline expired; the command may already have been applied.
    #[error("engine response timed out; query state before retrying")]
    Timeout,
    /// Listener address is not permitted.
    #[error("WebSocket address {0} is not loopback")]
    NonLocal(std::net::SocketAddr),
    /// Network setup failed.
    #[error("WebSocket transport I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Runtime task failed.
    #[error("WebSocket task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

/// One bounded work item consumed only by the engine control thread.
pub struct Envelope {
    /// Original correlated command.
    pub request: Request,
    /// Bounded one-event reply channel; control uses try_send.
    pub reply: Sender<Event>,
}

/// A client handle; it owns no mutable production state.
#[derive(Clone)]
pub struct Client {
    commands: Sender<Envelope>,
    state: Arc<ArcSwap<EngineState>>,
}

/// Allocate control channels before engine threads start.
pub fn channel(
    state: Arc<ArcSwap<EngineState>>,
    capacity: usize,
) -> Result<(Client, Receiver<Envelope>), ApiError> {
    if capacity == 0 {
        return Err(ApiError::Busy);
    }
    let (commands, receiver) = bounded(capacity);
    Ok((Client { commands, state }, receiver))
}

impl Client {
    /// Read the last engine-published snapshot without optimistic mutation.
    pub fn snapshot(&self) -> Arc<EngineState> {
        self.state.load_full()
    }

    /// Submit without waiting; a full queue fails explicitly.
    pub fn submit(&self, request: Request) -> Result<Receiver<Event>, ApiError> {
        let (reply, receiver) = bounded(1);
        self.commands
            .try_send(Envelope { request, reply })
            .map_err(|error| match error {
                crossbeam_channel::TrySendError::Full(_) => ApiError::Busy,
                crossbeam_channel::TrySendError::Disconnected(_) => ApiError::Closed,
            })?;
        Ok(receiver)
    }

    /// Wait for a correlated reply outside all media/control threads.
    pub fn request(&self, request: Request, timeout: Duration) -> Result<Event, ApiError> {
        self.submit(request)?
            .recv_timeout(timeout)
            .map_err(|error| match error {
                crossbeam_channel::RecvTimeoutError::Timeout => ApiError::Timeout,
                crossbeam_channel::RecvTimeoutError::Disconnected => ApiError::Closed,
            })
    }

    /// Poll the same bounded reply from the async control-plane runtime.
    pub async fn request_async(&self, request: Request) -> Result<Event, ApiError> {
        let reply = self.submit(request)?;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match reply.try_recv() {
                    Ok(event) => return Ok(event),
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        return Err(ApiError::Closed)
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        tokio::time::sleep(Duration::from_millis(1)).await
                    }
                }
            }
        })
        .await
        .map_err(|_| ApiError::Timeout)?
    }
}
