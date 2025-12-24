//! Sync protocol message definitions.
//!
//! This module defines the wire protocol for CRDT document synchronization
//! between BearBasket peers over the network.

use serde::{Deserialize, Serialize};

/// Protocol version for compatibility checking.
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size in bytes (16 MB).
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// A message in the sync protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Initial handshake message sent when connecting.
    Hello(HelloMessage),

    /// Request to sync a specific list.
    SyncRequest(SyncRequestMessage),

    /// Response containing list data.
    SyncResponse(SyncResponseMessage),

    /// Push update for a list (unsolicited).
    ListUpdate(ListUpdateMessage),

    /// Request the list of available lists.
    ListInventoryRequest,

    /// Response with available lists metadata.
    ListInventoryResponse(ListInventoryResponseMessage),

    /// Acknowledgement of received data.
    Ack(AckMessage),

    /// Error response.
    Error(ErrorMessage),

    /// Goodbye message before disconnecting.
    Goodbye,
}

/// Hello message for initial handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloMessage {
    /// Protocol version.
    pub version: u32,
    /// Sender's device ID.
    pub device_id: String,
    /// Sender's device name.
    pub device_name: String,
    /// List of available list IDs for sync.
    pub available_lists: Vec<String>,
}

impl HelloMessage {
    /// Creates a new hello message.
    pub fn new(device_id: String, device_name: String, available_lists: Vec<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            device_id,
            device_name,
            available_lists,
        }
    }
}

/// Request to sync a specific grocery list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestMessage {
    /// The list ID to sync.
    pub list_id: String,
    /// Optional: Request only changes since this timestamp.
    /// If None, request full CRDT document.
    pub since_timestamp: Option<i64>,
}

impl SyncRequestMessage {
    /// Creates a request for a full sync.
    pub fn full(list_id: String) -> Self {
        Self {
            list_id,
            since_timestamp: None,
        }
    }

    /// Creates a request for incremental sync.
    pub fn incremental(list_id: String, since: i64) -> Self {
        Self {
            list_id,
            since_timestamp: Some(since),
        }
    }
}

/// Response containing list CRDT data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponseMessage {
    /// The list ID.
    pub list_id: String,
    /// The list name.
    pub name: String,
    /// Optional store name.
    pub store: Option<String>,
    /// The CRDT document bytes (full or incremental).
    #[serde(with = "base64_bytes")]
    pub crdt_data: Vec<u8>,
    /// Whether this is a full document or incremental update.
    pub is_full: bool,
    /// Timestamp of this version.
    pub timestamp: i64,
}

/// Push update for a list (sent when changes are made locally).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUpdateMessage {
    /// The list ID.
    pub list_id: String,
    /// The CRDT changes to merge.
    #[serde(with = "base64_bytes")]
    pub crdt_data: Vec<u8>,
    /// Timestamp of this update.
    pub timestamp: i64,
}

/// Response with inventory of available lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInventoryResponseMessage {
    /// Available lists with their metadata.
    pub lists: Vec<ListMetadata>,
}

/// Metadata about a grocery list (without CRDT data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMetadata {
    /// The list ID.
    pub id: String,
    /// The list name.
    pub name: String,
    /// Optional store name.
    pub store: Option<String>,
    /// Last update timestamp.
    pub updated_at: i64,
    /// Number of items in the list.
    pub item_count: usize,
}

/// Acknowledgement message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    /// The message type being acknowledged.
    pub for_message: String,
    /// Optional correlation ID.
    pub correlation_id: Option<String>,
}

/// Error message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Error code.
    pub code: ErrorCode,
    /// Human-readable error message.
    pub message: String,
    /// Related list ID, if applicable.
    pub list_id: Option<String>,
}

/// Error codes for the sync protocol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Protocol version mismatch.
    VersionMismatch,
    /// Requested list not found.
    ListNotFound,
    /// Invalid message format.
    InvalidMessage,
    /// Merge conflict (should not happen with CRDTs, but just in case).
    MergeError,
    /// Internal error.
    Internal,
    /// Message too large.
    MessageTooLarge,
}

/// Helper module for base64 serialization of byte vectors.
mod base64_bytes {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        use base64::Engine;
        let s = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(D::Error::custom)
    }
}

impl SyncMessage {
    /// Serializes the message to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserializes a message from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Creates an error message.
    pub fn error(code: ErrorCode, message: impl Into<String>, list_id: Option<String>) -> Self {
        SyncMessage::Error(ErrorMessage {
            code,
            message: message.into(),
            list_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_serialization() {
        let msg = SyncMessage::Hello(HelloMessage::new(
            "device-123".into(),
            "My Device".into(),
            vec!["list-1".into(), "list-2".into()],
        ));

        let bytes = msg.to_bytes().unwrap();
        let parsed = SyncMessage::from_bytes(&bytes).unwrap();

        if let SyncMessage::Hello(hello) = parsed {
            assert_eq!(hello.device_id, "device-123");
            assert_eq!(hello.version, PROTOCOL_VERSION);
        } else {
            panic!("Expected Hello message");
        }
    }

    #[test]
    fn test_sync_response_with_bytes() {
        let crdt_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let msg = SyncMessage::SyncResponse(SyncResponseMessage {
            list_id: "list-1".into(),
            name: "Groceries".into(),
            store: Some("Costco".into()),
            crdt_data: crdt_data.clone(),
            is_full: true,
            timestamp: 1234567890,
        });

        let bytes = msg.to_bytes().unwrap();
        let parsed = SyncMessage::from_bytes(&bytes).unwrap();

        if let SyncMessage::SyncResponse(resp) = parsed {
            assert_eq!(resp.crdt_data, crdt_data);
        } else {
            panic!("Expected SyncResponse message");
        }
    }
}
