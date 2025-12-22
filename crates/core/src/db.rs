use rusqlite::{Connection, Result};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS lists (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                store TEXT,
                crdt BLOB NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn create_list(&self, name: &str, store: Option<String>) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let doc = crate::crdt::ListDoc::new();
        self.conn.execute(
            "INSERT INTO lists (id, name, store, crdt) VALUES (?1, ?2, ?3, ?4)",
            (id, name, store, doc.save()),
        )?;
        Ok(())
    }
}
