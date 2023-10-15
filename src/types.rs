use std::time::SystemTime;
use bytes::BytesMut;
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
    pub content: Option<Cursor<Vec<u8>>>,
    pub metadata: Option<Metadata>
}
