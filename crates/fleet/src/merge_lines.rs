//! The merge line `armada land` keeps in each repository Fleet serves, read off
//! disk and put on the wire. `docs/capabilities/merge-line.md`, *In Bridge*.
//!
//! **Read, never run.** The runner is `armada land`, another process; Fleet
//! reads its files through `adapters::land_state::line`, which starts nothing.
//! Nothing tells Fleet when that process writes, so [`keep_reading`] reads every
//! [`EVERY`] and publishes `merge_lines.changed` only when a line moved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapters::land_state::line::{self, Line, Located};
use adapters::land_state::outcome::{together, Outcome, OutcomeState};
use api::{Broadcaster, Queries};
use ipc::{Event, LandState, MergeLine, MergeLineEntry, MergeLinePullRequest, MergeLines};
use tokio::task::JoinHandle;

/// How often the lines are read. **Per repository with a line: one listing of
/// the queue, one of the outcomes and a few small files**, and the `git` calls
/// that find them are made once per repository, not once a read. Two seconds is
/// well inside a turn's own pace, which is minutes.
pub const EVERY: Duration = Duration::from_secs(2);

/// Where each repository's line is, by root. `None` is a root `git` would not
/// answer for, remembered so it is not asked again every read.
type Found = HashMap<String, Option<Located>>;

/// Every root's line, read now, and a sentence for each line that would not
/// read. A root with no line, or one that will not read, is left out.
pub fn read(roots: &[String], found: &mut Found) -> (MergeLines, Vec<String>) {
    let mut lines = Vec::new();
    let mut unread = Vec::new();
    for root in roots {
        let at = found
            .entry(root.clone())
            .or_insert_with(|| line::locate(Path::new(root)));
        let Some(at) = at.as_ref() else { continue };
        match line::read(at) {
            Ok(Some(read)) => lines.push(merge_line(root, at, read)),
            Ok(None) => {}
            Err(why) => unread.push(format!("the merge line in {root} could not be read: {why}")),
        }
    }
    (MergeLines { lines }, unread)
}

/// The served roots, Manifest or none, in the order they were added.
async fn roots<D: Queries>(daemon: &D) -> Vec<String> {
    daemon
        .list_repositories()
        .await
        .map(|list| list.repositories.into_iter().map(|one| one.root).collect())
        .unwrap_or_default()
}

/// `get_merge_lines`: read now, off the async workers.
pub async fn answer<D: Queries>(daemon: &D) -> MergeLines {
    let roots = roots(daemon).await;
    // What would not read is said by the reading loop, once, rather than per request.
    tokio::task::spawn_blocking(move || read(&roots, &mut Found::new()).0)
        .await
        .unwrap_or_default()
}

/// Read every [`EVERY`] and publish `merge_lines.changed` when the lines moved.
///
/// **What it costs the event stream is bounded by the files, not by Fleet**: at
/// most one event a read, and only on a change, so a quiet line publishes
/// nothing. It adds no queue of its own; the shared drop-oldest backlog is
/// still the only one.
///
/// A line that will not read is handed to `unread` when it first fails, not
/// every read: the composition root is what writes to the console.
pub fn keep_reading<D>(
    daemon: Arc<D>,
    events: Broadcaster,
    every: Duration,
    unread: impl Fn(&str) + Send + 'static,
) -> JoinHandle<()>
where
    D: Queries + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut found = Found::new();
        let mut last: Option<MergeLines> = None;
        let mut failing: Vec<String> = Vec::new();
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let roots = roots(daemon.as_ref()).await;
            let Ok(((now, said), kept)) = tokio::task::spawn_blocking(move || {
                let now = read(&roots, &mut found);
                (now, found)
            })
            .await
            else {
                return;
            };
            found = kept;
            said.iter()
                .filter(|one| !failing.contains(one))
                .for_each(|one| unread(one));
            failing = said;
            // The first read sets the baseline: a client reads the route on connect.
            if last.as_ref().is_some_and(|last| *last != now) {
                events.publish(Event::MergeLinesChanged(now.clone()));
            }
            last = Some(now);
        }
    })
}

