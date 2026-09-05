use crate::{ApiError, Client, Event, Request};
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{oneshot, watch, Semaphore},
    task::{JoinHandle, JoinSet},
};
use tokio_tungstenite::{
    accept_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};
use tracing::Instrument;

/// Owned loopback listener with bounded connections and joined cancellation.
pub struct WebSocketServer {
    address: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl WebSocketServer {
    /// Bind a local address; port zero requests an available port for tests.
    pub async fn bind(address: SocketAddr, client: Client) -> Result<Self, ApiError> {
        if !address.ip().is_loopback() {
            return Err(ApiError::NonLocal(address));
        }
        let listener = TcpListener::bind(address).await?;
        let address = listener.local_addr()?;
        let (shutdown, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let permits = Arc::new(Semaphore::new(16));
            let mut connections = JoinSet::new();
            let (cancel, cancelled) = watch::channel(false);
            loop {
                tokio::select! {
                    _ = &mut stopped => break,
                    _ = connections.join_next(), if !connections.is_empty() => {},
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, peer)) => {
                                let Ok(permit) = permits.clone().try_acquire_owned() else { continue; };
                                let client = client.clone();
                                let cancelled = cancelled.clone();
                                connections.spawn(async move {
                                    let _permit = permit;
                                    if let Err(error) = connection(stream, client, cancelled).await {
                                        tracing::debug!(%error, "WebSocket connection ended");
                                    }
                                }.instrument(tracing::info_span!("websocket_client", %peer)));
                            }
                            Err(error) => { tracing::error!(%error, "WebSocket accept failed"); break; }
                        }
                    }
                }
            }
            let _ = cancel.send(true);
            let drained = tokio::time::timeout(Duration::from_secs(6), async {
                while connections.join_next().await.is_some() {}
            }).await;
            if drained.is_err() { connections.abort_all(); }
            while connections.join_next().await.is_some() {}
        }.instrument(tracing::info_span!("websocket_listener", %address)));
        Ok(Self {
            address,
            shutdown: Some(shutdown),
            task: Some(task),
        })
    }

    /// Actual bound address.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Cancel listener and connections and wait for their completion.
    pub async fn shutdown(mut self) -> Result<(), ApiError> {
        if let Some(stop) = self.shutdown.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            task.await?;
        }
        Ok(())
    }
}

impl Drop for WebSocketServer {
    fn drop(&mut self) {
        if let Some(stop) = self.shutdown.take() {
            let _ = stop.send(());
        }
    }
}

async fn connection(
    stream: TcpStream,
    client: Client,
    mut cancelled: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(64 * 1024))
        .max_frame_size(Some(64 * 1024));
    let mut socket = tokio::time::timeout(
        Duration::from_secs(5),
        accept_async_with_config(stream, Some(config)),
    )
    .await??;
    loop {
        let message = tokio::select! {
            biased;
            _ = cancelled.changed() => break,
            message = socket.next() => match message { Some(message) => message, None => break },
        };
        let event = match message? {
            Message::Text(text) => match serde_json::from_str::<Request>(&text) {
                Ok(request) => {
                    let id = request.id;
                    client.request_async(request).await.unwrap_or_else(|error| {
                        Event::TransportError {
                            id,
                            may_have_applied: !matches!(error, ApiError::Busy),
                            reason: error.to_string(),
                        }
                    })
                }
                Err(error) => Event::Rejected {
                    id: None,
                    reason: format!("invalid JSON command: {error}"),
                },
            },
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {
                socket.flush().await?;
                continue;
            }
            _ => Event::Rejected {
                id: None,
                reason: "expected a JSON text command".into(),
            },
        };
        let text = serde_json::to_string(&event)?;
        tokio::time::timeout(
            Duration::from_secs(5),
            socket.send(Message::Text(text.into())),
        )
        .await??;
    }
    Ok(())
}
