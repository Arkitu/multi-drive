use rusqlite::{OptionalExtension, params};
use tokio_rusqlite::Connection;
use crate::{error::{Result, Error}, types::{Metadata, File, DirEntry}};

#[derive(Debug)]
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
                CREATE TABLE IF NOT EXISTS dir_entries (
                    id INTEGER PRIMARY KEY,
                    parent_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    path TEXT NOT NULL UNIQUE,
                    meta_len INTEGER NOT NULL,
                    meta_modified TEXT,
                    meta_is_dir BOOLEAN NOT NULL,
                    discord_msg_id TEXT UNIQUE
                )
            ", ())?;

            Ok(())
        }).await.expect("Failed to create tables");
    }

    // files
    pub async fn insert_dir_entry(&self, parent_id: Option<usize>, path: String, metadata: Metadata) -> Result<()> {
        self.conn.call(move |conn| {
            conn.execute("
                INSERT INTO dir_entries (parent_id, path, meta_len, meta_modified, meta_is_dir)
                VALUES (?1, ?2, ?3, ?4, ?5)
            ", params![parent_id, path, metadata.len, metadata.modified, metadata.is_dir])
        }).await?;
        Ok(())
    }
    pub async fn get_dir_entry_by_path(&self, path: String) -> Result<Option<DirEntry>> {
        Ok(self.conn.call(move |conn| {
            conn.query_row("
                SELECT id, parent_id, path, meta_len, meta_modified, meta_is_dir
                FROM dir_entries
                WHERE path = ?1
            ", [path], |row| {
                Ok(DirEntry {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    path: row.get(2)?,
                    metadata: Metadata {
                        len: row.get(3)?,
                        modified: row.get(4).ok(),
                        is_dir: row.get(5)?
                    }
                })
            }).optional()
        }).await?)
    }
    pub async fn get_dir_entry_by_id(&self, id: usize) -> Result<Option<DirEntry>> {
        Ok(self.conn.call(move |conn| {
            conn.query_row("
                SELECT id, parent_id, path, meta_len, meta_modified, meta_is_dir
                FROM dir_entries
                WHERE id = ?1
            ", [id], |row| {
                Ok(DirEntry {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    path: row.get(2)?,
                    metadata: Metadata {
                        len: row.get(3)?,
                        modified: row.get(4).ok(),
                        is_dir: row.get(5)?
                    }
                })
            }).optional()
        }).await?)
    }
    pub async fn get_file_by_path(&self, path: String) -> Result<Option<File>> {
        Ok(Some(File {
            dir_entry: match self.get_dir_entry_by_path(path).await? {
                Some(de) => de,
                None => return Ok(None)
            },
            content: None,
            cursor_pos: 0
        }))
    }
    pub async fn get_file_by_id(&self, id: usize) -> Result<Option<File>> {
        Ok(Some(File {
            dir_entry: match self.get_dir_entry_by_id(id).await? {
                Some(de) => de,
                None => return Ok(None)
            },
            content: None,
            cursor_pos: 0
        }))
    }
    pub async fn get_dir_entries_by_parent_id(&self, parent_id: usize) -> Result<Vec<DirEntry>> {
        self.conn.call(move |conn| {
            let mut stmt = conn.prepare("
                SELECT id, parent_id, path, meta_len, meta_modified, meta_is_dir
                FROM dir_entries
                WHERE parent_id = ?1
            ")?;
            let entries: Vec<DirEntry> = stmt.query_map([parent_id], |row| {
                Ok(DirEntry {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    path: row.get(2)?,
                    metadata: Metadata {
                        len: row.get(3)?,
                        modified: row.get(4).ok(),
                        is_dir: row.get(5)?
                    }
                })
            })?.collect::<std::result::Result<Vec<DirEntry>, rusqlite::Error>>()?;
            Ok(entries)
        }).await?;
        Ok(Vec::new())
    }
}