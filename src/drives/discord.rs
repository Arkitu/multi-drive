use std::{collections::HashMap, sync::Arc};
use futures::FutureExt;
use tokio::sync::RwLock;
use webdav_handler::fs::{DavFileSystem, DavFile, DavMetaData, FsError, FsResult};

use crate::types::File;

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

#[derive(Debug)]
pub struct DiscordFile {
    cached: Arc<RwLock<File>>
}
impl DiscordFile {
    fn load(&self) {
        self.cached
    }
}
impl DavFile for DiscordFile {
    fn metadata<'a>(&'a mut self) -> webdav_handler::fs::FsFuture<Box<dyn DavMetaData>> {
        async {
            match self.cached.read().await.metadata {
                Some(m) => Ok(Box::new(m) as Box<(dyn DavMetaData + 'static)>),
                None => Err(FsError::GeneralFailure)
            }
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

struct DiscordClient {
    token: String,
    http: reqwest::Client
}
impl DiscordClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            http: reqwest::Client::new()
        }
    }
    pub async fn get_message(&self, id: String) {
        self.http.get("https://truc")
            .header("User-Agent", "DiscordBot ($url, $versionNumber)")
            .send()
            .await;
    }
}