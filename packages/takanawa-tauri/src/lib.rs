#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value
)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde::{Deserialize, Serialize};
use takanawa_core::{HashConfig, HashKind, TakanawaError};
use takanawa_http::{
    DEFAULT_MAX_IO, DownloadConfig, DownloadEngine, DownloadHandle, DownloadPhase,
    DownloadSnapshot, DownloadSpeedSnapshot, ProgressCallback, RetryConfig, SpeedCallback,
    TimeoutConfig, download_to_completion as http_download_to_completion,
};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime as TokioRuntime};

const PROGRESS_EVENT: &str = "takanawa://download-progress";
const SPEED_EVENT: &str = "takanawa://download-speed";

type CommandResult<T> = Result<T, String>;

#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("takanawa")
        .setup(|app, _api| {
            let state = PluginState::new().map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create,
            start,
            pause,
            cancel,
            snapshot,
            bitmap,
            close,
            download_to_completion,
        ])
        .build()
}

struct PluginState {
    tasks: TaskRegistry,
    runtime: TokioRuntime,
    engine: DownloadEngine,
}

impl PluginState {
    fn new() -> CommandResult<Self> {
        Ok(Self {
            tasks: TaskRegistry::default(),
            runtime: RuntimeBuilder::new_multi_thread()
                .enable_all()
                .thread_name("takanawa-tauri")
                .build()
                .map_err(|err| to_command_error_with_code(-101, err))?,
            engine: DownloadEngine::new(DEFAULT_MAX_IO).map_err(to_command_error)?,
        })
    }

    fn set_max_io(&self, max_io: usize) {
        self.engine.set_max_io(max_io.max(1));
    }
}

