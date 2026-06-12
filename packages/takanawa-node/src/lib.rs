use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::{Buffer, Result};
use napi_derive::napi;
use takanawa_core::{HashConfig, HashKind};
use takanawa_http::{
    DownloadConfig, DownloadEngine, DownloadHandle, DownloadPhase, DownloadSnapshot, RetryConfig,
    TimeoutConfig, download_to_completion,
};
use tokio::runtime::{Builder, Runtime};

const DEFAULT_MAX_IO: usize = 4;

/// JavaScript-facing download options.
#[napi(object)]
pub struct NativeDownloadOptions {
    /// Source URL to download.
    pub url: String,
    /// Final output path.
    pub target_path: String,
    /// Requested chunk size as a decimal string. Missing or `0` selects the default.
    pub chunk_size: Option<String>,
    /// Requested chunk parallelism.
    pub parallelism: Option<u32>,
    /// Maximum chunks to download at the same time.
    pub max_parallel_chunks: Option<u32>,
    /// Maximum in-flight I/O operations.
    pub max_io: Option<u32>,
    /// Number of retries after the first attempt.
    pub max_retries: Option<u32>,
    /// Initial retry backoff in milliseconds.
    pub backoff_initial_ms: Option<u32>,
    /// Maximum retry backoff in milliseconds.
    pub backoff_max_ms: Option<u32>,
    /// Connection timeout in milliseconds.
    pub connect_timeout_ms: Option<u32>,
    /// Per-read timeout in milliseconds.
    pub read_timeout_ms: Option<u32>,
    /// Total timeout per probe/chunk attempt in milliseconds.
    pub total_timeout_ms: Option<u32>,
    /// Aggregate response-body bandwidth limit as a decimal string.
    pub bytes_per_second_limit: Option<String>,
    /// General hash verification configuration.
    pub hash: Option<NativeHashConfig>,
    /// Legacy SHA-256 expected digest string.
    pub sha256: Option<String>,
}

/// JavaScript-facing hash verification configuration.
#[napi(object)]
pub struct NativeHashConfig {
    /// Hash algorithm name, such as `sha256`, `sha1`, `sha512`, `md5`, or `crc32`.
    pub kind: String,
    /// Expected digest as lowercase or uppercase hex, with an optional algorithm prefix.
    pub expected: String,
}

/// JavaScript-facing download progress snapshot.
#[napi(object)]
pub struct NativeDownloadSnapshot {
    /// Current lifecycle phase.
    pub phase: String,
    /// Total content length as a decimal string.
    pub content_len: String,
    /// Number of bytes represented by committed chunks as a decimal string.
    pub downloaded_bytes: String,
    /// Chunk size as a decimal string.
    pub chunk_size: String,
    /// Total chunk count as a decimal string.
    pub chunk_count: String,
    /// Completed chunk count as a decimal string.
    pub completed_chunks: String,
    /// Current number of active I/O operations.
    pub active_io: u32,
    /// Last failure message, when present.
    pub last_error: Option<String>,
}

/// Downloads a URL to completion from JavaScript.
///
/// # Errors
///
/// Returns an error if options cannot be parsed, the native engine cannot be
/// created, or the download fails.
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

/// JavaScript-owned download task that supports start, pause, cancel, and polling.
#[napi]
pub struct NativeDownloadTask {
    runtime: Runtime,
    handle: Arc<DownloadHandle>,
}

#[napi]
impl NativeDownloadTask {
    /// Creates a new JavaScript download task.
    ///
    /// # Errors
    ///
    /// Returns an error if options cannot be parsed, the native engine cannot
    /// be created, or the Tokio runtime cannot be initialized.
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

    /// Starts or resumes the download.
    ///
    /// # Errors
    ///
    /// Returns an error if the task is already running.
    #[napi]
    pub fn start(&self) -> Result<()> {
        self.handle.start_on(&self.runtime).map_err(to_napi_error)
    }

