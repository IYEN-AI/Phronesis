use thiserror::Error;

#[derive(Error, Debug)]
pub enum PhronesisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Validation warning: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, PhronesisError>;
