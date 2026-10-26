//! Why a run was not started, stopped, drafted or saved, and how each is spelled on the wire.
//! **Each is checked before anything is spawned or written.**

use std::fmt;

use api::Refusal;
use ipc::WireError;

/// A name that declares no run in this repository's Manifest.
const NOT_AN_APP_RUN: &str = "fleet.not_an_app_run";
/// A Job Fleet does not hold, so there is no worktree to run.
const NO_JOB_TO_RUN: &str = "fleet.no_job_to_run";
/// A Job that has not leased a worktree yet, or has given it back.
const JOB_HAS_NO_WORKTREE: &str = "fleet.job_has_no_worktree";
/// A draft asked for while the Manifest already declares a run.
const APP_RUNS_ALREADY_SAVED: &str = "fleet.app_runs_already_saved";
/// A draft that would not load as part of the Manifest: the sentence says which line.
const APP_RUN_DRAFT_REFUSED: &str = "fleet.app_run_draft_refused";
/// A run that could not be written to the file or the place that keeps it.
const APP_RUN_UNSAVED: &str = "fleet.app_run_unsaved";

#[derive(Debug)]
pub(crate) enum Unrunnable {
    NotARun { name: String, runs: Vec<String> },
    NoSuchJob { id: String },
    JobHasNoWorktree { id: String },
    AlreadySaved { runs: Vec<String> },
    DraftRefused { why: String },
    NotSaved { why: String },
}

impl Unrunnable {
    /// The code and the refusal. **A conflict with where things stand is a 409; a name or a draft
    /// that is wrong is a 422**; a file that would not write is Fleet's fault.
    pub(crate) fn spelled(&self) -> (&'static str, fn(WireError) -> Refusal) {
        use Unrunnable::*;
        match self {
            NotARun { .. } => (NOT_AN_APP_RUN, Refusal::Unacceptable),
            NoSuchJob { .. } => (NO_JOB_TO_RUN, Refusal::NotFound),
            JobHasNoWorktree { .. } => (JOB_HAS_NO_WORKTREE, Refusal::IllegalMove),
            AlreadySaved { .. } => (APP_RUNS_ALREADY_SAVED, Refusal::IllegalMove),
            DraftRefused { .. } => (APP_RUN_DRAFT_REFUSED, Refusal::Unacceptable),
            NotSaved { .. } => (APP_RUN_UNSAVED, Refusal::Fault),
        }
    }
}

impl fmt::Display for Unrunnable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Unrunnable::*;
        match self {
            NotARun { name, runs } if runs.is_empty() => write!(
                out,
                "`{name}` is not a run this project declares, and it declares none"
            ),
            NotARun { name, runs } => write!(
                out,
                "`{name}` is not a run this project declares. It declares {}",
                runs.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", ")
            ),
            NoSuchJob { id } => write!(out, "no job Fleet holds is named {id}"),
            JobHasNoWorktree { id } => write!(
                out,
                "job {id} holds no worktree, so there is nothing of its work to run"
            ),
            AlreadySaved { runs } => write!(
                out,
                "this project already declares {}, so there is nothing to draft",
                runs.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", ")
            ),
            DraftRefused { why } => write!(out, "nothing was saved: {why}"),
            NotSaved { why } => write!(out, "nothing was saved: {why}"),
        }
    }
}

impl std::error::Error for Unrunnable {}
