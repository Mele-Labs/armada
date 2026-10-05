//! A branch with a need waits in line until every need ahead of it on the same
//! path is spent or given back. `crate::need` keeps the needs; this is where
//! the line reads them. Spending happens in [`super::turn`], when a branch lands.

use crate::need::{waiting_text, Needs};

use super::dir::StateDir;
use super::outcome::{read_outcome, OutcomePatch, OutcomeState};
use super::queue::QueueEntry;
use super::say::say;

/// What `branch` waits behind, or `None` where nothing is ahead of it.
pub fn waiting_behind(state: &StateDir, branch: &str) -> Option<String> {
    let behind = Needs::beside(state).behind(branch);
    (!behind.is_empty()).then(|| waiting_text(&behind))
}

/// The entries a turn may take. A held one stays queued, and says what it
/// waits behind.
pub fn unheld(state: &StateDir, line: Vec<QueueEntry>) -> Vec<QueueEntry> {
    let mut free = Vec::new();
    for entry in line {
        match waiting_behind(state, &entry.branch) {
            None => free.push(entry),
            Some(detail) => {
                let said = read_outcome(state, &entry.branch)
                    .ok()
                    .flatten()
                    .is_some_and(|held| held.detail == detail);
                if !said {
                    let _ = say(
                        state,
                        &entry.branch,
                        OutcomeState::Waiting,
                        detail,
                        OutcomePatch::default(),
                    );
                }
            }
        }
    }
    free
}
