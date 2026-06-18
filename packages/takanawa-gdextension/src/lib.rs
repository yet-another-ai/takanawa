#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value,
    unsafe_code
)]

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use godot::builtin::{PackedByteArray, VarDictionary, Variant, VariantType};
use godot::classes::{INode, Node};
use godot::init::{ExtensionLibrary, gdextension};
use godot::prelude::*;
use takanawa_core::{HashConfig, HashKind};
use takanawa_http::{
    DEFAULT_MAX_IO, DownloadConfig, DownloadEngine, DownloadHandle, DownloadPhase,
    DownloadSnapshot, DownloadSpeedSnapshot, ProgressCallback, RetryConfig, SpeedCallback,
    TimeoutConfig,
};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime as TokioRuntime};

const EVENT_CHANNEL_CAPACITY: usize = 1024;

type CommandResult<T> = Result<T, String>;

static RUNTIME: OnceLock<CommandResult<TokioRuntime>> = OnceLock::new();

struct TakanawaGodot;

#[gdextension]
unsafe impl ExtensionLibrary for TakanawaGodot {}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct TakanawaDownload {
    base: Base<Node>,
    handle: Option<Arc<DownloadHandle>>,
    receiver: Option<Receiver<DownloadEvent>>,
    last_error: String,
    last_error_code: i32,
    terminal_signal_emitted: bool,
}

#[godot_api]
impl INode for TakanawaDownload {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            handle: None,
            receiver: None,
            last_error: String::new(),
            last_error_code: 0,
            terminal_signal_emitted: false,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_process(false);
    }

    fn process(&mut self, _delta: f64) {
        self.drain_events();
    }
}

#[godot_api]
impl TakanawaDownload {
    #[signal]
    fn progress(snapshot: Variant);

    #[signal]
    fn speed(snapshot: Variant);

    #[signal]
    fn completed(snapshot: Variant);

    #[signal]
    fn failed(message: GString, snapshot: Variant);

    #[signal]
    fn cancelled(snapshot: Variant);

