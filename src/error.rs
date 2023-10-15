use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    DB(tokio_rusqlite::Error),
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Serde(serde_json::Error),
    Env(std::env::VarError),
    Parsing(std::num::ParseIntError),
    NotFound,
    BadContent,
    DiscordError,
    String(StringError)
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       match self {
           Self::DB(e) => write!(f, "Database error: {}", e),
           Self::Reqwest(e) => write!(f, "Reqwest error: {}", e),
           Self::Io(e) => write!(f, "IO error: {}", e),
           Self::Serde(e) => write!(f, "Serde error: {}", e),
           Self::Env(e) => write!(f, "Env error: {}", e),
           Self::Parsing(e) => write!(f, "Parsing error: {}", e),
           Self::NotFound => write!(f, "Not found"),
           Self::BadContent => write!(f, "Bad content"),
           Self::DiscordError => write!(f, "Discord error"),
           Self::String(e) => write!(f, "String error: {}", e.0)
       }
    }
}

impl From<tokio_rusqlite::Error> for Error {
    fn from(value: tokio_rusqlite::Error) -> Self {
        Self::DB(value)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Into::<tokio_rusqlite::Error>::into(value).into()
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<std::env::VarError> for Error {
    fn from(value: std::env::VarError) -> Self {
        Self::Env(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::Parsing(value)
    }
}

impl From<Error> for webdav_handler::fs::FsError {
    fn from(value: Error) -> Self {
        webdav_handler::fs::FsError::GeneralFailure
    }
}

impl From<StringError> for Error {
    fn from(value: StringError) -> Self {
        Self::String(value)
    }
}



pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct StringError (String);

impl<T: ToString> From<T> for StringError {
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

impl From<StringError> for webdav_handler::fs::FsError {
    fn from(value: StringError) -> Self {
        webdav_handler::fs::FsError::GeneralFailure
    }
}

pub type StringResult<T> = std::result::Result<T, StringError>;