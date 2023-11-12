mod drives;
mod db;
mod error;
mod types;

use types::Metadata;
use error::Result;
use std::{env, fs};
use std::sync::Arc;
use actix_web::{web, App, HttpServer};
use webdav_handler::actix::*;
use webdav_handler::{fakels::FakeLs, DavConfig, DavHandler};

pub async fn dav_handler(req: DavRequest, davhandler: web::Data<DavHandler>) -> DavResponse {
    dbg!(req.request.uri());
    dbg!(req.request.headers());
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
    let cache = "./cache";

    if fs::metadata(cache).is_err() {
        fs::create_dir(cache)?;
    }

    let db = Arc::new(db::DB::new(Some("test.db")).await);
    db.create_tables().await;
    db.insert_dir_entry(None, "/".to_string(), Metadata {
        len: 0,
        modified: None,
        is_dir: true
    }).await?;

    let d_fs = drives::discord::DiscordFs::new(db, env::var("DISCORD_TOKEN")?, env::var("DISCORD_CHANNEL")?);

    let dav_server = DavHandler::builder()
        .filesystem(Box::new(d_fs))
        .locksystem(FakeLs::new())
        .build_handler();

    println!("actix-web example: listening on {}", addr);

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