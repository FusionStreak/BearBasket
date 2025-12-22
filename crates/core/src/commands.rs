use std::sync::Mutex;

use tauri::State;

use crate::db::Database;
use crate::error::CoreError;
use crate::model::{GroceryList, GroceryListWithItems, Item};

/// Thread-safe database wrapper for Tauri state management.
pub struct AppDatabase(pub Mutex<Database>);

// ============================================================================
// List Management Commands
// ============================================================================

/// Creates a new grocery list.
///
/// # Arguments
/// * `name` - The name of the grocery list
/// * `store` - Optional store name
///
/// # Returns
/// The created grocery list with its generated ID.
#[tauri::command]
pub fn create_list(
    db: State<AppDatabase>,
    name: String,
    store: Option<String>,
) -> Result<GroceryList, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.create_list(&name, store)
}

/// Retrieves all grocery lists.
///
/// # Returns
/// A vector of all grocery lists sorted by most recently updated.
#[tauri::command]
pub fn get_lists(db: State<AppDatabase>) -> Result<Vec<GroceryList>, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.get_all_lists()
}

/// Retrieves a single grocery list by ID.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
///
/// # Returns
/// The grocery list or an error if not found.
#[tauri::command]
pub fn get_list(db: State<AppDatabase>, list_id: String) -> Result<GroceryList, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.get_list(&list_id)
}

/// Updates a grocery list's metadata.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
/// * `name` - Optional new name
/// * `store` - Optional new store (use Some(None) to clear)
///
/// # Returns
/// The updated grocery list.
#[tauri::command]
pub fn update_list(
    db: State<AppDatabase>,
    list_id: String,
    name: Option<String>,
    store: Option<Option<String>>,
) -> Result<GroceryList, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.update_list(&list_id, name, store)
}

/// Deletes a grocery list.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list to delete
#[tauri::command]
pub fn delete_list(db: State<AppDatabase>, list_id: String) -> Result<(), CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.delete_list(&list_id)
}

/// Retrieves a grocery list with all its items.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
///
/// # Returns
/// The grocery list with all its items.
#[tauri::command]
pub fn get_list_with_items(
    db: State<AppDatabase>,
    list_id: String,
) -> Result<GroceryListWithItems, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let list = db.get_list(&list_id)?;
    let crdt = db.get_list_crdt(&list_id)?;
    let items = crdt.get_all_items();

    Ok(GroceryListWithItems { list, items })
}

// ============================================================================
// Item Management Commands
// ============================================================================

/// Adds a new item to a grocery list.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
/// * `name` - The name of the item
/// * `quantity` - Optional quantity
/// * `category` - Optional category for grouping
/// * `notes` - Optional notes
///
/// # Returns
/// The created item with its generated ID.
#[tauri::command]
pub fn add_item(
    db: State<AppDatabase>,
    list_id: String,
    name: String,
    quantity: Option<u32>,
    category: Option<String>,
    notes: Option<String>,
) -> Result<Item, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let mut crdt = db.get_list_crdt(&list_id)?;
    let item = crdt.add_item(&name, quantity, category, notes)?;
    db.update_list_crdt(&list_id, &mut crdt)?;
    Ok(item)
}

/// Removes an item from a grocery list.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
/// * `item_id` - The unique identifier of the item to remove
#[tauri::command]
pub fn remove_item(
    db: State<AppDatabase>,
    list_id: String,
    item_id: String,
) -> Result<(), CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let mut crdt = db.get_list_crdt(&list_id)?;
    crdt.remove_item(&item_id)?;
    db.update_list_crdt(&list_id, &mut crdt)?;
    Ok(())
}

/// Toggles an item's checked state.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
/// * `item_id` - The unique identifier of the item to toggle
///
/// # Returns
/// The updated item.
#[tauri::command]
pub fn toggle_item(
    db: State<AppDatabase>,
    list_id: String,
    item_id: String,
) -> Result<Item, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let mut crdt = db.get_list_crdt(&list_id)?;
    let item = crdt.toggle_item(&item_id)?;
    db.update_list_crdt(&list_id, &mut crdt)?;
    Ok(item)
}

/// Updates an item's properties.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
/// * `item_id` - The unique identifier of the item to update
/// * `name` - Optional new name
/// * `quantity` - Optional new quantity (use Some(None) to clear)
/// * `category` - Optional new category (use Some(None) to clear)
/// * `notes` - Optional new notes (use Some(None) to clear)
///
/// # Returns
/// The updated item.
#[tauri::command]
pub fn update_item(
    db: State<AppDatabase>,
    list_id: String,
    item_id: String,
    name: Option<String>,
    quantity: Option<Option<u32>>,
    category: Option<Option<String>>,
    notes: Option<Option<String>>,
) -> Result<Item, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let mut crdt = db.get_list_crdt(&list_id)?;
    let item = crdt.update_item(&item_id, name, quantity, category, notes)?;
    db.update_list_crdt(&list_id, &mut crdt)?;
    Ok(item)
}

/// Gets all items in a grocery list.
///
/// # Arguments
/// * `list_id` - The unique identifier of the list
///
/// # Returns
/// A vector of all items in the list.
#[tauri::command]
pub fn get_items(db: State<AppDatabase>, list_id: String) -> Result<Vec<Item>, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let crdt = db.get_list_crdt(&list_id)?;
    Ok(crdt.get_all_items())
}