/// One repository's line, as the wire carries it.
///
/// **The redaction is here**: a queue entry's head, tree, worktree and nonce,
/// and an outcome's logs, cleanup commands, runner pid and base readings stay on
/// disk. What crosses is what `--status` prints and the panel draws.
pub fn merge_line(root: &str, at: &Located, read: Line) -> MergeLine {
    let order: Vec<&str> = read
        .waiting
        .iter()
        .map(|(entry, _)| entry.branch.as_str())
        .collect();
    let line = read
        .waiting
        .iter()
        .enumerate()
        .map(|(n, (entry, outcome))| {
            let state = outcome
                .as_ref()
                .map_or(OutcomeState::Waiting, |held| held.state);
            let live = matches!(state, OutcomeState::Gating | OutcomeState::Merging);
            let (doing, others) = outcome
                .as_ref()
                .filter(|_| live)
                .map(|held| together(&held.detail))
                .unwrap_or_default();
            let batch = (!others.is_empty()).then(|| {
                // The member first in place order names the batch, so every member says the same.
                std::iter::once(entry.branch.as_str())
                    .chain(others)
                    .min_by_key(|branch| {
                        order
                            .iter()
                            .position(|one| one == branch)
                            .unwrap_or(usize::MAX)
                    })
                    .unwrap_or(entry.branch.as_str())
                    .to_string()
            });
            let mut row = ended(
                at,
                &entry.branch,
                state,
                entry.pr.or(outcome.as_ref().and_then(|o| o.pr)),
                outcome.as_ref(),
            );
            row.place = Some(u32::try_from(n + 1).unwrap_or(u32::MAX));
            row.doing = (live && !doing.is_empty()).then(|| doing.to_string());
            row.batch = batch;
            row
        })
        .collect();
    let off = read
        .off
        .iter()
        .map(|outcome| {
            ended(
                at,
                &outcome.branch,
                outcome.state,
                outcome.pr,
                Some(outcome),
            )
        })
        .collect();
    MergeLine {
        root: root.to_string(),
        line,
        off,
    }
}

/// A row with the facts an outcome carries for its own state, and only those:
/// an outcome keeps fields from earlier turns, so a red after a landing still
/// holds the old merge commit.
fn ended(
    at: &Located,
    branch: &str,
    state: OutcomeState,
    pr: Option<u64>,
    outcome: Option<&Outcome>,
) -> MergeLineEntry {
    let facts = |pick: fn(&Outcome) -> &Vec<String>, when: bool| {
        outcome
            .filter(|_| when)
            .map(|held| pick(held).clone())
            .unwrap_or_default()
    };
    MergeLineEntry {
        branch: branch.to_string(),
        place: None,
        pull_request: pr.and_then(|number| {
            at.pull_request(number)
                .map(|url| MergeLinePullRequest { number, url })
        }),
        state: land_state(state),
        doing: None,
        batch: None,
        merge_commit: outcome
            .filter(|_| state == OutcomeState::Landed)
            .and_then(|held| held.merge_commit.clone()),
        failed: facts(
            |held| &held.failed,
            matches!(state, OutcomeState::Red | OutcomeState::Stopped),
        ),
        conflicts: facts(|held| &held.conflicts, state == OutcomeState::Conflict),
    }
}

fn land_state(state: OutcomeState) -> LandState {
    match state {
        OutcomeState::Waiting => LandState::Waiting,
        OutcomeState::Gating => LandState::Gating,
        OutcomeState::Merging => LandState::Merging,
        OutcomeState::Landed => LandState::Landed,
        OutcomeState::Red => LandState::Red,
        OutcomeState::Conflict => LandState::Conflict,
        OutcomeState::Stopped => LandState::Stopped,
    }
}
