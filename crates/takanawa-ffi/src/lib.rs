#![allow(unsafe_code)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use takanawa_core::{HashConfig, HashKind, TakanawaError};
use takanawa_http::{
    DEFAULT_MAX_IO, DownloadConfig, DownloadEngine, DownloadHandle, DownloadPhase,
    DownloadSnapshot, DownloadSpeedSnapshot, ProgressCallback, RetryConfig, SpeedCallback,
    TimeoutConfig,
};
use tokio::runtime::{Builder, Runtime};

/// ABI version expected by all C-facing configuration structs.
pub const TKNW_ABI_VERSION: u32 = 1;

/// Status codes returned by the C ABI.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TknwStatus {
    /// Operation completed successfully.
    Ok = 0,
    /// Caller-provided output buffer was too small.
    BufferTooSmall = 1,
    /// A required pointer argument was null.
    NullPointer = -1,
    /// ABI version or struct size did not match this library.
    AbiMismatch = -2,
    /// Configuration was invalid.
    InvalidConfig = -3,
    /// The global runtime has not been initialized.
    RuntimeNotInitialized = -4,
    /// The final target file already exists.
    TargetExists = -10,
    /// The part file is locked by another process or handle.
    PartBusy = -11,
    /// Existing part-file size did not match expected metadata.
    PartSizeMismatch = -12,
    /// Stored part metadata is corrupt.
    PartCorrupt = -13,
    /// Remote validators or size changed while resuming.
    RemoteChanged = -14,
    /// HTTP response did not satisfy range download requirements.
    HttpProtocol = -20,
    /// Network transport failed.
    Network = -21,
    /// Filesystem I/O failed.
    Io = -30,
    /// Downloaded bytes did not match the configured hash.
    HashMismatch = -40,
    /// Download was cancelled.
    Cancelled = -50,
    /// Download was already running.
    AlreadyStarted = -51,
    /// A panic was caught at the FFI boundary.
    Panic = -100,
    /// Internal task or FFI boundary failure.
    Internal = -101,
}

/// Hash algorithm identifiers used by the C ABI.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TknwHashKind {
    /// No hash verification.
    None = 0,
    /// SHA-256 verification.
    Sha256 = 1,
    /// SHA-1 verification.
    Sha1 = 2,
    /// SHA-512 verification.
    Sha512 = 3,
    /// MD5 verification.
    Md5 = 4,
    /// CRC-32 verification.
    Crc32 = 5,
}

/// Global runtime configuration for [`tknw_global_init`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwGlobalConfig {
    /// Must be [`TKNW_ABI_VERSION`].
    pub abi_version: u32,
    /// Must be at least `size_of::<TknwGlobalConfig>()`.
    pub struct_size: usize,
    /// Maximum in-flight I/O operations. `0` selects the default.
    pub max_io: usize,
}

/// Download creation configuration for [`tknw_download_create`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwDownloadConfig {
    /// Must be [`TKNW_ABI_VERSION`].
    pub abi_version: u32,
    /// Must be at least `size_of::<TknwDownloadConfig>()`.
    pub struct_size: usize,
    /// Null-terminated source URL string.
    pub url: *const c_char,
    /// Null-terminated final target path string.
    pub target_path: *const c_char,
    /// Requested chunk size in bytes. `0` selects the default.
    pub chunk_size: u64,
    /// Requested chunk parallelism. `0` lets the engine choose a default.
    pub parallelism: usize,
    /// Maximum chunks to download at the same time. `0` falls back to `parallelism`.
    pub max_parallel_chunks: usize,
    /// Number of retries after the first attempt.
    pub max_retries: u32,
    /// Initial retry backoff in milliseconds. `0` selects the default.
    pub backoff_initial_millis: u64,
    /// Maximum retry backoff in milliseconds. `0` selects the default.
    pub backoff_max_millis: u64,
    /// Connection timeout in milliseconds. `0` selects the default.
    pub connect_timeout_millis: u64,
    /// Per-read timeout in milliseconds. `0` disables this timeout.
    pub read_timeout_millis: u64,
    /// Total timeout per probe/chunk attempt in milliseconds. `0` disables this timeout.
    pub total_timeout_millis: u64,
    /// Aggregate response-body bandwidth limit in bytes per second. `0` disables limiting.
    pub bytes_per_second_limit: u64,
    /// Hash algorithm identifier from [`TknwHashKind`].
    pub hash_kind: u32,
    /// Pointer to expected hash bytes for the configured hash algorithm.
    pub expected_sha256: *const c_uchar,
    /// Length of `expected_sha256` in bytes.
    pub expected_sha256_len: usize,
}

/// Progress snapshot written by the C ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwDownloadSnapshot {
    /// Always [`TKNW_ABI_VERSION`] on output and required on input.
    pub abi_version: u32,
    /// Must be at least `size_of::<TknwDownloadSnapshot>()` on input.
    pub struct_size: usize,
    /// Current phase as a `TknwDownloadPhase` numeric value.
    pub phase: u32,
    /// Total content length in bytes.
    pub content_len: u64,
    /// Number of bytes represented by committed chunks.
    pub downloaded_bytes: u64,
    /// Chunk size in bytes.
    pub chunk_size: u64,
    /// Total chunk count.
    pub chunk_count: u64,
    /// Number of chunks committed complete.
    pub completed_chunks: u64,
    /// Current number of active I/O operations.
    pub active_io: usize,
}

/// Download speed sample written by the C ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwDownloadSpeedSnapshot {
    /// Always [`TKNW_ABI_VERSION`] on output and required on input.
    pub abi_version: u32,
    /// Must be at least `size_of::<TknwDownloadSpeedSnapshot>()` on input.
    pub struct_size: usize,
    /// Current phase as a `TknwDownloadPhase` numeric value.
    pub phase: u32,
    /// Total content length in bytes.
    pub content_len: u64,
    /// Bytes represented by committed chunks plus response-body bytes observed for this task.
    pub received_bytes: u64,
    /// Bytes observed since the previous speed sample.
    pub interval_bytes: u64,
    /// Milliseconds elapsed since the previous speed sample.
    pub elapsed_millis: u64,
    /// Current transfer speed in bytes per second for this sample interval.
    pub bytes_per_second: f64,
    /// Current number of active I/O operations.
    pub active_io: usize,
}

