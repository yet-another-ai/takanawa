use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use takanawa_core::PartMetadata;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPhase {
    Created = 0,
    Running = 1,
    Paused = 2,
    Cancelled = 3,
    Completed = 4,
    Failed = 5,
}

#[derive(Debug, Clone)]
pub struct DownloadSnapshot {
    pub phase: DownloadPhase,
    pub content_len: u64,
    pub downloaded_bytes: u64,
    pub chunk_size: u64,
    pub chunk_count: u64,
    pub completed_chunks: u64,
    pub active_io: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SharedState {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    progress: Mutex<Progress>,
    active_io: AtomicUsize,
}

#[derive(Debug, Clone)]
struct Progress {
    phase: DownloadPhase,
    content_len: u64,
    downloaded_bytes: u64,
    chunk_size: u64,
    chunk_count: u64,
    completed_chunks: u64,
    bitmap: Vec<u8>,
    last_error: Option<String>,
}

impl SharedState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                progress: Mutex::new(Progress {
                    phase: DownloadPhase::Created,
                    content_len: 0,
                    downloaded_bytes: 0,
                    chunk_size: 0,
                    chunk_count: 0,
                    completed_chunks: 0,
                    bitmap: Vec::new(),
                    last_error: None,
                }),
                active_io: AtomicUsize::new(0),
            }),
        }
    }

    pub fn set_phase(&self, phase: DownloadPhase) {
        self.inner
            .progress
            .lock()
            .expect("download state mutex poisoned")
            .phase = phase;
    }

    pub fn set_error(&self, message: impl Into<String>) {
        let mut progress = self
            .inner
            .progress
            .lock()
            .expect("download state mutex poisoned");
        progress.phase = DownloadPhase::Failed;
        progress.last_error = Some(message.into());
    }

    pub fn clear_error(&self) {
        self.inner
            .progress
            .lock()
            .expect("download state mutex poisoned")
            .last_error = None;
    }

    pub fn update_from_metadata(&self, metadata: &PartMetadata) {
        let mut progress = self
            .inner
            .progress
            .lock()
            .expect("download state mutex poisoned");
        progress.content_len = metadata.content_len;
        progress.downloaded_bytes = metadata.completed_bytes();
        progress.chunk_size = metadata.chunk_size;
        progress.chunk_count = metadata.chunk_count;
        progress.completed_chunks = metadata.completed_chunks();
        progress.bitmap = metadata.bitmap.as_bytes().to_vec();
    }

    pub fn increment_active_io(&self) {
        self.inner.active_io.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active_io(&self) {
        self.inner.active_io.fetch_sub(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn snapshot(&self) -> DownloadSnapshot {
        let progress = self
            .inner
            .progress
            .lock()
            .expect("download state mutex poisoned")
            .clone();
        DownloadSnapshot {
            phase: progress.phase,
            content_len: progress.content_len,
            downloaded_bytes: progress.downloaded_bytes,
            chunk_size: progress.chunk_size,
            chunk_count: progress.chunk_count,
            completed_chunks: progress.completed_chunks,
            active_io: self.inner.active_io.load(Ordering::Relaxed),
            last_error: progress.last_error,
        }
    }

    #[must_use]
    pub fn bitmap(&self) -> Vec<u8> {
        self.inner
            .progress
            .lock()
            .expect("download state mutex poisoned")
            .bitmap
            .clone()
    }
}
