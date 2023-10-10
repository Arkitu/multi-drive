use std::{collections::HashMap, sync::Arc, env};
use futures::FutureExt;
use tokio::sync::RwLock;
use webdav_handler::fs::{DavFileSystem, DavFile, DavMetaData, FsError, FsResult};
use crate::error::Result;

use crate::types::File;

#[derive(Clone, Debug)]
struct Cache (Arc<RwLock<HashMap<usize, Arc<RwLock<File>>>>>);
impl Cache {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    pub async fn get(&self, discord_id: &usize) -> Option<Arc<RwLock<File>>> {
        if let Some(f) = self.0.read().await.get(discord_id) {
            Some(f.clone())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct DiscordFile {
    discord_id: String,
    cached: Arc<RwLock<File>>,
    client: Arc<DiscordClient>
}
impl DiscordFile {
    pub async fn load(&self) {
        self.client.get_message(&self.discord_id);
    }
}
// impl DavFile for DiscordFile {
//     fn metadata<'a>(&'a mut self) -> webdav_handler::fs::FsFuture<Box<dyn DavMetaData>> {
//         async {
//             match self.cached.read().await.metadata {
//                 Some(m) => Ok(Box::new(m) as Box<(dyn DavMetaData + 'static)>),
//                 None => Err(FsError::GeneralFailure)
//             }
//         }.boxed()
//     }
// }

// #[derive(Clone)]
// pub struct DiscordFs {
//     cache: Cache
// }
// impl DavFileSystem for DiscordFs {
//     fn open<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath, options: webdav_handler::fs::OpenOptions) -> webdav_handler::fs::FsFuture<Box<dyn webdav_handler::fs::DavFile>> {
//         async {

//         }.boxed()
//     }
// }

#[derive(Debug)]
pub struct DiscordClient {
    token: String,
    channel_id: String,
    http: reqwest::Client
}
impl DiscordClient {
    pub fn new(token: String, channel_id: String) -> Self {
        Self {
            token,
            channel_id,
            http: reqwest::Client::new()
        }
    }
    pub async fn get_message(&self, discord_id: &str) -> Result<()> {
        println!("{}", self.http.get(format!("https://discord.com/api/v10/channels/{}/messages/{}", self.channel_id, discord_id))
            .header("User-Agent", "DiscordBot (https://github.com/Arkitu/multi-drive, 0.0.1)")
            .header("Authorization", "Bot ".to_string() + &self.token)
            .send()
            .await?
            .text()
            .await?
        );
        Ok(())
    }
}