use crate::db::Database;
use tauri::State;

#[tauri::command]
pub fn create_list(db: State<Database>, name: String, store: Option<String>) -> Result<(), String> {
    db.create_list(&name, store).map_err(|e| e.to_string())
}
