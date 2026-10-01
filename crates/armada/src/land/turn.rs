//! One branch's turn: gate the commit it was queued at, merge it onto the
//! base and push — retried up to [`ROUNDS`] times against a base that keeps
//! moving. `scripts/land`'s own `take_turn`.

use std::path::Path;

use super::dir::StateDir;
use super::env::{Env, ROUNDS};
use super::gating::gate;
use super::git::checked;
use super::onto_main::{already_landed, landed, local_head, merge_commit, message, push, Pushed};
use super::outcome::OutcomePatch;
use super::outcome::OutcomeState;
use super::queue::QueueEntry;
use super::repo::{is_ancestor, remote_head};
use super::say::say;
use super::stop::Stopped;

pub fn take_turn(repo: &Path, state: &StateDir, env: &Env, entry: &QueueEntry) -> Stopped {
    let branch = entry.branch.as_str();
    let logs = state.path().join("logs").join(super::dir::key(branch));
    let recorded = super::outcome::read_outcome(state, branch)
        .ok()
        .flatten()
        .and_then(|outcome| outcome.merge_commit);
    let _ = std::fs::remove_dir_all(&logs);
    if let Err(why) = std::fs::create_dir_all(&logs) {
        return Stopped::stopped(format!("{} could not be created: {why}", logs.display()));
    }

    if let Err(stopped) = say(
        state,
        branch,
        OutcomeState::Gating,
        "reading the branch",
        OutcomePatch {
            logs: Some(Vec::new()),
            failed: Some(Vec::new()),
            already: Some(Vec::new()),
            new_lines: Some(Vec::new()),
            conflicts: Some(Vec::new()),
            ..OutcomePatch::default()
        },
    ) {
        return stopped;
    }

    let head = entry.head.as_str();
    for _ in 0..ROUNDS {
        if local_head(repo, branch).as_deref() != Some(head) {
            return Stopped::stopped(format!(
                "{branch} moved since it was queued — preflight and land again"
            ));
        }
        if let Err(why) = checked(repo, &["fetch", "--quiet", &env.remote, &env.base]) {
            return why.into();
        }
        let base = match remote_head(repo, &env.remote, &env.base) {
            Ok(Some(base)) => base,
            Ok(None) => {
                return Stopped::stopped(format!("{}/{} does not exist", env.remote, env.base))
            }
            Err(why) => return Stopped::stopped(why.to_string()),
        };
        // A runner killed after its push left the entry queued.
        if is_ancestor(repo, head, &base) {
            let merge = already_landed(repo, recorded.as_deref(), &base);
            return landed(repo, state, env, entry, &base, &merge);
        }

        let moved = !is_ancestor(repo, &base, head);
        let candidate = match gate(repo, state, env, entry, head, &base, &logs, moved) {
            Ok(candidate) => candidate,
            Err(stopped) => return stopped,
        };
        if local_head(repo, branch).as_deref() != Some(head) {
            return Stopped::stopped(format!(
                "{branch} moved while it was gated — preflight and land again"
            ));
        }
        let merge = match merge_commit(repo, &base, &candidate, &message(branch, entry.pr)) {
            Ok(merge) => merge,
            Err(stopped) => return stopped,
        };

        if let Err(stopped) = say(
            state,
            branch,
            OutcomeState::Merging,
            format!(
                "pushing {} onto {} at {}",
                short(&merge),
                env.base,
                short(&base)
            ),
            OutcomePatch {
                gated_base: Some(base.clone()),
                candidate: Some(candidate.clone()),
                merge_commit: Some(merge.clone()),
                ..OutcomePatch::default()
            },
        ) {
            return stopped;
        }

        match push(repo, env, &merge, &base, &logs.join("merge.log")) {
            Ok(Pushed::Landed) => return landed(repo, state, env, entry, &base, &merge),
            Ok(Pushed::Moved) => continue,
            Err(stopped) => return stopped,
        }
    }
    Stopped::stopped(format!(
        "{} moved during each of {ROUNDS} gates; land again when it is quieter",
        env.base
    ))
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}
