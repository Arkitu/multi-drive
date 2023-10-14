use std::{collections::HashMap, sync::Arc, io::{Write, Read}};
use bytes::{Bytes, BufMut, BytesMut};
use futures::{FutureExt, io::Cursor};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use webdav_handler::fs::{DavMetaData, DavFileSystem, FsError, DavFile};
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
    pub msg_id: Option<String>,
    pub cached: Arc<RwLock<File>>,
    pub client: Arc<DiscordClient>
}
impl DiscordFile {
    pub async fn load(&self) -> Result<()> {
        let cached = self.cached.read().await;
        let meta = cached.metadata.clone().ok_or(Error::NotFound)?;
        drop(cached);

        let msg = self.client.get_message(self.msg_id.as_ref().ok_or(Error::NotFound)?).await?;

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
    /// Generate the message to send to Discord from the local file
    pub async fn get_msg_data(&self) -> Result<(String, Vec<(String, Cursor<BytesMut>)>)> {
        let cached = self.cached.read().await;
        let content = cached.content.clone().ok_or(Error::NotFound)?;
        let meta = cached.metadata.clone().ok_or(Error::NotFound)?;
        let id = cached.id;
        drop(cached);

        Ok((serde_json::to_string(&meta)?, [(id.to_string(), content)].to_vec()))
    }
    /// Send the file to Discord for the first time
    pub async fn send_create(&mut self) -> Result<()> {
        let (content, attachments) = self.get_msg_data().await?;
        
        let msg_id = self.client.send_msg_with_attachment(&content, attachments).await?;

        self.msg_id = Some(msg_id);

        Ok(())
    }
    /// Edit the file on Discord, return an error if the file was not sent
    pub async fn send_edit(&self) -> Result<()> {
        let msg_data = self.get_msg_data().await?;

        self.client.edit_msg_with_attachments(self.msg_id.as_ref().ok_or(Error::NotFound)?, &msg_data.0, msg_data.1).await?;

        Ok(())
    }
    // Create or edit distant file
    pub async fn send(&mut self) -> Result<()> {
        match self.msg_id {
            Some(_) => self.send_edit().await,
            None => self.send_create().await
        }
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
    fn write_bytes<'a>(&'a mut self, buf: bytes::Bytes) -> webdav_handler::fs::FsFuture<()> {
        async move {
            self.load().await?;
            let mut cached = self.cached.write().await;
            cached.content.unwrap().put(buf);
            self.send_edit().await?;
            Ok(())
        }.boxed()
    }
    fn write_buf<'a>(&'a mut self, buf: Box<dyn bytes::Buf + Send>) -> webdav_handler::fs::FsFuture<()> {
        async move {
            self.load().await?;
            let mut cached = self.cached.write().await;
            cached.content.unwrap().put(buf);
            self.send_edit().await?;
            Ok(())
        }.boxed()
    }
    fn read_bytes<'a>(&'a mut self, count: usize) -> webdav_handler::fs::FsFuture<bytes::Bytes> {
        async move {
            self.load().await?;
            let content: Bytes = self.cached.read().await.content.unwrap().into();
            Ok(content)
        }.boxed()
    }
}

#[derive(Clone)]
pub struct DiscordFs {
    cache: DiscordCache
}
impl DavFileSystem for DiscordFs {
    fn open<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath, options: webdav_handler::fs::OpenOptions) -> webdav_handler::fs::FsFuture<Box<dyn webdav_handler::fs::DavFile>> {
        async {

        }.boxed()
    }
}

#[derive(Deserialize, Debug)]
pub struct MsgAttachmentJson {
    id: String,
    filename: String,
    size: usize,
    url: String,
    proxy_url: String
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
    pub async fn send_msg_with_attachment(&self, content: &str, attachment: Vec<(String, &BytesMut)>) -> Result<String> {
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

        let res = self.http.post(format!("https://discord.com/api/v10/channels/{}/messages", self.channel_id))
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

        Ok(res.id)
    }
    pub async fn edit_msg_with_attachments(&self, msg_id: &str, content: &str, attachment: Vec<(String, &Bytes)>) -> Result<()> {
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

        let res = self.http.patch(format!("https://discord.com/api/v10/channels/{}/messages/{}", self.channel_id, msg_id))
            .header("User-Agent", "DiscordBot (https://github.com/Arkitu/multi-drive, 0.0.1)")
            .header("Authorization", "Bot ".to_string() + &self.token)
            .multipart(form)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::DiscordError)
        }

        Ok(())
    }
}
