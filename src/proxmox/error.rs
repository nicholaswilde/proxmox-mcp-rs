use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxmoxError {
    #[error("API request failed: {0} - {1}")]
    Api(reqwest::StatusCode, String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[allow(dead_code)]
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Network/Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    #[allow(dead_code)]
    #[error("Task failed: UPID {0}")]
    Task(String),

    #[allow(dead_code)]
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Operation timed out: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, ProxmoxError>;
