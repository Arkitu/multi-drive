use std::{collections::HashMap, sync::Arc};
use bytes::Bytes;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use webdav_handler::fs::DavMetaData;
use crate::error::{Result, Error};
use crate::types::{File, Metadata};

#[derive(Clone, Debug)]
struct DiscordCache (Arc<RwLock<HashMap<usize, Arc<RwLock<File>>>>>);
impl DiscordCache {
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

        let new_meta: Metadata = serde_json::from_str(&msg.content)?;

        if meta.is_dir() != new_meta.is_dir() {
            eprintln!("Distant file is_dir value is different from local file is_dir value");
            return Err(Error::BadContent)
        }

        if !meta.is_dir() {
            self.cached.write().await.metadata = Some(new_meta);
        } else {
            let url = &msg.attachments.get(0).ok_or(Error::NotFound)?.url;
            let content = self.client.get_attachment(url).await?;
            let mut cached = self.cached.write().await;
            cached.content = Some(content);
            cached.metadata = Some(new_meta);
        }

        Ok(())
    }
    pub async fn send(&self) -> Result<()> {
        let cached = self.cached.read().await;
        let content = cached.content.clone().ok_or(Error::NotFound)?;
        let meta = cached.metadata.clone().ok_or(Error::NotFound)?;
        let id = cached.id;
        drop(cached);
        
        self.client.send_msg_with_attachment(&serde_json::to_string(&meta)?, [(id.to_string(), content)].to_vec()).await?;


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

#[derive(Serialize)]
pub struct SendAttachmentJson<'a> {
    pub id: usize,
    pub description: &'a str,
    pub filename: &'a str
}

#[derive(Serialize)]
pub struct SendMsgReqJson<'a> {
    pub content: &'a str,
    pub attachments: Vec<SendAttachmentJson<'a>>
}

#[derive(Deserialize)]
pub struct SendMsgResJson {
    pub id: String
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
    pub async fn send_msg_with_attachment(&self, content: &str, attachment: Vec<(String, Bytes)>) -> Result<usize> {
        let mut form = multipart::Form::new()
            .part("payload_json", multipart::Part::text(serde_json::to_string(&SendMsgReqJson {
                content,
                attachments: attachment.iter().enumerate().map(|(i, (n, _))| SendAttachmentJson {
                    id: i,
                    description: "",
                    filename: n
                }).collect()
            })?));
        
        for (i, (n, a)) in attachment.into_iter().enumerate() {
            form = form.part(format!("files[{}]", i), multipart::Part::bytes(a.to_vec()).file_name(n))
        }

        let res = self.http.post(format!("https://discord.com/api/v10/channels/{}/messages", self.channel_id,))
            .header("User-Agent", "DiscordBot (https://github.com/Arkitu/multi-drive, 0.0.1)")
            .header("Authorization", "Bot ".to_string() + &self.token)
            .multipart(form)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::DiscordError)
        }

        let res = res.text().await?;
        let res: SendMsgResJson = serde_json::from_str(&res)?;

        Ok(res.id.parse()?)
    }
}