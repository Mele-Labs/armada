//! The pull request and issue rotations, beside the turn.
//!
//! **A turn waits behind each Job's Checks**, so a landing that rode only on
//! the turn could be told minutes late. This is `crate::main_ci::keep_reading_main`'s
//! shape for the two other forge readings: whoever reaches the interval first
//! reads, the interval gates stay where they were, and the turn still calls
//! both.

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
    /// Ask about one pull request and one issue, outside any turn. **A Job owed
    /// news of a landing is told here**, because `tell_peers` otherwise waits
    /// for a turn; it only moves what the landing queued.
    pub(crate) async fn read_landings(&self) -> (Result<(), Adrift>, Result<(), Adrift>) {
        let merge = async {
            if self.notice_a_merge().await?.is_some() {
                self.tell_peers().await;
            }
            Ok(())
        }
        .await;
        let issue = self.notice_an_issue().await.map(|_| ());
        (merge, issue)
    }
}

/// Read the pull request and issue rotations every `tick`; their own
/// intervals still gate them.
pub fn keep_noticing<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    tick: Duration,
    adrift: impl Fn(Adrift) + Send + 'static,
) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let (merge, issue) = fleet.read_landings().await;
            for why in [merge, issue].into_iter().filter_map(Result::err) {
                adrift(why);
            }
        }
    })
}

/// Work the failed Triggers waiting for a repair Drone, one at a time, every
/// `tick`. Apart from [`keep_noticing`] because a repair runs a Drone for as
/// long as a Check's budget, and nothing waiting behind it should be a landing.
pub fn keep_repairing<H, V, W>(fleet: Arc<Fleet<H, V, W>>, tick: Duration) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        fleet.repairs_recovered().await;
        let mut ticker = tokio::time::interval(tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            while fleet.repair_next().await {}
            fleet.chosen_fixes_retried().await;
        }
    })
}
