//! mDNS-based peer discovery for LAN synchronization.
//!
//! This module handles advertising our presence on the network and discovering
//! other BearBasket instances for peer-to-peer synchronization.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{Result, SyncError};

/// The mDNS service type for BearBasket sync.
pub const SERVICE_TYPE: &str = "_grocery-sync._tcp.local.";

/// Default port for the sync service.
pub const DEFAULT_PORT: u16 = 47284;

/// A discovered peer on the local network.
#[derive(Debug, Clone)]
pub struct Peer {
    /// Unique identifier for this peer (device ID).
    pub device_id: String,
    /// Human-readable name for this peer (hostname).
    pub name: String,
    /// Network addresses where this peer can be reached.
    pub addresses: Vec<IpAddr>,
    /// Port the peer is listening on.
    pub port: u16,
    /// When we last saw this peer.
    pub last_seen: std::time::Instant,
}

impl Peer {
    /// Returns the best socket address to connect to this peer.
    pub fn socket_addr(&self) -> Option<SocketAddr> {
        // Prefer IPv4 addresses for better compatibility
        let addr = self
            .addresses
            .iter()
            .find(|a| a.is_ipv4())
            .or_else(|| self.addresses.first())?;
        Some(SocketAddr::new(*addr, self.port))
    }
}

/// Events emitted by the discovery service.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new peer was discovered.
    PeerDiscovered(Peer),
    /// A peer was updated (e.g., address changed).
    PeerUpdated(Peer),
    /// A peer went offline.
    PeerLost(String),
}

/// Manages mDNS service advertising and peer discovery.
pub struct Discovery {
    /// The mDNS service daemon.
    daemon: ServiceDaemon,
    /// Our device ID.
    device_id: String,
    /// Our device name.
    device_name: String,
    /// Port we're listening on.
    port: u16,
    /// Known peers indexed by device_id.
    peers: Arc<RwLock<HashMap<String, Peer>>>,
    /// Whether we're currently advertising.
    is_advertising: Arc<RwLock<bool>>,
    /// Whether we're currently browsing.
    is_browsing: Arc<RwLock<bool>>,
}