    /// Requests that the download pause.
    ///
    /// # Errors
    ///
    /// This method currently does not fail, but returns `Result` to preserve the
    /// JavaScript API shape.
    #[napi]
    pub fn pause(&self) -> Result<()> {
        self.handle.pause().map_err(to_napi_error)
    }

    /// Requests cancellation of the download.
    ///
    /// # Errors
    ///
    /// This method currently does not fail, but returns `Result` to preserve the
    /// JavaScript API shape.
    #[napi]
    pub fn cancel(&self) -> Result<()> {
        self.handle.cancel().map_err(to_napi_error)
    }

    /// Returns the latest download snapshot.
    #[napi]
    pub fn snapshot(&self) -> NativeDownloadSnapshot {
        self.handle.snapshot().into()
    }

    /// Returns the serialized completion bitmap.
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
        hash: hash_config(options.hash, options.sha256)?,
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

fn hash_config(hash: Option<NativeHashConfig>, sha256: Option<String>) -> Result<HashConfig> {
    match (hash, sha256) {
        (None, None) => Ok(HashConfig::None),
        (None, Some(expected)) => hash_config_from_parts(HashKind::Sha256, &expected),
        (Some(hash), None) => {
            let kind = parse_hash_kind(&hash.kind)?;
            hash_config_from_parts(kind, &hash.expected)
        }
        (Some(_), Some(_)) => Err(napi::Error::from_reason(
            "use either hash or sha256, not both".to_string(),
        )),
    }
}

fn parse_hash_kind(value: &str) -> Result<HashKind> {
    match value.to_ascii_lowercase().as_str() {
        "sha1" | "sha-1" => Ok(HashKind::Sha1),
        "sha256" | "sha-256" => Ok(HashKind::Sha256),
        "sha512" | "sha-512" => Ok(HashKind::Sha512),
        "md5" => Ok(HashKind::Md5),
        "crc32" | "crc-32" => Ok(HashKind::Crc32),
        _ => Err(napi::Error::from_reason(format!(
            "unsupported hash kind: {value}"
        ))),
    }
}

fn hash_config_from_parts(kind: HashKind, value: &str) -> Result<HashConfig> {
    let normalized = value
        .strip_prefix(hash_prefix(kind))
        .or_else(|| value.strip_prefix(legacy_hash_prefix(kind)))
        .unwrap_or(value);
    let expected_len = kind.expected_len();
    if normalized.len() != expected_len * 2 {
        return Err(napi::Error::from_reason(format!(
            "invalid {}: expected {} hex characters",
            hash_label(kind),
            expected_len * 2
        )));
    }

    let mut hash = vec![0_u8; expected_len];
    for (index, byte) in hash.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&normalized[start..start + 2], 16).map_err(|err| {
            napi::Error::from_reason(format!("invalid {}: {err}", hash_label(kind)))
        })?;
    }
    HashConfig::from_expected_bytes(kind, &hash).ok_or_else(|| {
        napi::Error::from_reason(format!(
            "invalid {}: expected {} bytes",
            hash_label(kind),
            expected_len
        ))
    })
}

fn hash_prefix(kind: HashKind) -> &'static str {
    match kind {
        HashKind::None => "",
        HashKind::Sha1 => "sha1:",
        HashKind::Sha256 => "sha256:",
        HashKind::Sha512 => "sha512:",
        HashKind::Md5 => "md5:",
        HashKind::Crc32 => "crc32:",
    }
}

fn legacy_hash_prefix(kind: HashKind) -> &'static str {
    match kind {
        HashKind::Sha1 => "sha-1:",
        HashKind::Sha256 => "sha-256:",
        HashKind::Sha512 => "sha-512:",
        HashKind::Crc32 => "crc-32:",
        HashKind::None | HashKind::Md5 => hash_prefix(kind),
    }
}

fn hash_label(kind: HashKind) -> &'static str {
    match kind {
        HashKind::None => "none",
        HashKind::Sha1 => "sha1",
        HashKind::Sha256 => "sha256",
        HashKind::Sha512 => "sha512",
        HashKind::Md5 => "md5",
        HashKind::Crc32 => "crc32",
    }
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
