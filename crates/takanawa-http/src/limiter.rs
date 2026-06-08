use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

pub const DEFAULT_MAX_IO: usize = 16;

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

#[derive(Debug)]
pub struct IoPermit {
    inner: Arc<Inner>,
}

impl IoLimiter {
    #[must_use]
    pub fn new(max: usize) -> Self {
        let max = max.max(1);
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State { max, in_flight: 0 }),
                notify: Notify::new(),
            }),
        }
    }

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

    pub fn set_max(&self, max: usize) {
        let mut state = self.inner.state.lock().expect("I/O limiter mutex poisoned");
        state.max = max.max(1);
        drop(state);
        self.inner.notify.notify_waiters();
    }

    #[must_use]
    pub fn max(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("I/O limiter mutex poisoned")
            .max
    }

    #[must_use]
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
