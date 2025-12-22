use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use uuid::Uuid;

use crate::crdt::ListDoc;
use crate::error::{CoreError, Result};
use crate::model::GroceryList;

/// SQLite-backed database for persisting grocery lists with CRDT state.
///
/// `Database` provides persistence for grocery lists, storing each list's name,
/// optional store location, timestamps, and its complete CRDT state as a binary blob.
/// All operations use SQLite transactions for reliability.
pub struct Database {
    conn: Connection,
}

/// Returns the current Unix timestamp in seconds.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl Database {
    /// Opens or creates a database at the specified path.
    ///
    /// Initializes the `lists` table if it doesn't already exist.
    /// Runs migrations to update schema if needed.
    ///
    /// # Arguments
    /// * `path` - File path to the SQLite database
    ///
    /// # Returns
    /// A new `Database` instance or an error.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// ```
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Create initial schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS lists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                store TEXT,
                crdt BLOB NOT NULL,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;

        // Run migrations for existing databases
        Self::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Runs database migrations to update schema.
    fn run_migrations(conn: &Connection) -> Result<()> {
        // Check if columns exist and add them if missing
        let has_created_at: bool = conn.prepare("SELECT created_at FROM lists LIMIT 1").is_ok();

        if !has_created_at {
            conn.execute_batch(
                r#"
                ALTER TABLE lists ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE lists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
                "#,
            )?;
        }

        Ok(())
    }

    /// Creates a new grocery list in the database.
    ///
    /// Generates a unique ID and initializes an empty CRDT document for the list.
    ///
    /// # Arguments
    /// * `name` - The name of the grocery list
    /// * `store` - Optional name of the store
    ///
    /// # Returns
    /// The created GroceryList with its generated ID.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// let list = db.create_list("Weekly Groceries", Some("Whole Foods".to_string()))?;
    /// ```
    pub fn create_list(&self, name: &str, store: Option<String>) -> Result<GroceryList> {
        let id = Uuid::new_v4().to_string();
        let mut doc = ListDoc::new();
        let timestamp = now();

        self.conn.execute(
            "INSERT INTO lists (id, name, store, crdt, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&id, name, &store, doc.save(), timestamp, timestamp),
        )?;

        Ok(GroceryList {
            id,
            name: name.to_string(),
            store,
            created_at: timestamp,
            updated_at: timestamp,
        })
    }

    /// Retrieves all grocery lists from the database.
    ///
    /// # Returns
    /// A vector of all grocery lists (without CRDT data).
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// let lists = db.get_all_lists()?;
    /// ```
    pub fn get_all_lists(&self) -> Result<Vec<GroceryList>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, store, created_at, updated_at FROM lists ORDER BY updated_at DESC",
        )?;

        let lists = stmt
            .query_map([], |row| {
                Ok(GroceryList {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    store: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(lists)
    }

    /// Retrieves a single grocery list by ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the list
    ///
    /// # Returns
    /// The grocery list or an error if not found.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// let list = db.get_list("some-uuid")?;
    /// ```
    pub fn get_list(&self, id: &str) -> Result<GroceryList> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, store, created_at, updated_at FROM lists WHERE id = ?1")?;

        let list = stmt
            .query_row([id], |row| {
                Ok(GroceryList {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    store: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CoreError::ListNotFound(id.to_string()),
                _ => CoreError::Database(e),
            })?;

        Ok(list)
    }

    /// Updates the metadata of a grocery list.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the list
    /// * `name` - Optional new name
    /// * `store` - Optional new store
    ///
    /// # Returns
    /// The updated grocery list.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// let updated = db.update_list("some-uuid", Some("New Name"), None)?;
    /// ```
    pub fn update_list(
        &self,
        id: &str,
        name: Option<String>,
        store: Option<Option<String>>,
    ) -> Result<GroceryList> {
        // First verify the list exists
        let existing = self.get_list(id)?;
        let timestamp = now();

        let new_name = name.unwrap_or(existing.name);
        let new_store = store.unwrap_or(existing.store);

        self.conn.execute(
            "UPDATE lists SET name = ?1, store = ?2, updated_at = ?3 WHERE id = ?4",
            (&new_name, &new_store, timestamp, id),
        )?;

        Ok(GroceryList {
            id: id.to_string(),
            name: new_name,
            store: new_store,
            created_at: existing.created_at,
            updated_at: timestamp,
        })
    }

    /// Retrieves and deserializes a grocery list's CRDT document by ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the list
    ///
    /// # Returns
    /// The loaded `ListDoc` or an error if the list doesn't exist.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// let list = db.get_list_crdt("some-uuid")?;
    /// ```
    pub fn get_list_crdt(&self, id: &str) -> Result<ListDoc> {
        let mut stmt = self.conn.prepare("SELECT crdt FROM lists WHERE id = ?1")?;
        let crdt_blob: Vec<u8> = stmt
            .query_row([id], |row| row.get(0))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CoreError::ListNotFound(id.to_string()),
                _ => CoreError::Database(e),
            })?;
        let list_doc = ListDoc::load(&crdt_blob)?;
        Ok(list_doc)
    }

    /// Persists changes to a grocery list's CRDT back to the database.
    ///
    /// Also updates the `updated_at` timestamp.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the list
    /// * `list_doc` - The modified list document to save
    ///
    /// # Returns
    /// Ok if the update succeeded, or an error.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// let mut list = db.get_list_crdt("some-uuid")?;
    /// list.add_item("milk", None, None, None);
    /// db.update_list_crdt("some-uuid", &mut list)?;
    /// ```
    pub fn update_list_crdt(&self, id: &str, list_doc: &mut ListDoc) -> Result<()> {
        let timestamp = now();
        self.conn.execute(
            "UPDATE lists SET crdt = ?1, updated_at = ?2 WHERE id = ?3",
            (list_doc.save(), timestamp, id),
        )?;
        Ok(())
    }

    /// Deletes a grocery list from the database.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the list to delete
    ///
    /// # Returns
    /// Ok if the deletion succeeded, or an error.
    ///
    /// # Example
    /// ```
    /// let db = Database::new("grocery.db")?;
    /// db.delete_list("some-uuid")?;
    /// ```
    pub fn delete_list(&self, id: &str) -> Result<()> {
        let rows = self.conn.execute("DELETE FROM lists WHERE id = ?1", [id])?;

        if rows == 0 {
            return Err(CoreError::ListNotFound(id.to_string()));
        }

        Ok(())
    }
}