    #[func]
    pub fn configure(&mut self, options: VarDictionary) -> bool {
        self.close_task();
        self.last_error.clear();
        self.last_error_code = 0;
        self.terminal_signal_emitted = false;

        let options = match NativeDownloadOptions::from_dictionary(&options) {
            Ok(options) => options,
            Err(error) => return self.fail_with_code(-3, error),
        };
        let max_io = options.max_io();
        let config = match config_from_options(options) {
            Ok(config) => config,
            Err(error) => return self.fail_with_code(-3, error),
        };
        let engine = match DownloadEngine::new(max_io) {
            Ok(engine) => engine,
            Err(error) => return self.fail_with_code(error.status_code(), error.to_string()),
        };

        let handle = Arc::new(DownloadHandle::new(engine, config));
        let (sender, receiver) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);
        install_callbacks(&handle, sender);
        self.handle = Some(handle);
        self.receiver = Some(receiver);
        self.base_mut().set_process(true);
        true
    }

    #[func]
    pub fn start(&mut self) -> bool {
        let Some(handle) = self.handle.as_ref() else {
            return self.fail_with_code(-3, "download task is not configured".to_string());
        };
        let runtime = match runtime() {
            Ok(runtime) => runtime,
            Err(error) => return self.fail_with_code(-101, error),
        };
        match handle.start_on(runtime) {
            Ok(()) => true,
            Err(error) => self.fail_with_code(error.status_code(), error.to_string()),
        }
    }

    #[func]
    pub fn pause(&mut self) -> bool {
        let Some(handle) = self.handle.as_ref() else {
            return self.fail_with_code(-3, "download task is not configured".to_string());
        };
        match handle.pause() {
            Ok(()) => true,
            Err(error) => self.fail_with_code(error.status_code(), error.to_string()),
        }
    }

    #[func]
    pub fn cancel(&mut self) -> bool {
        let Some(handle) = self.handle.as_ref() else {
            return self.fail_with_code(-3, "download task is not configured".to_string());
        };
        match handle.cancel() {
            Ok(()) => true,
            Err(error) => self.fail_with_code(error.status_code(), error.to_string()),
        }
    }

    #[func]
    pub fn snapshot(&self) -> VarDictionary {
        self.handle
            .as_ref()
            .map_or_else(VarDictionary::new, |handle| {
                snapshot_to_dictionary(handle.snapshot())
            })
    }

    #[func]
    pub fn speed_snapshot(&self) -> VarDictionary {
        self.handle
            .as_ref()
            .map_or_else(VarDictionary::new, |handle| {
                speed_snapshot_to_dictionary(handle.speed_snapshot())
            })
    }

    #[func]
    pub fn bitmap(&self) -> PackedByteArray {
        self.handle
            .as_ref()
            .map_or_else(PackedByteArray::new, |handle| {
                PackedByteArray::from(handle.bitmap().as_slice())
            })
    }

    #[func]
    pub fn close(&mut self) {
        self.close_task();
        self.base_mut().set_process(false);
    }

    #[func]
    #[must_use]
    pub fn last_error(&self) -> GString {
        GString::from(self.last_error.as_str())
    }

    #[func]
    #[must_use]
    pub fn last_error_code(&self) -> i32 {
        self.last_error_code
    }

    fn close_task(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.set_progress_callback(None);
            handle.set_speed_callback(None);
        }
        self.receiver = None;
        self.terminal_signal_emitted = false;
    }

    fn drain_events(&mut self) {
        let events = self.collect_pending_events();
        for event in events {
            match event {
                DownloadEvent::Progress(snapshot) => self.emit_progress(snapshot),
                DownloadEvent::Speed(snapshot) => self.emit_speed(snapshot),
            }
        }
    }

    fn collect_pending_events(&self) -> Vec<DownloadEvent> {
        self.receiver
            .as_ref()
            .map_or_else(Vec::new, collect_pending_events)
    }

    fn emit_progress(&mut self, snapshot: DownloadSnapshot) {
        let payload = snapshot_to_dictionary(snapshot.clone());
        self.base_mut()
            .emit_signal("progress", &[payload.to_variant()]);

        if self.terminal_signal_emitted {
            return;
        }

        match snapshot.phase {
            DownloadPhase::Completed => {
                self.terminal_signal_emitted = true;
                self.base_mut()
                    .emit_signal("completed", &[payload.to_variant()]);
            }
            DownloadPhase::Cancelled => {
                self.terminal_signal_emitted = true;
                self.base_mut()
                    .emit_signal("cancelled", &[payload.to_variant()]);
            }
            DownloadPhase::Failed => {
                self.terminal_signal_emitted = true;
                let message = snapshot.last_error.as_deref().unwrap_or("download failed");
                self.base_mut().emit_signal(
                    "failed",
                    &[GString::from(message).to_variant(), payload.to_variant()],
                );
            }
            _ => {}
        }
    }

    fn emit_speed(&mut self, snapshot: DownloadSpeedSnapshot) {
        let payload = speed_snapshot_to_dictionary(snapshot);
        self.base_mut()
            .emit_signal("speed", &[payload.to_variant()]);
    }

    fn fail_with_code(&mut self, code: i32, error: String) -> bool {
        self.last_error = error;
        self.last_error_code = code;
        godot::global::godot_error!("[TakanawaDownload] {}", self.last_error);
        false
    }
}

impl Drop for TakanawaDownload {
    fn drop(&mut self) {
        self.close_task();
    }
}

enum DownloadEvent {
    Progress(DownloadSnapshot),
    Speed(DownloadSpeedSnapshot),
}

#[derive(Debug)]
struct NativeDownloadOptions {
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
    fn from_dictionary(dict: &VarDictionary) -> CommandResult<Self> {
        Ok(Self {
            url: required_string(dict, &["url"])?,
            target_path: required_string(dict, &["target_path", "targetPath"])?,
            chunk_size: optional_string(dict, &["chunk_size", "chunkSize"]),
            parallelism: optional_usize(dict, &["parallelism"], "parallelism")?,
            max_parallel_chunks: optional_usize(
                dict,
                &["max_parallel_chunks", "maxParallelChunks"],
                "maxParallelChunks",
            )?,
            max_io: optional_usize(dict, &["max_io", "maxIo"], "maxIo")?,
            max_retries: optional_u32(dict, &["max_retries", "maxRetries"], "maxRetries")?,
            backoff_initial_ms: optional_u32(
                dict,
                &["backoff_initial_ms", "backoffInitialMs"],
                "backoffInitialMs",
            )?,
            backoff_max_ms: optional_u32(
                dict,
                &["backoff_max_ms", "backoffMaxMs"],
                "backoffMaxMs",
            )?,
            connect_timeout_ms: optional_u32(
                dict,
                &["connect_timeout_ms", "connectTimeoutMs"],
                "connectTimeoutMs",
            )?,
            read_timeout_ms: optional_u32(
                dict,
                &["read_timeout_ms", "readTimeoutMs"],
                "readTimeoutMs",
            )?,
            total_timeout_ms: optional_u32(
                dict,
                &["total_timeout_ms", "totalTimeoutMs"],
                "totalTimeoutMs",
            )?,
            bytes_per_second_limit: optional_string(
                dict,
                &["bytes_per_second_limit", "bytesPerSecondLimit"],
            ),
            hash: optional_hash_config(dict)?,
            sha256: optional_string(dict, &["sha256"]),
        })
    }

