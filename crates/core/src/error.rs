use thiserror::Error;

/// Errors that can occur in BearBasket core operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Database-related errors
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// CRDT-related errors
    #[error("CRDT error: {0}")]
    Crdt(String),

    /// Item not found at the specified index
    #[error("Item not found at index {0}")]
    ItemNotFound(usize),

    /// List not found with the specified ID
    #[error("List not found: {0}")]
    ListNotFound(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Automerge load error
    #[error("Failed to load CRDT document: {0}")]
    AutomergeLoad(String),
}

/// Result type alias for core operations.
pub type Result<T> = std::result::Result<T, CoreError>;

// Implement conversion to String for Tauri command error handling
impl From<CoreError> for String {
    fn from(error: CoreError) -> Self {
        error.to_string()
    }
}

impl serde::Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
