use serde::{Deserialize, Serialize};

/// A grocery list containing items to purchase.
///
/// Each list has a unique ID, a name, an optional store location,
/// and timestamps for tracking creation and modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroceryList {
    pub id: String,
    pub name: String,
    pub store: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// An individual item within a grocery list.
///
/// Items have a name, optional quantity, optional category for grouping,
/// optional notes, and a checked state for marking as purchased.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub checked: bool,
    pub quantity: Option<u32>,
    pub category: Option<String>,
    pub notes: Option<String>,
}

impl Item {
    /// Creates a new unchecked item with the given name.
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            checked: false,
            quantity: None,
            category: None,
            notes: None,
        }
    }

    /// Creates an item with all fields specified.
    pub fn with_details(
        id: String,
        name: String,
        quantity: Option<u32>,
        category: Option<String>,
        notes: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            checked: false,
            quantity,
            category,
            notes,
        }
    }
}

/// Request payload for creating a new grocery list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub store: Option<String>,
}

/// Request payload for updating an existing grocery list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub store: Option<String>,
}

/// Request payload for adding a new item to a list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemRequest {
    pub name: String,
    pub quantity: Option<u32>,
    pub category: Option<String>,
    pub notes: Option<String>,
}

/// Request payload for updating an existing item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    pub name: Option<String>,
    pub quantity: Option<u32>,
    pub category: Option<String>,
    pub notes: Option<String>,
}

/// A grocery list with all its items for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroceryListWithItems {
    pub list: GroceryList,
    pub items: Vec<Item>,
}
