#![allow(unsafe_code)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex};

use takanawa_core::{HashConfig, TakanawaError};
use takanawa_http::{
    DEFAULT_MAX_IO, DownloadConfig, DownloadEngine, DownloadHandle, DownloadPhase,
};
use tokio::runtime::{Builder, Runtime};

pub const TKNW_ABI_VERSION: u32 = 1;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TknwStatus {
    Ok = 0,
    BufferTooSmall = 1,
    NullPointer = -1,
    AbiMismatch = -2,
    InvalidConfig = -3,
    RuntimeNotInitialized = -4,
    TargetExists = -10,
    PartBusy = -11,
    PartSizeMismatch = -12,
    PartCorrupt = -13,
    RemoteChanged = -14,
    HttpProtocol = -20,
    Network = -21,
    Io = -30,
    HashMismatch = -40,
    Cancelled = -50,
    AlreadyStarted = -51,
    Panic = -100,
    Internal = -101,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TknwHashKind {
    None = 0,
    Sha256 = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwGlobalConfig {
    pub abi_version: u32,
    pub struct_size: usize,
    pub max_io: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwDownloadConfig {
    pub abi_version: u32,
    pub struct_size: usize,
    pub url: *const c_char,
    pub target_path: *const c_char,
    pub chunk_size: u64,
    pub parallelism: usize,
    pub hash_kind: u32,
    pub expected_sha256: *const c_uchar,
    pub expected_sha256_len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TknwDownloadSnapshot {
    pub abi_version: u32,
    pub struct_size: usize,
    pub phase: u32,
    pub content_len: u64,
    pub downloaded_bytes: u64,
    pub chunk_size: u64,
    pub chunk_count: u64,
    pub completed_chunks: u64,
    pub active_io: usize,
}

pub struct TknwDownload {
    global: Arc<GlobalRuntime>,
    inner: DownloadHandle,
    last_error: Mutex<Option<String>>,
}

struct GlobalRuntime {
    runtime: Runtime,
    engine: DownloadEngine,
}

static GLOBAL: LazyLock<Mutex<Option<Arc<GlobalRuntime>>>> = LazyLock::new(|| Mutex::new(None));

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

#[unsafe(no_mangle)]
pub extern "C" fn tknw_global_shutdown() -> TknwStatus {
    ffi_boundary(|| {
        let mut global = GLOBAL.lock().expect("global runtime mutex poisoned");
        let _ = global.take();
        Ok(TknwStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tknw_global_set_max_io(max_io: usize) -> TknwStatus {
    ffi_boundary(|| {
        let global = current_global()?;
        global.engine.set_max_io(max_io);
        Ok(TknwStatus::Ok)
    })
}

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

#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_start(download: *mut TknwDownload) -> TknwStatus {
    ffi_download_boundary(download, |download| {
        download.inner.start_on(&download.global.runtime)?;
        Ok(TknwStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_pause(download: *mut TknwDownload) -> TknwStatus {
    ffi_download_boundary(download, |download| {
        download.inner.pause()?;
        Ok(TknwStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tknw_download_cancel(download: *mut TknwDownload) -> TknwStatus {
    ffi_download_boundary(download, |download| {
        download.inner.cancel()?;
        Ok(TknwStatus::Ok)
    })
}

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
        snapshot_ref.phase = phase_to_u32(current.phase);
        snapshot_ref.content_len = current.content_len;
        snapshot_ref.downloaded_bytes = current.downloaded_bytes;
        snapshot_ref.chunk_size = current.chunk_size;
        snapshot_ref.chunk_count = current.chunk_count;
        snapshot_ref.completed_chunks = current.completed_chunks;
        snapshot_ref.active_io = current.active_io;
        Ok(TknwStatus::Ok)
    })
}

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
    match config.hash_kind {
        0 => Ok(HashConfig::None),
        1 => {
            if config.expected_sha256.is_null() {
                return Err(TakanawaError::NullPointer("expected_sha256"));
            }
            if config.expected_sha256_len != 32 {
                return Err(TakanawaError::InvalidConfig(format!(
                    "SHA-256 expected hash length must be 32, got {}",
                    config.expected_sha256_len
                )));
            }
            let mut hash = [0; 32];
            // SAFETY: expected_sha256 is non-null and expected_sha256_len was validated as 32.
            unsafe {
                ptr::copy_nonoverlapping(config.expected_sha256, hash.as_mut_ptr(), 32);
            }
            Ok(HashConfig::Sha256(hash))
        }
        other => Err(TakanawaError::InvalidConfig(format!(
            "unsupported hash kind {other}"
        ))),
    }
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

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::sync::{LazyLock, Mutex};

    use tempfile::TempDir;

    use super::*;

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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

        assert_eq!(tknw_download_release(&raw mut handle), TknwStatus::Ok);
        assert!(handle.is_null());
        assert_eq!(
            tknw_download_release(&raw mut handle),
            TknwStatus::NullPointer
        );
        assert_eq!(tknw_global_shutdown(), TknwStatus::Ok);
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
