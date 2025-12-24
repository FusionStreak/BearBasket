//! Tauri commands for peer-to-peer sync functionality.
//!
//! These commands expose the sync service to the frontend, allowing
//! discovery of peers, manual sync triggers, and sync toggle.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

use bearbasket_core::db::Database;
use bearbasket_sync::{
    Peer, SyncConfig, SyncService, create_inventory_handler, create_sync_handler,
    create_update_handler,
};

use crate::state::{AppDatabase, AppSyncService};

/// A peer device visible on the local network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique device identifier.
    pub device_id: String,
    /// Human-readable device name.
    pub name: String,
    /// Network addresses.
    pub addresses: Vec<String>,
    /// Port the peer is listening on.
    pub port: u16,
    /// Whether we are currently connected to this peer.
    pub connected: bool,
}

impl From<Peer> for PeerInfo {
    fn from(peer: Peer) -> Self {
        Self {
            device_id: peer.device_id.clone(),
            name: peer.name.clone(),
            addresses: peer.addresses.iter().map(|a| a.to_string()).collect(),
            port: peer.port,
            connected: false, // Will be updated by caller
        }
    }
}

/// Current sync status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Our device ID (if sync has been started).
    pub device_id: Option<String>,
    /// Number of discovered peers.
    pub peer_count: usize,
    /// Number of active connections.
    pub connection_count: usize,
}

/// Result of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Whether the sync was successful.
    pub success: bool,
    /// The list ID that was synced.
    pub list_id: String,
    /// Human-readable message.
    pub message: String,
}

/// Gets the list of discovered peers on the local network.
///
/// Returns an empty list if sync is not enabled.
#[tauri::command]
pub async fn get_peers(sync_state: State<'_, AppSyncService>) -> Result<Vec<PeerInfo>, String> {
    let service_guard = sync_state.service.read().await;

    if let Some(ref service) = *service_guard {
        let peers = service.get_peers().await;
        Ok(peers.into_iter().map(PeerInfo::from).collect())
    } else {
        Ok(vec![])
    }
}

/// Gets the current sync status.
#[tauri::command]
pub async fn get_sync_status(sync_state: State<'_, AppSyncService>) -> Result<SyncStatus, String> {
    let enabled = *sync_state.enabled.read().await;
    let device_id = sync_state.device_id.read().await.clone();

    let (peer_count, connection_count) = {
        let service_guard = sync_state.service.read().await;
        if let Some(ref service) = *service_guard {
            (service.get_peers().await.len(), 0) // TODO: get connection count
        } else {
            (0, 0)
        }
    };

    Ok(SyncStatus {
        enabled,
        device_id,
        peer_count,
        connection_count,
    })
}

