pub mod actions;
pub mod engine;
pub mod graph;
pub mod model;
pub mod recognition;
pub mod serial;
pub mod signal;
pub mod simulation;
pub mod storage;
pub mod training;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid request: {0}")]
    Invalid(String),
    #[error("Device: {0}")]
    Device(String),
    #[error("Input control: {0}")]
    Input(String),
    #[error("Storage: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("File: {0}")]
    Io(#[from] std::io::Error),
    #[error("Data: {0}")]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}
