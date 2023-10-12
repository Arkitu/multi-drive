#[derive(Debug)]
pub enum Error {
    DB(tokio_rusqlite::Error),
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Serde(serde_json::Error),
    Env(std::env::VarError),
    NotFound
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

pub type Result<T> = std::result::Result<T, Error>;