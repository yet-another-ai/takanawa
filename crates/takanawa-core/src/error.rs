use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, TakanawaError>;

#[derive(Debug, Error)]
pub enum TakanawaError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("target file already exists: {0}")]
    TargetExists(PathBuf),

    #[error("part file is busy: {0}")]
    PartBusy(PathBuf),

    #[error("part file size mismatch: expected {expected} bytes, got {actual} bytes")]
    PartSizeMismatch { expected: u64, actual: u64 },

    #[error("part metadata is corrupt: {0}")]
    PartCorrupt(String),

    #[error("remote resource changed: {0}")]
    RemoteChanged(String),

    #[error("HTTP protocol violation: {0}")]
    HttpProtocol(String),

    #[error("retryable HTTP status: {0}")]
    RetryableHttpStatus(u16),

    #[error("network error: {0}")]
    Network(String),

    #[error("hash mismatch")]
    HashMismatch,

    #[error("download was cancelled")]
    Cancelled,

    #[error("download is already running")]
    AlreadyStarted,

    #[error("download is not running")]
    NotRunning,

    #[error("runtime is not initialized")]
    RuntimeNotInitialized,

    #[error("null pointer: {0}")]
    NullPointer(&'static str),

    #[error("ABI struct size mismatch for {name}: expected at least {expected}, got {actual}")]
    StructSizeMismatch {
        name: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("ABI mismatch: {0}")]
    AbiMismatch(String),

    #[error("invalid UTF-8: {0}")]
    Utf8(String),

    #[error("FFI error: {0}")]
    Ffi(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl TakanawaError {
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::RetryableHttpStatus(_))
    }
}
