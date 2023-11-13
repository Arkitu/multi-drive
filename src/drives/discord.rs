use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, borrow::Cow};
use futures::io::Cursor;
use futures::{FutureExt, AsyncWriteExt, AsyncReadExt, AsyncSeekExt};
use reqwest::multipart;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Mutex};
use webdav_handler::fs::{DavMetaData, DavFileSystem, FsError, DavFile, DavDirEntry};
use crate::db::DB;
use crate::error::{Result, Error};
use crate::types::{File, Metadata, DirEntry};
use bytes::Buf;

impl DB {
    pub async fn get_discord_file_by_path(&self, path: String, fs: Arc<DiscordFs>) -> Result<Option<DiscordFile>> {
        Ok(self.conn.call(move |conn| {
            conn.query_row("
                SELECT id, parent_id, path, meta_len, meta_modified, meta_is_dir, discord_msg_id
                FROM dir_entries
                WHERE path = ?1
            ", [path], |row| {
                let file = File {
                    dir_entry: DirEntry {
                        id: row.get(0)?,
                        parent_id: row.get(1)?,
                        path: row.get(2)?,
                        metadata: Metadata {
                            len: row.get(3)?,
                            modified: row.get(4).ok(),
                            is_dir: row.get(5)?
                        }
                    },
                    cached: None,
                    cursor_pos: 0
                };
                let msg_id = row.get::<_, Option<String>>(6)?;
                Ok(DiscordFile::new(msg_id, Arc::new(RwLock::new(file)), fs))
            }).optional()
        }).await?)
    }
    pub async fn get_discord_file_by_id(&self, id: usize, fs: Arc<DiscordFs>) -> Result<Option<DiscordFile>> {
        Ok(self.conn.call(move |conn| {
            conn.query_row("
                SELECT id, parent_id, path, meta_len, meta_modified, meta_is_dir, discord_msg_id
                FROM dir_entries
                WHERE id = ?1
            ", [id], |row| {
                let file = File {
                    dir_entry: DirEntry {
                        id: row.get(0)?,
                        parent_id: row.get(1)?,
                        path: row.get(2)?,
                        metadata: Metadata {
                            len: row.get(3)?,
                            modified: row.get(4).ok(),
                            is_dir: row.get(5)?
                        }
                    },
                    cached: None,
                    cursor_pos: 0
                };
                let msg_id = row.get::<_, Option<String>>(6)?;
                Ok(DiscordFile::new(msg_id, Arc::new(RwLock::new(file)), fs))
            }).optional()
        }).await?)
    }
    pub async fn edit_discord_file_msg_id_by_id(&self, id: usize, msg_id: String) -> Result<()> {
        self.conn.call(move |conn| {
            conn.execute("
                UPDATE dir_entries
                SET discord_msg_id = ?1
                WHERE id = ?2
            ", params![msg_id, id])
        }).await?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct DiscordCache (Arc<RwLock<HashMap<u64, Arc<RwLock<File>>>>>);
impl DiscordCache {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    pub async fn get(&self, discord_id: &u64) -> Option<Arc<RwLock<File>>> {
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
    pub cached: webdav_handler::localfs:://Arc<RwLock<File>>,
    pub fs: Arc<DiscordFs>,
    /// Lock when you are sure that the file is loaded. This way the file is not loaded multiple time
    pub loaded: Arc<Mutex<()>>
}
impl DiscordFile {
    pub fn new(msg_id: Option<String>, cached: Arc<RwLock<File>>, fs: Arc<DiscordFs>) -> Self {
        Self {
            msg_id,
            cached,
            fs,
            loaded: Arc::new(Mutex::new(()))
        }
    }
    pub fn client(&self) -> &Arc<DiscordClient> {
        &self.fs.client
    }
    pub fn db(&self) -> &Arc<DB> {
        &self.fs.db
    }
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
    pub async fn load(&mut self) -> Result<()> {
        if let Err(_) = self.loaded.try_lock() {
            return Ok(())
        }
        let cached = self.cached.read().await;
        let meta = cached.metadata().clone();
        drop(cached);

        match &self.msg_id {
            Some(msg_id) => {
                let msg = self.client().get_message(msg_id).await?;

                let new_meta: Metadata = serde_json::from_str(&msg.content)?;

                if meta.is_dir() != new_meta.is_dir() {
                    eprintln!("Distant file is_dir value is different from local file is_dir value");
                    return Err(Error::BadContent)
                }

                if !meta.is_dir() {
                    *self.cached.write().await.metadata_mut() = new_meta;
                } else {
                    let url = &msg.attachments.get(0).ok_or(Error::DiscordAttachmentNotFound)?.url;
                    let content = self.client().get_attachment(url).await?;
                    let mut cached = self.cached.write().await;
                    cached.set_content(Some(content));
                    *cached.metadata_mut() = new_meta;
                }
            },
            None => {
                let mut cached = self.cached.write().await;
                if let None = cached.cached {
                    cached.cached = Some(Cursor::new(Vec::new()));
                }
                drop(cached);
                self.send_create().await?;
            }
        }

        

        Ok(())
    }
    /// Generate the message to send to Discord from the local file
    pub async fn get_msg_data(&self) -> Result<(String, Vec<(String, Vec<u8>)>)> {
        let cached = self.cached.read().await;
        let content = cached.cached.as_ref().ok_or(Error::FileContentIsNone)?.get_ref().to_owned();
        let meta = cached.metadata().clone();
        let id = *cached.id();
        drop(cached);

        Ok((serde_json::to_string(&meta)?, [(id.to_string(), content)].to_vec()))
    }
    /// Send the file to Discord for the first time
    pub async fn send_create(&mut self) -> Result<()> {
        let (content, attachments) = self.get_msg_data().await?;
        
        let msg_id = self.client().send_msg_with_attachment(&content, attachments).await?;

        self.msg_id = Some(msg_id.clone());

        self.db().edit_discord_file_msg_id_by_id(*self.cached.read().await.id(), msg_id).await?;

        Ok(())
    }
    /// Edit the file on Discord, return an error if the file was not sent
    pub async fn send_edit(&self) -> Result<()> {
        let msg_data = self.get_msg_data().await?;

        self.client().edit_msg_with_attachments(self.msg_id.as_ref().ok_or(Error::DiscordMessageIdIsNone)?, &msg_data.0, msg_data.1).await?;

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
            Ok(self.cached.read().await.metadata().clone().boxed() as Box<(dyn DavMetaData + 'static)>)
        }.boxed()
    }
    fn write_bytes<'a>(&'a mut self, buf: bytes::Bytes) -> webdav_handler::fs::FsFuture<()> {
        async move {
            self.load().await?;
            let _loaded_lock = self.loaded.try_lock();
            let mut cached = self.cached.write().await;
            let content = cached.cached.as_mut().unwrap();
            content.write_all(&buf[..]).await?;
            self.send_edit().await?;
            Ok(())
        }.boxed()
    }
    fn write_buf<'a>(&'a mut self, mut buf: Box<dyn bytes::Buf + Send>) -> webdav_handler::fs::FsFuture<()> {
        async move {
            self.load().await?;
            let _loaded_lock = self.loaded.try_lock();
            let mut cached = self.cached.write().await;
            let content = cached.cached.as_mut().unwrap();
            while buf.has_remaining() {
                let chunk = buf.chunk();
                let n = content.write(chunk).await?;
                buf.advance(n);
            }
            self.send_edit().await?;
            Ok(())
        }.boxed()
    }
    fn read_bytes<'a>(&'a mut self, count: usize) -> webdav_handler::fs::FsFuture<bytes::Bytes> {
        async move {
            self.load().await?;
            let _loaded_lock = self.loaded.try_lock();
            let mut buf = Vec::with_capacity(count);
            buf.fill(0);
            self.cached.write().await.cached.as_mut().unwrap().read_exact(&mut buf).await?;
            Ok(buf.into())
        }.boxed()
    }
    fn seek<'a>(&'a mut self, pos: std::io::SeekFrom) -> webdav_handler::fs::FsFuture<u64> {
        async move {
            self.load().await?;
            let _loaded_lock = self.loaded.try_lock();
            let mut cached = self.cached.write().await;
            let content = cached.cached.as_mut().unwrap();
            let res = content.seek(pos).await?;
            Ok(res)
        }.boxed()
    }
    fn flush<'a>(&'a mut self) -> webdav_handler::fs::FsFuture<()> {
        async move {
            let mut cached = self.cached.write().await;
            if let Some(c) = &mut cached.cached {
                c.flush().await?;
            }
            Ok(())
        }.boxed()
    }
}

