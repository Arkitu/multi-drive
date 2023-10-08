use std::{time::SystemTime, collections::HashMap, sync::Arc};
use futures::FutureExt;
use tokio::sync::RwLock;
use webdav_handler::fs::{DavFileSystem, DavFile, DavMetaData, FsError};
use crate::db::File;

#[derive(Clone, Debug)]
struct Cache (Arc<RwLock<HashMap<usize, Arc<RwLock<File>>>>>);
impl Cache {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    pub async fn get(&self, id: &usize) -> Option<Arc<RwLock<File>>> {
        if let Some(f) = self.0.read().await.get(id) {
            Some(f.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    len: u64,
    modified: Option<SystemTime>,
    is_dir: bool
}
impl DavMetaData for Metadata {
    fn len(&self) -> u64 {
        self.len
    }
    fn modified(&self) -> webdav_handler::fs::FsResult<SystemTime> {
        self.modified.ok_or(FsError::GeneralFailure)
    }
    fn is_dir(&self) -> bool {
        self.is_dir
    }
}

#[derive(Debug)]
pub struct DiscordFile {
    cached: Arc<RwLock<File>>
}
impl DavFile for DiscordFile {
    fn metadata<'a>(&'a mut self) -> webdav_handler::fs::FsFuture<Box<dyn DavMetaData>> {
        async {
            let file = self.cached.read()
            Ok(Box::new(self.meta) as Box<dyn DavMetaData>)
        }.boxed()
    }
}

#[derive(Clone)]
pub struct DiscordFs {
    cache: Cache
}
impl DavFileSystem for DiscordFs {
    fn open<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath, options: webdav_handler::fs::OpenOptions) -> webdav_handler::fs::FsFuture<Box<dyn webdav_handler::fs::DavFile>> {
        async {

        }.boxed()
    }
}