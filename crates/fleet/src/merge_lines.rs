//! The merge line of each repository Fleet serves, as the wire carries it: the
//! hub Fleet reads off the forge, folded in beside each root.
//! `docs/capabilities/merge-line.md`, *In Bridge*.
//!
//! **No queue is read from disk any more.** The local runner is gone, so every
//! line is empty and the hub is the whole answer. [`keep_reading`] publishes
//! `merge_lines.changed` when the hubs moved.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use api::{Broadcaster, Queries};
use ipc::{Event, LandCheckState, LandState, MergeLine, MergeLineCheck, MergeLineHub, MergeLines};
use tokio::task::JoinHandle;

/// How often the hubs are folded and compared.
pub const EVERY: Duration = Duration::from_secs(2);

/// Each root's line with the hub Fleet read for it, in the order the roots were
/// added. **A root with a hub is here with an empty line**: main's state is
/// still worth drawing. A root with no hub is left out.
pub fn with_hubs(
    lines: MergeLines,
    roots: &[String],
    mut hubs: Vec<(String, MergeLineHub)>,
) -> MergeLines {
    let mut lines: HashMap<String, MergeLine> = lines
        .lines
        .into_iter()
        .map(|line| (line.root.clone(), line))
        .collect();
    let mut folded = Vec::new();
    for root in roots {
        let hub = hubs
            .iter()
            .position(|(held, _)| held == root)
            .map(|at| hubs.swap_remove(at).1);
        match (lines.remove(root), hub) {
            (None, None) => {}
            (Some(line), hub) => folded.push(MergeLine { hub, ..line }),
            (None, Some(hub)) => folded.push(MergeLine {
                root: root.clone(),
                line: Vec::new(),
                off: Vec::new(),
                landed: Vec::new(),
                sent_back: Vec::new(),
                hub: Some(hub),
            }),
        }
    }
    MergeLines { lines: folded }
}

/// A Job's branch, id and handle: what names the Job a branch on the line is.
pub(crate) type JobBranch = (String, ipc::JobId, String);

/// Every Job's branch, for [`naming_jobs`]. A Job with no worktree has none.
pub(crate) async fn job_branches<D: Queries>(daemon: &D) -> Vec<JobBranch> {
    daemon
        .list_jobs(None)
        .await
        .map(|list| {
            list.jobs
                .into_iter()
                .filter_map(|job| Some((job.branch?, job.id, job.handle)))
                .collect()
        })
        .unwrap_or_default()
}

/// Each Check's requester on a branch some Job owns, carrying that Job's id and
/// handle, so a surface can narrow the line's Checks to the Job whose they are.
/// A branch no Job owns stays as it was.
///
/// **A branch queued and not yet gating lists the repository's declared Checks as
/// waiting**, since the line writes none until it gates the branch, and a Job's
/// Waiting filter would otherwise show nothing while the Jobs ahead land. Once
/// the line gates it, its own rows replace these.
pub(crate) fn naming_jobs(
    mut lines: MergeLines,
    jobs: &[JobBranch],
    declared: &[(String, Vec<String>)],
) -> MergeLines {
    for line in &mut lines.lines {
        let queued = declared
            .iter()
            .find(|(root, _)| *root == line.root)
            .map(|(_, names)| names.as_slice())
            .unwrap_or_default();
        for row in line
            .line
            .iter_mut()
            .chain(&mut line.sent_back)
            .chain(&mut line.landed)
            .chain(&mut line.off)
        {
            let Some((_, id, handle)) = jobs.iter().find(|(branch, ..)| *branch == row.branch)
            else {
                continue;
            };
            if row.state == LandState::Waiting && row.checks.is_empty() {
                row.checks = queued
                    .iter()
                    .map(|name| MergeLineCheck {
                        name: name.clone(),
                        requester: ipc::Requester::merge_line(&row.branch),
                        started_at: None,
                        state: LandCheckState::Waiting,
                    })
                    .collect();
            }
            for check in &mut row.checks {
                check.requester.job_id = Some(id.clone());
                check.requester.handle = Some(handle.clone());
            }
        }
    }
    lines
}

/// The served roots, Manifest or none, in the order they were added.
pub(crate) async fn roots<D: Queries>(daemon: &D) -> Vec<String> {
    daemon
        .list_repositories()
        .await
        .map(|list| list.repositories.into_iter().map(|one| one.root).collect())
        .unwrap_or_default()
}

/// `get_merge_lines`: every root with the hub Fleet read for it.
pub async fn answer<D: Queries>(daemon: &D) -> MergeLines {
    let roots = roots(daemon).await;
    let hubs = daemon.merge_hubs().await;
    let jobs = job_branches(daemon).await;
    let declared = daemon.land_checks().await;
    naming_jobs(with_hubs(MergeLines::default(), &roots, hubs), &jobs, &declared)
}

/// Fold the hubs every [`EVERY`] and publish `merge_lines.changed` when they moved.
///
/// **At most one event a read, and only on a change**, so a quiet forge
/// publishes nothing. It adds no queue of its own; the shared drop-oldest
/// backlog is still the only one.
pub fn keep_reading<D>(daemon: Arc<D>, events: Broadcaster, every: Duration) -> JoinHandle<()>
where
    D: Queries + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut last: Option<MergeLines> = None;
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let now = answer(daemon.as_ref()).await;
            // The first read sets the baseline: a client reads the route on connect.
            if last.as_ref().is_some_and(|last| *last != now) {
                events.publish(Event::MergeLinesChanged(now.clone()));
            }
            last = Some(now);
        }
    })
}
