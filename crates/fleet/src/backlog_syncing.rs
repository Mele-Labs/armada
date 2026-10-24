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

use crate::backlog::{claim_settled, ClaimLost, Keeping, Looked};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// One repository's [`Keeping`], and the failure last said for it.
#[derive(Default)]
struct Kept {
    keeping: Keeping,
    failing: Option<String>,
}

/// Every repository's [`Kept`], by root. Never written down: a restart syncs at its first look.
pub(crate) struct Keepings {
    started: Instant,
    by_root: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<Kept>>>>,
}

impl Keepings {
    pub(crate) fn new() -> Keepings {
        Keepings { started: Instant::now(), by_root: Mutex::new(BTreeMap::new()) }
    }

    fn of(&self, root: &str) -> Arc<tokio::sync::Mutex<Kept>> {
        let mut held = self.by_root.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(held.entry(root.to_string()).or_default())
    }

    fn now(&self) -> Duration {
        self.started.elapsed()
    }
}

/// Look at every backlog every `every`. [`crate::backlog::LOOK_EVERY`] is what it is run at.
pub fn keep_syncing<H, V, W>(fleet: Arc<Fleet<H, V, W>>, every: Duration) -> JoinHandle<()>
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
            fleet.backlogs_looked().await;
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
    /// One look at each served repository whose tracker Fleet writes to, one after another.
    pub(crate) async fn backlogs_looked(&self) {
        for served in self.repositories().served() {
            let Some(backlog) = self.backlog_of(&served) else {
                continue;
            };
            let Ok(mut kept) = self.tracking().keepings.of(served.root()).try_lock_owned() else {
                continue;
            };
            let (root, now) = (served.root().to_string(), self.tracking().keepings.now());
            let looked = tokio::task::spawn_blocking(move || {
                let looked = kept.keeping.look(backlog.as_ref(), &root, now);
                let said = match &looked {
                    Looked::Failed(why) => Some(format!("{} did not happen: {}", why.doing, why.said)),
                    _ => None,
                };
                // Said when a repository starts failing and when it stops, not every turn.
                if matches!(looked, Looked::Waited) {
                    return looked;
                }
                match (&kept.failing, &said) {
                    (was, Some(now)) if was.as_ref() != Some(now) => eprintln!("{root}: {now}"),
                    (Some(_), None) => eprintln!("{root}: the backlog synced again"),
                    _ => {}
                }
                kept.failing = said;
                looked
            })
            .await;
            if let Ok(Looked::Synced { moved: true }) = looked {
                self.tracking().forget(served.manifest().id().as_str());
            }
        }
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
