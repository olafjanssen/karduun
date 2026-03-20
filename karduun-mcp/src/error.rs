use thiserror::Error;

#[derive(Error, Debug)]
pub enum KarduunMcpError {
    #[error("MCP protocol error: {0}")]
    McpError(String),

    #[error("Cardstack error: {0}")]
    CardstackError(String),

    #[error("Handler not found: {0}")]
    HandlerNotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}
