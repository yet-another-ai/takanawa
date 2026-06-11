use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::{Buffer, Result};
use napi_derive::napi;
use takanawa_core::HashConfig;
use takanawa_http::{
    DownloadConfig, DownloadEngine, DownloadHandle, DownloadPhase, DownloadSnapshot, RetryConfig,
    TimeoutConfig, download_to_completion,
};
use tokio::runtime::{Builder, Runtime};

const DEFAULT_MAX_IO: usize = 4;

#[napi(object)]
pub struct NativeDownloadOptions {
    pub url: String,
    pub target_path: String,
    pub chunk_size: Option<String>,
    pub parallelism: Option<u32>,
    pub max_parallel_chunks: Option<u32>,
    pub max_io: Option<u32>,
    pub max_retries: Option<u32>,
    pub backoff_initial_ms: Option<u32>,
    pub backoff_max_ms: Option<u32>,
    pub connect_timeout_ms: Option<u32>,
    pub read_timeout_ms: Option<u32>,
    pub total_timeout_ms: Option<u32>,
    pub bytes_per_second_limit: Option<String>,
    pub sha256: Option<String>,
}

#[napi(object)]
pub struct NativeDownloadSnapshot {
    pub phase: String,
    pub content_len: String,
    pub downloaded_bytes: String,
    pub chunk_size: String,
    pub chunk_count: String,
    pub completed_chunks: String,
    pub active_io: u32,
    pub last_error: Option<String>,
}

#[napi(js_name = "nativeDownloadToCompletion")]
pub async fn native_download_to_completion(
    options: NativeDownloadOptions,
) -> Result<NativeDownloadSnapshot> {
    let max_io = max_io_from_options(&options);
    let engine = DownloadEngine::new(max_io).map_err(to_napi_error)?;
    let config = config_from_options(options)?;
    let snapshot = download_to_completion(engine, config)
        .await
        .map_err(to_napi_error)?;
    Ok(snapshot.into())
}

#[napi]
pub struct NativeDownloadTask {
    runtime: Runtime,
    handle: Arc<DownloadHandle>,
}

#[napi]
impl NativeDownloadTask {
    #[napi(constructor)]
    pub fn new(options: NativeDownloadOptions) -> Result<Self> {
        let max_io = max_io_from_options(&options);
        let engine = DownloadEngine::new(max_io).map_err(to_napi_error)?;
        let config = config_from_options(options)?;
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .thread_name("takanawa-node")
            .build()
            .map_err(to_napi_error)?;
        Ok(Self {
            runtime,
            handle: Arc::new(DownloadHandle::new(engine, config)),
        })
    }

    #[napi]
    pub fn start(&self) -> Result<()> {
        self.handle.start_on(&self.runtime).map_err(to_napi_error)
    }

    #[napi]
    pub fn pause(&self) -> Result<()> {
        self.handle.pause().map_err(to_napi_error)
    }

    #[napi]
    pub fn cancel(&self) -> Result<()> {
        self.handle.cancel().map_err(to_napi_error)
    }

    #[napi]
    pub fn snapshot(&self) -> NativeDownloadSnapshot {
        self.handle.snapshot().into()
    }

    #[napi]
    pub fn bitmap(&self) -> Buffer {
        self.handle.bitmap().into()
    }
}

fn max_io_from_options(options: &NativeDownloadOptions) -> usize {
    options
        .max_io
        .map_or(DEFAULT_MAX_IO, |max_io| max_io.max(1) as usize)
}

fn config_from_options(options: NativeDownloadOptions) -> Result<DownloadConfig> {
    Ok(DownloadConfig {
        url: options.url,
        target_path: PathBuf::from(options.target_path),
        chunk_size: parse_optional_u64(options.chunk_size, "chunkSize")?.unwrap_or(0),
        parallelism: options.parallelism.unwrap_or(0) as usize,
        max_parallel_chunks: options.max_parallel_chunks.unwrap_or(0) as usize,
        retry: RetryConfig {
            max_retries: options.max_retries.unwrap_or(4),
            backoff_initial: duration_from_ms(options.backoff_initial_ms),
            backoff_max: duration_from_ms(options.backoff_max_ms),
        },
        timeout: TimeoutConfig {
            connect: duration_from_ms(options.connect_timeout_ms),
            read: duration_from_ms(options.read_timeout_ms),
            total: duration_from_ms(options.total_timeout_ms),
        },
        bytes_per_second_limit: parse_optional_u64(
            options.bytes_per_second_limit,
            "bytesPerSecondLimit",
        )?
        .unwrap_or(0),
        hash: hash_config(options.sha256)?,
    })
}

fn duration_from_ms(value: Option<u32>) -> Duration {
    value.map_or(Duration::ZERO, |ms| Duration::from_millis(u64::from(ms)))
}

fn parse_optional_u64(value: Option<String>, field: &str) -> Result<Option<u64>> {
    value
        .map(|value| {
            value.parse::<u64>().map_err(|err| {
                napi::Error::from_reason(format!("invalid {field}: expected u64 string: {err}"))
            })
        })
        .transpose()
}

fn hash_config(value: Option<String>) -> Result<HashConfig> {
    let Some(value) = value else {
        return Ok(HashConfig::None);
    };
    let normalized = value.strip_prefix("sha256:").unwrap_or(&value);
    if normalized.len() != 64 {
        return Err(napi::Error::from_reason(
            "invalid sha256: expected 64 hex characters".to_string(),
        ));
    }
    let mut hash = [0_u8; 32];
    for (index, byte) in hash.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&normalized[start..start + 2], 16)
            .map_err(|err| napi::Error::from_reason(format!("invalid sha256: {err}")))?;
    }
    Ok(HashConfig::Sha256(hash))
}

fn phase_to_string(phase: DownloadPhase) -> String {
    match phase {
        DownloadPhase::Created => "created",
        DownloadPhase::Running => "running",
        DownloadPhase::Pausing => "pausing",
        DownloadPhase::Paused => "paused",
        DownloadPhase::Cancelling => "cancelling",
        DownloadPhase::Cancelled => "cancelled",
        DownloadPhase::Completed => "completed",
        DownloadPhase::Failed => "failed",
    }
    .to_string()
}

fn to_napi_error(error: impl std::error::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

impl From<DownloadSnapshot> for NativeDownloadSnapshot {
    fn from(snapshot: DownloadSnapshot) -> Self {
        Self {
            phase: phase_to_string(snapshot.phase),
            content_len: snapshot.content_len.to_string(),
            downloaded_bytes: snapshot.downloaded_bytes.to_string(),
            chunk_size: snapshot.chunk_size.to_string(),
            chunk_count: snapshot.chunk_count.to_string(),
            completed_chunks: snapshot.completed_chunks.to_string(),
            active_io: snapshot.active_io.try_into().unwrap_or(u32::MAX),
            last_error: snapshot.last_error,
        }
    }
}
