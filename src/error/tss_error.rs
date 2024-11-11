use thiserror::Error;

#[derive(Error, Debug)]
pub enum TssError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] mongodb::error::Error),

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClientError(#[from] reqwest::Error),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JWT error: {0}")]
    JWTError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    #[error("Timeout error")]
    TimeoutError,

    #[error("Invalid party ID: {0}")]
    InvalidPartyId(u16),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(u16),

    #[error("Invalid number of parties: {0}")]
    InvalidNumberOfParties(u16),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl From<lapin::Error> for TssError {
    fn from(err: lapin::Error) -> Self {
        TssError::QueueError(err.to_string())
    }
}

impl From<anyhow::Error> for TssError {
    fn from(err: anyhow::Error) -> Self {
        TssError::UnexpectedError(err.to_string())
    }
}