#[derive(Clone, Debug)]
pub struct DiscordFs {
    db: Arc<DB>,
    client: Arc<DiscordClient>
}
impl DiscordFs {
    pub fn new(db: Arc<DB>, token: String, channel_id: String) -> Self {
        let client = DiscordClient::new(token, channel_id);

        Self {
            db,
            client: Arc::new(client)
        }
    }
}

impl DavFileSystem for DiscordFs {
    fn metadata<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath) -> webdav_handler::fs::FsFuture<Box<dyn DavMetaData>> {
        async move {
            let mut path = path.as_url_string();
            path = percent_encoding::percent_decode_str(&path).decode_utf8().map_err(|_|FsError::Forbidden)?.to_string();
            println!("metadata on {}", path);
            let mut file = self.db.get_discord_file_by_path(path, Arc::new(self.clone())).await?.unwrap();//.ok_or(FsError::NotFound)?;
            Ok(file.metadata().await?)
        }.boxed()
    }
    fn read_dir<'a>(
            &'a self,
            path: &'a webdav_handler::davpath::DavPath,
            meta: webdav_handler::fs::ReadDirMeta,
    ) -> webdav_handler::fs::FsFuture<webdav_handler::fs::FsStream<Box<dyn webdav_handler::fs::DavDirEntry>>> {
        async move {
            let mut path = path.as_url_string();
            path = percent_encoding::percent_decode_str(&path).decode_utf8().map_err(|_|FsError::Forbidden)?.to_string();
            println!("read_dir on {}", path);
            let dir = self.db.get_dir_entry_by_path(path).await?.unwrap();//.ok_or(FsError::NotFound)?;
            let entries: Vec<Box<dyn DavDirEntry>> = self.db.get_dir_entries_by_parent_id(dir.id).await?.into_iter().map(|e|Box::new(e) as Box<dyn DavDirEntry>).collect();
            let stream = futures::stream::iter(entries);
            Ok(Box::pin(stream) as webdav_handler::fs::FsStream<Box<dyn webdav_handler::fs::DavDirEntry>>)
        }.boxed()
    }
    fn open<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath, options: webdav_handler::fs::OpenOptions) -> webdav_handler::fs::FsFuture<Box<dyn DavFile>> {
        async move {
            let original_path = path;
            let mut path = path.as_url_string();
            path = percent_encoding::percent_decode_str(&path).decode_utf8().map_err(|_|FsError::Forbidden)?.to_string();
            println!("open on {}", path);
            dbg!(options);
            let file = self.db.get_file_by_path(path.to_string()).await?;
            if file.is_some() && options.create_new {
                return Err(FsError::Exists)
            }
            if file.is_none() {
                if options.create_new || options.create {
                    let p = original_path.as_pathbuf();
                    let parent_path = p.parent().unwrap().to_str().ok_or(FsError::Forbidden)?;//.ok_or(FsError::NotFound)?.to_str().ok_or(FsError::Forbidden)?;
                    let parent = if parent_path.is_empty() {
                        None
                    } else {
                        Some(self.db.get_dir_entry_by_path(parent_path.to_owned()).await?.unwrap())//.ok_or(FsError::NotFound)?)
                    };
                    self.db.insert_dir_entry(parent.map(|p|p.id), path.clone(), Metadata { len: 0, modified: None, is_dir: false }).await?;
                    return self.open(original_path, options).await
                } else {
                    return Err(FsError::NotFound)
                }
            }
            let mut file = file.unwrap();
            if options.append {
                file.cursor_pos = file.metadata().len;
            }

            Ok(Box::new(DiscordFile::new(None, Arc::new(RwLock::new(file)), Arc::new(self.clone()))))
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
    pub async fn get_attachment(&self, url: &str) -> Result<Vec<u8>> {
        Ok(self.http.get(url).send().await?.bytes().await?.to_vec())
    }
    pub async fn send_msg_with_attachment<T>(&self, content: &str, attachment: Vec<(String, T)>) -> Result<String>
    where T: Into<Cow<'static, [u8]>> {
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
            form = form.part(format!("files[{}]", i), multipart::Part::bytes(a).file_name(n))
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
    pub async fn edit_msg_with_attachments<T>(&self, msg_id: &str, content: &str, attachment: Vec<(String, T)>) -> Result<()>
    where T: Into<Cow<'static, [u8]>> {
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
            form = form.part(format!("files[{}]", i), multipart::Part::bytes(a).file_name(n))
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
