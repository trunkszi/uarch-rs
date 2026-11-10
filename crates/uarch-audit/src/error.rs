#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] sonic_rs::Error),
    #[error("{0}")]
    Gate(String),
}

pub type Result<T> = std::result::Result<T, Error>;
