//! `get_session_file_change`: one file a Job has been seen writing, as its worktree changed it since
//! its branch met the base. The diff lens on a Job's Field reads it.
//!
//! **The write table is the whitelist**: a path the Job is not credited with is refused before git
//! is asked. The read is one path's `git diff`, never the worktree whole.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::clash::{self, FileHere, NotChecked};
use api::Refusal;
use ipc::{SessionFileChange, WireError};

use super::checking::base_of;
use super::Who;
use crate::catching_up::cut_for_the_card;
use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// The Job ended, or holds no worktree now. A 409.
const NO_WORKTREE: &str = "fleet.session_no_worktree";
/// A path the write table does not credit the Job with. A 409.
const PATH_NOT_EDITED: &str = "fleet.session_path_not_edited";
/// Git would not print the change. A 500.
const UNREAD: &str = "fleet.session_file_change_unread";

/// The file in the first worktree that changed it, else in the first. Against `origin/<base>` where
/// the repository has one, as the clash checks read it.
fn change_in(worktrees: &[PathBuf], base: &str, path: &str, seen: Option<&str>) -> Result<FileHere, NotChecked> {
    let mut first = None;
    for worktree in worktrees {
        let here = clash::file_here(worktree, &format!("origin/{base}"), path, seen)
            .or_else(|_| clash::file_here(worktree, base, path, seen))?;
        if here.diff.as_ref().is_none_or(|diff| !diff.is_empty()) {
            return Ok(here);
        }
        first.get_or_insert(here);
    }
    first.ok_or_else(|| NotChecked(String::from("no worktree to read")))
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
    /// `path` as the Job `session_id` changed it. `seen` is the stamp of the change Bridge holds.
    pub(crate) async fn session_file_change(
        &self,
        session_id: &str,
        path: &str,
        seen: Option<String>,
    ) -> Result<SessionFileChange, Refusal> {
        let raised = |code: &'static str, said: String| WireError::raised(code, said, self.run_id());
        let known = self.store().lock().await.session(session_id).map_err(|why| self.ledger_fault(why))?;
        if known.is_none() {
            return Err(Refusal::NotFound(raised(NO_SUCH_SESSION, format!("no job is named {session_id}"))));
        }
        let who = Who::Session(session_id.to_string());
        let Some(standing) = self.writing_in(&who).await else {
            return Err(Refusal::IllegalMove(raised(NO_WORKTREE, format!("job {session_id} holds no worktree"))));
        };
        if !self.writes().written_by(&who).contains(path) {
            return Err(Refusal::IllegalMove(raised(
                PATH_NOT_EDITED,
                format!("job {session_id} has not edited `{path}`"),
            )));
        }
        let Some(served) = self.repositories().serving(&standing.manifest) else {
            return Err(Refusal::IllegalMove(raised(NO_WORKTREE, format!("job {session_id} holds no worktree"))));
        };
        let base = base_of(&served);
        let wanted = path.to_string();
        let read = tokio::task::spawn_blocking(move || change_in(&standing.worktrees, &base, &wanted, seen.as_deref()))
            .await
            .unwrap_or_else(|why| Err(NotChecked(format!("the read did not finish: {why}"))));
        match read {
            Ok(FileHere { stamp, diff: None }) => {
                Ok(SessionFileChange { stamp, diff: String::new(), cut: false, unchanged: true })
            }
            Ok(FileHere { stamp, diff: Some(diff) }) => {
                let (diff, cut) = cut_for_the_card(diff);
                Ok(SessionFileChange { stamp, diff, cut, unchanged: false })
            }
            Err(why) => Err(Refusal::Fault(raised(UNREAD, why.to_string()))),
        }
    }
}