    fn max_io(&self) -> usize {
        self.max_io.map_or(DEFAULT_MAX_IO, |max_io| max_io.max(1))
    }
}

#[derive(Debug)]
struct NativeHashConfig {
    kind: String,
    expected: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeDownloadSnapshot {
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

#[derive(Debug, Clone, PartialEq)]
struct NativeDownloadSpeedSnapshot {
    phase: String,
    content_len: String,
    received_bytes: String,
    interval_bytes: String,
    elapsed_millis: String,
    bytes_per_second: f64,
    active_io: usize,
}

fn install_callbacks(handle: &Arc<DownloadHandle>, sender: SyncSender<DownloadEvent>) {
    let progress_sender = sender.clone();
    let progress_callback: ProgressCallback = Arc::new(move |snapshot| {
        let _ = progress_sender.try_send(DownloadEvent::Progress(snapshot));
    });
    handle.set_progress_callback(Some(progress_callback));

    let speed_callback: SpeedCallback = Arc::new(move |snapshot| {
        let _ = sender.try_send(DownloadEvent::Speed(snapshot));
    });
    handle.set_speed_callback(Some(speed_callback));
}

fn runtime() -> CommandResult<&'static TokioRuntime> {
    RUNTIME
        .get_or_init(|| {
            RuntimeBuilder::new_multi_thread()
                .enable_all()
                .thread_name("takanawa-gdextension")
                .build()
                .map_err(|err| err.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn collect_pending_events(receiver: &Receiver<DownloadEvent>) -> Vec<DownloadEvent> {
    let mut events = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        events.push(event);
    }
    events
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

fn optional_hash_config(dict: &VarDictionary) -> CommandResult<Option<NativeHashConfig>> {
    let Some(value) = dictionary_value(dict, &["hash"]) else {
        return Ok(None);
    };
    if value.get_type() != VariantType::DICTIONARY {
        return Err("hash must be a Dictionary with kind and expected fields".to_string());
    }
    let hash = value.to::<VarDictionary>();
    Ok(Some(NativeHashConfig {
        kind: required_string(&hash, &["kind"])?,
        expected: required_string(&hash, &["expected"])?,
    }))
}

fn required_string(dict: &VarDictionary, keys: &[&str]) -> CommandResult<String> {
    optional_string(dict, keys)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing required field: {}", keys[0]))
}

fn optional_string(dict: &VarDictionary, keys: &[&str]) -> Option<String> {
    dictionary_value(dict, keys).map(|value| value.stringify().to_string())
}

fn optional_u32(dict: &VarDictionary, keys: &[&str], field: &str) -> CommandResult<Option<u32>> {
    optional_u64(dict, keys, field)?
        .map(|value| {
            u32::try_from(value).map_err(|_| format!("invalid {field}: value is larger than u32"))
        })
        .transpose()
}

fn optional_usize(
    dict: &VarDictionary,
    keys: &[&str],
    field: &str,
) -> CommandResult<Option<usize>> {
    optional_u64(dict, keys, field)?
        .map(|value| {
            usize::try_from(value)
                .map_err(|_| format!("invalid {field}: value is larger than usize"))
        })
        .transpose()
}

fn optional_u64(dict: &VarDictionary, keys: &[&str], field: &str) -> CommandResult<Option<u64>> {
    parse_optional_u64(optional_string(dict, keys).as_deref(), field)
}

fn parse_optional_u64(value: Option<&str>, field: &str) -> CommandResult<Option<u64>> {
    value
        .map(|value| {
            value.parse::<u64>().map_err(|err| {
                format!("invalid {field}: expected unsigned 64-bit integer string: {err}")
            })
        })
        .transpose()
}

fn dictionary_value(dict: &VarDictionary, keys: &[&str]) -> Option<Variant> {
    keys.iter().find_map(|key| dict.get(*key))
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
        (Some(_), Some(_)) => Err("use either hash or sha256, not both".to_string()),
    }
}

fn parse_hash_kind(value: &str) -> CommandResult<HashKind> {
    match value.to_ascii_lowercase().as_str() {
        "sha1" | "sha-1" => Ok(HashKind::Sha1),
        "sha256" | "sha-256" => Ok(HashKind::Sha256),
        "sha512" | "sha-512" => Ok(HashKind::Sha512),
        "md5" => Ok(HashKind::Md5),
        "crc32" | "crc-32" => Ok(HashKind::Crc32),
        _ => Err(format!("unsupported hash kind: {value}")),
    }
}

fn hash_config_from_parts(kind: HashKind, value: &str) -> CommandResult<HashConfig> {
    let normalized = value
        .strip_prefix(hash_prefix(kind))
        .or_else(|| value.strip_prefix(legacy_hash_prefix(kind)))
        .unwrap_or(value);
    let expected_len = kind.expected_len();
    if normalized.len() != expected_len * 2 {
        return Err(format!(
            "invalid {}: expected {} hex characters",
            hash_label(kind),
            expected_len * 2
        ));
    }

    let mut hash = vec![0_u8; expected_len];
    for (index, byte) in hash.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&normalized[start..start + 2], 16)
            .map_err(|err| format!("invalid {}: {err}", hash_label(kind)))?;
    }
    HashConfig::from_expected_bytes(kind, &hash).ok_or_else(|| {
        format!(
            "invalid {}: expected {} bytes",
            hash_label(kind),
            expected_len
        )
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

fn snapshot_to_dictionary(snapshot: DownloadSnapshot) -> VarDictionary {
    let snapshot = NativeDownloadSnapshot::from(snapshot);
    let mut dict = VarDictionary::new();
    dict.set("phase", snapshot.phase);
    dict.set("content_len", snapshot.content_len);
    dict.set("downloaded_bytes", snapshot.downloaded_bytes);
    dict.set("chunk_size", snapshot.chunk_size);
    dict.set("chunk_count", snapshot.chunk_count);
    dict.set("completed_chunks", snapshot.completed_chunks);
    dict.set("active_io", active_io_to_i64(snapshot.active_io));
    if let Some(error) = snapshot.last_error {
        dict.set("last_error", error);
    } else {
        dict.set("last_error", &Variant::nil());
    }
    if let Some(error_code) = snapshot.last_error_code {
        dict.set("last_error_code", error_code);
    } else {
        dict.set("last_error_code", &Variant::nil());
    }
    dict
}

fn speed_snapshot_to_dictionary(snapshot: DownloadSpeedSnapshot) -> VarDictionary {
    let snapshot = NativeDownloadSpeedSnapshot::from(snapshot);
    let mut dict = VarDictionary::new();
    dict.set("phase", snapshot.phase);
    dict.set("content_len", snapshot.content_len);
    dict.set("received_bytes", snapshot.received_bytes);
    dict.set("interval_bytes", snapshot.interval_bytes);
    dict.set("elapsed_millis", snapshot.elapsed_millis);
    dict.set("bytes_per_second", snapshot.bytes_per_second);
    dict.set("active_io", active_io_to_i64(snapshot.active_io));
    dict
}

fn active_io_to_i64(active_io: usize) -> i64 {
    i64::try_from(active_io).unwrap_or(i64::MAX)
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
    fn parses_options_and_hashes() {
        let config = config_from_options(NativeDownloadOptions {
            hash: Some(NativeHashConfig {
                kind: "sha-512".to_string(),
                expected: "00".repeat(64),
            }),
            ..test_options()
        })
        .expect("options should parse");

        assert_eq!(config.url, "https://example.test/file.bin");
        assert_eq!(config.target_path, PathBuf::from("/tmp/file.bin"));
        assert_eq!(config.hash.kind(), HashKind::Sha512);
    }

    #[test]
    fn rejects_conflicting_hash_forms() {
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
    fn maps_snapshot_to_payload() {
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
        assert_eq!(snapshot.last_error.as_deref(), Some("waiting"));
        assert_eq!(snapshot.last_error_code, Some(-13));
    }

    #[test]
    fn collects_pending_events_without_blocking() {
        let (sender, receiver) = mpsc::sync_channel(2);
        sender
            .try_send(DownloadEvent::Progress(DownloadSnapshot {
                phase: DownloadPhase::Created,
                content_len: 0,
                downloaded_bytes: 0,
                chunk_size: 0,
                chunk_count: 0,
                completed_chunks: 0,
                active_io: 0,
                last_error: None,
                last_error_code: None,
            }))
            .unwrap();

        assert_eq!(collect_pending_events(&receiver).len(), 1);
        assert!(collect_pending_events(&receiver).is_empty());
    }

    fn test_options() -> NativeDownloadOptions {
        NativeDownloadOptions {
            url: "https://example.test/file.bin".to_string(),
            target_path: "/tmp/file.bin".to_string(),
            chunk_size: Some("1024".to_string()),
            parallelism: Some(2),
            max_parallel_chunks: Some(4),
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
}
