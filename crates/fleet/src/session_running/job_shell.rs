//! A Job's shell: one interactive `$SHELL -l` a Job, in the directory its runs take, open until it
//! exits, is stopped, or the Job ends.
//!
//! **It is a run like any other**, so typing, resizing, watching and stopping it are the run
//! operations on its id. **Its end tells the Job nothing**: no command was run to be told about.

use std::future::Future;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;

use super::{refusal, size, Ended, Launch, SessionRuns, COLS, ROWS};
use crate::daemon::Fleet;

impl SessionRuns {
    /// The Job's shell, started with `launch` where none is in flight, and whether it started now.
    /// `on_end` is handed the new shell's id and answers what runs once it has ended.
    fn open_shell<E, F, Fut>(&self, launch: Launch<'_>, on_end: E) -> std::io::Result<(String, bool)>
    where
        E: FnOnce(String) -> F,
        F: FnOnce(Ended) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Held across the start, so two opens at once start one shell.
        let mut shells = self.shells.lock().expect("not poisoned");
        if let Some(open) = shells.get(launch.session).filter(|id| self.in_flight(id).is_some()) {
            return Ok((open.clone(), false));
        }
        let session = launch.session.to_string();
        let id = self.mint();
        self.start_as(id.clone(), launch, on_end(id.clone()))?;
        shells.insert(session, id.clone());
        Ok((id, true))
    }

    /// Forget `run_id` as `session`'s shell, unless a newer one has taken its place.
    fn shell_ended(&self, session: &str, run_id: &str) {
        let mut shells = self.shells.lock().expect("not poisoned");
        if shells.get(session).is_some_and(|id| id == run_id) {
            shells.remove(session);
        }
    }
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
    /// The Job's shell, opened where it has none. A Job that has ended, or that Fleet has no row
    /// for, is refused as a run is.
    pub(crate) async fn open_job_shell(
        self: Arc<Self>,
        session_id: &str,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<String, Refusal> {
        Ok(self.job_shell(session_id, cols, rows).await?.0)
    }

    /// [`Self::open_job_shell`], and whether the shell started now.
    async fn job_shell(
        self: Arc<Self>,
        session_id: &str,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<(String, bool), Refusal> {
        let (cols, rows) = (cols.unwrap_or(COLS), rows.unwrap_or(ROWS));
        if cols == 0 || rows == 0 {
            return Err(self.no_cells());
        }
        let dir = self.run_dir(session_id).await?;
        let launch = Launch { session: session_id, command: None, dir: &dir, size: size(cols, rows) };
        let fleet = Arc::clone(&self);
        let session = session_id.to_string();
        self.session_runs()
            .open_shell(launch, move |run_id| {
                move |_: Ended| async move { fleet.job_shell_ended(&session, &run_id) }
            })
            .map_err(|why| refusal(format!("the shell could not start in {dir}: {why}"), self.run_id()))
    }

    fn job_shell_ended(&self, session: &str, run_id: &str) {
        self.session_runs().shell_ended(session, run_id);
    }
}
