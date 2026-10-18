//! The merge line of each repository Fleet serves, as the wire carries it: the
//! hub Fleet reads off the forge, folded in beside each root.
//! `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *In Bridge*.
//!
//! **No queue is read from disk any more.** The local runner is gone, so every
//! line is empty and the hub is the whole answer. [`keep_reading`] publishes
//! `merge_lines.changed` when the hubs moved.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use api::{Broadcaster, Queries};
use ipc::{Event, MergeLine, MergeLineHub, MergeLines};
use tokio::task::JoinHandle;

use crate::timeline::{Between, Timeline};

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

/// The served roots, Manifest or none, in the order they were added.
pub(crate) async fn roots<D: Queries>(daemon: &D) -> Vec<String> {
    daemon
        .list_repositories()
        .await
        .map(|list| list.repositories.into_iter().map(|one| one.root).collect())
        .unwrap_or_default()
}

/// Each served root's Manifest id, for the entries the timeline keeps against a repository.
async fn manifests_of<D: Queries>(daemon: &D) -> HashMap<String, String> {
    daemon
        .list_repositories()
        .await
        .map(|list| {
            list.repositories
                .into_iter()
                .filter_map(|one| Some((one.root, one.manifest?.id.as_str().to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// `get_merge_lines`: every root with the hub Fleet read for it.
pub async fn answer<D: Queries>(daemon: &D) -> MergeLines {
    let roots = roots(daemon).await;
    let hubs = daemon.merge_hubs().await;
    with_hubs(MergeLines::default(), &roots, hubs)
}

/// Fold the hubs every [`EVERY`] and publish `merge_lines.changed` when they moved.
///
/// **At most one event a read, and only on a change.** Every forge visit moves
/// `forge_read_at`, so a forge that answers publishes about once a visit, one
/// repository a minute; a Bridge that did not hear it would show a read time
/// older than the reading. It adds no queue of its own; the shared drop-oldest
/// backlog is still the only one.
///
/// **What a change was is entered on the timeline**: the list is carried whole,
/// so a pull request opening, its checks, its place in the queue, its merge and
/// main's colour are read off the difference here, where the read before it is.
pub fn keep_reading<D>(
    daemon: Arc<D>,
    events: Broadcaster,
    timeline: Timeline,
    every: Duration,
) -> JoinHandle<()>
where
    D: Queries + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut last: Option<MergeLines> = None;
        let mut between = Between::default();
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let now = answer(daemon.as_ref()).await;
            // The first read sets the baseline: a client reads the route on connect.
            if last.as_ref().is_none_or(|last| *last != now) {
                let manifests = manifests_of(daemon.as_ref()).await;
                let at = timeline.now();
                let named = |root: &str| manifests.get(root).cloned();
                for fact in between.read(&now, &at, &named) {
                    timeline.note(fact);
                }
            }
            if last.as_ref().is_some_and(|last| *last != now) {
                events.publish(Event::MergeLinesChanged(now.clone()));
            }
            last = Some(now);
        }
    })
}
