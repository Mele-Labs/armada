//! The one lock on the store, which names whoever holds it too long.

use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::panic::Location;
use std::time::{Duration, Instant};

use store::Store;
use tokio::sync::{Mutex, MutexGuard};

/// Every request waits on this lock, so a holder past this is the next latency report.
const LONG: Duration = Duration::from_millis(250);

/// The store's `Mutex`, whose guard says on stderr (Fleet's log) where it was taken when held past [`LONG`].
pub(crate) struct StoreLock(Mutex<Store>);

impl StoreLock {
    pub(crate) fn new(store: Store) -> Self {
        Self(Mutex::new(store))
    }

    /// Not an `async fn`, so `#[track_caller]` sees the call site rather than this file.
    #[track_caller]
    pub(crate) fn lock(&self) -> impl Future<Output = StoreGuard<'_>> + '_ {
        let at = Location::caller();
        async move {
            StoreGuard { held: self.0.lock().await, since: Instant::now(), at }
        }
    }
}

pub(crate) struct StoreGuard<'a> {
    held: MutexGuard<'a, Store>,
    since: Instant,
    at: &'static Location<'static>,
}

impl Deref for StoreGuard<'_> {
    type Target = Store;
    fn deref(&self) -> &Store {
        &self.held
    }
}

impl DerefMut for StoreGuard<'_> {
    fn deref_mut(&mut self) -> &mut Store {
        &mut self.held
    }
}

impl Drop for StoreGuard<'_> {
    fn drop(&mut self) {
        let held = self.since.elapsed();
        if held > LONG {
            eprintln!("store held {}ms by {}:{}", held.as_millis(), self.at.file(), self.at.line());
        }
    }
}
