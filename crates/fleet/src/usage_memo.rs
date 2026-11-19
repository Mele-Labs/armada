//! The Usage page's two heavy answers, kept in memory for five minutes and read off the store's lock.
//! `docs/practices/protocol.md`, *The waste in the burn* and *Settings that change the burn*.
//!
//! **Per window, and only an answer that was made**: a refusal is made again. The adjustments are dropped the
//! moment a setting they show changes, so the page sees its own change; the waste has no setting behind it
//! and only ages. The clock is Fleet's, so a test can hold it.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};

use api::Refusal;
use tokio::sync::Mutex;

use crate::daemon::Fleet;
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

/// How long an answer is served from memory: five minutes.
pub(crate) const FRESH_MILLIS: i64 = 5 * 60 * 1_000;

type Kept<T> = Mutex<HashMap<String, (i64, T)>>;

#[derive(Default)]
pub(crate) struct Memo {
    waste: Kept<ipc::UsageWaste>,
    adjustments: Kept<ipc::UsageAdjustments>,
    made: [AtomicUsize; 2],
}

impl Memo {
    /// The waste of `window`, from memory where it was made less than [`FRESH_MILLIS`] before `now`, and made
    /// by `make` where it was not. **The lock is held while it is made**, so two readers of one window make it once.
    pub(crate) async fn waste(&self, window: &str, now: Option<i64>, make: impl Future<Output = Result<ipc::UsageWaste, Refusal>>) -> Result<ipc::UsageWaste, Refusal> {
        through(&self.waste, &self.made[0], window, now, make).await
    }

    pub(crate) async fn adjustments(
        &self,
        window: &str,
        now: Option<i64>,
        make: impl Future<Output = Result<ipc::UsageAdjustments, Refusal>>,
    ) -> Result<ipc::UsageAdjustments, Refusal> {
        through(&self.adjustments, &self.made[1], window, now, make).await
    }

    /// A setting the adjustments show changed. Waits for an answer being made, so it cannot be stored over this.
    pub(crate) async fn adjustments_changed(&self) {
        self.adjustments.lock().await.clear();
    }

    /// How many waste answers and how many adjustments answers have been made, as a test counts them.
    #[cfg(test)]
    pub(crate) fn made(&self) -> (usize, usize) {
        (self.made[0].load(Ordering::SeqCst), self.made[1].load(Ordering::SeqCst))
    }
}

async fn through<T: Clone>(
    kept: &Kept<T>,
    made: &AtomicUsize,
    window: &str,
    now: Option<i64>,
    make: impl Future<Output = Result<T, Refusal>>,
) -> Result<T, Refusal> {
    let mut held = kept.lock().await;
    if let (Some(now), Some((at, answer))) = (now, held.get(window)) {
        if (0..FRESH_MILLIS).contains(&(now - at)) {
            return Ok(answer.clone());
        }
    }
    let answer = make.await?;
    made.fetch_add(1, Ordering::SeqCst);
    match now {
        Some(now) => {
            held.insert(window.to_string(), (now, answer.clone()));
        }
        None => {
            held.remove(window);
        }
    }
    Ok(answer)
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Run `work` on a connection of its own that only reads, off the async threads and **off the store's lock**,
    /// which is held only to learn where the file is. A long sum over the kept turns held it for seconds, and
    /// every other request waited behind it.
    pub(crate) async fn read_apart<T: Send + 'static>(
        &self,
        work: impl FnOnce(&store::Store) -> Result<T, store::WriteError> + Send + 'static,
    ) -> Result<T, Refusal> {
        let path = self.store().lock().await.path().to_string();
        let done = tokio::task::spawn_blocking(move || work(&store::Store::open_reading(std::path::Path::new(&path))?)).await;
        let done = done.unwrap_or_else(|why| {
            Err(store::WriteError::Database(store::DatabaseFault { doing: "reading the usage off the lock", cause: Box::new(why) }))
        });
        done.map_err(|why| self.ledger_fault(why))
    }
}