impl Discovery {
    /// Creates a new discovery service.
    ///
    /// # Arguments
    /// * `port` - The port we're listening on for sync connections.
    ///
    /// # Returns
    /// A new Discovery instance or an error.
    pub fn new(port: u16) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let device_id = Uuid::new_v4().to_string();
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| format!("BearBasket-{}", &device_id[..8]));

        Ok(Self {
            daemon,
            device_id,
            device_name,
            port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            is_advertising: Arc::new(RwLock::new(false)),
            is_browsing: Arc::new(RwLock::new(false)),
        })
    }

    /// Creates a discovery service with a specific device ID.
    ///
    /// Useful for persistent device identity across restarts.
    pub fn with_device_id(port: u16, device_id: String) -> Result<Self> {
        let mut discovery = Self::new(port)?;
        discovery.device_id = device_id;
        Ok(discovery)
    }

    /// Returns our device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Returns our device name.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Starts advertising our presence on the network.
    ///
    /// Other BearBasket instances will be able to discover us.
    pub async fn start_advertising(&self) -> Result<()> {
        let mut is_advertising = self.is_advertising.write().await;
        if *is_advertising {
            return Ok(());
        }

        let service_name = format!("BearBasket-{}", &self.device_id[..8]);
        let properties = [
            ("device_id", self.device_id.as_str()),
            ("device_name", self.device_name.as_str()),
            ("version", env!("CARGO_PKG_VERSION")),
        ];

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &service_name,
            &format!("{}.local.", self.device_name),
            "",
            self.port,
            &properties[..],
        )
        .map_err(|e| SyncError::Discovery(format!("Failed to create service info: {}", e)))?;

        self.daemon.register(service_info.clone())?;

        info!(
            device_id = %self.device_id,
            port = %self.port,
            "Started advertising on mDNS"
        );

        *is_advertising = true;
        Ok(())
    }

    /// Stops advertising our presence.
    pub async fn stop_advertising(&self) -> Result<()> {
        let mut is_advertising = self.is_advertising.write().await;
        if !*is_advertising {
            return Ok(());
        }

        let service_name = format!("BearBasket-{}.{}", &self.device_id[..8], SERVICE_TYPE);
        self.daemon.unregister(&service_name)?;

        info!("Stopped advertising on mDNS");
        *is_advertising = false;
        Ok(())
    }

    /// Starts browsing for other peers on the network.
    ///
    /// Returns a channel that emits discovery events.
    pub async fn start_browsing(&self) -> Result<mpsc::Receiver<DiscoveryEvent>> {
        let mut is_browsing = self.is_browsing.write().await;
        if *is_browsing {
            return Err(SyncError::Discovery("Already browsing".into()));
        }

        let receiver = self.daemon.browse(SERVICE_TYPE)?;
        let (tx, rx) = mpsc::channel(100);
        let peers = Arc::clone(&self.peers);
        let our_device_id = self.device_id.clone();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        // Extract device_id from TXT records
                        let device_id = info
                            .get_property_val_str("device_id")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| info.get_fullname().to_string());

                        // Skip ourselves
                        if device_id == our_device_id {
                            continue;
                        }

                        let device_name = info
                            .get_property_val_str("device_name")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| info.get_hostname().to_string());

                        let addresses: Vec<IpAddr> = info
                            .get_addresses()
                            .iter()
                            .map(|s| s.to_ip_addr())
                            .collect();
                        let port = info.get_port();

                        let peer = Peer {
                            device_id: device_id.clone(),
                            name: device_name,
                            addresses,
                            port,
                            last_seen: std::time::Instant::now(),
                        };

                        let mut peers_guard = peers.write().await;
                        let event = if peers_guard.contains_key(&device_id) {
                            peers_guard.insert(device_id.clone(), peer.clone());
                            DiscoveryEvent::PeerUpdated(peer)
                        } else {
                            info!(peer_id = %device_id, peer_name = %peer.name, "Discovered new peer");
                            peers_guard.insert(device_id.clone(), peer.clone());
                            DiscoveryEvent::PeerDiscovered(peer)
                        };

                        if tx.send(event).await.is_err() {
                            debug!("Discovery event receiver dropped");
                            break;
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        let mut peers_guard = peers.write().await;
                        // Try to find the peer by matching the fullname
                        let device_id = peers_guard
                            .iter()
                            .find(|(_, p)| fullname.contains(&p.device_id[..8]))
                            .map(|(id, _)| id.clone());

                        if let Some(id) = device_id {
                            peers_guard.remove(&id);
                            info!(peer_id = %id, "Peer went offline");
                            if tx.send(DiscoveryEvent::PeerLost(id)).await.is_err() {
                                debug!("Discovery event receiver dropped");
                                break;
                            }
                        }
                    }
                    ServiceEvent::SearchStarted(_) => {
                        debug!("mDNS search started");
                    }
                    ServiceEvent::SearchStopped(_) => {
                        debug!("mDNS search stopped");
                        break;
                    }
                    _ => {
                        // Handle future variants of the non-exhaustive enum
                        debug!("Unhandled mDNS service event");
                    }
                }
            }
        });

        info!("Started browsing for peers on mDNS");
        *is_browsing = true;
        Ok(rx)
    }

    /// Stops browsing for peers.
    pub async fn stop_browsing(&self) -> Result<()> {
        let mut is_browsing = self.is_browsing.write().await;
        if !*is_browsing {
            return Ok(());
        }

        self.daemon.stop_browse(SERVICE_TYPE)?;

        info!("Stopped browsing for peers");
        *is_browsing = false;
        Ok(())
    }

    /// Returns a snapshot of all currently known peers.
    pub async fn get_peers(&self) -> Vec<Peer> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Returns a specific peer by device ID.
    pub async fn get_peer(&self, device_id: &str) -> Option<Peer> {
        self.peers.read().await.get(device_id).cloned()
    }

    /// Removes stale peers that haven't been seen for a while.
    pub async fn cleanup_stale_peers(&self, max_age: Duration) {
        let mut peers = self.peers.write().await;
        let now = std::time::Instant::now();
        peers.retain(|id, peer| {
            let age = now.duration_since(peer.last_seen);
            if age > max_age {
                warn!(peer_id = %id, "Removing stale peer");
                false
            } else {
                true
            }
        });
    }

    /// Shuts down the discovery service.
    pub async fn shutdown(&self) -> Result<()> {
        self.stop_advertising().await?;
        self.stop_browsing().await?;
        self.daemon.shutdown()?;
        info!("Discovery service shut down");
        Ok(())
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        // Best-effort cleanup
        let _ = self.daemon.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_socket_addr() {
        let peer = Peer {
            device_id: "test-123".into(),
            name: "Test Peer".into(),
            addresses: vec!["192.168.1.100".parse().unwrap()],
            port: 47284,
            last_seen: std::time::Instant::now(),
        };

        let addr = peer.socket_addr().unwrap();
        assert_eq!(addr.port(), 47284);
        assert_eq!(addr.ip().to_string(), "192.168.1.100");
    }

    #[test]
    fn test_peer_prefers_ipv4() {
        let peer = Peer {
            device_id: "test-123".into(),
            name: "Test Peer".into(),
            addresses: vec![
                "::1".parse().unwrap(),
                "192.168.1.100".parse().unwrap(),
                "fe80::1".parse().unwrap(),
            ],
            port: 47284,
            last_seen: std::time::Instant::now(),
        };

        let addr = peer.socket_addr().unwrap();
        assert!(addr.ip().is_ipv4());
    }
}
