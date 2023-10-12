use std::{collections::HashMap, sync::Arc, env};
use bytes::Bytes;
use futures::FutureExt;
use serde::Deserialize;
use tokio::sync::RwLock;
use webdav_handler::fs::{DavFileSystem, DavFile, DavMetaData, FsError, FsResult};
use crate::error::{Result, Error};

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
    pub msg_id: String,
    pub cached: Arc<RwLock<File>>,
    pub client: Arc<DiscordClient>
}
impl DiscordFile {
    pub async fn load(&self) -> Result<()> {
        let cached = self.cached.read().await;
        let meta = cached.metadata.clone().ok_or(Error::NotFound)?;
        drop(cached);

        let msg = self.client.get_message(&self.msg_id).await?;

        if meta.is_dir() {
            
        }
        let url = &msg.attachments.get(0).ok_or(Error::NotFound)?.url;
        let content = self.client.get_attachment(url).await?;

        self.cached.write().await.content = Some(content);

        Ok(())
    }
    pub async fn save_edit(&self) -> Result<()> {
        Ok(())
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

#[derive(Deserialize, Debug)]
pub struct MsgAttachmentJson {
    id: String,
    filename: String,
    size: usize,
    url: String,
    proxy_url: String,
    content_type: String
}

#[derive(Deserialize, Debug)]
pub struct MsgJson {
    content: String,
    attachments: Vec<MsgAttachmentJson>
}

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
    pub async fn get_message(&self, msg_id: &str) -> Result<MsgJson> {
        let res = self.http.get(format!("https://discord.com/api/v10/channels/{}/messages/{}", self.channel_id, msg_id))
            .header("User-Agent", "DiscordBot (https://github.com/Arkitu/multi-drive, 0.0.1)")
            .header("Authorization", "Bot ".to_string() + &self.token)
            .send()
            .await?
            .text()
            .await?;

        let res: MsgJson = serde_json::from_str(&res)?;

        Ok(res)
    }
    pub async fn get_attachment(&self, url: &str) -> Result<Bytes> {
        Ok(self.http.get(url).send().await?.bytes().await?)
    }
}