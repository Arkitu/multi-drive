use std::time::SystemTime;

use futures::FutureExt;
use webdav_handler::fs::{DavFileSystem, DavFile, DavMetaData, FsError};

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
    meta: Metadata
}
impl DavFile for DiscordFile {
    fn metadata<'a>(&'a mut self) -> webdav_handler::fs::FsFuture<Box<dyn DavMetaData>> {
        async {
            Ok(Box::new(self.meta) as Box<dyn DavMetaData>)
        }.boxed()
    }
}

#[derive(Clone)]
pub struct DiscordFs {}

impl DavFileSystem for DiscordFs {
    fn open<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath, options: webdav_handler::fs::OpenOptions) -> webdav_handler::fs::FsFuture<Box<dyn webdav_handler::fs::DavFile>> {
        async {

        }.boxed()
    }
}