//! Fleet over each repository's backlog: the look that syncs it while Fleet runs, and the claim
//! `claim_issue` makes in it. What a look and a claim do is [`crate::backlog`]'s.
//!
//! **One [`Keeping`] a repository, behind one lock**, which the look and the claim both take, so
//! no two syncs of one repository overlap. A look that finds the lock held skips that repository
//! for the turn rather than waiting behind a claim.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Backlog, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use tokio::task::JoinHandle;

use crate::backlog::{claim_settled, claims_lost, ClaimLost, Keeping, Looked, SYNC_EVERY};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// One repository's [`Keeping`], and the failures last said for it.
#[derive(Default)]
pub(crate) struct Kept {
    keeping: Keeping,
    failing: Option<String>,
    /// What the writes of the last look said went wrong, joined.
    writing: Option<String>,
    /// When a write for something last failed, by what it was for. It is tried again once a
    /// [`SYNC_EVERY`] has passed, as a sync is.
    pub(crate) failed_at: BTreeMap<String, Duration>,
}

impl Kept {
    /// Whether the write for `what` may be tried: it has not failed, or not lately.
    pub(crate) fn due(&self, what: &str, now: Duration) -> bool {
        self.failed_at.get(what).is_none_or(|at| now.saturating_sub(*at) >= SYNC_EVERY)
    }
}

/// Every repository's [`Kept`], by root. Never written down: a restart syncs at its first look.
pub(crate) struct Keepings {
    started: Mutex<Instant>,
    by_root: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<Kept>>>>,
}

impl Keepings {
    pub(crate) fn new() -> Keepings {
        Keepings { started: Mutex::new(Instant::now()), by_root: Mutex::new(BTreeMap::new()) }
    }

    fn of(&self, root: &str) -> Arc<tokio::sync::Mutex<Kept>> {
        let mut held = self.by_root.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(held.entry(root.to_string()).or_default())
    }

    fn now(&self) -> Duration {
        self.started.lock().unwrap_or_else(PoisonError::into_inner).elapsed()
    }

    /// Let `by` pass without waiting for it, so a test reaches the timer.
    #[cfg(test)]
    pub(crate) fn later(&self, by: Duration) {
        let mut started = self.started.lock().unwrap_or_else(PoisonError::into_inner);
        *started -= by;
    }
}

/// Look at every backlog every `every`. [`crate::backlog::LOOK_EVERY`] is what it is run at.
/// `said` hears a repository's sync start failing and stop.
pub fn keep_syncing<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    every: Duration,
    said: impl Fn(&str) + Send + Sync + 'static,
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
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            for line in fleet.backlogs_looked().await {
                said(&line);
            }
        }
    })
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
    /// One look at each served repository whose tracker Fleet writes to, one after another, and
    /// what is worth saying of them: a sync or a write that starts failing or stops, once each, not
    /// every turn.
    ///
    /// **Writes come before the look**, so the look sees them in the copy's mark and syncs them: the
    /// plan mirrored into a Job's issue, and the issue closed for a merge. **After a sync**, each
    /// claim a Job made is read back, and a Job whose claim now names someone else is told.
    pub(crate) async fn backlogs_looked(self: &Arc<Self>) -> Vec<String> {
        let mut lines = Vec::new();
        for served in self.repositories().served() {
            let Some(backlog) = self.backlog_of(&served) else {
                continue;
            };
            let Ok(mut kept) = self.tracking().keepings.of(served.root()).try_lock_owned() else {
                continue;
            };
            let (root, now) = (served.root().to_string(), self.tracking().keepings.now());
            let manifest = served.manifest().id().as_str().to_string();

            let mut failed = self.plans_mirrored(&served, &backlog, &mut kept, now).await;
            failed.extend(self.merged_issues_closed(&served, &backlog, &mut kept, now).await);
            // A write waiting out its turn to be tried again has not recovered.
            let writing = match (failed.is_empty(), kept.failed_at.is_empty()) {
                (false, _) => Some(failed.join("; ")),
                (true, true) => None,
                (true, false) => kept.writing.clone(),
            };
            match (&kept.writing, &writing) {
                (was, Some(now)) if was.as_ref() != Some(now) => lines.push(format!("{root}: {now}")),
                (Some(_), None) => lines.push(format!("{root}: the backlog writes went through again")),
                _ => {}
            }
            kept.writing = writing;

            let (held, tracker) = (self.claims_held(&manifest).await, self.tracker_of(&served));
            let looked = tokio::task::spawn_blocking(move || {
                let looked = kept.keeping.look(backlog.as_ref(), &root, now);
                let failing = match &looked {
                    Looked::Waited => return (looked, None, Vec::new()),
                    Looked::Failed(why) => Some(format!("{} did not happen: {}", why.doing, why.said)),
                    Looked::Synced { .. } => None,
                };
                let line = match (&kept.failing, &failing) {
                    (was, Some(now)) if was.as_ref() != Some(now) => Some(format!("{root}: {now}")),
                    (Some(_), None) => Some(format!("{root}: the backlog synced again")),
                    _ => None,
                };
                kept.failing = failing;
                let lost = match &looked {
                    Looked::Synced { .. } => claims_lost(tracker.as_ref(), &root, &held),
                    _ => Vec::new(),
                };
                (looked, line, lost)
            })
            .await;
            let Ok((looked, line, lost)) = looked else {
                continue;
            };
            lines.extend(line);
            if let Looked::Synced { moved: true } = looked {
                self.tracking().forget(served.manifest().id().as_str());
            }
            for one in &lost {
                self.claim_lost_told(&manifest, one).await;
            }
        }
        lines
    }

    /// `actor` claims `reference` in `served`'s backlog: sync, claim, sync, read back.
    pub(crate) async fn claimed_in_backlog(
        &self,
        served: &Served,
        backlog: Arc<dyn Backlog + Send + Sync>,
        reference: &str,
        actor: &str,
    ) -> Result<(), ClaimLost> {
        let mut kept = self.tracking().keepings.of(served.root()).lock_owned().await;
        let tracker = self.tracker_of(served);
        let (root, reference, actor) = (served.root().to_string(), reference.to_string(), actor.to_string());
        let now = self.tracking().keepings.now();
        let settled = tokio::task::spawn_blocking(move || {
            claim_settled(&mut kept.keeping, backlog.as_ref(), tracker.as_ref(), &root, &reference, &actor, now)
        })
        .await
        .unwrap_or_else(|why| {
            Err(ClaimLost::Unsettled(adapter_traits::NotTracked::of("claiming the issue", why.to_string())))
        });
        self.tracking().forget(served.manifest().id().as_str());
        settled
    }

    /// Who a claim by Job `id` is made as: this machine's name and the start of the Job's id.
    pub(crate) async fn claim_actor(&self, id: &str) -> String {
        let configured: String = self.machine_settings().now().get(keys::MACHINE_NAME);
        let job: String = id.chars().take(8).collect();
        tokio::task::spawn_blocking(move || format!("{}/{job}", crate::signing_in::this_machine(&configured)))
            .await
            .unwrap_or_else(|_| format!("this machine/{}", id.chars().take(8).collect::<String>()))
    }
}
