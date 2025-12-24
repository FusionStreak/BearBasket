//! TCP transport layer for peer-to-peer synchronization.
//!
//! This module provides a simple TCP-based transport for exchanging CRDT
//! documents between peers. It handles connection management, message framing,
//! and the sync handshake.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::discovery::Peer;
use crate::error::{Result, SyncError};
use crate::protocol::{
    HelloMessage, ListMetadata, MAX_MESSAGE_SIZE, PROTOCOL_VERSION, SyncMessage,
    SyncRequestMessage, SyncResponseMessage,
};

/// Connection timeout for establishing connections.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Read/write timeout for message exchange.
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Callback type for handling sync requests.
pub type SyncHandler =
    Box<dyn Fn(SyncRequestMessage) -> Result<SyncResponseMessage> + Send + Sync + 'static>;

/// Callback type for getting list metadata.
pub type ListInventoryHandler = Box<dyn Fn() -> Vec<ListMetadata> + Send + Sync + 'static>;

/// Callback type for applying received updates.
pub type UpdateHandler = Box<dyn Fn(String, Vec<u8>) -> Result<()> + Send + Sync + 'static>;

/// Context for handling incoming connections.
struct ConnectionContext {
    our_device_id: String,
    our_device_name: String,
    inventory_handler: Arc<ListInventoryHandler>,
    sync_handler: Arc<SyncHandler>,
    update_handler: Arc<UpdateHandler>,
    connections: Arc<RwLock<HashMap<String, SocketAddr>>>,
    event_tx: mpsc::Sender<TransportEvent>,
}

/// Events emitted by the transport layer.
#[derive(Debug)]
pub enum TransportEvent {
    /// A peer connected to us.
    PeerConnected { device_id: String, addr: SocketAddr },
    /// A peer disconnected.
    PeerDisconnected { device_id: String },
    /// Received a sync request from a peer.
    SyncRequested {
        peer_device_id: String,
        list_id: String,
    },
    /// Received list data from a peer.
    ListReceived {
        peer_device_id: String,
        list_id: String,
        crdt_data: Vec<u8>,
    },
    /// An error occurred with a connection.
    ConnectionError {
        peer_device_id: String,
        error: String,
    },
}

/// TCP transport for sync protocol.
pub struct Transport {
    /// Our device ID.
    device_id: String,
    /// Our device name.
    device_name: String,
    /// Port we're listening on.
    port: u16,
    /// Active connections by device_id.
    connections: Arc<RwLock<HashMap<String, SocketAddr>>>,
    /// Shutdown signal sender.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Event sender.
    event_tx: Option<mpsc::Sender<TransportEvent>>,
}

impl Transport {
    /// Creates a new transport.
    pub fn new(device_id: String, device_name: String, port: u16) -> Self {
        Self {
            device_id,
            device_name,
            port,
            connections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
            event_tx: None,
        }
    }

