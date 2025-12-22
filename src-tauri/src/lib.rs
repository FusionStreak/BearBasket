use std::sync::Mutex;

use bearbasket_core::db::Database;
use bearbasket_core::error::CoreError;
use bearbasket_core::model::{GroceryList, GroceryListWithItems, Item};
use tauri::{Manager, State};

/// Thread-safe database wrapper for Tauri state management.
pub struct AppDatabase(pub Mutex<Database>);

/// Returns the path to the database file in the app's data directory.
fn get_db_path(app: &tauri::App) -> String {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");

    // Ensure the directory exists
    std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

    app_data_dir
        .join("bearbasket.db")
        .to_string_lossy()
        .to_string()
}

// ============================================================================
// List Management Commands
// ============================================================================

/// Creates a new grocery list.
#[tauri::command]
fn create_list(
    db: State<AppDatabase>,
    name: String,
    store: Option<String>,
) -> Result<GroceryList, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.create_list(&name, store)
}

/// Retrieves all grocery lists.
#[tauri::command]
fn get_lists(db: State<AppDatabase>) -> Result<Vec<GroceryList>, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.get_all_lists()
}

/// Retrieves a single grocery list by ID.
#[tauri::command]
fn get_list(db: State<AppDatabase>, list_id: String) -> Result<GroceryList, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.get_list(&list_id)
}

/// Updates a grocery list's metadata.
#[tauri::command]
fn update_list(
    db: State<AppDatabase>,
    list_id: String,
    name: Option<String>,
    store: Option<Option<String>>,
) -> Result<GroceryList, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.update_list(&list_id, name, store)
}

/// Deletes a grocery list.
#[tauri::command]
fn delete_list(db: State<AppDatabase>, list_id: String) -> Result<(), CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    db.delete_list(&list_id)
}

/// Retrieves a grocery list with all its items.
#[tauri::command]
fn get_list_with_items(
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
#[tauri::command]
fn add_item(
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
#[tauri::command]
fn remove_item(db: State<AppDatabase>, list_id: String, item_id: String) -> Result<(), CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let mut crdt = db.get_list_crdt(&list_id)?;
    crdt.remove_item(&item_id)?;
    db.update_list_crdt(&list_id, &mut crdt)?;
    Ok(())
}

/// Toggles an item's checked state.
#[tauri::command]
fn toggle_item(
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
#[tauri::command]
fn update_item(
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
#[tauri::command]
fn get_items(db: State<AppDatabase>, list_id: String) -> Result<Vec<Item>, CoreError> {
    let db = db.0.lock().map_err(|e| CoreError::Crdt(e.to_string()))?;
    let crdt = db.get_list_crdt(&list_id)?;
    Ok(crdt.get_all_items())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize the database
            let db_path = get_db_path(app);
            let db = Database::new(&db_path).expect("Failed to initialize database");

            // Manage database state
            app.manage(AppDatabase(Mutex::new(db)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // List management
            create_list,
            get_lists,
            get_list,
            update_list,
            delete_list,
            get_list_with_items,
            // Item management
            add_item,
            remove_item,
            toggle_item,
            update_item,
            get_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
