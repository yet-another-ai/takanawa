use std::path::PathBuf;

use thiserror::Error;

/// Result type used by Takanawa core APIs.
pub type Result<T> = std::result::Result<T, TakanawaError>;

/// Errors returned by Takanawa core, HTTP, and FFI layers.
#[derive(Debug, Error)]
pub enum TakanawaError {
    /// The caller supplied an invalid configuration value.
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// The final target file already exists.
    #[error("target file already exists: {0}")]
    TargetExists(PathBuf),

    /// Another process or handle owns the part-file lock.
    #[error("part file is busy: {0}")]
    PartBusy(PathBuf),

    /// The existing part file does not have the expected size.
    #[error("part file size mismatch: expected {expected} bytes, got {actual} bytes")]
    PartSizeMismatch { expected: u64, actual: u64 },

    /// Stored part metadata could not be decoded or validated.
    #[error("part metadata is corrupt: {0}")]
    PartCorrupt(String),

    /// Remote validators or size no longer match the stored part metadata.
    #[error("remote resource changed: {0}")]
    RemoteChanged(String),

    /// The server response violates the HTTP range download contract.
    #[error("HTTP protocol violation: {0}")]
    HttpProtocol(String),

    /// HTTP status that can be retried by the caller.
    #[error("retryable HTTP status: {0}")]
    RetryableHttpStatus(u16),

    /// Network transport failure.
    #[error("network error: {0}")]
    Network(String),

    /// Downloaded bytes did not match the configured hash.
    #[error("hash mismatch")]
    HashMismatch,

    /// The download was cancelled.
    #[error("download was cancelled")]
    Cancelled,

    /// The download was already started.
    #[error("download is already running")]
    AlreadyStarted,

    /// The download is not currently running.
    #[error("download is not running")]
    NotRunning,

    /// The runtime required by the requested operation is unavailable.
    #[error("runtime is not initialized")]
    RuntimeNotInitialized,

    /// A required FFI pointer was null.
    #[error("null pointer: {0}")]
    NullPointer(&'static str),

    /// An FFI struct was smaller than the ABI requires.
    #[error("ABI struct size mismatch for {name}: expected at least {expected}, got {actual}")]
    StructSizeMismatch {
        /// ABI struct name.
        name: &'static str,
        /// Minimum size required by this library.
        expected: usize,
        /// Size reported by the caller.
        actual: usize,
    },

    /// An FFI ABI version or layout did not match this library.
    #[error("ABI mismatch: {0}")]
    AbiMismatch(String),

    /// A string from an external boundary was not valid UTF-8.
    #[error("invalid UTF-8: {0}")]
    Utf8(String),

    /// Internal FFI/task boundary failure.
    #[error("FFI error: {0}")]
    Ffi(String),

    /// Filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl TakanawaError {
    #[must_use]
    /// Returns whether retrying the operation may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::RetryableHttpStatus(_))
    }
}