    /// Starts listening for incoming connections.
    ///
    /// Returns a channel for receiving transport events.
    pub async fn start(
        &mut self,
        sync_handler: SyncHandler,
        inventory_handler: ListInventoryHandler,
        update_handler: UpdateHandler,
    ) -> Result<mpsc::Receiver<TransportEvent>> {
        let listener = TcpListener::bind(("0.0.0.0", self.port)).await?;
        let actual_port = listener.local_addr()?.port();
        info!(port = actual_port, "Transport listening on TCP");

        let (event_tx, event_rx) = mpsc::channel(100);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        self.event_tx = Some(event_tx.clone());
        self.shutdown_tx = Some(shutdown_tx);

        let device_id = self.device_id.clone();
        let device_name = self.device_name.clone();
        let connections = Arc::clone(&self.connections);

        let sync_handler = Arc::new(sync_handler);
        let inventory_handler = Arc::new(inventory_handler);
        let update_handler = Arc::new(update_handler);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                info!(addr = %addr, "Incoming connection");
                                let event_tx = event_tx.clone();
                                let connections = Arc::clone(&connections);
                                let device_id = device_id.clone();
                                let device_name = device_name.clone();
                                let sync_handler = Arc::clone(&sync_handler);
                                let inventory_handler = Arc::clone(&inventory_handler);
                                let update_handler = Arc::clone(&update_handler);

                                tokio::spawn(async move {
                                    let ctx = ConnectionContext {
                                        our_device_id: device_id,
                                        our_device_name: device_name,
                                        inventory_handler,
                                        sync_handler,
                                        update_handler,
                                        connections,
                                        event_tx,
                                    };
                                    if let Err(e) = handle_incoming_connection(stream, addr, ctx).await {
                                        error!(error = %e, "Connection handler error");
                                    }
                                });
                            }
                            Err(e) => {
                                error!(error = %e, "Accept error");
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        info!("Transport shutting down");
                        break;
                    }
                }
            }
        });

        Ok(event_rx)
    }

    /// Connects to a peer and syncs a specific list.
    pub async fn sync_with_peer(
        &self,
        peer: &Peer,
        list_id: &str,
        get_local_crdt: impl FnOnce() -> Result<(String, Option<String>, Vec<u8>)>,
        available_lists: Vec<String>,
    ) -> Result<SyncResponseMessage> {
        let addr = peer
            .socket_addr()
            .ok_or_else(|| SyncError::Connection("No address for peer".into()))?;

        info!(peer = %peer.name, addr = %addr, list = %list_id, "Connecting to peer for sync");

        // Connect with timeout
        let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(addr))
            .await
            .map_err(|_| SyncError::Timeout("Connection timeout".into()))?
            .map_err(SyncError::Io)?;

        let mut stream = stream;

        // Send hello
        let hello = SyncMessage::Hello(HelloMessage::new(
            self.device_id.clone(),
            self.device_name.clone(),
            available_lists,
        ));
        send_message(&mut stream, &hello).await?;

        // Wait for their hello
        let response = recv_message(&mut stream).await?;
        let peer_hello = match response {
            SyncMessage::Hello(h) => h,
            SyncMessage::Error(e) => return Err(SyncError::Protocol(e.message)),
            _ => return Err(SyncError::Protocol("Expected Hello message".into())),
        };

        if peer_hello.version != PROTOCOL_VERSION {
            return Err(SyncError::Protocol(format!(
                "Version mismatch: {} vs {}",
                peer_hello.version, PROTOCOL_VERSION
            )));
        }

        debug!(peer_device = %peer_hello.device_id, "Handshake complete");

        // Request the list
        let request = SyncMessage::SyncRequest(SyncRequestMessage::full(list_id.to_string()));
        send_message(&mut stream, &request).await?;

        // Wait for response
        let response = recv_message(&mut stream).await?;
        let sync_response = match response {
            SyncMessage::SyncResponse(r) => r,
            SyncMessage::Error(e) => return Err(SyncError::Protocol(e.message)),
            _ => return Err(SyncError::Protocol("Expected SyncResponse message".into())),
        };

        // Send our version back
        let (name, store, crdt_data) = get_local_crdt()?;
        let our_response = SyncMessage::SyncResponse(SyncResponseMessage {
            list_id: list_id.to_string(),
            name,
            store,
            crdt_data,
            is_full: true,
            timestamp: chrono_timestamp(),
        });
        send_message(&mut stream, &our_response).await?;

        // Send goodbye
        send_message(&mut stream, &SyncMessage::Goodbye).await?;

        info!(peer = %peer.name, list = %list_id, "Sync complete");
        Ok(sync_response)
    }

    /// Returns the number of active connections.
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Checks if we're connected to a specific peer.
    pub async fn is_connected(&self, device_id: &str) -> bool {
        self.connections.read().await.contains_key(device_id)
    }

    /// Shuts down the transport.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        info!("Transport shut down");
        Ok(())
    }
}

