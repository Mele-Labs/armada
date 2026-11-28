//! A merge queue that holds runners for nothing, or cannot get one. `docs/concepts/fleet.md`,
//! *A stuck merge queue*.
//!
//! **Fleet handles it itself, from the forge and never an agent's word.** A queue run no entry
//! stands behind is stopped, a pull request the queue dropped for want of a runner is put back as
//! often as `limits.queueRequeuesPerCommit` allows, and what that cannot clear is called stuck:
//! on the merge line, and as one line on the Job that holds the first pull request.
//!
//! The only forge writes are stopping a run and queueing. Nothing is posted anywhere.

use std::time::Duration;

use adapter_traits::{
    AgentHarness, Delivery, InfraCause, InfraFailure, QueueEntry, QueueRun, Vcs, WorkProduct, MOST_QUEUE_ENTRIES,
};
use config::settings as keys;
use core_model::Timestamp;
use ipc::QueueStuckCause;

use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::repositories::Served;

/// An entry may always wait this long, whatever the forge estimates.
const FLOOR: Duration = Duration::from_secs(60 * 60);

/// The kind of a call raised for a stuck queue, one a repository.
const NOTICE: &str = "queue_stuck:";

/// Why and how a queue is stuck, as the one Needs you line and the merge line say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stuck {
    pub(crate) since: Timestamp,
    pub(crate) cause: QueueStuckCause,
    pub(crate) job: Option<String>,
    pub(crate) log_url: Option<String>,
    /// Pull requests the queue dropped for want of a runner and no requeue is left for.
    pub(crate) dropped: Vec<u64>,
    /// The pull requests waiting on it, in order: the dropped ones, then the queue's.
    pub(crate) pulls: Vec<u64>,
    /// The line for the owner is on a Job's list.
    pub(crate) raised: bool,
}

/// What Fleet holds of one repository's queue between visits. **In memory**, as the rest of the
/// sweep: a restart re-derives it from the forge.
#[derive(Clone, Debug, Default)]
pub(crate) struct QueueWatch {
    pub(crate) stuck: Option<Stuck>,
    /// When a stuck queue last cleared. An entry enqueued before then is not read as waiting past
    /// its estimate again: the queue moved since.
    cleared_at: Option<Timestamp>,
}

/// What a visit read, for deciding whether the queue is stuck or stopped being.
pub(crate) struct Seen<'a> {
    pub(crate) queue: &'a [QueueEntry],
    /// When each recently merged pull request merged, in milliseconds since the epoch.
    pub(crate) merged_at: &'a [i64],
    /// The numbers of the open pull requests.
    pub(crate) open: &'a [u64],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Moved {
    Nothing,
    Lifted,
}

/// Why the jobs of a dropped queue run failed, where every one failed for want of a runner or time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Infrastructure {
    pub(crate) cause: QueueStuckCause,
    pub(crate) job: String,
    pub(crate) log_url: Option<String>,
}

/// **Infrastructure only where every failed job says so**: one failed in its own code makes the
/// drop the Job's to hear of. `None` for no failed job, which is a drop by hand.
pub(crate) fn infrastructure_of(failures: &[InfraFailure]) -> Option<Infrastructure> {
    if failures.is_empty() || failures.iter().any(|one| one.cause == InfraCause::Other) {
        return None;
    }
    let first = failures.iter().find(|one| one.cause == InfraCause::NoRunner).unwrap_or(&failures[0]);
    Some(Infrastructure {
        cause: match first.cause {
            InfraCause::NoRunner => QueueStuckCause::NoRunner,
            _ => QueueStuckCause::TimedOut,
        },
        job: first.job.as_written().to_string(),
        log_url: first.log_url.as_ref().map(|it| it.as_written().to_string()),
    })
}

