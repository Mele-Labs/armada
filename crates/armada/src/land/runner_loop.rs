//! The detached runner's own loop: hold the turn, sweep what an older
//! script left, then take a turn — the first [`size::current`] entries in
//! line — until the line is empty. `scripts/land`'s own `runner`.

use std::path::Path;
use std::process::ExitCode;

use super::dir::StateDir;
use super::env::Env;
use super::lock::TurnLock;
use super::queue::queued;
use super::size;
use super::turn::take_turn;
use super::worktree::{drop_worktree, main_tree};

pub fn run_runner(common_git_dir: &Path, env: &Env) -> ExitCode {
    let Ok(state) = StateDir::at_common(common_git_dir) else {
        return ExitCode::FAILURE;
    };
    let repo = common_git_dir;

    let mut lock = match TurnLock::try_acquire(&state) {
        Ok(Some(lock)) => lock,
        Ok(None) => return ExitCode::SUCCESS,
        Err(_) => return ExitCode::FAILURE,
    };

    sweep_stale_gates(repo);

    loop {
        let line = match queued(&state) {
            Ok(line) => line,
            Err(_) => return ExitCode::FAILURE,
        };
        if line.is_empty() {
            // Release, then look once more: an entry written after the
            // last look either shows up now, or found the lock free and
            // started a runner of its own.
            drop(lock);
            match queued(&state) {
                Ok(line) if line.is_empty() => return ExitCode::SUCCESS,
                Err(_) => return ExitCode::FAILURE,
                _ => {}
            }
            lock = match TurnLock::try_acquire(&state) {
                Ok(Some(lock)) => lock,
                _ => return ExitCode::SUCCESS,
            };
            continue;
        }

        let batch = &line[..line.len().min(size::current(&state, env.batch))];
        if let Some(went) = take_turn(repo, &state, env, batch) {
            size::after(&state, env.batch, went);
        }
    }
}

/// What an older version of this line left behind, taken back on the first
/// turn that holds the lock — only what this line made under
/// `.armada/gates/`, the name this used before settling on `.armada/land/`.
fn sweep_stale_gates(repo: &Path) {
    let Ok(main) = main_tree(repo) else {
        return;
    };
    let stale = main.join(".armada").join("gates");
    let Ok(entries) = std::fs::read_dir(&stale) else {
        return;
    };
    let mut names: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    names.sort();
    for path in names {
        drop_worktree(repo, &path);
    }
    if std::fs::read_dir(&stale).is_ok_and(|mut left| left.next().is_none()) {
        let _ = std::fs::remove_dir(&stale);
    }
}
