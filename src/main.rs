mod drives;
mod db;
mod error;
mod types;
use bytes::Bytes;
use error::Result;
use drives::discord::DiscordFile;
use types::File;

use tokio::sync::RwLock;
use std::sync::Arc;
use std::env;
use actix_web::{web, App, HttpServer};
use webdav_handler::actix::*;
use webdav_handler::{fakels::FakeLs, localfs::LocalFs, DavConfig, DavHandler};

use crate::types::Metadata;

pub async fn dav_handler(req: DavRequest, davhandler: web::Data<DavHandler>) -> DavResponse {
    if let Some(prefix) = req.prefix() {
        let config = DavConfig::new().strip_prefix(prefix);
        davhandler.handle_with(config, req.request).await.into()
    } else {
        davhandler.handle(req.request).await.into()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv::from_filename("config.env").unwrap();

    let addr = "127.0.0.1:4918";
    let dir = "/tmp";

    let d_client = drives::discord::DiscordClient::new(env::var("DISCORD_TOKEN")?, env::var("DISCORD_CHANNEL")?);
    
    d_client.send_msg_with_attachment("lalalal", [("fichier.txt".to_string(), Bytes::from_static(b"sqjdksvbxwcn"))].to_vec()).await?;

    let mut file = DiscordFile {
        msg_id: None,
        cached: Arc::new(RwLock::new(
            File {
                path: "lalala.txt".to_string(),
                id: 0,
                content: Some(Bytes::from_static(b"avant")),
                metadata: Some(Metadata {
                    len: 5,
                    modified: None,
                    is_dir: false
                })
            }
        )),
        client: Arc::new(d_client)
    };

    file.send_create().await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    file.cached.write().await.content = Some(Bytes::from_static(b"apres"));

    file.send_edit().await?;

    file.load().await?;
    //.get_message("1161354218779717843").await;

    let dav_server = DavHandler::builder()
        .filesystem(LocalFs::new(dir, false, false, false))
        .locksystem(FakeLs::new())
        .build_handler();

    println!("actix-web example: listening on {} serving {}", addr, dir);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dav_server.clone()))
            .service(web::resource("/{tail:.*}").to(dav_handler))
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}