use std::{path::PathBuf, vec, pin::Pin};
use futures::FutureExt;
use tokio::{fs, io::{AsyncWriteExt, AsyncReadExt, AsyncSeekExt}};
use webdav_handler::{fs::{DavFileSystem, DavFile, DavMetaData, FsError, FsFuture, DavDirEntry}, davpath::DavPath};

#[derive(Debug, Clone)]
pub struct LocalMetaData (std::fs::Metadata);
impl DavMetaData for LocalMetaData {
    fn len(&self) -> u64 {
        self.0.len()
    }
    fn modified(&self) -> webdav_handler::fs::FsResult<std::time::SystemTime> {
        self.0.modified().map_err(|_|FsError::GeneralFailure)
    }
    fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
}

#[derive(Debug)]
pub struct LocalFile (fs::File);
impl DavFile for LocalFile {
    fn metadata<'a>(&'a mut self) -> webdav_handler::fs::FsFuture<Box<dyn webdav_handler::fs::DavMetaData>> {
        Box::pin(async {
            let meta = self.0.metadata().await.map_err(|_|FsError::GeneralFailure)?;
            Ok::<Box<(dyn DavMetaData + 'static)>, FsError>(Box::new(LocalMetaData(meta)))
        })
    }
    fn write_buf<'a>(&'a mut self, buf: Box<dyn bytes::Buf + Send>) -> FsFuture<()> {
        Box::pin(async move {
            let buf = buf.chunk();
            self.0.write_all(buf).await.map_err(|_|FsError::GeneralFailure)?;
            Ok(())
        })
    }
    fn write_bytes<'a>(&'a mut self, buf: bytes::Bytes) -> FsFuture<()> {
        Box::pin(async move {
            self.0.write_all(&buf).await.map_err(|_|FsError::GeneralFailure)?;
            Ok(())
        })
    }
    fn read_bytes<'a>(&'a mut self, count: usize) -> FsFuture<bytes::Bytes> {
        Box::pin(async move {
            let mut buf = vec![0;count];
            self.0.read_exact(&mut buf).await.map_err(|_|FsError::GeneralFailure)?;
            Ok(buf.into())
        })
    }
    fn seek<'a>(&'a mut self, pos: std::io::SeekFrom) -> FsFuture<u64> {
        Box::pin(async move {
            self.0.seek(pos).await.map_err(|_|FsError::GeneralFailure)
        })
    }
    fn flush<'a>(&'a mut self) -> FsFuture<()> {
        Box::pin(async {
            self.0.flush().await.map_err(|_|FsError::GeneralFailure)
        })
    }
}

pub struct LocalDirEntry {
    name: Vec<u8>,
    meta: LocalMetaData
}
impl DavDirEntry for LocalDirEntry {
    fn name(&self) -> Vec<u8> {
        self.name
    }
    fn metadata<'a>(&'a self) -> FsFuture<Box<dyn DavMetaData>> {
        Box::pin(async {
            Ok::<Box<dyn DavMetaData>, FsError>(Box::new(self.meta))
        })
    }
}

#[derive(Clone)]
pub struct LocalFs {
    root: PathBuf
}
impl LocalFs {
    fn root_path(&self, path: &DavPath) -> PathBuf {
        self.root.join(path.as_rel_ospath())
    }
}

impl DavFileSystem for LocalFs {
    fn open<'a>(&'a self, path: &'a webdav_handler::davpath::DavPath, options: webdav_handler::fs::OpenOptions) -> webdav_handler::fs::FsFuture<Box<dyn webdav_handler::fs::DavFile>> {
        async move {
            let file = fs::File::options()
                    .read(options.read)
                    .write(options.write)
                    .append(options.append)
                    .truncate(options.truncate)
                    .create(options.create)
                    .create_new(options.create_new)
                    .open(self.root_path(path))
                    .await
                    .map_err(|_|FsError::GeneralFailure)?;
            Ok::<Box<dyn DavFile>, FsError>(Box::new(LocalFile(file)))
        }.boxed()
    }
    fn read_dir<'a>(
            &'a self,
            path: &'a webdav_handler::davpath::DavPath,
            meta: webdav_handler::fs::ReadDirMeta,
    ) -> FsFuture<webdav_handler::fs::FsStream<Box<dyn webdav_handler::fs::DavDirEntry>>> {
        Box::pin(async {
            let path = self.root_path(path);
            let name = match path.file_name() {
                None => Vec::new(),
                Some(n) => n.to_str().unwrap().as_bytes().to_owned()
            };
            let meta = LocalMetaData(fs::metadata(path).await.map_err(|_|FsError::GeneralFailure)?);

            Ok::<Pin<Box<(dyn futures::Stream<Item = Box<(dyn DavDirEntry + 'static)>> + std::marker::Send + 'static)>>, FsError>(Box::pin(futures::stream::once(async {Box::new(LocalDirEntry {
                name,
                meta
            })})))
        })
    }
}