//! One turn: a batch of waiting branches built into one candidate, gated
//! once, and pushed onto the base — split in half on a red or a clash between
//! members, unless a new gate line names whose it is, and retried up to
//! [`ROUNDS`] times against a base that keeps moving. `scripts/land`'s own
//! `take_turn`.

use std::collections::VecDeque;
use std::path::Path;

use super::batch::{build, halves, tell, NotBuilt};
use super::blame::blame;
use super::dir::StateDir;
use super::env::{Env, ROUNDS};
use super::gating::{checks, foundations, foundations_red, Read};
use super::git::checked;
use super::logs::{prune, turn_logs, KEPT_FOR};
use super::onto_main::{already_landed, landed, local_head, push, Pushed};
use super::outcome::{read_outcome, OutcomePatch, OutcomeState};
use super::queue::{read_queue_entry, QueueEntry};
use super::repo::{is_ancestor, remote_head};
use super::say::say;
use super::size::Went;
use super::stop::Stopped;
use super::withdraw::still_in_line;

/// What a turn's groups ended as, for the size of the next turn.
#[derive(Default)]
struct Seen {
    red: bool,
    landed: bool,
}

/// Take `entries`, in place order, and end every one of their turns. `None`
/// is a turn that neither went red nor landed anything.
pub fn take_turn(repo: &Path, state: &StateDir, env: &Env, entries: &[QueueEntry]) -> Option<Went> {
    prune(&state.path().join("logs"), KEPT_FOR);
    let mut seen = Seen::default();
    let mut pending = VecDeque::from([entries.to_vec()]);
    while let Some(group) = pending.pop_front() {
        if let Some((first, second)) = land_group(repo, state, env, &mut seen, group) {
            pending.push_front(second);
            pending.push_front(first);
        }
    }
    match seen {
        Seen { red: true, .. } => Some(Went::Red),
        Seen { landed: true, .. } => Some(Went::Green),
        _ => None,
    }
}

/// Say how one entry's turn ended, and take it out of the line — only if its
/// nonce still matches: a `land` that resubmitted the branch while its turn
/// ran wrote a fresh entry this must not delete.
fn finish(state: &StateDir, seen: &mut Seen, entry: &QueueEntry, stopped: Stopped) {
    seen.red |= stopped.state == OutcomeState::Red;
    seen.landed |= stopped.state == OutcomeState::Landed;
    let _ = say(
        state,
        &entry.branch,
        stopped.state,
        stopped.detail,
        stopped.patch,
    );
    if let Ok(Some(current)) = read_queue_entry(state, &entry.branch) {
        if current.nonce == entry.nonce {
            let _ = std::fs::remove_file(state.queue_entry_path(&entry.branch));
        }
    }
}

/// End every member's turn the same way — unless it is a red on more than one
/// member, which is split to find whose it is. `Some` is the two halves.
fn end_all(
    state: &StateDir,
    seen: &mut Seen,
    group: Vec<QueueEntry>,
    stopped: Stopped,
) -> Option<(Vec<QueueEntry>, Vec<QueueEntry>)> {
    if stopped.state == OutcomeState::Red && group.len() > 1 {
        seen.red = true;
        let _ = tell(
            state,
            &group,
            OutcomeState::Gating,
            "red together; splitting the batch to find which",
            OutcomePatch::default(),
        );
        return Some(halves(group));
    }
    for entry in &group {
        finish(state, seen, entry, stopped.clone());
    }
    None
}

/// Report each member its own gate lines, and keep the rest to gate again
/// at once. Each red keeps a copy of the gate's log in a turn of its own,
/// where its own outcome points.
fn red_alone(
    state: &StateDir,
    seen: &mut Seen,
    env: &Env,
    moved: bool,
    logs: &Path,
    group: Vec<QueueEntry>,
    shares: Vec<Vec<String>>,
) -> Vec<QueueEntry> {
    let mut rest = Vec::new();
    for (entry, lines) in group.into_iter().zip(shares) {
        if lines.is_empty() {
            rest.push(entry);
            continue;
        }
        let log = match turn_logs(state, std::slice::from_ref(&entry)) {
            Ok(own) => {
                let log = own.join("foundations.log");
                let _ = std::fs::copy(logs.join("foundations.log"), &log);
                log
            }
            Err(_) => logs.join("foundations.log"),
        };
        finish(
            state,
            seen,
            &entry,
            foundations_red(env, moved, lines, &log),
        );
    }
    rest
}