/// C callback invoked when download progress changes.
pub type TknwProgressCallback =
    Option<extern "C" fn(snapshot: *const TknwDownloadSnapshot, context: *mut c_void)>;
/// C callback invoked when download speed samples change.
pub type TknwSpeedCallback =
    Option<extern "C" fn(snapshot: *const TknwDownloadSpeedSnapshot, context: *mut c_void)>;
/// C callback invoked when a progress callback context is released.
pub type TknwProgressCallbackRelease = Option<extern "C" fn(context: *mut c_void)>;
/// C callback invoked when a speed callback context is released.
pub type TknwSpeedCallbackRelease = Option<extern "C" fn(context: *mut c_void)>;

/// Opaque download handle owned by the C ABI caller.
pub struct TknwDownload {
    global: Arc<GlobalRuntime>,
    inner: DownloadHandle,
    last_error: Mutex<Option<String>>,
}

struct CallbackContext {
    context: usize,
    release: TknwProgressCallbackRelease,
}

impl CallbackContext {
    const fn ptr(&self) -> *mut c_void {
        self.context as *mut c_void
    }
}

impl Drop for CallbackContext {
    fn drop(&mut self) {
        if let Some(release) = self.release {
            release(self.ptr());
        }
    }
}

struct GlobalRuntime {
    runtime: Runtime,
    engine: DownloadEngine,
}

static GLOBAL: LazyLock<Mutex<Option<Arc<GlobalRuntime>>>> = LazyLock::new(|| Mutex::new(None));

/// Initializes or updates the global runtime used by C ABI downloads.
///
/// Pass a null `config` pointer to use defaults.
///
/// # Panics
///
/// Panics if the global runtime mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_global_init(config: *const TknwGlobalConfig) -> TknwStatus {
    ffi_boundary(|| {
        let max_io = if config.is_null() {
            DEFAULT_MAX_IO
        } else {
            // SAFETY: config was checked for null and is only read for the duration of this call.
            let config = unsafe { &*config };
            validate_abi(
                "TknwGlobalConfig",
                config.abi_version,
                config.struct_size,
                size_of::<TknwGlobalConfig>(),
            )?;
            if config.max_io == 0 {
                DEFAULT_MAX_IO
            } else {
                config.max_io
            }
        };
        let mut global = GLOBAL.lock().expect("global runtime mutex poisoned");
        if let Some(existing) = global.as_ref() {
            existing.engine.set_max_io(max_io);
            return Ok(TknwStatus::Ok);
        }

        *global = Some(Arc::new(GlobalRuntime::new(max_io)?));
        Ok(TknwStatus::Ok)
    })
}

/// Shuts down the global runtime and drops shared engine state.
///
/// # Panics
///
/// Panics if the global runtime mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_global_shutdown() -> TknwStatus {
    ffi_boundary(|| {
        let mut global = GLOBAL.lock().expect("global runtime mutex poisoned");
        let _ = global.take();
        Ok(TknwStatus::Ok)
    })
}

/// Updates the global maximum number of in-flight I/O operations.
///
/// # Panics
///
/// Panics if the global runtime mutex or shared limiter mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_global_set_max_io(max_io: usize) -> TknwStatus {
    ffi_boundary(|| {
        let global = current_global()?;
        global.engine.set_max_io(max_io);
        Ok(TknwStatus::Ok)
    })
}

/// Creates a download handle.
///
/// On success, writes a non-null handle to `out_download`. Release it with
/// [`tknw_download_release`].
///
/// # Panics
///
/// Panics if the global runtime mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_create(
    config: *const TknwDownloadConfig,
    out_download: *mut *mut TknwDownload,
) -> TknwStatus {
    ffi_boundary(|| {
        if config.is_null() {
            return Err(TakanawaError::NullPointer("config"));
        }
        if out_download.is_null() {
            return Err(TakanawaError::NullPointer("out_download"));
        }

        // SAFETY: pointers were checked for null and are only read/written during this call.
        let config = unsafe { &*config };
        validate_abi(
            "TknwDownloadConfig",
            config.abi_version,
            config.struct_size,
            size_of::<TknwDownloadConfig>(),
        )?;

        let global = current_global()?;
        let url = read_c_string(config.url, "url")?;
        let target_path = read_c_string(config.target_path, "target_path")?;
        let hash = read_hash_config(config)?;
        let download_config = DownloadConfig {
            url,
            target_path: PathBuf::from(target_path),
            chunk_size: config.chunk_size,
            parallelism: config.parallelism,
            max_parallel_chunks: config.max_parallel_chunks,
            retry: RetryConfig {
                max_retries: config.max_retries,
                backoff_initial: millis(config.backoff_initial_millis),
                backoff_max: millis(config.backoff_max_millis),
            },
            timeout: TimeoutConfig {
                connect: millis(config.connect_timeout_millis),
                read: millis(config.read_timeout_millis),
                total: millis(config.total_timeout_millis),
            },
            bytes_per_second_limit: config.bytes_per_second_limit,
            hash,
        };
        let download = Box::new(TknwDownload {
            inner: DownloadHandle::new(global.engine.clone(), download_config),
            global,
            last_error: Mutex::new(None),
        });

        // SAFETY: out_download is valid for writes by the function contract.
        unsafe {
            *out_download = Box::into_raw(download);
        }
        Ok(TknwStatus::Ok)
    })
}

/// Starts or resumes a download.
///
/// # Panics
///
/// Panics if the handle's join-handle mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_start(download: *mut TknwDownload) -> TknwStatus {
    ffi_download_boundary(download, |download| {
        download.inner.start_on(&download.global.runtime)?;
        Ok(TknwStatus::Ok)
    })
}

/// Requests that a download pause after in-flight work winds down.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_pause(download: *mut TknwDownload) -> TknwStatus {
    ffi_download_boundary(download, |download| {
        download.inner.pause()?;
        Ok(TknwStatus::Ok)
    })
}

/// Requests cancellation of a download.
///
/// # Panics
///
/// Panics if the handle's join-handle mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_cancel(download: *mut TknwDownload) -> TknwStatus {
    ffi_download_boundary(download, |download| {
        download.inner.cancel()?;
        Ok(TknwStatus::Ok)
    })
}