/// Syncs a specific list with a peer.
///
/// # Arguments
/// * `peer_device_id` - The device ID of the peer to sync with
/// * `list_id` - The ID of the list to sync
#[tauri::command]
pub async fn sync_with_peer(
    db: State<'_, AppDatabase>,
    sync_state: State<'_, AppSyncService>,
    peer_device_id: String,
    list_id: String,
) -> Result<SyncResult, String> {
    // First, extract all the data we need from the database synchronously
    let (list_name, list_store, crdt_data, available_lists) = {
        let db_guard = db.0.lock().map_err(|e| e.to_string())?;

        let list = db_guard
            .get_list(&list_id)
            .map_err(|e| format!("List not found: {}", e))?;

        let available_lists: Vec<String> = db_guard
            .get_all_lists()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|l| l.id)
            .collect();

        let mut crdt = db_guard
            .get_list_crdt(&list_id)
            .map_err(|e| e.to_string())?;
        let crdt_data = crdt.save();

        (list.name, list.store, crdt_data, available_lists)
        // db_guard is dropped here at the end of the block
    };

    // Now do the async operations
    let service_guard = sync_state.service.read().await;

    let service = service_guard
        .as_ref()
        .ok_or_else(|| "Sync service not running".to_string())?;

    // Find the peer
    let peers = service.get_peers().await;
    let peer = peers
        .iter()
        .find(|p| p.device_id == peer_device_id)
        .ok_or_else(|| format!("Peer not found: {}", peer_device_id))?
        .clone();

    // Drop the service guard before sync
    drop(service_guard);

    // Re-acquire for sync operation
    let service_guard = sync_state.service.read().await;
    let service = service_guard
        .as_ref()
        .ok_or_else(|| "Sync service not running".to_string())?;

    // Sync with peer
    let list_id_for_closure = list_id.clone();
    let response = service
        .sync_with_peer(
            &peer,
            &list_id_for_closure,
            || Ok((list_name.clone(), list_store.clone(), crdt_data.clone())),
            available_lists,
        )
        .await
        .map_err(|e| format!("Sync failed: {}", e))?;

    // Drop the service guard
    drop(service_guard);

    // Apply the received CRDT data
    {
        let db_guard = db.0.lock().map_err(|e| e.to_string())?;
        let mut local_crdt = db_guard
            .get_list_crdt(&list_id)
            .map_err(|e| e.to_string())?;

        local_crdt
            .merge_bytes(&response.crdt_data)
            .map_err(|e| format!("Merge failed: {}", e))?;

        db_guard
            .update_list_crdt(&list_id, &mut local_crdt)
            .map_err(|e| format!("Save failed: {}", e))?;
    }

    Ok(SyncResult {
        success: true,
        list_id,
        message: format!("Successfully synced with {}", peer.name),
    })
}

/// Toggles sync on or off.
///
/// When enabled, starts advertising and browsing for peers.
/// When disabled, stops all sync activity.
#[tauri::command]
pub async fn toggle_sync(
    _db: State<'_, AppDatabase>,
    sync_state: State<'_, AppSyncService>,
) -> Result<SyncStatus, String> {
    let mut enabled = sync_state.enabled.write().await;
    let mut service_guard = sync_state.service.write().await;
    let mut device_id_guard = sync_state.device_id.write().await;

    if *enabled {
        // Disable sync
        if let Some(ref mut service) = *service_guard {
            service.stop().await.map_err(|e| e.to_string())?;
        }
        *service_guard = None;
        *enabled = false;
        *device_id_guard = None;

        Ok(SyncStatus {
            enabled: false,
            device_id: None,
            peer_count: 0,
            connection_count: 0,
        })
    } else {
        // Enable sync
        let config = SyncConfig::default();

        let service = SyncService::new(config).map_err(|e| e.to_string())?;
        let device_id = service.device_id().to_string();

        // For now, start in discovery-only mode
        // Full sync with handlers requires async-compatible database access
        // The sync_with_peer command handles actual syncing manually
        // TODO: Implement proper async database sharing for background sync

        *device_id_guard = Some(device_id.clone());
        *service_guard = Some(service);
        *enabled = true;

        Ok(SyncStatus {
            enabled: true,
            device_id: Some(device_id),
            peer_count: 0,
            connection_count: 0,
        })
    }
}

/// Starts the sync service with full functionality.
///
/// This is called during app setup to initialize sync if previously enabled.
#[allow(dead_code)]
pub async fn start_sync_service(
    db: Arc<Mutex<Database>>,
    sync_state: &AppSyncService,
) -> Result<(), String> {
    let config = SyncConfig::default();

    let mut service = SyncService::new(config).map_err(|e| e.to_string())?;
    let device_id = service.device_id().to_string();

    // Create handlers connected to the database
    let sync_handler = create_sync_handler(Arc::clone(&db));
    let inventory_handler = create_inventory_handler(Arc::clone(&db));
    let update_handler = create_update_handler(Arc::clone(&db));

    // Start the service
    let _event_rx = service
        .start(sync_handler, inventory_handler, update_handler)
        .await
        .map_err(|e| e.to_string())?;

    // Store the service
    *sync_state.service.write().await = Some(service);
    *sync_state.enabled.write().await = true;
    *sync_state.device_id.write().await = Some(device_id);

    Ok(())
}
