use rusqlite::OptionalExtension;
use tokio_rusqlite::Connection;
use crate::{error::Result, types::{Metadata, File}};

pub struct DB {
    pub conn: Connection,
}
impl DB {
    pub async fn new(path: Option<&str>) -> Self {
        let conn = match path {
            Some(path) => Connection::open(path).await.expect("Failed to open database"),
            None => Connection::open_in_memory().await.expect("Failed to open database")
        };
        let db = Self { conn };
        db.create_tables().await;
        db
    }
    pub async fn create_tables(&self) {
        self.conn.call(|conn| {
            conn.execute("
                CREATE TABLE IF NOT EXISTS files (
                    id INTEGER PRIMARY KEY,
                    meta_len INTEGER NOT NULL,
                    meta_modified TEXT,
                    meta_is_dir BOOLEAN NOT NULL
                )
            ", ())?;

            Ok(())
        }).await.expect("Failed to create tables");
    }

    // files
    pub async fn insert_file(&self, path: String) -> Result<()> {
        self.conn.call(move |conn| {
            conn.execute("
                INSERT INTO files (path)
                VALUES (?1)
            ", [path])
        }).await?;
        Ok(())
    }
    pub async fn get_file_by_path(&self, path: String) -> Result<Option<File>> {
        Ok(self.conn.call(move |conn| {
            conn.query_row("
                SELECT id, path, meta_len, meta_modified, meta_is_dir
                FROM files
                WHERE path = ?1
            ", [path], |row| {
                Ok(File {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    content: None,
                    metadata: Some(Metadata {
                        len: row.get(2)?,
                        modified: row.get(3).ok(),
                        is_dir: row.get(4)?
                    })
                })
            }).optional()
        }).await?)
    }
    pub async fn get_file_by_id(&self, id: usize) -> Result<Option<File>> {
        Ok(self.conn.call(move |conn| {
            conn.query_row("
                SELECT id, path, meta_len, meta_modified, meta_is_dir
                FROM files
                WHERE id = ?1
            ", [id], |row| {
                Ok(File {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    content: None,
                    metadata: Some(Metadata {
                        len: row.get(2)?,
                        modified: row.get(3).ok(),
                        is_dir: row.get(4)?
                    })
                })
            }).optional()
        }).await?)
    }
}