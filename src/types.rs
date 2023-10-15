use std::time::SystemTime;
use chrono::{DateTime, Utc};
use futures::io::Cursor;
use serde::{Deserialize, Serialize};
use webdav_handler::fs::{DavMetaData, FsError};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub len: u64,
    pub modified: Option<DateTime<Utc>>,
    pub is_dir: bool
}
impl Metadata {
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

impl DavMetaData for Metadata {
    fn len(&self) -> u64 {
        self.len
    }
    fn modified(&self) -> webdav_handler::fs::FsResult<SystemTime> {
        match self.modified.ok_or(FsError::GeneralFailure) {
            Ok(t) => Ok(t.into()),
            Err(e) => Err(e)
        }
    }
    fn is_dir(&self) -> bool {
        self.is_dir
    }
}

#[derive(Debug)]
pub struct File {
    pub path: String,
    pub id: usize,
    /// Set this with the set_content method
    pub content: Option<Cursor<Vec<u8>>>,
    /// This is used to store the cursor position when content is not loaded. This is not accurate the rest of the time
    pub cursor_pos: u64,
    pub metadata: Option<Metadata>
}
impl File {
    pub fn set_content(&mut self, new: Option<Vec<u8>>) {
        match new {
            Some(n) => {
                let pos = match self.content {
                    Some(o) => o.position(),
                    None => self.cursor_pos
                };
                let mut cursor = Cursor::new(n);
                cursor.set_position(self.cursor_pos);
                self.content = Some(cursor);
            }
            None => if let Some(o) = self.content {
                self.cursor_pos = o.position();
                self.content = None;
            }
        }
    }
}