use std::{time::SystemTime, path::Path, os::unix::prelude::OsStrExt};
use chrono::{DateTime, Utc};
use futures::{io::Cursor, FutureExt};
use serde::{Deserialize, Serialize};
use webdav_handler::fs::{DavMetaData, FsError, DavDirEntry};

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
    pub dir_entry: DirEntry,
    /// Set this with the set_content method
    pub content: Option<Cursor<Vec<u8>>>,
    /// This is used to store the cursor position when content is not loaded. This is not accurate the rest of the time
    pub cursor_pos: u64
}
impl File {
    pub fn set_content(&mut self, new: Option<Vec<u8>>) {
        match new {
            Some(n) => {
                let pos = match &self.content {
                    Some(o) => o.position(),
                    None => self.cursor_pos
                };
                let mut cursor = Cursor::new(n);
                cursor.set_position(self.cursor_pos);
                self.content = Some(cursor);
            }
            None => if let Some(o) = &self.content {
                self.cursor_pos = o.position();
                self.content = None;
            }
        }
    }
    pub fn metadata(&self) -> &Metadata {
        &self.dir_entry.metadata
    }
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.dir_entry.metadata
    }
    pub fn id(&self) -> &usize {
        &self.dir_entry.id
    }
}

#[derive(Debug)]
pub struct DirEntry {
    pub path: String,
    pub id: usize,
    pub parent_id: Option<usize>,
    pub metadata: Metadata
}

impl DavDirEntry for DirEntry {
    fn name(&self) -> Vec<u8> {
        match Path::new(&self.path).file_name() {
            Some(p) => p.as_bytes().to_owned(),
            None => Vec::new()
        }
    }
    fn metadata<'a>(&'a self) -> webdav_handler::fs::FsFuture<Box<dyn DavMetaData>> {
        async {
            Ok::<Box<(dyn DavMetaData + 'static)>, FsError>(self.metadata.clone().boxed())
        }.boxed()
    }
}