fn land_group(
    repo: &Path,
    state: &StateDir,
    env: &Env,
    seen: &mut Seen,
    mut group: Vec<QueueEntry>,
) -> Option<(Vec<QueueEntry>, Vec<QueueEntry>)> {
    // What a killed runner's turn recorded, read before this turn says anything.
    let recorded: Vec<(String, Option<String>)> = group
        .iter()
        .map(|entry| {
            let merge = read_outcome(state, &entry.branch)
                .ok()
                .flatten()
                .and_then(|outcome| outcome.merge_commit);
            (entry.branch.clone(), merge)
        })
        .collect();
    let recorded_for = |branch: &str| {
        recorded
            .iter()
            .find(|(name, _)| name == branch)
            .and_then(|(_, merge)| merge.clone())
    };
    if let Err(stopped) = tell(
        state,
        &group,
        OutcomeState::Gating,
        "reading the branch",
        OutcomePatch {
            logs: Some(Vec::new()),
            failed: Some(Vec::new()),
            already: Some(Vec::new()),
            new_lines: Some(Vec::new()),
            conflicts: Some(Vec::new()),
            checks: Some(Vec::new()),
            ..OutcomePatch::default()
        },
    ) {
        return end_all(state, seen, group, stopped);
    }

    let mut rounds = 0;
    while rounds < ROUNDS {
        group.retain(|entry| still_in_line(state, entry));
        group.retain(|entry| {
            let kept = local_head(repo, &entry.branch).as_deref() == Some(entry.head.as_str());
            if !kept {
                let detail = format!(
                    "{} moved since it was queued — preflight and land again",
                    entry.branch
                );
                finish(state, seen, entry, Stopped::stopped(detail));
            }
            kept
        });
        if group.is_empty() {
            return None;
        }
        if let Err(why) = checked(repo, &["fetch", "--quiet", &env.remote, &env.base]) {
            return end_all(state, seen, group, why.into());
        }
        let base = match remote_head(repo, &env.remote, &env.base) {
            Ok(Some(base)) => base,
            Ok(None) => {
                let detail = format!("{}/{} does not exist", env.remote, env.base);
                return end_all(state, seen, group, Stopped::stopped(detail));
            }
            Err(why) => return end_all(state, seen, group, Stopped::stopped(why.to_string())),
        };
        // A runner killed after its push left these queued.
        group.retain(|entry| {
            let on_base = is_ancestor(repo, &entry.head, &base);
            if on_base {
                let merge = already_landed(repo, recorded_for(&entry.branch).as_deref(), &base);
                finish(
                    state,
                    seen,
                    entry,
                    landed(repo, state, env, entry, &base, &merge),
                );
            }
            !on_base
        });
        if group.is_empty() {
            return None;
        }

        // A directory per gate, so a regate in this turn keeps the last one's.
        let logs = match turn_logs(state, &group) {
            Ok(logs) => logs,
            Err(why) => {
                let detail = format!("a log directory could not be created: {why}");
                return end_all(state, seen, group, Stopped::stopped(detail));
            }
        };
        let built = match build(repo, state, env, &group, &base, &logs) {
            Ok(built) => built,
            Err(NotBuilt::Conflict(index, stopped)) => {
                let entry = group.remove(index);
                finish(state, seen, &entry, stopped);
                if group.is_empty() {
                    return None;
                }
                continue;
            }
            Err(NotBuilt::Between) => return Some(halves(group)),
            Err(NotBuilt::Stopped(stopped)) => return end_all(state, seen, group, stopped),
        };
        let passed = match foundations(repo, state, env, &group, &base, &built, &logs) {
            Ok(Read::Passed(passed)) => passed,
            Ok(Read::New(lines)) => {
                let shares = (group.len() > 1)
                    .then(|| blame(&lines, &built.own))
                    .flatten();
                let Some(shares) = shares else {
                    let red =
                        foundations_red(env, built.moved, lines, &logs.join("foundations.log"));
                    return end_all(state, seen, group, red);
                };
                group = red_alone(state, seen, env, built.moved, &logs, group, shares);
                if group.is_empty() {
                    return None;
                }
                continue;
            }
            Err(stopped) => return end_all(state, seen, group, stopped),
        };
        let narrowed = match checks(repo, state, env, &group, &base, &built, &logs, passed) {
            Ok(narrowed) => narrowed,
            Err(stopped) => return end_all(state, seen, group, stopped),
        };
        let before = group.len();
        group.retain(|entry| {
            let kept = local_head(repo, &entry.branch).as_deref() == Some(entry.head.as_str());
            if !kept {
                let detail = format!(
                    "{} moved while it was gated — preflight and land again",
                    entry.branch
                );
                finish(state, seen, entry, Stopped::stopped(detail));
            }
            kept
        });
        if group.len() != before {
            rounds += 1;
            continue;
        }

        for (entry, merge) in group.iter().zip(&built.merges) {
            let said = say(
                state,
                &entry.branch,
                OutcomeState::Merging,
                format!(
                    "pushing {} onto {} at {}",
                    short(&built.top),
                    env.base,
                    short(&base)
                ),
                OutcomePatch {
                    gated_base: Some(base.clone()),
                    candidate: Some(built.top.clone()),
                    merge_commit: Some(merge.clone()),
                    ..OutcomePatch::default()
                },
            );
            if let Err(stopped) = said {
                return end_all(state, seen, group, stopped);
            }
        }
        match push(repo, env, &built.top, &base, &logs.join("merge.log")) {
            Ok(Pushed::Landed) => {
                for (entry, merge) in group.iter().zip(&built.merges) {
                    let mut done = landed(repo, state, env, entry, &base, merge);
                    done.detail.push_str(&narrowed);
                    finish(state, seen, entry, done);
                }
                return None;
            }
            Ok(Pushed::Moved) => rounds += 1,
            Err(stopped) => return end_all(state, seen, group, stopped),
        }
    }
    let detail = format!(
        "{} moved during each of {ROUNDS} gates; land again when it is quieter",
        env.base
    );
    end_all(state, seen, group, Stopped::stopped(detail))
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}
