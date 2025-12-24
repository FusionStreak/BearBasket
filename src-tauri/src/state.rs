//! Application state management for Tauri.
//!
//! This module provides thread-safe state wrappers for the database and sync service.

use std::sync::{Arc, Mutex};

use bearbasket_core::db::Database;
use bearbasket_sync::SyncService;
use tokio::sync::RwLock;

/// Thread-safe database wrapper for Tauri state management.
pub struct AppDatabase(pub Mutex<Database>);

/// Thread-safe sync service wrapper for Tauri state management.
pub struct AppSyncService {
    /// The sync service instance (optional because sync can be disabled).
    pub service: Arc<RwLock<Option<SyncService>>>,
    /// Whether sync is currently enabled.
    pub enabled: Arc<RwLock<bool>>,
    /// The device ID for this instance.
    pub device_id: Arc<RwLock<Option<String>>>,
}

impl AppSyncService {
    /// Creates a new sync service state wrapper.
    pub fn new() -> Self {
        Self {
            service: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(false)),
            device_id: Arc::new(RwLock::new(None)),
        }
    }
}

impl Default for AppSyncService {
    fn default() -> Self {
        Self::new()
    }
}
