use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
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
    Pausing = 6,
    Cancelling = 7,
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

pub type ProgressCallback = Arc<dyn Fn(DownloadSnapshot) + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub(crate) struct SharedState {
    inner: Arc<Inner>,
}

struct Inner {
    progress: Mutex<Progress>,
    active_io: AtomicUsize,
    progress_callback: Mutex<Option<ProgressCallback>>,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("progress", &self.progress)
            .field("active_io", &self.active_io)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct Progress {
    lifecycle: DownloadLifecycle,
    content_len: u64,
    downloaded_bytes: u64,
    chunk_size: u64,
    chunk_count: u64,
    completed_chunks: u64,
    bitmap: Vec<u8>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadLifecycle {
    Created,
    Running,
    Pausing,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

impl DownloadLifecycle {
    const fn phase(self) -> DownloadPhase {
        match self {
            Self::Created => DownloadPhase::Created,
            Self::Running => DownloadPhase::Running,
            Self::Pausing => DownloadPhase::Pausing,
            Self::Paused => DownloadPhase::Paused,
            Self::Cancelling => DownloadPhase::Cancelling,
            Self::Cancelled => DownloadPhase::Cancelled,
            Self::Completed => DownloadPhase::Completed,
            Self::Failed => DownloadPhase::Failed,
        }
    }

    const fn start(self) -> Self {
        match self {
            Self::Cancelling => Self::Cancelling,
            Self::Running | Self::Pausing => self,
            Self::Created | Self::Paused | Self::Cancelled | Self::Completed | Self::Failed => {
                Self::Running
            }
        }
    }

    const fn request_pause(self) -> Self {
        match self {
            Self::Running | Self::Pausing => Self::Pausing,
            _ => self,
        }
    }

    const fn mark_paused(self) -> Self {
        match self {
            Self::Running | Self::Pausing => Self::Paused,
            _ => self,
        }
    }

    const fn request_cancel(self) -> Self {
        match self {
            Self::Created => Self::Cancelled,
            Self::Running | Self::Pausing | Self::Paused => Self::Cancelling,
            _ => self,
        }
    }

    const fn mark_cancelled(self) -> Self {
        match self {
            Self::Created | Self::Running | Self::Pausing | Self::Paused | Self::Cancelling => {
                Self::Cancelled
            }
            _ => self,
        }
    }

    const fn mark_completed(self) -> Self {
        match self {
            Self::Running | Self::Pausing | Self::Paused => Self::Completed,
            _ => self,
        }
    }

    const fn mark_failed(self) -> Self {
        match self {
            Self::Created
            | Self::Running
            | Self::Pausing
            | Self::Paused
            | Self::Cancelling
            | Self::Cancelled
            | Self::Completed
            | Self::Failed => Self::Failed,
        }
    }
}

impl SharedState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                progress: Mutex::new(Progress {
                    lifecycle: DownloadLifecycle::Created,
                    content_len: 0,
                    downloaded_bytes: 0,
                    chunk_size: 0,
                    chunk_count: 0,
                    completed_chunks: 0,
                    bitmap: Vec::new(),
                    last_error: None,
                }),
                active_io: AtomicUsize::new(0),
                progress_callback: Mutex::new(None),
            }),
        }
    }

    pub fn set_progress_callback(&self, callback: Option<ProgressCallback>) {
        let should_notify = callback.is_some();
        let previous = {
            let mut progress_callback = self
                .inner
                .progress_callback
                .lock()
                .expect("download callback mutex poisoned");
            std::mem::replace(&mut *progress_callback, callback)
        };
        drop(previous);
        if should_notify {
            self.notify_progress();
        }
    }

    pub fn mark_running(&self) {
        self.transition(DownloadLifecycle::start);
    }

    pub fn request_pause(&self) {
        self.transition(DownloadLifecycle::request_pause);
    }

    pub fn mark_paused(&self) {
        self.transition(DownloadLifecycle::mark_paused);
    }

    pub fn request_cancel(&self) {
        self.transition(DownloadLifecycle::request_cancel);
    }

    pub fn mark_cancelled(&self) {
        self.transition(DownloadLifecycle::mark_cancelled);
    }

    pub fn mark_completed(&self) {
        self.transition(DownloadLifecycle::mark_completed);
    }

    fn transition(&self, transition: impl FnOnce(DownloadLifecycle) -> DownloadLifecycle) {
        let changed = {
            let mut progress = self
                .inner
                .progress
                .lock()
                .expect("download state mutex poisoned");
            let next = transition(progress.lifecycle);
            let changed = progress.lifecycle != next;
            progress.lifecycle = next;
            changed
        };
        if changed {
            self.notify_progress();
        }
    }

    pub fn mark_failed(&self, message: impl Into<String>) {
        {
            let mut progress = self
                .inner
                .progress
                .lock()
                .expect("download state mutex poisoned");
            progress.lifecycle = progress.lifecycle.mark_failed();
            progress.last_error = Some(message.into());
        }
        self.notify_progress();
    }

    pub fn clear_error(&self) {
        self.inner
            .progress
            .lock()
            .expect("download state mutex poisoned")
            .last_error = None;
    }

    pub fn update_from_metadata(&self, metadata: &PartMetadata) {
        {
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
        self.notify_progress();
    }

    pub fn increment_active_io(&self) {
        self.inner.active_io.fetch_add(1, Ordering::Relaxed);
        self.notify_progress();
    }

    pub fn decrement_active_io(&self) {
        self.inner.active_io.fetch_sub(1, Ordering::Relaxed);
        self.notify_progress();
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
            phase: progress.lifecycle.phase(),
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

    fn notify_progress(&self) {
        let callback = self
            .inner
            .progress_callback
            .lock()
            .expect("download callback mutex poisoned")
            .clone();
        if let Some(callback) = callback {
            let snapshot = self.snapshot();
            let _ = catch_unwind(AssertUnwindSafe(|| callback(snapshot)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_reports_transitional_pause_and_cancel_phases() {
        let state = SharedState::new();

        assert_eq!(state.snapshot().phase, DownloadPhase::Created);
        state.mark_running();
        assert_eq!(state.snapshot().phase, DownloadPhase::Running);
        state.request_pause();
        assert_eq!(state.snapshot().phase, DownloadPhase::Pausing);
        state.mark_paused();
        assert_eq!(state.snapshot().phase, DownloadPhase::Paused);
        state.mark_running();
        assert_eq!(state.snapshot().phase, DownloadPhase::Running);
        state.request_cancel();
        assert_eq!(state.snapshot().phase, DownloadPhase::Cancelling);
        state.mark_cancelled();
        assert_eq!(state.snapshot().phase, DownloadPhase::Cancelled);
    }

    #[test]
    fn lifecycle_keeps_terminal_states_stable_for_late_events() {
        let state = SharedState::new();

        state.mark_running();
        state.mark_completed();
        state.request_pause();
        state.request_cancel();

        assert_eq!(state.snapshot().phase, DownloadPhase::Completed);
    }

    #[test]
    fn progress_callback_receives_current_and_changed_snapshots() {
        let state = SharedState::new();
        let phases = Arc::new(Mutex::new(Vec::new()));
        let callback_phases = Arc::clone(&phases);

        state.set_progress_callback(Some(Arc::new(move |snapshot| {
            callback_phases.lock().unwrap().push(snapshot.phase);
        })));
        state.mark_running();
        state.mark_completed();
        state.set_progress_callback(None);
        state.mark_failed("ignored");

        assert_eq!(
            *phases.lock().unwrap(),
            vec![
                DownloadPhase::Created,
                DownloadPhase::Running,
                DownloadPhase::Completed,
            ]
        );
    }
}