/// Writes the current download snapshot to `snapshot`.
///
/// `snapshot` must point to writable memory initialized with ABI metadata.
///
/// # Panics
///
/// Panics if shared progress state is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_snapshot(
    download: *const TknwDownload,
    snapshot: *mut TknwDownloadSnapshot,
) -> TknwStatus {
    ffi_boundary(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        if snapshot.is_null() {
            return Err(TakanawaError::NullPointer("snapshot"));
        }

        // SAFETY: pointers were checked for null and are only accessed during this call.
        let download = unsafe { &*download };
        // SAFETY: snapshot was checked for null and points to caller-owned writable memory.
        let snapshot_ref = unsafe { &mut *snapshot };
        validate_abi(
            "TknwDownloadSnapshot",
            snapshot_ref.abi_version,
            snapshot_ref.struct_size,
            size_of::<TknwDownloadSnapshot>(),
        )?;

        let current = download.inner.snapshot();
        *snapshot_ref = snapshot_to_ffi(&current);
        Ok(TknwStatus::Ok)
    })
}

/// Installs or removes a progress callback for a download.
///
/// Passing `None` as `callback` removes the callback. A non-null `context` or
/// release callback requires a non-null progress callback.
///
/// # Panics
///
/// Panics if the last-error mutex or callback mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_set_progress_callback(
    download: *mut TknwDownload,
    callback: TknwProgressCallback,
    context: *mut c_void,
    context_release: TknwProgressCallbackRelease,
) -> TknwStatus {
    ffi_boundary(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        // SAFETY: download was checked for null and is borrowed only for this call.
        let download = unsafe { &*download };

        let Some(callback) = callback else {
            if !context.is_null() || context_release.is_some() {
                let err =
                    TakanawaError::InvalidConfig("callback context requires a callback".to_owned());
                *download
                    .last_error
                    .lock()
                    .expect("last error mutex poisoned") = Some(err.to_string());
                return Err(err);
            }
            download.inner.set_progress_callback(None);
            return Ok(TknwStatus::Ok);
        };

        let callback_context = CallbackContext {
            context: context as usize,
            release: context_release,
        };
        let progress_callback: ProgressCallback = Arc::new(move |snapshot| {
            let native = snapshot_to_ffi(&snapshot);
            callback(&raw const native, callback_context.ptr());
        });
        download
            .inner
            .set_progress_callback(Some(progress_callback));
        Ok(TknwStatus::Ok)
    })
}

/// Installs or removes a speed callback for a download.
///
/// Passing `None` as `callback` removes the callback. A non-null `context` or
/// release callback requires a non-null speed callback.
///
/// # Panics
///
/// Panics if the last-error mutex or callback mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_set_speed_callback(
    download: *mut TknwDownload,
    callback: TknwSpeedCallback,
    context: *mut c_void,
    context_release: TknwSpeedCallbackRelease,
) -> TknwStatus {
    ffi_boundary(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        // SAFETY: download was checked for null and is borrowed only for this call.
        let download = unsafe { &*download };

        let Some(callback) = callback else {
            if !context.is_null() || context_release.is_some() {
                let err =
                    TakanawaError::InvalidConfig("callback context requires a callback".to_owned());
                *download
                    .last_error
                    .lock()
                    .expect("last error mutex poisoned") = Some(err.to_string());
                return Err(err);
            }
            download.inner.set_speed_callback(None);
            return Ok(TknwStatus::Ok);
        };

        let callback_context = CallbackContext {
            context: context as usize,
            release: context_release,
        };
        let speed_callback: SpeedCallback = Arc::new(move |snapshot| {
            let native = speed_snapshot_to_ffi(&snapshot);
            callback(&raw const native, callback_context.ptr());
        });
        download.inner.set_speed_callback(Some(speed_callback));
        Ok(TknwStatus::Ok)
    })
}

/// Copies the serialized completion bitmap into `buffer`.
///
/// Always writes the required byte count to `written`. If `buffer_len` is too
/// small, returns [`TknwStatus::BufferTooSmall`] without copying bytes.
///
/// # Panics
///
/// Panics if shared progress state is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_copy_bitmap(
    download: *const TknwDownload,
    buffer: *mut c_uchar,
    buffer_len: usize,
    written: *mut usize,
) -> TknwStatus {
    ffi_boundary(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        if written.is_null() {
            return Err(TakanawaError::NullPointer("written"));
        }
        // SAFETY: download and written were checked for null and are only accessed during this call.
        let download = unsafe { &*download };
        let bitmap = download.inner.bitmap();
        // SAFETY: written points to caller-owned writable memory.
        unsafe {
            *written = bitmap.len();
        }
        if bitmap.len() > buffer_len {
            return Ok(TknwStatus::BufferTooSmall);
        }
        if !bitmap.is_empty() {
            if buffer.is_null() {
                return Err(TakanawaError::NullPointer("buffer"));
            }
            // SAFETY: buffer is non-null and buffer_len is at least bitmap.len().
            unsafe {
                ptr::copy_nonoverlapping(bitmap.as_ptr(), buffer, bitmap.len());
            }
        }
        Ok(TknwStatus::Ok)
    })
}

/// Copies the most recent download error message into `buffer` as a C string.
///
/// Always writes the required byte count, including the null terminator, to
/// `written`. If `buffer_len` is too small, returns
/// [`TknwStatus::BufferTooSmall`] without copying bytes.
///
/// # Panics
///
/// Panics if shared progress state or the last-error mutex is poisoned.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_last_error(
    download: *const TknwDownload,
    buffer: *mut c_char,
    buffer_len: usize,
    written: *mut usize,
) -> TknwStatus {
    ffi_boundary(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        if written.is_null() {
            return Err(TakanawaError::NullPointer("written"));
        }
        // SAFETY: download was checked for null and is only read during this call.
        let download = unsafe { &*download };
        let message = download
            .inner
            .snapshot()
            .last_error
            .or_else(|| {
                download
                    .last_error
                    .lock()
                    .expect("last error mutex poisoned")
                    .clone()
            })
            .unwrap_or_default();
        let bytes = message.as_bytes();
        let required = bytes.len() + 1;
        // SAFETY: written points to caller-owned writable memory.
        unsafe {
            *written = required;
        }
        if required > buffer_len {
            return Ok(TknwStatus::BufferTooSmall);
        }
        if buffer.is_null() {
            return Err(TakanawaError::NullPointer("buffer"));
        }
        // SAFETY: buffer is non-null and large enough for message plus null terminator.
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, bytes.len());
            *buffer.add(bytes.len()) = 0;
        }
        Ok(TknwStatus::Ok)
    })
}

