//! BearBasket LAN Sync
//!
//! This crate provides peer-to-peer synchronization for BearBasket grocery lists
//! over the local network using mDNS discovery and a simple TCP protocol.
//!
//! # Architecture
//!
//! - **Discovery**: Uses mDNS to advertise and discover other BearBasket instances
//! - **Transport**: TCP-based message transport with length-prefixed framing
//! - **Protocol**: JSON-based message protocol for CRDT synchronization
//!
//! # Usage
//!
//! The sync service requires three handler callbacks:
//! - A sync handler to respond to sync requests from peers
//! - An inventory handler to list available grocery lists
//! - An update handler to apply received CRDT updates
//!
//! See [`SyncService`] for more details.

pub mod discovery;
pub mod error;
pub mod protocol;
pub mod transport;

pub use discovery::{Discovery, DiscoveryEvent, Peer, DEFAULT_PORT, SERVICE_TYPE};
pub use error::{Result, SyncError};
pub use protocol::{
    HelloMessage, ListMetadata, ListUpdateMessage, SyncMessage, SyncRequestMessage,
    SyncResponseMessage, PROTOCOL_VERSION,
};
pub use transport::{Transport, TransportEvent};

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

/// Configuration for the sync service.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Port to listen on for sync connections.
    pub port: u16,
    /// Optional persistent device ID (will be generated if not provided).
    pub device_id: Option<String>,
    /// Whether to automatically start advertising on startup.
    pub auto_advertise: bool,
    /// Whether to automatically start browsing on startup.
    pub auto_browse: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            device_id: None,
            auto_advertise: true,
            auto_browse: true,
        }
    }
}

/// Events from the sync service.
#[derive(Debug)]
pub enum SyncEvent {
    /// Discovery-related event.
    Discovery(DiscoveryEvent),
    /// Transport-related event.
    Transport(TransportEvent),
    /// Service started.
    Started { device_id: String, port: u16 },
    /// Service stopped.
    Stopped,
    /// Error occurred.
    Error(String),
}

/// High-level sync service that combines discovery and transport.
pub struct SyncService {
    config: SyncConfig,
    discovery: Arc<RwLock<Option<Discovery>>>,
    transport: Arc<RwLock<Option<Transport>>>,
    device_id: String,
}

impl SyncService {
    /// Creates a new sync service with the given configuration.
    pub fn new(config: SyncConfig) -> Result<Self> {
        let device_id = config
            .device_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(Self {
            config,
            discovery: Arc::new(RwLock::new(None)),
            transport: Arc::new(RwLock::new(None)),
            device_id,
        })
    }

    /// Returns the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Starts the sync service.
    ///
    /// Returns a channel for receiving sync events.
    pub async fn start(
        &mut self,
        sync_handler: transport::SyncHandler,
        inventory_handler: transport::ListInventoryHandler,
        update_handler: transport::UpdateHandler,
    ) -> Result<mpsc::Receiver<SyncEvent>> {
        let (event_tx, event_rx) = mpsc::channel(100);

        // Initialize discovery
        let discovery = if let Some(ref id) = self.config.device_id {
            Discovery::with_device_id(self.config.port, id.clone())?
        } else {
            Discovery::new(self.config.port)?
        };

        // Start advertising if configured
        if self.config.auto_advertise {
            discovery.start_advertising().await?;
        }

        // Start browsing if configured
        let discovery_events = if self.config.auto_browse {
            Some(discovery.start_browsing().await?)
        } else {
            None
        };

        // Initialize transport
        let mut transport = Transport::new(
            self.device_id.clone(),
            discovery.device_name().to_string(),
            self.config.port,
        );

        let transport_events = transport
            .start(sync_handler, inventory_handler, update_handler)
            .await?;

        // Store instances
        *self.discovery.write().await = Some(discovery);
        *self.transport.write().await = Some(transport);

        // Merge event streams
        let event_tx_clone = event_tx.clone();
        if let Some(mut discovery_rx) = discovery_events {
            tokio::spawn(async move {
                while let Some(event) = discovery_rx.recv().await {
                    if event_tx_clone
                        .send(SyncEvent::Discovery(event))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }

        let event_tx_clone = event_tx.clone();
        let mut transport_rx = transport_events;
        tokio::spawn(async move {
            while let Some(event) = transport_rx.recv().await {
                if event_tx_clone
                    .send(SyncEvent::Transport(event))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        // Send started event
        let _ = event_tx
            .send(SyncEvent::Started {
                device_id: self.device_id.clone(),
                port: self.config.port,
            })
            .await;

        info!(
            device_id = %self.device_id,
            port = %self.config.port,
            "Sync service started"
        );

        Ok(event_rx)
    }

    /// Gets the list of discovered peers.
    pub async fn get_peers(&self) -> Vec<Peer> {
        if let Some(ref discovery) = *self.discovery.read().await {
            discovery.get_peers().await
        } else {
            vec![]
        }
    }

    /// Syncs a list with a specific peer.
    pub async fn sync_with_peer(
        &self,
        peer: &Peer,
        list_id: &str,
        get_local_crdt: impl FnOnce() -> Result<(String, Option<String>, Vec<u8>)>,
        available_lists: Vec<String>,
    ) -> Result<SyncResponseMessage> {
        let transport = self.transport.read().await;
        let transport = transport
            .as_ref()
            .ok_or_else(|| SyncError::Connection("Transport not initialized".into()))?;

        transport
            .sync_with_peer(peer, list_id, get_local_crdt, available_lists)
            .await
    }

    /// Stops the sync service.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref discovery) = *self.discovery.read().await {
            discovery.shutdown().await?;
        }

        if let Some(ref mut transport) = *self.transport.write().await {
            transport.shutdown().await?;
        }

        *self.discovery.write().await = None;
        *self.transport.write().await = None;

        info!("Sync service stopped");
        Ok(())
    }
}
