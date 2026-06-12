use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

/// Default maximum number of simultaneous HTTP/file I/O operations.
pub const DEFAULT_MAX_IO: usize = 16;

/// Async limiter for coordinating in-flight HTTP and file I/O.
#[derive(Debug, Clone)]
pub struct IoLimiter {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    state: Mutex<State>,
    notify: Notify,
}

#[derive(Debug)]
struct State {
    max: usize,
    in_flight: usize,
}

/// Permit returned by [`IoLimiter::acquire`].
///
/// Dropping the permit releases one in-flight slot.
#[derive(Debug)]
pub struct IoPermit {
    inner: Arc<Inner>,
}

impl IoLimiter {
    #[must_use]
    /// Creates an I/O limiter.
    ///
    /// A `max` value of `0` is normalized to `1`.
    pub fn new(max: usize) -> Self {
        let max = max.max(1);
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State { max, in_flight: 0 }),
                notify: Notify::new(),
            }),
        }
    }

    /// Waits until an I/O slot is available and returns a permit.
    ///
    /// # Panics
    ///
    /// Panics if the limiter mutex is poisoned.
    pub async fn acquire(&self) -> IoPermit {
        loop {
            let notified = {
                let mut state = self.inner.state.lock().expect("I/O limiter mutex poisoned");
                if state.in_flight < state.max {
                    state.in_flight += 1;
                    return IoPermit {
                        inner: Arc::clone(&self.inner),
                    };
                }
                self.inner.notify.notified()
            };
            notified.await;
        }
    }

    /// Updates the maximum number of in-flight I/O operations.
    ///
    /// A `max` value of `0` is normalized to `1`.
    ///
    /// # Panics
    ///
    /// Panics if the limiter mutex is poisoned.
    pub fn set_max(&self, max: usize) {
        let mut state = self.inner.state.lock().expect("I/O limiter mutex poisoned");
        state.max = max.max(1);
        drop(state);
        self.inner.notify.notify_waiters();
    }

    #[must_use]
    /// Returns the configured maximum number of in-flight I/O operations.
    ///
    /// # Panics
    ///
    /// Panics if the limiter mutex is poisoned.
    pub fn max(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("I/O limiter mutex poisoned")
            .max
    }

    #[must_use]
    /// Returns the current number of in-flight I/O operations.
    ///
    /// # Panics
    ///
    /// Panics if the limiter mutex is poisoned.
    pub fn in_flight(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("I/O limiter mutex poisoned")
            .in_flight
    }
}

impl Drop for IoPermit {
    fn drop(&mut self) {
        let mut state = self.inner.state.lock().expect("I/O limiter mutex poisoned");
        state.in_flight = state.in_flight.saturating_sub(1);
        drop(state);
        self.inner.notify.notify_one();
    }
}
