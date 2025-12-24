//! Error types for the sync crate.

use thiserror::Error;

/// Errors that can occur during sync operations.
#[derive(Debug, Error)]
pub enum SyncError {
    /// mDNS discovery error
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// mDNS service daemon error
    #[error("mDNS daemon error: {0}")]
    MdnsDaemon(#[from] mdns_sd::Error),

    /// Network I/O error
    #[error("Network error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol error (invalid message format, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Sync already in progress
    #[error("Sync already in progress with peer: {0}")]
    SyncInProgress(String),

    /// Channel send error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),
}

/// Result type alias for sync operations.
pub type Result<T> = std::result::Result<T, SyncError>;

impl From<SyncError> for String {
    fn from(error: SyncError) -> Self {
        error.to_string()
    }
}
