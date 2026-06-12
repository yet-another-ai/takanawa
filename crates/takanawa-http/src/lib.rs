//! HTTP range download engine built on the Takanawa core part-file primitives.

mod content_range;
mod downloader;
mod limiter;
mod state;

pub use downloader::{
    DownloadConfig, DownloadEngine, DownloadHandle, RetryConfig, TimeoutConfig,
    download_to_completion,
};
pub use limiter::{DEFAULT_MAX_IO, IoLimiter};
pub use state::{
    DownloadPhase, DownloadSnapshot, DownloadSpeedSnapshot, ProgressCallback, SpeedCallback,
};
