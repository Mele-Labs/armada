//! `armada land --withdraw [branch]` — take a branch out of the line, and
//! the runner's half of it: a turn holds its split halves in memory, so it
//! asks the queue again before gating each one.

use std::path::Path;

use super::dir::StateDir;
use super::outcome::{read_outcome, OutcomePatch, OutcomeState};
use super::queue::{read_queue_entry, QueueEntry};
use super::repo::current_branch;
use super::say::say;
use super::stop::Refused;

const WITHDRAWN: &str = "withdrawn with `armada land --withdraw`; nothing was merged";

pub enum Withdrawn {
    NotInLine(String),
    Waiting(String),
    /// In a turn now. A gate it is already in still finishes, and can land it.
    InTurn(String),
}

pub fn withdraw(cwd: &Path, branch: Option<&str>) -> Result<Withdrawn, Refused> {
    let state = StateDir::resolve(cwd).map_err(|why| Refused(why.to_string()))?;
    let Some(branch) = branch.map(str::to_string).or_else(|| current_branch(cwd)) else {
        return Err(Refused(
            "no branch is checked out here — name the one to withdraw".to_string(),
        ));
    };
    let refused = |why: &dyn std::fmt::Display| Refused(why.to_string());
    if read_queue_entry(&state, &branch)
        .map_err(|why| refused(&why))?
        .is_none()
    {
        return Ok(Withdrawn::NotInLine(branch));
    }
    std::fs::remove_file(state.queue_entry_path(&branch)).map_err(|why| refused(&why))?;
    let held = read_outcome(&state, &branch).map_err(|why| refused(&why))?;
    if held.is_some_and(|held| matches!(held.state, OutcomeState::Gating | OutcomeState::Merging)) {
        return Ok(Withdrawn::InTurn(branch));
    }
    say(
        &state,
        &branch,
        OutcomeState::Stopped,
        WITHDRAWN,
        OutcomePatch::default(),
    )
    .map_err(|stopped| Refused(stopped.detail))?;
    Ok(Withdrawn::Waiting(branch))
}

/// Whether `entry` is still the one in line: not withdrawn, and not
/// resubmitted under a fresh nonce, which a later turn takes instead.
pub fn still_in_line(state: &StateDir, entry: &QueueEntry) -> bool {
    match read_queue_entry(state, &entry.branch) {
        Ok(Some(current)) if current.nonce == entry.nonce => true,
        Ok(Some(_)) => {
            let _ = say(
                state,
                &entry.branch,
                OutcomeState::Waiting,
                "in line",
                OutcomePatch::default(),
            );
            false
        }
        Ok(None) => {
            let _ = say(
                state,
                &entry.branch,
                OutcomeState::Stopped,
                WITHDRAWN,
                OutcomePatch::default(),
            );
            false
        }
        // An entry that will not read is not evidence it was withdrawn.
        Err(_) => true,
    }
}
