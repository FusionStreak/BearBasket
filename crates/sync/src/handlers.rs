//! Database and CRDT integration handlers for sync operations.
//!
//! This module provides factory functions that create handler callbacks
//! connecting the sync transport layer to bearbasket-core's Database and ListDoc.

use std::sync::{Arc, Mutex};

use bearbasket_core::db::Database;

use crate::error::{Result, SyncError};
use crate::protocol::{ListMetadata, SyncRequestMessage, SyncResponseMessage};
use crate::transport::{ListInventoryHandler, SyncHandler, UpdateHandler};

/// Creates a sync handler that responds to sync requests from peers.
///
/// This handler:
/// 1. Looks up the requested list in the database
/// 2. Returns the CRDT data for the list
///
/// # Arguments
/// * `db` - Shared database instance
///
/// # Returns
/// A boxed sync handler function.
pub fn create_sync_handler(db: Arc<Mutex<Database>>) -> SyncHandler {
    Box::new(
        move |request: SyncRequestMessage| -> Result<SyncResponseMessage> {
            let db = db
                .lock()
                .map_err(|e| SyncError::Protocol(format!("Database lock error: {}", e)))?;

            // Get the list metadata
            let list = db
                .get_list(&request.list_id)
                .map_err(|e| SyncError::Protocol(format!("List not found: {}", e)))?;

            // Get the CRDT data
            let mut crdt = db
                .get_list_crdt(&request.list_id)
                .map_err(|e| SyncError::Protocol(format!("Failed to get CRDT: {}", e)))?;

            let crdt_data = crdt.save();

            Ok(SyncResponseMessage {
                list_id: request.list_id,
                name: list.name,
                store: list.store,
                crdt_data,
                is_full: true,
                timestamp: chrono_timestamp(),
            })
        },
    )
}

/// Creates an inventory handler that returns metadata for all lists.
///
/// This handler provides the list of available lists for sync negotiation.
///
/// # Arguments
/// * `db` - Shared database instance
///
/// # Returns
/// A boxed inventory handler function.
pub fn create_inventory_handler(db: Arc<Mutex<Database>>) -> ListInventoryHandler {
    Box::new(move || -> Vec<ListMetadata> {
        let db = match db.lock() {
            Ok(db) => db,
            Err(_) => return vec![],
        };

        let lists = match db.get_all_lists() {
            Ok(lists) => lists,
            Err(_) => return vec![],
        };

        lists
            .into_iter()
            .map(|list| {
                // Get item count from CRDT
                let item_count = db
                    .get_list_crdt(&list.id)
                    .map(|crdt| crdt.len())
                    .unwrap_or(0);

                ListMetadata {
                    id: list.id,
                    name: list.name,
                    store: list.store,
                    updated_at: list.updated_at,
                    item_count,
                }
            })
            .collect()
    })
}

/// Creates an update handler that applies received CRDT data.
///
/// This handler:
/// 1. Loads the local CRDT for the list (or creates if new)
/// 2. Merges the received CRDT data
/// 3. Saves the merged result
///
/// # Arguments
/// * `db` - Shared database instance
///
/// # Returns
/// A boxed update handler function.
pub fn create_update_handler(db: Arc<Mutex<Database>>) -> UpdateHandler {
    Box::new(move |list_id: String, crdt_data: Vec<u8>| -> Result<()> {
        let db = db
            .lock()
            .map_err(|e| SyncError::Protocol(format!("Database lock error: {}", e)))?;

        // Try to get existing CRDT
        let result = db.get_list_crdt(&list_id);

        match result {
            Ok(mut local_crdt) => {
                // Merge received data into local CRDT
                local_crdt
                    .merge_bytes(&crdt_data)
                    .map_err(|e| SyncError::Protocol(format!("CRDT merge error: {}", e)))?;

                // Save merged CRDT
                db.update_list_crdt(&list_id, &mut local_crdt)
                    .map_err(|e| SyncError::Protocol(format!("Failed to save CRDT: {}", e)))?;
            }
            Err(_) => {
                // List doesn't exist locally, we could create it
                // For now, just log and skip
                tracing::warn!(list_id = %list_id, "Received update for unknown list");
            }
        }

        Ok(())
    })
}

/// Returns the current Unix timestamp in seconds.
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_inventory_handler() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).unwrap();
        let db = Arc::new(Mutex::new(db));

        // Create a list
        {
            let db = db.lock().unwrap();
            db.create_list("Test List", None).unwrap();
        }

        let handler = create_inventory_handler(Arc::clone(&db));
        let inventory = handler();

        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].name, "Test List");
    }
}