impl Drop for PluginState {
    fn drop(&mut self) {
        self.tasks.close_all();
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDownloadOptions {
    url: String,
    target_path: String,
    chunk_size: Option<String>,
    parallelism: Option<usize>,
    max_parallel_chunks: Option<usize>,
    max_io: Option<usize>,
    max_retries: Option<u32>,
    backoff_initial_ms: Option<u32>,
    backoff_max_ms: Option<u32>,
    connect_timeout_ms: Option<u32>,
    read_timeout_ms: Option<u32>,
    total_timeout_ms: Option<u32>,
    bytes_per_second_limit: Option<String>,
    hash: Option<NativeHashConfig>,
    sha256: Option<String>,
}

impl NativeDownloadOptions {
    fn max_io(&self) -> usize {
        self.max_io.map_or(DEFAULT_MAX_IO, |max_io| max_io.max(1))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHashConfig {
    kind: String,
    expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NativeDownloadSnapshot {
    phase: String,
    content_len: String,
    downloaded_bytes: String,
    chunk_size: String,
    chunk_count: String,
    completed_chunks: String,
    active_io: usize,
    last_error: Option<String>,
    last_error_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NativeDownloadSpeedSnapshot {
    phase: String,
    content_len: String,
    received_bytes: String,
    interval_bytes: String,
    elapsed_millis: String,
    bytes_per_second: f64,
    active_io: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeTaskResult {
    task_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeSnapshotResult {
    snapshot: NativeDownloadSnapshot,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeBitmapResult {
    data: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeDownloadProgressEvent {
    task_id: String,
    snapshot: NativeDownloadSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeDownloadSpeedEvent {
    task_id: String,
    snapshot: NativeDownloadSpeedSnapshot,
}

#[derive(Default)]
struct TaskRegistry {
    next_id: AtomicU64,
    tasks: Mutex<HashMap<String, Arc<DownloadHandle>>>,
}

impl TaskRegistry {
    fn insert(&self, task: Arc<DownloadHandle>) -> String {
        let task_id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        self.tasks
            .lock()
            .expect("download task registry mutex poisoned")
            .insert(task_id.clone(), task);
        task_id
    }

    fn get(&self, task_id: &str) -> CommandResult<Arc<DownloadHandle>> {
        self.tasks
            .lock()
            .expect("download task registry mutex poisoned")
            .get(task_id)
            .cloned()
            .ok_or_else(|| invalid_config_error(format!("unknown download task: {task_id}")))
    }

    fn close(&self, task_id: &str) {
        if let Some(task) = self
            .tasks
            .lock()
            .expect("download task registry mutex poisoned")
            .remove(task_id)
        {
            task.set_progress_callback(None);
            task.set_speed_callback(None);
        }
    }

    fn close_all(&self) {
        let tasks = {
            let mut tasks = self
                .tasks
                .lock()
                .expect("download task registry mutex poisoned");
            std::mem::take(&mut *tasks)
        };
        for task in tasks.into_values() {
            task.set_progress_callback(None);
            task.set_speed_callback(None);
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.tasks
            .lock()
            .expect("download task registry mutex poisoned")
            .len()
    }
}

#[tauri::command]
fn create<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, PluginState>,
    options: NativeDownloadOptions,
) -> CommandResult<NativeTaskResult> {
    state.set_max_io(options.max_io());
    let config = config_from_options(options)?;
    let task = Arc::new(DownloadHandle::new(state.engine.clone(), config));
    let task_id = state.tasks.insert(Arc::clone(&task));
    let event_task_id = task_id.clone();
    let progress_app = app.clone();
    let progress_callback: ProgressCallback = Arc::new(move |snapshot| {
        let payload = NativeDownloadProgressEvent {
            task_id: event_task_id.clone(),
            snapshot: snapshot.into(),
        };
        let _ = progress_app.emit(PROGRESS_EVENT, payload);
    });
    task.set_progress_callback(Some(progress_callback));
    let speed_app = app.clone();
    let speed_task_id = task_id.clone();
    let speed_callback: SpeedCallback = Arc::new(move |snapshot| {
        let payload = NativeDownloadSpeedEvent {
            task_id: speed_task_id.clone(),
            snapshot: snapshot.into(),
        };
        let _ = speed_app.emit(SPEED_EVENT, payload);
    });
    task.set_speed_callback(Some(speed_callback));
    Ok(NativeTaskResult { task_id })
}

#[tauri::command]
fn start(state: State<'_, PluginState>, task_id: String) -> CommandResult<()> {
    state
        .tasks
        .get(&task_id)?
        .start_on(&state.runtime)
        .map_err(to_command_error)
}

#[tauri::command]
fn pause(state: State<'_, PluginState>, task_id: String) -> CommandResult<()> {
    state.tasks.get(&task_id)?.pause().map_err(to_command_error)
}

#[tauri::command]
fn cancel(state: State<'_, PluginState>, task_id: String) -> CommandResult<()> {
    state
        .tasks
        .get(&task_id)?
        .cancel()
        .map_err(to_command_error)
}

#[tauri::command]
fn snapshot(state: State<'_, PluginState>, task_id: String) -> CommandResult<NativeSnapshotResult> {
    Ok(NativeSnapshotResult {
        snapshot: state.tasks.get(&task_id)?.snapshot().into(),
    })
}

#[tauri::command]
fn bitmap(state: State<'_, PluginState>, task_id: String) -> CommandResult<NativeBitmapResult> {
    Ok(NativeBitmapResult {
        data: BASE64_STANDARD.encode(state.tasks.get(&task_id)?.bitmap()),
    })
}

#[tauri::command]
fn close(state: State<'_, PluginState>, task_id: String) {
    state.tasks.close(&task_id);
}

#[tauri::command]
async fn download_to_completion(
    state: State<'_, PluginState>,
    options: NativeDownloadOptions,
) -> CommandResult<NativeSnapshotResult> {
    state.set_max_io(options.max_io());
    let engine = state.engine.clone();
    let config = config_from_options(options)?;
    let snapshot = http_download_to_completion(engine, config)
        .await
        .map_err(to_command_error)?;
    Ok(NativeSnapshotResult {
        snapshot: snapshot.into(),
    })
}

fn config_from_options(options: NativeDownloadOptions) -> CommandResult<DownloadConfig> {
    Ok(DownloadConfig {
        url: options.url,
        target_path: PathBuf::from(options.target_path),
        chunk_size: parse_optional_u64(options.chunk_size.as_deref(), "chunkSize")?.unwrap_or(0),
        parallelism: options.parallelism.unwrap_or(0),
        max_parallel_chunks: options.max_parallel_chunks.unwrap_or(0),
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
            options.bytes_per_second_limit.as_deref(),
            "bytesPerSecondLimit",
        )?
        .unwrap_or(0),
        hash: hash_config(options.hash, options.sha256)?,
    })
}

fn duration_from_ms(value: Option<u32>) -> Duration {
    value.map_or(Duration::ZERO, |ms| Duration::from_millis(u64::from(ms)))
}

fn parse_optional_u64(value: Option<&str>, field: &str) -> CommandResult<Option<u64>> {
    value
        .map(|value| {
            value.parse::<u64>().map_err(|err| {
                invalid_config_error(format!(
                    "invalid {field}: expected unsigned 64-bit integer string: {err}"
                ))
            })
        })
        .transpose()
}

fn hash_config(
    hash: Option<NativeHashConfig>,
    sha256: Option<String>,
) -> CommandResult<HashConfig> {
    match (hash, sha256) {
        (None, None) => Ok(HashConfig::None),
        (None, Some(expected)) => hash_config_from_parts(HashKind::Sha256, &expected),
        (Some(hash), None) => {
            let kind = parse_hash_kind(&hash.kind)?;
            hash_config_from_parts(kind, &hash.expected)
        }
        (Some(_), Some(_)) => Err(invalid_config_error("use either hash or sha256, not both")),
    }
}

fn parse_hash_kind(value: &str) -> CommandResult<HashKind> {
    match value.to_ascii_lowercase().as_str() {
        "sha1" | "sha-1" => Ok(HashKind::Sha1),
        "sha256" | "sha-256" => Ok(HashKind::Sha256),
        "sha512" | "sha-512" => Ok(HashKind::Sha512),
        "md5" => Ok(HashKind::Md5),
        "crc32" | "crc-32" => Ok(HashKind::Crc32),
        _ => Err(invalid_config_error(format!(
            "unsupported hash kind: {value}"
        ))),
    }
}

fn hash_config_from_parts(kind: HashKind, value: &str) -> CommandResult<HashConfig> {
    let normalized = value
        .strip_prefix(hash_prefix(kind))
        .or_else(|| value.strip_prefix(legacy_hash_prefix(kind)))
        .unwrap_or(value);
    let expected_len = kind.expected_len();
    if normalized.len() != expected_len * 2 {
        return Err(invalid_config_error(format!(
            "invalid {}: expected {} hex characters",
            hash_label(kind),
            expected_len * 2
        )));
    }

    let mut hash = vec![0_u8; expected_len];
    for (index, byte) in hash.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&normalized[start..start + 2], 16)
            .map_err(|err| invalid_config_error(format!("invalid {}: {err}", hash_label(kind))))?;
    }
    HashConfig::from_expected_bytes(kind, &hash).ok_or_else(|| {
        invalid_config_error(format!(
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
        HashKind::None => "",
        HashKind::Sha1 => "sha-1:",
        HashKind::Sha256 => "sha-256:",
        HashKind::Sha512 => "sha-512:",
        HashKind::Md5 => "md5:",
        HashKind::Crc32 => "crc-32:",
    }
}

fn hash_label(kind: HashKind) -> &'static str {
    match kind {
        HashKind::None => "hash",
        HashKind::Sha1 => "SHA-1",
        HashKind::Sha256 => "SHA-256",
        HashKind::Sha512 => "SHA-512",
        HashKind::Md5 => "MD5",
        HashKind::Crc32 => "CRC32",
    }
}

fn to_command_error(error: TakanawaError) -> String {
    to_command_error_with_code(error.status_code(), error)
}

fn to_command_error_with_code(code: i32, error: impl std::fmt::Display) -> String {
    format!("takanawa error {code}: {error}")
}

fn invalid_config_error(message: impl std::fmt::Display) -> String {
    to_command_error_with_code(-3, message)
}

impl From<DownloadSnapshot> for NativeDownloadSnapshot {
    fn from(snapshot: DownloadSnapshot) -> Self {
        Self {
            phase: phase_to_string(snapshot.phase).to_string(),
            content_len: snapshot.content_len.to_string(),
            downloaded_bytes: snapshot.downloaded_bytes.to_string(),
            chunk_size: snapshot.chunk_size.to_string(),
            chunk_count: snapshot.chunk_count.to_string(),
            completed_chunks: snapshot.completed_chunks.to_string(),
            active_io: snapshot.active_io,
            last_error: snapshot.last_error,
            last_error_code: snapshot.last_error_code,
        }
    }
}

impl From<DownloadSpeedSnapshot> for NativeDownloadSpeedSnapshot {
    fn from(snapshot: DownloadSpeedSnapshot) -> Self {
        Self {
            phase: phase_to_string(snapshot.phase).to_string(),
            content_len: snapshot.content_len.to_string(),
            received_bytes: snapshot.received_bytes.to_string(),
            interval_bytes: snapshot.interval_bytes.to_string(),
            elapsed_millis: snapshot.elapsed_millis.to_string(),
            bytes_per_second: snapshot.bytes_per_second,
            active_io: snapshot.active_io,
        }
    }
}

fn phase_to_string(phase: DownloadPhase) -> &'static str {
    match phase {
        DownloadPhase::Created => "created",
        DownloadPhase::Starting => "starting",
        DownloadPhase::Allocating => "allocating",
        DownloadPhase::Running => "running",
        DownloadPhase::Pausing => "pausing",
        DownloadPhase::Paused => "paused",
        DownloadPhase::Verifying => "verifying",
        DownloadPhase::Cancelling => "cancelling",
        DownloadPhase::Cancelled => "cancelled",
        DownloadPhase::Completed => "completed",
        DownloadPhase::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_tracks_and_closes_tasks() {
        let registry = TaskRegistry::default();
        let engine = DownloadEngine::new(DEFAULT_MAX_IO).unwrap();
        let task = Arc::new(DownloadHandle::new(engine, test_config()));
        let task_id = registry.insert(Arc::clone(&task));

        assert_eq!(registry.len(), 1);
        assert!(Arc::ptr_eq(&registry.get(&task_id).unwrap(), &task));

        registry.close(&task_id);
        assert_eq!(registry.len(), 0);
        assert!(registry.get(&task_id).is_err());
    }

    #[test]
    fn maps_snapshot_to_camel_case_string_payload() {
        let snapshot = NativeDownloadSnapshot::from(DownloadSnapshot {
            phase: DownloadPhase::Allocating,
            content_len: 9_007_199_254_740_993,
            downloaded_bytes: 10,
            chunk_size: 5,
            chunk_count: 2,
            completed_chunks: 1,
            active_io: 1,
            last_error: Some("waiting".to_string()),
            last_error_code: Some(-13),
        });

        assert_eq!(snapshot.phase, "allocating");
        assert_eq!(snapshot.content_len, "9007199254740993");
        assert_eq!(snapshot.downloaded_bytes, "10");
        assert_eq!(snapshot.last_error.as_deref(), Some("waiting"));
        assert_eq!(snapshot.last_error_code, Some(-13));
    }

    #[test]
    fn parses_hashes_and_rejects_conflicts() {
        let config = config_from_options(NativeDownloadOptions {
            hash: Some(NativeHashConfig {
                kind: "sha-512".to_string(),
                expected: "00".repeat(64),
            }),
            ..test_options()
        })
        .unwrap();
        assert_eq!(config.hash.kind(), HashKind::Sha512);

        let err = config_from_options(NativeDownloadOptions {
            hash: Some(NativeHashConfig {
                kind: "sha256".to_string(),
                expected: "00".repeat(32),
            }),
            sha256: Some("00".repeat(32)),
            ..test_options()
        })
        .unwrap_err();
        assert!(err.contains("use either hash or sha256"));
    }

    #[test]
    fn rejects_invalid_u64_strings() {
        let err = config_from_options(NativeDownloadOptions {
            chunk_size: Some("-1".to_string()),
            ..test_options()
        })
        .unwrap_err();
        assert!(err.contains("chunkSize"));
    }

    fn test_options() -> NativeDownloadOptions {
        NativeDownloadOptions {
            url: "https://example.test/file.bin".to_string(),
            target_path: "/tmp/file.bin".to_string(),
            chunk_size: None,
            parallelism: None,
            max_parallel_chunks: None,
            max_io: Some(DEFAULT_MAX_IO),
            max_retries: None,
            backoff_initial_ms: None,
            backoff_max_ms: None,
            connect_timeout_ms: None,
            read_timeout_ms: None,
            total_timeout_ms: None,
            bytes_per_second_limit: None,
            hash: None,
            sha256: None,
        }
    }

    fn test_config() -> DownloadConfig {
        config_from_options(test_options()).unwrap()
    }
}
