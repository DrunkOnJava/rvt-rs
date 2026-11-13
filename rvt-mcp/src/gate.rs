//! How many models are read at once (#1883).
//!
//! The store keeps at most `--max-open-models` models, and the decode of one
//! peaks at several GB. Concurrent tool calls each ran their own decode before
//! the store dropped the models past that number, so N calls for N files
//! decoded N models at once. Every read of a file from disk and every decode
//! now takes a place at this gate first, as many places as models are kept.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rvt::control::WalkerControl;

use crate::config::Failure;

/// How long a call waits for a place before it looks at its cancellation again.
const RECHECK: Duration = Duration::from_millis(100);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A counting gate: `places` calls inside at once, the rest waiting.
#[derive(Debug)]
pub struct Gate {
    free: Mutex<usize>,
    changed: Condvar,
}

/// A place at the gate, given back when it is dropped, however the call ends.
pub struct Place<'a> {
    gate: &'a Gate,
}

impl Gate {
    pub fn new(places: usize) -> Self {
        Self {
            free: Mutex::new(places.max(1)),
            changed: Condvar::new(),
        }
    }

    /// Waits for a place. A call whose request was cancelled leaves the queue
    /// with [`Failure::Cancelled`] instead of waiting for one.
    pub fn enter(&self, control: &WalkerControl) -> Result<Place<'_>, Failure> {
        let mut free = lock(&self.free);
        loop {
            if *free > 0 {
                *free -= 1;
                return Ok(Place { gate: self });
            }
            control.check().map_err(|_| Failure::Cancelled)?;
            free = self
                .changed
                .wait_timeout(free, RECHECK)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

impl Drop for Place<'_> {
    fn drop(&mut self) {
        *lock(&self.gate.free) += 1;
        self.gate.changed.notify_one();
    }
}