/// Releases a download handle and sets the caller's handle pointer to null.
#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_release(download: *mut *mut TknwDownload) -> TknwStatus {
    ffi_boundary(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        // SAFETY: download was checked for null and points to caller-owned handle storage.
        let handle = unsafe { *download };
        if handle.is_null() {
            return Err(TakanawaError::NullPointer("*download"));
        }
        // SAFETY: handle was created by Box::into_raw in tknw_download_create and is released once here.
        unsafe {
            drop(Box::from_raw(handle));
            *download = ptr::null_mut();
        }
        Ok(TknwStatus::Ok)
    })
}

impl GlobalRuntime {
    fn new(max_io: usize) -> Result<Self, TakanawaError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .thread_name("takanawa")
            .build()
            .map_err(TakanawaError::Io)?;
        let engine = DownloadEngine::new(max_io)?;
        Ok(Self { runtime, engine })
    }
}

fn current_global() -> Result<Arc<GlobalRuntime>, TakanawaError> {
    GLOBAL
        .lock()
        .expect("global runtime mutex poisoned")
        .as_ref()
        .cloned()
        .ok_or(TakanawaError::RuntimeNotInitialized)
}

const fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn validate_abi(
    name: &'static str,
    abi_version: u32,
    actual_size: usize,
    expected_size: usize,
) -> Result<(), TakanawaError> {
    if abi_version != TKNW_ABI_VERSION {
        return Err(TakanawaError::AbiMismatch(format!(
            "{name} ABI version mismatch: expected {TKNW_ABI_VERSION}, got {abi_version}"
        )));
    }
    if actual_size < expected_size {
        return Err(TakanawaError::StructSizeMismatch {
            name,
            expected: expected_size,
            actual: actual_size,
        });
    }
    Ok(())
}

fn read_c_string(ptr: *const c_char, name: &'static str) -> Result<String, TakanawaError> {
    if ptr.is_null() {
        return Err(TakanawaError::NullPointer(name));
    }
    // SAFETY: ptr is non-null and the caller must provide a valid null-terminated string.
    let value = unsafe { CStr::from_ptr(ptr) };
    value
        .to_str()
        .map(str::to_owned)
        .map_err(|err| TakanawaError::Utf8(format!("{name}: {err}")))
}

fn read_hash_config(config: &TknwDownloadConfig) -> Result<HashConfig, TakanawaError> {
    let kind = HashKind::from_u32(config.hash_kind).ok_or_else(|| {
        TakanawaError::InvalidConfig(format!("unsupported hash kind {}", config.hash_kind))
    })?;
    let expected_len = kind.expected_len();
    if kind == HashKind::None {
        if config.expected_sha256_len != 0 {
            return Err(TakanawaError::InvalidConfig(format!(
                "none expected hash length must be 0, got {}",
                config.expected_sha256_len
            )));
        }
        return Ok(HashConfig::None);
    }
    if config.expected_sha256.is_null() {
        return Err(TakanawaError::NullPointer("expected_sha256"));
    }
    if config.expected_sha256_len != expected_len {
        return Err(TakanawaError::InvalidConfig(format!(
            "{} expected hash length must be {expected_len}, got {}",
            kind.name(),
            config.expected_sha256_len
        )));
    }

    // SAFETY: expected_sha256 is non-null and expected_sha256_len was validated for the hash kind.
    let expected =
        unsafe { std::slice::from_raw_parts(config.expected_sha256, config.expected_sha256_len) };
    HashConfig::from_expected_bytes(kind, expected).ok_or_else(|| {
        TakanawaError::InvalidConfig(format!(
            "{} expected hash length must be {expected_len}, got {}",
            kind.name(),
            config.expected_sha256_len
        ))
    })
}

fn ffi_boundary<F>(f: F) -> TknwStatus
where
    F: FnOnce() -> Result<TknwStatus, TakanawaError>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(status)) => status,
        Ok(Err(err)) => status_from_error(&err),
        Err(_) => TknwStatus::Panic,
    }
}

fn ffi_download_boundary<F>(download: *mut TknwDownload, f: F) -> TknwStatus
where
    F: FnOnce(&mut TknwDownload) -> Result<TknwStatus, TakanawaError>,
{
    match catch_unwind(AssertUnwindSafe(|| {
        if download.is_null() {
            return Err(TakanawaError::NullPointer("download"));
        }
        // SAFETY: download was checked for null and is borrowed only for this call.
        let download = unsafe { &mut *download };
        f(download).inspect_err(|err| {
            *download
                .last_error
                .lock()
                .expect("last error mutex poisoned") = Some(err.to_string());
        })
    })) {
        Ok(Ok(status)) => status,
        Ok(Err(err)) => status_from_error(&err),
        Err(_) => TknwStatus::Panic,
    }
}

fn status_from_error(err: &TakanawaError) -> TknwStatus {
    match err {
        TakanawaError::NullPointer(_) => TknwStatus::NullPointer,
        TakanawaError::StructSizeMismatch { .. } | TakanawaError::AbiMismatch(_) => {
            TknwStatus::AbiMismatch
        }
        TakanawaError::InvalidConfig(_) | TakanawaError::NotRunning | TakanawaError::Utf8(_) => {
            TknwStatus::InvalidConfig
        }
        TakanawaError::RuntimeNotInitialized => TknwStatus::RuntimeNotInitialized,
        TakanawaError::TargetExists(_) => TknwStatus::TargetExists,
        TakanawaError::PartBusy(_) => TknwStatus::PartBusy,
        TakanawaError::PartSizeMismatch { .. } => TknwStatus::PartSizeMismatch,
        TakanawaError::PartCorrupt(_) => TknwStatus::PartCorrupt,
        TakanawaError::RemoteChanged(_) => TknwStatus::RemoteChanged,
        TakanawaError::HttpProtocol(_) | TakanawaError::RetryableHttpStatus(_) => {
            TknwStatus::HttpProtocol
        }
        TakanawaError::Network(_) => TknwStatus::Network,
        TakanawaError::Io(_) => TknwStatus::Io,
        TakanawaError::HashMismatch => TknwStatus::HashMismatch,
        TakanawaError::Cancelled => TknwStatus::Cancelled,
        TakanawaError::AlreadyStarted => TknwStatus::AlreadyStarted,
        TakanawaError::Ffi(_) => TknwStatus::Internal,
    }
}