/// Handles an incoming connection.
async fn handle_incoming_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    ctx: ConnectionContext,
) -> Result<()> {
    let ConnectionContext {
        our_device_id,
        our_device_name,
        inventory_handler,
        sync_handler,
        update_handler,
        connections,
        event_tx,
    } = ctx;
    // Wait for their hello
    let message = recv_message(&mut stream).await?;
    let peer_hello = match message {
        SyncMessage::Hello(h) => h,
        _ => {
            let err = SyncMessage::error(
                crate::protocol::ErrorCode::InvalidMessage,
                "Expected Hello",
                None,
            );
            send_message(&mut stream, &err).await?;
            return Err(SyncError::Protocol("Expected Hello message".into()));
        }
    };

    if peer_hello.version != PROTOCOL_VERSION {
        let err = SyncMessage::error(
            crate::protocol::ErrorCode::VersionMismatch,
            format!(
                "Version mismatch: {} vs {}",
                peer_hello.version, PROTOCOL_VERSION
            ),
            None,
        );
        send_message(&mut stream, &err).await?;
        return Err(SyncError::Protocol("Version mismatch".into()));
    }

    // Send our hello
    let our_lists: Vec<String> = inventory_handler().iter().map(|m| m.id.clone()).collect();
    let our_hello = SyncMessage::Hello(HelloMessage::new(
        our_device_id.clone(),
        our_device_name.clone(),
        our_lists,
    ));
    send_message(&mut stream, &our_hello).await?;

    // Track connection
    {
        let mut conns = connections.write().await;
        conns.insert(peer_hello.device_id.clone(), addr);
    }

    let _ = event_tx
        .send(TransportEvent::PeerConnected {
            device_id: peer_hello.device_id.clone(),
            addr,
        })
        .await;

    let peer_device_id = peer_hello.device_id.clone();

    // Message loop
    loop {
        match recv_message(&mut stream).await {
            Ok(message) => match message {
                SyncMessage::SyncRequest(req) => {
                    debug!(list = %req.list_id, "Received sync request");

                    let _ = event_tx
                        .send(TransportEvent::SyncRequested {
                            peer_device_id: peer_device_id.clone(),
                            list_id: req.list_id.clone(),
                        })
                        .await;

                    match sync_handler(req) {
                        Ok(response) => {
                            send_message(&mut stream, &SyncMessage::SyncResponse(response)).await?;
                        }
                        Err(e) => {
                            let err = SyncMessage::error(
                                crate::protocol::ErrorCode::Internal,
                                e.to_string(),
                                None,
                            );
                            send_message(&mut stream, &err).await?;
                        }
                    }
                }
                SyncMessage::SyncResponse(resp) => {
                    debug!(list = %resp.list_id, "Received sync response");

                    // Apply the update
                    if let Err(e) = update_handler(resp.list_id.clone(), resp.crdt_data.clone()) {
                        warn!(error = %e, "Failed to apply sync update");
                    }

                    let _ = event_tx
                        .send(TransportEvent::ListReceived {
                            peer_device_id: peer_device_id.clone(),
                            list_id: resp.list_id,
                            crdt_data: resp.crdt_data,
                        })
                        .await;
                }
                SyncMessage::ListInventoryRequest => {
                    let lists = inventory_handler();
                    let response = SyncMessage::ListInventoryResponse(
                        crate::protocol::ListInventoryResponseMessage { lists },
                    );
                    send_message(&mut stream, &response).await?;
                }
                SyncMessage::Goodbye => {
                    debug!("Peer said goodbye");
                    break;
                }
                _ => {
                    debug!("Ignoring unexpected message type");
                }
            },
            Err(SyncError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                debug!("Peer disconnected");
                break;
            }
            Err(e) => {
                let _ = event_tx
                    .send(TransportEvent::ConnectionError {
                        peer_device_id: peer_device_id.clone(),
                        error: e.to_string(),
                    })
                    .await;
                break;
            }
        }
    }

    // Clean up connection
    {
        let mut conns = connections.write().await;
        conns.remove(&peer_device_id);
    }

    let _ = event_tx
        .send(TransportEvent::PeerDisconnected {
            device_id: peer_device_id,
        })
        .await;

    Ok(())
}

/// Sends a message over the stream with length framing.
async fn send_message(stream: &mut TcpStream, message: &SyncMessage) -> Result<()> {
    let data = message.to_bytes()?;

    if data.len() > MAX_MESSAGE_SIZE {
        return Err(SyncError::Protocol("Message too large".into()));
    }

    let mut buf = BytesMut::with_capacity(4 + data.len());
    buf.put_u32(data.len() as u32);
    buf.put_slice(&data);

    timeout(IO_TIMEOUT, stream.write_all(&buf))
        .await
        .map_err(|_| SyncError::Timeout("Write timeout".into()))?
        .map_err(SyncError::Io)?;

    Ok(())
}

/// Receives a message from the stream with length framing.
async fn recv_message(stream: &mut TcpStream) -> Result<SyncMessage> {
    // Read length prefix
    let mut len_buf = [0u8; 4];
    timeout(IO_TIMEOUT, stream.read_exact(&mut len_buf))
        .await
        .map_err(|_| SyncError::Timeout("Read timeout".into()))?
        .map_err(SyncError::Io)?;

    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_SIZE {
        return Err(SyncError::Protocol(format!(
            "Message too large: {} bytes",
            len
        )));
    }

    // Read message body
    let mut buf = vec![0u8; len];
    timeout(IO_TIMEOUT, stream.read_exact(&mut buf))
        .await
        .map_err(|_| SyncError::Timeout("Read timeout".into()))?
        .map_err(SyncError::Io)?;

    SyncMessage::from_bytes(&buf).map_err(SyncError::Serialization)
}

/// Returns current Unix timestamp.
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_framing() {
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let msg = recv_message(&mut stream).await.unwrap();
            send_message(&mut stream, &SyncMessage::Goodbye)
                .await
                .unwrap();
            msg
        });

        let client = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let hello = SyncMessage::Hello(HelloMessage::new(
                "test".into(),
                "Test Device".into(),
                vec![],
            ));
            send_message(&mut stream, &hello).await.unwrap();
            recv_message(&mut stream).await.unwrap()
        });

        let (server_result, client_result) = tokio::join!(server, client);

        if let SyncMessage::Hello(h) = server_result.unwrap() {
            assert_eq!(h.device_id, "test");
        } else {
            panic!("Expected Hello");
        }

        assert!(matches!(client_result.unwrap(), SyncMessage::Goodbye));
    }
}