/// The ids of the runs no entry stands behind: the pull request has none in the queue, or the entry
/// stands at another commit than the run tests. **None from a queue as long as one read holds**,
/// which may be cut short.
pub(crate) fn orphans(runs: &[QueueRun], queue: &[QueueEntry]) -> Vec<u64> {
    if queue.len() >= MOST_QUEUE_ENTRIES {
        return Vec::new();
    }
    runs.iter()
        .filter(|run| {
            let Some(number) = run.number else { return false };
            match queue.iter().find(|entry| entry.number == number) {
                None => true,
                Some(entry) => matches!((&entry.head, &run.head), (Some(held), Some(tested)) if held != tested),
            }
        })
        .map(|run| run.id)
        .collect()
}

/// Whether `entry` waited more than twice the forge's estimate, and an hour at the least.
fn waited_too_long(entry: &QueueEntry, now_ms: i64) -> bool {
    let Some(at) = entry.enqueued_at.as_ref().and_then(|at| Timestamp::from_rfc3339(at.as_written()).epoch_millis()) else {
        return false;
    };
    let allowed = Duration::from_secs(entry.estimated_seconds.unwrap_or(0).saturating_mul(2)).max(FLOOR);
    now_ms.saturating_sub(at) > allowed.as_millis() as i64
}

impl QueueWatch {
    /// What this visit says: a stuck queue clears when a pull request merged after it began or
    /// nothing waits, and a clear one is stuck when an entry waited too long.
    pub(crate) fn evaluated(&mut self, seen: &Seen<'_>, now: &Timestamp) -> Moved {
        let Some(now_ms) = now.epoch_millis() else { return Moved::Nothing };
        let in_queue: Vec<u64> = seen.queue.iter().map(|entry| entry.number).collect();
        if let Some(stuck) = self.stuck.as_mut() {
            let began = stuck.since.epoch_millis().unwrap_or(now_ms);
            let merged = seen.merged_at.iter().any(|at| *at > began);
            let mut waiting: Vec<u64> =
                stuck.dropped.iter().copied().filter(|number| seen.open.contains(number) && !in_queue.contains(number)).collect();
            let queued: Vec<u64> = in_queue.iter().copied().filter(|number| !waiting.contains(number)).collect();
            waiting.extend(queued);
            if merged || waiting.is_empty() {
                self.stuck = None;
                self.cleared_at = Some(now.clone());
                return Moved::Lifted;
            }
            stuck.pulls = waiting;
            return Moved::Nothing;
        }
        let cleared = self.cleared_at.as_ref().and_then(Timestamp::epoch_millis);
        let late = seen.queue.iter().any(|entry| {
            let at = entry.enqueued_at.as_ref().and_then(|at| Timestamp::from_rfc3339(at.as_written()).epoch_millis());
            at.is_some_and(|at| cleared.is_none_or(|cleared| at > cleared)) && waited_too_long(entry, now_ms)
        });
        if late {
            self.stuck = Some(Stuck {
                since: now.clone(),
                cause: QueueStuckCause::PastEstimate,
                job: None,
                log_url: None,
                dropped: Vec::new(),
                pulls: in_queue,
                raised: false,
            });
        }
        Moved::Nothing
    }

    /// A pull request dropped for want of a runner with its requeues spent. **Stuck from the first
    /// such drop**; the next ones only add themselves to what waits.
    pub(crate) fn declared(&mut self, found: &Infrastructure, number: u64, now: &Timestamp) {
        match self.stuck.as_mut() {
            Some(stuck) => {
                if !stuck.dropped.contains(&number) {
                    stuck.dropped.push(number);
                    stuck.pulls.insert(0, number);
                }
            }
            None => {
                self.stuck = Some(Stuck {
                    since: now.clone(),
                    cause: found.cause,
                    job: Some(found.job.clone()),
                    log_url: found.log_url.clone(),
                    dropped: vec![number],
                    pulls: vec![number],
                    raised: false,
                });
            }
        }
    }
}

impl Stuck {
    /// As the merge line draws it.
    pub(crate) fn wire(&self) -> ipc::QueueStuck {
        ipc::QueueStuck {
            since: ipc::Instant::carried(self.since.as_str()),
            cause: self.cause,
            job: self.job.clone(),
            pulls: self.pulls.clone(),
            log_url: self.log_url.clone(),
        }
    }