const fn phase_to_u32(phase: DownloadPhase) -> u32 {
    phase as u32
}

fn snapshot_to_ffi(snapshot: &DownloadSnapshot) -> TknwDownloadSnapshot {
    TknwDownloadSnapshot {
        abi_version: TKNW_ABI_VERSION,
        struct_size: size_of::<TknwDownloadSnapshot>(),
        phase: phase_to_u32(snapshot.phase),
        content_len: snapshot.content_len,
        downloaded_bytes: snapshot.downloaded_bytes,
        chunk_size: snapshot.chunk_size,
        chunk_count: snapshot.chunk_count,
        completed_chunks: snapshot.completed_chunks,
        active_io: snapshot.active_io,
    }
}

fn speed_snapshot_to_ffi(snapshot: &DownloadSpeedSnapshot) -> TknwDownloadSpeedSnapshot {
    TknwDownloadSpeedSnapshot {
        abi_version: TKNW_ABI_VERSION,
        struct_size: size_of::<TknwDownloadSpeedSnapshot>(),
        phase: phase_to_u32(snapshot.phase),
        content_len: snapshot.content_len,
        received_bytes: snapshot.received_bytes,
        interval_bytes: snapshot.interval_bytes,
        elapsed_millis: snapshot.elapsed_millis,
        bytes_per_second: snapshot.bytes_per_second,
        active_io: snapshot.active_io,
    }
}

#[cfg(feature = "jni")]
mod android_jni {
    use std::ffi::CString;
    use std::os::raw::c_void;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::ptr;

    use jni::JNIEnv;
    use jni::JavaVM;
    use jni::errors::Error as JniError;
    use jni::objects::{GlobalRef, JByteArray, JClass, JLongArray, JObject, JString, JValue};
    use jni::sys::{jbyte, jint, jlong, jstring};

    use super::{
        HashKind, TKNW_ABI_VERSION, TknwDownload, TknwDownloadConfig, TknwDownloadSnapshot,
        TknwDownloadSpeedSnapshot, TknwGlobalConfig, TknwStatus, tknw_download_cancel,
        tknw_download_copy_bitmap, tknw_download_create, tknw_download_last_error,
        tknw_download_pause, tknw_download_release, tknw_download_set_progress_callback,
        tknw_download_set_speed_callback, tknw_download_snapshot, tknw_download_start,
        tknw_global_init, tknw_global_set_max_io, tknw_global_shutdown,
    };

    struct AndroidProgressCallback {
        java_vm: JavaVM,
        listener: GlobalRef,
    }

