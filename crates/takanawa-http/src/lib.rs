#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

mod content_range;
mod downloader;
mod limiter;
mod state;

pub use downloader::{
    DownloadConfig, DownloadEngine, DownloadHandle, RetryConfig, TimeoutConfig,
    download_to_completion,
};
pub use limiter::{DEFAULT_MAX_IO, IoLimiter};
pub use state::{DownloadPhase, DownloadSnapshot, ProgressCallback};