    /// The one line on the owner's Needs you, in bare facts.
    fn words(&self) -> String {
        match (self.cause, self.job.as_deref()) {
            (QueueStuckCause::NoRunner, Some(job)) => format!("The merge queue is stuck: {job} got no runner."),
            (QueueStuckCause::TimedOut, Some(job)) => format!("The merge queue is stuck: {job} ran out of time."),
            (QueueStuckCause::NoRunner, None) => String::from("The merge queue is stuck: a job got no runner."),
            (QueueStuckCause::TimedOut, None) => String::from("The merge queue is stuck: a job ran out of time."),
            (QueueStuckCause::PastEstimate, _) => match self.pulls.first() {
                Some(number) => format!("The merge queue is stuck: #{number} waited past the forge's estimate."),
                None => String::from("The merge queue is stuck: an entry waited past the forge's estimate."),
            },
        }
    }
}

/// What a drop out of the queue asks of whoever tells the owners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropSaid {
    /// Tell the owners as ever: a failure in its own code, or no cause the forge named.
    Tell,
    /// Said nothing to anybody: put back in the queue, or counted as stuck.
    Quiet,
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
    /// Stop the unfinished queue runs no entry stands behind. **Only after the queue was read**,
    /// and `runs` were listed before it, so an entry made since is never taken for a run's
    /// absence. Nothing is told to anybody.
    pub(crate) async fn orphans_stopped(&self, served: &Served, runs: &[QueueRun], queue: &[QueueEntry]) {
        for id in orphans(runs, queue) {
            let root = served.root().to_string();
            let stopped = self.forge_asked(&root, move |vcs: &V, root: &str| vcs.cancel_run(root, id)).await;
            use core_model::{Component, Envelope, FieldValue, Level};
            let (level, msg, why) = match stopped {
                Ok(()) => (Level::Info, "stopped a queue run no merge queue entry stands behind", None),
                Err(said) => (Level::Warn, "a queue run no entry stands behind was not stopped", Some(said.to_string())),
            };
            let mut envelope = Envelope::new(self.now(), level, Component::Fleet, self.run().clone(), msg)
                .with_field("repository", FieldValue::Str(root))
                .with_field("run", FieldValue::Str(id.to_string()));
            if let Some(why) = why {
                envelope = envelope.with_field("why", FieldValue::Str(why));
            }
            crate::transcript::note_aloud(&envelope);
        }
    }

    /// One visit's reading of the queue: whether it is stuck, and the line on a Job's list that says
    /// so. **Called only where the queue was read this visit.**
    pub(crate) async fn queue_watched(&self, served: &Served) {
        let root = served.root().to_string();
        let now = self.now();
        let (fresh, moved) = {
            let mut sweep = self.sweeping().lock().await;
            let queue = sweep.queues.get(&root).cloned().unwrap_or_default();
            let merged_at: Vec<i64> = sweep
                .merged
                .get(&root)
                .map(|list| list.iter().filter_map(|one| Timestamp::from_rfc3339(one.pull.merged_at.as_written()).epoch_millis()).collect())
                .unwrap_or_default();
            let open: Vec<u64> = sweep.pulls.get(&root).map(|list| list.iter().map(|one| one.pull.number).collect()).unwrap_or_default();
            let fresh = !sweep.queue_watch.contains_key(&root);
            let moved = sweep.queue_watch.entry(root.clone()).or_default().evaluated(&Seen { queue: &queue, merged_at: &merged_at, open: &open }, &now);
            (fresh, moved)
        };
        // A line kept by an earlier run of Fleet, for a queue that is not known to be stuck now.
        if fresh || moved == Moved::Lifted {
            self.stuck_lifted(&root).await;
        }
        self.stuck_called(served).await;
    }

    /// The line for the owner, where the queue is stuck and none stands yet.
    async fn stuck_called(&self, served: &Served) {
        let root = served.root().to_string();
        let Some(stuck) = self.sweeping().lock().await.queue_watch.get(&root).and_then(|watch| watch.stuck.clone()).filter(|it| !it.raised) else {
            return;
        };
        let Some(&number) = stuck.pulls.first() else { return };
        let branch = self
            .sweeping()
            .lock()
            .await
            .pulls
            .get(&root)
            .and_then(|list| list.iter().find(|one| one.pull.number == number))
            .map(|one| one.pull.branch.as_written().to_string())
            .unwrap_or_default();
        let Some(Owner::Session(session)) = self.owners_of(served, number, &branch, false).await.into_iter().next() else {
            return;
        };
        let notice = format!("{NOTICE}{root}");
        let at = self.now().as_str().to_string();
        let kept = {
            let mut store = self.store().lock().await;
            let _ = store.drop_calls(&notice);
            store.keep_call(&session, &notice, &stuck.words(), &at).is_ok()
        };
        if !kept {
            return;
        }
        if let Some(watch) = self.sweeping().lock().await.queue_watch.get_mut(&root) {
            if let Some(stuck) = watch.stuck.as_mut() {
                stuck.raised = true;
            }
        }
        let _ = self.published_hosted(&session).await;
    }

    /// Take the line back off whatever Job held it.
    async fn stuck_lifted(&self, root: &str) {
        let held = self.store().lock().await.drop_calls(&format!("{NOTICE}{root}")).unwrap_or_default();
        for session in held {
            let _ = self.published_hosted(&session).await;
        }
    }

    /// A person asked for the pull request to be queued: the put-backs Fleet kept for each of its commits
    /// are forgotten, so a later runner drop is put back again up to the setting.
    pub(crate) async fn requeues_forgotten(&self, manifest_id: &ipc::ManifestId, number: u64) {
        let Ok(served) = self.served_named(Some(manifest_id)) else { return };
        let limit = usize::try_from(self.machine_settings().now().get(keys::QUEUE_REQUEUES_PER_COMMIT)).unwrap_or(0);
        let mut store = self.store().lock().await;
        for time in 1..=limit {
            let _ = store.forget_pull_notices(served.root(), number, &format!("requeue-{time}"));
        }
    }

    /// A pull request left the queue: **put it back, quietly, where the forge says it was a runner
    /// or the time that failed it**, at most `limits.queueRequeuesPerCommit` times a commit. A
    /// drop past that is stuck. Anything else is the owners' to hear, as ever.
    pub(crate) async fn drop_judged(&self, served: &Served, pull: &adapter_traits::FollowedPull, head: &str) -> DropSaid {
        let Some(base) = served.manifest().base().map(str::to_string) else { return DropSaid::Tell };
        let (root, number) = (served.root().to_string(), pull.number);
        let Some(tested) = self.forge_asked(&root, move |vcs: &V, root: &str| vcs.queue_commit_of(root, &base, number)).await else {
            return DropSaid::Tell;
        };
        let failures = self.forge_asked(&root, move |vcs: &V, root: &str| vcs.infra_failures(root, &tested)).await;
        let Some(found) = failures.as_deref().and_then(infrastructure_of) else { return DropSaid::Tell };
        let limit = usize::try_from(self.machine_settings().now().get(keys::QUEUE_REQUEUES_PER_COMMIT)).unwrap_or(0);
        let manifest = ipc::ManifestId::carried(served.manifest().id().as_str());
        for time in 1..=limit {
            let key = format!("requeue-{time}");
            match self.store().lock().await.pull_notice_kept(&root, number, head, &key, "fleet") {
                Ok(true) => continue,
                Ok(false) => {}
                Err(_) => return DropSaid::Tell,
            }
            return match self.auto_merge_asked_for(&manifest, number, Some(head)).await {
                Ok(None) => {
                    let now = self.now();
                    let _ = self.store().lock().await.keep_pull_notice(&root, number, head, &key, "fleet", &now);
                    DropSaid::Quiet
                }
                _ => DropSaid::Tell,
            };
        }
        let now = self.now();
        self.sweeping().lock().await.queue_watch.entry(root).or_default().declared(&found, number, &now);
        self.stuck_called(served).await;
        DropSaid::Quiet
    }
}
