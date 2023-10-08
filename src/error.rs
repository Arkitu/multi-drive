pub enum Error {
    DB(tokio_rusqlite::Error)
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

pub type Result<T> = std::result::Result<T, Error>;