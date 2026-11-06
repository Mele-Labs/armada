//! The Job's terminals a Fleet before this one left running, taken back at boot. `crate::terminal_keeping`
//! holds the keeper and the record this reads.
//!
//! **A terminal comes back under the run id it had**, so the agent's last-seen shell is still the one
//! it saw, and its screen is rebuilt from the tail of the spool. The name and number were never
//! lost: the store keeps them (`super::job_terminals`).
//!
//! **A record is removed unless its terminal is taken back**: its keeper does not answer, or its Job
//! is gone or ended, which also ends a keeper that does answer.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::SessionState;

use super::driven::{driven_by, Driven};
use super::{Ended, SessionRuns};
use crate::daemon::Fleet;
use crate::terminal_keeping::{client, records_in, Files, Record, TAIL};

impl SessionRuns {
    /// Hold the shell `driven` as run `id`, the terminal `number` of `session`.
    fn adopt<F, Fut>(&self, id: String, session: &str, number: u32, driven: Driven, ended: F) -> std::io::Result<()>
    where
        F: FnOnce(Ended) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut shells = self.shells.lock().expect("not poisoned");
        self.drive(id.clone(), Some(session), None, driven, ended)?;
        shells.entry(session.to_string()).or_default().insert(number, id);
        Ok(())
    }

    /// A test's stand-in for a Fleet that died: every connection to a keeper closes, and what finds
    /// the keepers is left where it is.
    #[cfg(test)]
    pub(crate) fn connections_dropped(&self) {
        for run in self.held.lock().expect("not poisoned").values() {
            if let Some(kept) = &run.kept {
                kept.link.close();
            }
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
    /// How the Job's terminal `number` is kept: where its files are, and how its keeper starts.
    pub(super) fn terminal_keeping(&self, session: &str, number: u32) -> super::Keeping {
        super::Keeping {
            starting: self.host().terminal_keeper.clone(),
            files: Files::of(Path::new(&self.host().keepers_dir), session, number),
            number,
        }
    }

    /// Take back every terminal whose keeper answers and whose Job is still open, at boot and before
    /// the watches that listen to them tick. How many were taken back.
    pub async fn reattached_terminals(self: &Arc<Self>) -> usize {
        let mut taken = 0;
        for path in records_in(Path::new(&self.host().keepers_dir)) {
            let Some(record) = Record::read(&path) else {
                let _ = std::fs::remove_file(&path);
                continue;
            };
            let files = record.files(&path);
            if !self.terminal_is_open(&record).await {
                client::end_at(&record.socket);
                files.removed();
                continue;
            }
            let attaching = record.socket.clone();
            let attached = tokio::task::spawn_blocking(move || client::connect(&attaching).and_then(|stream| client::attach(stream, TAIL).ok())).await;
            let Ok(Some(attached)) = attached else {
                files.removed();
                continue;
            };
            let fleet = Arc::clone(self);
            let (session, number, run_id) = (record.job.clone(), record.number, record.run_id.clone());
            let ended_run = run_id.clone();
            let ended_session = session.clone();
            let adopted = self.session_runs().adopt(run_id, &session, number, driven_by(attached, files.clone()), move |_: Ended| async move {
                fleet.job_shell_ended(&ended_session, number, &ended_run)
            });
            match adopted {
                Ok(()) => taken += 1,
                Err(_) => {
                    client::end_at(&record.socket);
                    files.removed();
                }
            }
        }
        taken
    }

    /// Whether the Job `record` names is open and has had that terminal.
    async fn terminal_is_open(&self, record: &Record) -> bool {
        let store = self.store().lock().await;
        let open = store.session(&record.job).ok().flatten().is_some_and(|session| session.state != SessionState::Ended);
        open && store.job_terminals(&record.job).is_ok_and(|kept| kept.iter().any(|one| one.number == record.number))
    }
}