    struct AndroidSpeedCallback {
        java_vm: JavaVM,
        listener: GlobalRef,
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_globalInit<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
        max_io: jint,
    ) -> jint {
        let Ok(max_io) = usize::try_from(max_io) else {
            return status_code(TknwStatus::InvalidConfig);
        };
        let config = TknwGlobalConfig {
            abi_version: TKNW_ABI_VERSION,
            struct_size: size_of::<TknwGlobalConfig>(),
            max_io,
        };
        status_code(tknw_global_init(&raw const config))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_globalShutdown<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
    ) -> jint {
        status_code(tknw_global_shutdown())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_globalSetMaxIo<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
        max_io: jint,
    ) -> jint {
        let Ok(max_io) = usize::try_from(max_io) else {
            return status_code(TknwStatus::InvalidConfig);
        };
        status_code(tknw_global_set_max_io(max_io))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadCreate<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
        url: JString<'local>,
        target_path: JString<'local>,
        chunk_size: jlong,
        parallelism: jint,
        max_parallel_chunks: jint,
        max_retries: jint,
        backoff_initial_millis: jlong,
        backoff_max_millis: jlong,
        connect_timeout_millis: jlong,
        read_timeout_millis: jlong,
        total_timeout_millis: jlong,
        bytes_per_second_limit: jlong,
        hash_kind: jint,
        expected_sha256: JByteArray<'local>,
        out_handle: JLongArray<'local>,
    ) -> jint {
        jni_status(|| {
            let Ok(chunk_size) = u64::try_from(chunk_size) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(parallelism) = usize::try_from(parallelism) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(max_parallel_chunks) = usize::try_from(max_parallel_chunks) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(max_retries) = u32::try_from(max_retries) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(backoff_initial_millis) = u64::try_from(backoff_initial_millis) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(backoff_max_millis) = u64::try_from(backoff_max_millis) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(connect_timeout_millis) = u64::try_from(connect_timeout_millis) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(read_timeout_millis) = u64::try_from(read_timeout_millis) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(total_timeout_millis) = u64::try_from(total_timeout_millis) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(bytes_per_second_limit) = u64::try_from(bytes_per_second_limit) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };

            let url = match read_java_string(&mut env, &url) {
                Ok(url) => url,
                Err(status) => return Ok(status_code(status)),
            };
            let target_path = match read_java_string(&mut env, &target_path) {
                Ok(target_path) => target_path,
                Err(status) => return Ok(status_code(status)),
            };
            let hash_kind = HashKind::from_u32(u32::try_from(hash_kind).unwrap_or(u32::MAX));
            let Some(hash_kind) = hash_kind else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let expected_hash = match read_optional_hash(&mut env, &expected_sha256) {
                Ok(expected_hash) => expected_hash,
                Err(status) => return Ok(status_code(status)),
            };
            let hash_ptr = expected_hash.as_ref().map_or(ptr::null(), Vec::as_ptr);
            let hash_len = expected_hash.as_ref().map_or(0, Vec::len);
            let config = TknwDownloadConfig {
                abi_version: TKNW_ABI_VERSION,
                struct_size: size_of::<TknwDownloadConfig>(),
                url: url.as_ptr(),
                target_path: target_path.as_ptr(),
                chunk_size,
                parallelism,
                max_parallel_chunks,
                max_retries,
                backoff_initial_millis,
                backoff_max_millis,
                connect_timeout_millis,
                read_timeout_millis,
                total_timeout_millis,
                bytes_per_second_limit,
                hash_kind: u32::from(hash_kind),
                expected_sha256: hash_ptr,
                expected_sha256_len: hash_len,
            };
            let mut download = ptr::null_mut();
            let status = tknw_download_create(&raw const config, &raw mut download);
            if status != TknwStatus::Ok {
                return Ok(status_code(status));
            }

            Ok(write_long_array(
                &mut env,
                &out_handle,
                &[download as jlong],
            ))
        })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadStart<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
    ) -> jint {
        status_code(tknw_download_start(download_mut(handle)))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadPause<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
    ) -> jint {
        status_code(tknw_download_pause(download_mut(handle)))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadCancel<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
    ) -> jint {
        status_code(tknw_download_cancel(download_mut(handle)))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadSnapshot<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
        out_snapshot: JLongArray<'local>,
    ) -> jint {
        let mut snapshot = TknwDownloadSnapshot {
            abi_version: TKNW_ABI_VERSION,
            struct_size: size_of::<TknwDownloadSnapshot>(),
            phase: 0,
            content_len: 0,
            downloaded_bytes: 0,
            chunk_size: 0,
            chunk_count: 0,
            completed_chunks: 0,
            active_io: 0,
        };
        let status = tknw_download_snapshot(download_const(handle), &raw mut snapshot);
        if status != TknwStatus::Ok {
            return status_code(status);
        }

        let values = match snapshot_values(&snapshot) {
            Ok(values) => values,
            Err(status) => return status_code(status),
        };
        jni_status(|| Ok(write_long_array(&mut env, &out_snapshot, &values)))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadSetProgressCallback<
        'local,
    >(
        env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
        listener: JObject<'local>,
    ) -> jint {
        jni_status(|| {
            if listener.as_raw().is_null() {
                return Ok(status_code(tknw_download_set_progress_callback(
                    download_mut(handle),
                    None,
                    ptr::null_mut(),
                    None,
                )));
            }

            let Ok(java_vm) = env.get_java_vm() else {
                return Ok(status_code(TknwStatus::Internal));
            };
            let Ok(listener) = env.new_global_ref(listener) else {
                return Ok(status_code(TknwStatus::Internal));
            };
            let callback = Box::new(AndroidProgressCallback { java_vm, listener });
            let context = Box::into_raw(callback).cast::<c_void>();
            let status = tknw_download_set_progress_callback(
                download_mut(handle),
                Some(android_progress_callback),
                context,
                Some(release_android_progress_callback),
            );
            if status != TknwStatus::Ok {
                // SAFETY: context was created by Box::into_raw immediately above.
                unsafe {
                    drop(Box::from_raw(context.cast::<AndroidProgressCallback>()));
                }
            }
            Ok(status_code(status))
        })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadSetSpeedCallback<'local>(
        env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
        listener: JObject<'local>,
    ) -> jint {
        jni_status(|| {
            if listener.as_raw().is_null() {
                return Ok(status_code(tknw_download_set_speed_callback(
                    download_mut(handle),
                    None,
                    ptr::null_mut(),
                    None,
                )));
            }

            let Ok(java_vm) = env.get_java_vm() else {
                return Ok(status_code(TknwStatus::Internal));
            };
            let Ok(listener) = env.new_global_ref(listener) else {
                return Ok(status_code(TknwStatus::Internal));
            };
            let callback = Box::new(AndroidSpeedCallback { java_vm, listener });
            let context = Box::into_raw(callback).cast::<c_void>();
            let status = tknw_download_set_speed_callback(
                download_mut(handle),
                Some(android_speed_callback),
                context,
                Some(release_android_speed_callback),
            );
            if status != TknwStatus::Ok {
                // SAFETY: context was created by Box::into_raw immediately above.
                unsafe {
                    drop(Box::from_raw(context.cast::<AndroidSpeedCallback>()));
                }
            }
            Ok(status_code(status))
        })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadBitmapSize<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
        out_size: JLongArray<'local>,
    ) -> jint {
        let mut written = 0;
        let status =
            tknw_download_copy_bitmap(download_const(handle), ptr::null_mut(), 0, &raw mut written);
        if !matches!(status, TknwStatus::Ok | TknwStatus::BufferTooSmall) {
            return status_code(status);
        }
        let Ok(written) = jlong::try_from(written) else {
            return status_code(TknwStatus::Internal);
        };
        jni_status(|| Ok(write_long_array(&mut env, &out_size, &[written])))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadCopyBitmap<'local>(
        env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
        out_bitmap: JByteArray<'local>,
    ) -> jint {
        jni_status(|| {
            if out_bitmap.as_raw().is_null() {
                return Ok(status_code(TknwStatus::NullPointer));
            }
            let Ok(len) = env.get_array_length(&out_bitmap) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let Ok(len) = usize::try_from(len) else {
                return Ok(status_code(TknwStatus::InvalidConfig));
            };
            let mut buffer = vec![0; len];
            let mut written = 0;
            let status = tknw_download_copy_bitmap(
                download_const(handle),
                buffer.as_mut_ptr(),
                buffer.len(),
                &raw mut written,
            );
            if status != TknwStatus::Ok {
                return Ok(status_code(status));
            }
            let signed = buffer
                .into_iter()
                .take(written)
                .map(|byte| jbyte::from_ne_bytes([byte]))
                .collect::<Vec<_>>();
            Ok(match env.set_byte_array_region(&out_bitmap, 0, &signed) {
                Ok(()) => status_code(TknwStatus::Ok),
                Err(_) => status_code(TknwStatus::Internal),
            })
        })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadLastError<'local>(
        env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
    ) -> jstring {
        match catch_unwind(AssertUnwindSafe(|| {
            let message = last_error(download_const(handle));
            Ok::<jstring, JniError>(
                env.new_string(message)
                    .map_or_else(|_| ptr::null_mut(), JString::into_raw),
            )
        })) {
            Ok(Ok(value)) => value,
            Ok(Err(_)) | Err(_) => ptr::null_mut(),
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn Java_ai_yetanother_takanawa_NativeBridge_downloadRelease<'local>(
        _env: JNIEnv<'local>,
        _class: JClass<'local>,
        handle: jlong,
    ) -> jint {
        let mut download = download_mut(handle);
        status_code(tknw_download_release(&raw mut download))
    }

    fn read_java_string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Result<CString, TknwStatus> {
        if value.as_raw().is_null() {
            return Err(TknwStatus::NullPointer);
        }
        let value: String = env
            .get_string(value)
            .map_err(|_| TknwStatus::InvalidConfig)?
            .into();
        CString::new(value).map_err(|_| TknwStatus::InvalidConfig)
    }

    fn read_optional_hash(
        env: &mut JNIEnv<'_>,
        value: &JByteArray<'_>,
    ) -> Result<Option<Vec<u8>>, TknwStatus> {
        if value.as_raw().is_null() {
            return Ok(None);
        }
        let hash = env
            .convert_byte_array(value)
            .map_err(|_| TknwStatus::InvalidConfig)?;
        Ok(Some(hash))
    }

    fn write_long_array(env: &mut JNIEnv<'_>, array: &JLongArray<'_>, values: &[jlong]) -> jint {
        if array.as_raw().is_null() {
            return status_code(TknwStatus::NullPointer);
        }
        let Ok(len) = env.get_array_length(array) else {
            return status_code(TknwStatus::InvalidConfig);
        };
        let Ok(len) = usize::try_from(len) else {
            return status_code(TknwStatus::InvalidConfig);
        };
        if len < values.len() {
            return status_code(TknwStatus::BufferTooSmall);
        }
        match env.set_long_array_region(array, 0, values) {
            Ok(()) => status_code(TknwStatus::Ok),
            Err(_) => status_code(TknwStatus::Internal),
        }
    }

    extern "C" fn android_progress_callback(
        snapshot: *const TknwDownloadSnapshot,
        context: *mut c_void,
    ) {
        if snapshot.is_null() || context.is_null() {
            return;
        }

        // SAFETY: context is allocated by downloadSetProgressCallback and released by
        // release_android_progress_callback after the native callback is unregistered.
        let callback = unsafe { &*context.cast::<AndroidProgressCallback>() };
        let Ok(mut env) = callback.java_vm.attach_current_thread() else {
            return;
        };
        // SAFETY: snapshot is non-null and only read during this callback.
        let snapshot = unsafe { &*snapshot };
        let Ok(values) = snapshot_values(snapshot) else {
            return;
        };
        let Ok(phase_code) = jint::try_from(snapshot.phase) else {
            return;
        };
        let Ok(phase) = env
            .call_static_method(
                "ai/yetanother/takanawa/DownloadPhase",
                "fromCode",
                "(I)Lai/yetanother/takanawa/DownloadPhase;",
                &[JValue::Int(phase_code)],
            )
            .and_then(jni::objects::JValueGen::l)
        else {
            clear_exception(&mut env);
            return;
        };
        let Ok(snapshot_object) = env.new_object(
            "ai/yetanother/takanawa/DownloadSnapshot",
            "(Lai/yetanother/takanawa/DownloadPhase;JJJJJI)V",
            &[
                JValue::Object(&phase),
                JValue::Long(values[1]),
                JValue::Long(values[2]),
                JValue::Long(values[3]),
                JValue::Long(values[4]),
                JValue::Long(values[5]),
                JValue::Int(match jint::try_from(values[6]) {
                    Ok(value) => value,
                    Err(_) => return,
                }),
            ],
        ) else {
            clear_exception(&mut env);
            return;
        };
        if env
            .call_method(
                callback.listener.as_obj(),
                "onProgress",
                "(Lai/yetanother/takanawa/DownloadSnapshot;)V",
                &[JValue::Object(&snapshot_object)],
            )
            .is_err()
        {
            clear_exception(&mut env);
        }
    }

    extern "C" fn release_android_progress_callback(context: *mut c_void) {
        if context.is_null() {
            return;
        }
        // SAFETY: context was created by Box::into_raw in downloadSetProgressCallback.
        unsafe {
            drop(Box::from_raw(context.cast::<AndroidProgressCallback>()));
        }
    }

    extern "C" fn android_speed_callback(
        snapshot: *const TknwDownloadSpeedSnapshot,
        context: *mut c_void,
    ) {
        if snapshot.is_null() || context.is_null() {
            return;
        }

        // SAFETY: context is allocated by downloadSetSpeedCallback and released by
        // release_android_speed_callback after the native callback is unregistered.
        let callback = unsafe { &*context.cast::<AndroidSpeedCallback>() };
        let Ok(mut env) = callback.java_vm.attach_current_thread() else {
            return;
        };
        // SAFETY: snapshot is non-null and only read during this callback.
        let snapshot = unsafe { &*snapshot };
        let Ok(values) = speed_values(snapshot) else {
            return;
        };
        let Ok(phase_code) = jint::try_from(snapshot.phase) else {
            return;
        };
        let Ok(phase) = env
            .call_static_method(
                "ai/yetanother/takanawa/DownloadPhase",
                "fromCode",
                "(I)Lai/yetanother/takanawa/DownloadPhase;",
                &[JValue::Int(phase_code)],
            )
            .and_then(jni::objects::JValueGen::l)
        else {
            clear_exception(&mut env);
            return;
        };
        let Ok(speed_object) = env.new_object(
            "ai/yetanother/takanawa/DownloadSpeedSnapshot",
            "(Lai/yetanother/takanawa/DownloadPhase;JJJJDI)V",
            &[
                JValue::Object(&phase),
                JValue::Long(values[1]),
                JValue::Long(values[2]),
                JValue::Long(values[3]),
                JValue::Long(values[4]),
                JValue::Double(snapshot.bytes_per_second),
                JValue::Int(match jint::try_from(values[5]) {
                    Ok(value) => value,
                    Err(_) => return,
                }),
            ],
        ) else {
            clear_exception(&mut env);
            return;
        };
        if env
            .call_method(
                callback.listener.as_obj(),
                "onSpeed",
                "(Lai/yetanother/takanawa/DownloadSpeedSnapshot;)V",
                &[JValue::Object(&speed_object)],
            )
            .is_err()
        {
            clear_exception(&mut env);
        }
    }

    extern "C" fn release_android_speed_callback(context: *mut c_void) {
        if context.is_null() {
            return;
        }
        // SAFETY: context was created by Box::into_raw in downloadSetSpeedCallback.
        unsafe {
            drop(Box::from_raw(context.cast::<AndroidSpeedCallback>()));
        }
    }

    fn clear_exception(env: &mut JNIEnv<'_>) {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }

    fn jni_status(action: impl FnOnce() -> Result<jint, JniError>) -> jint {
        match catch_unwind(AssertUnwindSafe(action)) {
            Ok(Ok(status)) => status,
            Ok(Err(_)) | Err(_) => status_code(TknwStatus::Internal),
        }
    }

    fn last_error(download: *const TknwDownload) -> String {
        if download.is_null() {
            return String::new();
        }
        let mut written = 0;
        let status = tknw_download_last_error(download, ptr::null_mut(), 0, &raw mut written);
        if !matches!(status, TknwStatus::Ok | TknwStatus::BufferTooSmall) || written == 0 {
            return String::new();
        }

        let mut buffer = vec![0; written];
        let status = tknw_download_last_error(
            download,
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &raw mut written,
        );
        if status != TknwStatus::Ok {
            return String::new();
        }
        let len = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(buffer.len());
        String::from_utf8_lossy(&buffer[..len]).into_owned()
    }

    const fn download_mut(handle: jlong) -> *mut TknwDownload {
        handle as *mut TknwDownload
    }

    const fn download_const(handle: jlong) -> *const TknwDownload {
        handle as *const TknwDownload
    }

    fn snapshot_values(snapshot: &TknwDownloadSnapshot) -> Result<[jlong; 7], TknwStatus> {
        Ok([
            jlong::from(snapshot.phase),
            jlong::try_from(snapshot.content_len).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.downloaded_bytes).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.chunk_size).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.chunk_count).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.completed_chunks).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.active_io).map_err(|_| TknwStatus::Internal)?,
        ])
    }

    fn speed_values(snapshot: &TknwDownloadSpeedSnapshot) -> Result<[jlong; 6], TknwStatus> {
        Ok([
            jlong::from(snapshot.phase),
            jlong::try_from(snapshot.content_len).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.received_bytes).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.interval_bytes).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.elapsed_millis).map_err(|_| TknwStatus::Internal)?,
            jlong::try_from(snapshot.active_io).map_err(|_| TknwStatus::Internal)?,
        ])
    }

    const fn status_code(status: TknwStatus) -> jint {
        status as jint
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{LazyLock, Mutex};

    use tempfile::TempDir;

    use super::*;

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct CallbackCounters {
        calls: AtomicUsize,
        releases: AtomicUsize,
    }

    extern "C" fn count_progress(snapshot: *const TknwDownloadSnapshot, context: *mut c_void) {
        assert!(!snapshot.is_null());
        assert!(!context.is_null());
        // SAFETY: tests pass a valid CallbackCounters pointer for the registration lifetime.
        let counters = unsafe { &*context.cast::<CallbackCounters>() };
        counters.calls.fetch_add(1, Ordering::SeqCst);
    }

    extern "C" fn count_release(context: *mut c_void) {
        assert!(!context.is_null());
        // SAFETY: tests pass a valid CallbackCounters pointer for the registration lifetime.
        let counters = unsafe { &*context.cast::<CallbackCounters>() };
        counters.releases.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn creates_snapshots_and_releases_handle() {
        let _guard = TEST_LOCK.lock().unwrap();
        let global_config = TknwGlobalConfig {
            abi_version: TKNW_ABI_VERSION,
            struct_size: size_of::<TknwGlobalConfig>(),
            max_io: 2,
        };
        assert_eq!(tknw_global_init(&raw const global_config), TknwStatus::Ok);

        let dir = TempDir::new().unwrap();
        let url = CString::new("https://example.test/file").unwrap();
        let target =
            CString::new(dir.path().join("file.bin").to_string_lossy().as_bytes()).unwrap();
        let config = TknwDownloadConfig {
            abi_version: TKNW_ABI_VERSION,
            struct_size: size_of::<TknwDownloadConfig>(),
            url: url.as_ptr(),
            target_path: target.as_ptr(),
            chunk_size: 0,
            parallelism: 0,
            max_parallel_chunks: 0,
            max_retries: 4,
            backoff_initial_millis: 100,
            backoff_max_millis: 3_000,
            connect_timeout_millis: 30_000,
            read_timeout_millis: 0,
            total_timeout_millis: 0,
            bytes_per_second_limit: 0,
            hash_kind: TknwHashKind::None as u32,
            expected_sha256: ptr::null(),
            expected_sha256_len: 0,
        };
        let mut handle = ptr::null_mut();
        assert_eq!(
            tknw_download_create(&raw const config, &raw mut handle),
            TknwStatus::Ok
        );
        assert!(!handle.is_null());

        let mut snapshot = TknwDownloadSnapshot {
            abi_version: TKNW_ABI_VERSION,
            struct_size: size_of::<TknwDownloadSnapshot>(),
            phase: 0,
            content_len: 0,
            downloaded_bytes: 0,
            chunk_size: 0,
            chunk_count: 0,
            completed_chunks: 0,
            active_io: 0,
        };
        assert_eq!(
            tknw_download_snapshot(handle, &raw mut snapshot),
            TknwStatus::Ok
        );
        assert_eq!(snapshot.phase, DownloadPhase::Created as u32);

        let counters = CallbackCounters {
            calls: AtomicUsize::new(0),
            releases: AtomicUsize::new(0),
        };
        assert_eq!(
            tknw_download_set_progress_callback(
                handle,
                Some(count_progress),
                (&raw const counters).cast_mut().cast::<c_void>(),
                Some(count_release),
            ),
            TknwStatus::Ok
        );
        assert_eq!(counters.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            tknw_download_set_progress_callback(handle, None, ptr::null_mut(), None),
            TknwStatus::Ok
        );
        assert_eq!(counters.releases.load(Ordering::SeqCst), 1);

        assert_eq!(tknw_download_release(&raw mut handle), TknwStatus::Ok);
        assert!(handle.is_null());
        assert_eq!(
            tknw_download_release(&raw mut handle),
            TknwStatus::NullPointer
        );
        assert_eq!(tknw_global_shutdown(), TknwStatus::Ok);
    }

    #[test]
    fn maps_new_download_phase_values() {
        assert_eq!(phase_to_u32(DownloadPhase::Starting), 8);
        assert_eq!(phase_to_u32(DownloadPhase::Allocating), 9);
        assert_eq!(phase_to_u32(DownloadPhase::Verifying), 10);
    }

    #[test]
    fn rejects_bad_struct_size() {
        let _guard = TEST_LOCK.lock().unwrap();
        let global_config = TknwGlobalConfig {
            abi_version: TKNW_ABI_VERSION,
            struct_size: size_of::<TknwGlobalConfig>() - 1,
            max_io: 2,
        };

        assert_eq!(
            tknw_global_init(&raw const global_config),
            TknwStatus::AbiMismatch
        );
    }
}
