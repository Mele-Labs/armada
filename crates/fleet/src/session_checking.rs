//! The Checks a Session's agent runs with `armada check` in the slot it holds. `ipc::session_checks`.
//!
//! **The caller is a bare CLI and writes nothing, so it tells Fleet twice.** Fleet places the run
//! by the directory it ran in, resolved to a pool slot and then to the Session the pool's lease
//! names there, else the one whose ledger row stands in it, never by anything the call claims. A
//! Session leases more slots than its ledger row names, so the lease answers first
//! (`slot_holders::session_in_slot`). A directory no Session holds is a person's own run and
//! answers no run, which is not a refusal.
//!
//! A run is a thread row on the Session, replaced by id as it ends, and a row on the Checks page.
//! Each change publishes the Session whole, so a client replaces it as it does for any ledger fact.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    SessionCheckAct, SessionCheckAnswer, SessionCheckCall,
    SessionCheckState, SessionCheckStop, WireError,
};

use crate::check_output::{windowed, A_READING, MOST};
use crate::daemon::Fleet;
use crate::sessioning::placed;
use crate::slot_holders::session_in_slot;

/// A run that no Session kept, asked for by its id.
const NO_SUCH_RUN: &str = "fleet.session_check_unknown";
/// A stop asked of a run that is not out.
const NOT_RUNNING: &str = "fleet.session_check_not_running";
/// How long a pause waits for the runs it stopped to say they ended. A run's `armada check` hears
/// the stop at its next chunk, a quarter of a second on, and ends the Check's whole process group.
const STOPPED_WITHIN: std::time::Duration = std::time::Duration::from_secs(3);
/// How often a pause looks at whether they have.
const LOOKED_EVERY: std::time::Duration = std::time::Duration::from_millis(25);

/// A call that cannot be a run's start or end.
const UNNAMED: &str = "fleet.session_check_unnamed";

/// The tail of `log` that fits one read, cut at a character.
fn tail(log: &str) -> &str {
    if log.len() <= MOST {
        return log;
    }
    let mut from = log.len() - MOST;
    while !log.is_char_boundary(from) {
        from += 1;
    }
    &log[from..]
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
    fn session_check_refusal(&self, code: &'static str, why: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why.to_string(), self.run_id()))
    }

    /// `report_session_check`.
    pub(crate) async fn session_check_reported(
        &self,
        call: SessionCheckCall,
    ) -> Result<SessionCheckAnswer, Refusal> {
        match call.act {
            SessionCheckAct::Start => self.session_check_started(call).await,
            SessionCheckAct::Chunk => self.session_check_heard(&call),
            SessionCheckAct::End => self.session_check_ended(call).await,
        }
    }

    /// What a run printed since its last call, and the stop a person asked for, answered. **A run
    /// Fleet never heard begin is not made up**: an id that is not a running run answers nothing.
    fn session_check_heard(&self, call: &SessionCheckCall) -> Result<SessionCheckAnswer, Refusal> {
        let Some(run) = call.run else {
            return Err(self.session_check_refusal(UNNAMED, "a chunk of a Check run needs its run"));
        };
        let id = i64::try_from(run).map_err(|_| self.session_check_refusal(UNNAMED, "no such run"))?;
        let stop = self.session_check_live().run(id).heard(call.log.as_deref().unwrap_or_default());
        Ok(SessionCheckAnswer { run: Some(run), stop })
    }

    async fn session_check_started(
        &self,
        call: SessionCheckCall,
    ) -> Result<SessionCheckAnswer, Refusal> {
        let name = call.name.trim();
        let target = call.target.as_deref().map(str::trim).filter(|target| !target.is_empty());
        if name.is_empty() {
            return Err(self.session_check_refusal(UNNAMED, "a Check run needs the Check's name"));
        }
        let roots = self.ledger_roots();
        let place = placed(&call.cwd, &roots);
        let (Some(manifest), Some(slot)) = (place.manifest, place.slot) else {
            return Ok(SessionCheckAnswer { run: None, stop: None });
        };
        let at = self.clock().now().as_str().to_string();
        let mut store = self.store().lock().await;
        let root = roots.iter().find(|(id, _)| *id == manifest).map(|(_, root)| root.as_str());
        let Some(session) = root.and_then(|root| session_in_slot(&store, &manifest, root, slot)) else {
            return Ok(SessionCheckAnswer { run: None, stop: None });
        };
        let run = store
            .start_session_check(&session, &manifest, slot, name, target, &at)
            .map_err(|why| self.ledger_fault(why))?;
        drop(store);
        self.session_check_row(&session, name, target, None, run, &at, SessionCheckState::Running)
            .await?;
        Ok(SessionCheckAnswer {
            run: u64::try_from(run).ok(),
            stop: None,
        })
    }

    async fn session_check_ended(
        &self,
        call: SessionCheckCall,
    ) -> Result<SessionCheckAnswer, Refusal> {
        let (Some(run), Some(outcome)) = (call.run.and_then(|run| i64::try_from(run).ok()), call.outcome)
        else {
            return Err(self.session_check_refusal(UNNAMED, "the end of a Check run needs its run and how it ended"));
        };
        if outcome == SessionCheckState::Running {
            return Err(self.session_check_refusal(UNNAMED, "a Check run ends as passed or failed"));
        }
        let target = call.target.as_deref().map(str::trim).filter(|target| !target.is_empty());
        let at = self.clock().now().as_str().to_string();
        let mut store = self.store().lock().await;
        let Some(kept) = store.session_check(run).map_err(|why| self.ledger_fault(why))? else {
            return Ok(SessionCheckAnswer { run: None, stop: None });
        };
        let ended = store
            .end_session_check(
                run,
                outcome.as_wire(),
                &at,
                call.took_ms.unwrap_or_default(),
                tail(call.log.as_deref().unwrap_or_default()),
                target,
            )
            .map_err(|why| self.ledger_fault(why))?;
        drop(store);
        self.session_check_live().ended(run);
        if ended {
            let target = target.or(kept.target.as_deref());
            let blame = match (outcome, call.log) {
                (SessionCheckState::Failed, Some(log)) => self.blamed(&kept, log).await,
                _ => None,
            };
            self.session_check_row(&kept.session_id, &kept.name, target, blame, run, &kept.started_at, outcome)
                .await?;
        }
        Ok(SessionCheckAnswer {
            run: u64::try_from(run).ok(),
            stop: None,
        })
    }

    /// Where a failed run's log says it failed, read in the slot it ran in. Read once, here, and
    /// kept on the row, so no read of the thread parses a log.
    async fn blamed(&self, kept: &store::KeptSessionCheck, log: String) -> Option<ipc::CheckBlame> {
        let roots = self.ledger_roots();
        let root = roots.iter().find(|(id, _)| *id == kept.manifest_id)?.1.clone();
        let slot = adapter_traits::slot_path(&root, kept.slot);
        tokio::task::spawn_blocking(move || adapters::check_blame::blame_of(&log, std::path::Path::new(&slot)))
            .await
            .ok()
            .flatten()
    }

    /// Put the run's row on the thread and publish the Session. `at` is when the run began, so
    /// the row keeps its place in the thread as it ends.
    async fn session_check_row(
        &self,
        session: &str,
        name: &str,
        target: Option<&str>,
        blame: Option<ipc::CheckBlame>,
        run: i64,
        at: &str,
        state: SessionCheckState,
    ) -> Result<(), Refusal> {
        self.row_put(
            session,
            ipc::SessionRow::Check {
                id: format!("check-{run}"),
                at: ipc::Instant::carried(at),
                name: name.to_string(),
                run: u64::try_from(run).unwrap_or_default(),
                state,
                target: target.map(str::to_string),
                blame,
            },
        )
        .await;
        let record = {
            let store = self.store().lock().await;
            match store.session(session).map_err(|why| self.ledger_fault(why))? {
                Some(kept) => Some(self.ledger_row(&store, &kept)?),
                None => None,
            }
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
        Ok(())
    }

    /// A Session ended: the runs still out will never be ended by their CLI, so they end failed.
    pub(crate) async fn session_checks_ended(&self, session: &str) {
        self.session_checks_failed(session, "the Job ended").await;
    }

    /// A Job is being paused: **every Check it has running is stopped, as a person's force stop stops
    /// it, before its slot is parked**, so none runs on under the next holder. Waits for each to say
    /// it ended; one whose `armada check` does not (it died with the agent, or does not answer)
    /// ends failed here instead.
    pub(crate) async fn session_checks_stopped(&self, session: &str) {
        let Ok(running) = self.store().lock().await.running_session_checks(session) else {
            return;
        };
        for run in &running {
            if let Ok(id) = u64::try_from(run.id) {
                let _ = self.session_check_stopped(id, true).await;
            }
        }
        let waited = std::time::Instant::now();
        while !running.is_empty() && waited.elapsed() < STOPPED_WITHIN {
            match self.store().lock().await.running_session_checks(session) {
                Ok(left) if !left.is_empty() => tokio::time::sleep(LOOKED_EVERY).await,
                _ => return,
            }
        }
        self.session_checks_failed(session, "the Job was paused").await;
    }

    async fn session_checks_failed(&self, session: &str, why: &str) {
        let at = self.clock().now().as_str().to_string();
        let Ok(runs) = self
            .store()
            .lock()
            .await
            .fail_running_session_checks(session, &at, why)
        else {
            return;
        };
        for run in runs {
            let _ = self
                .session_check_row(
                    session,
                    &run.name,
                    run.target.as_deref(),
                    None,
                    run.id,
                    &run.started_at,
                    SessionCheckState::Failed,
                )
                .await;
        }
    }

    /// `get_session_check_output`: one run's log, as the Job's Check output read answers it.
    pub(crate) async fn session_check_output(&self, run: u64) -> Result<ipc::CheckOutput, Refusal> {
        let no_such = || self.session_check_refusal(NO_SUCH_RUN, "no Job ran a Check under that id");
        let run = i64::try_from(run).map_err(|_| no_such())?;
        let store = self.store().lock().await;
        let (Some(kept), Some(log)) = (
            store.session_check(run).map_err(|why| self.ledger_fault(why))?,
            store.session_check_log(run).map_err(|why| self.ledger_fault(why))?,
        ) else {
            return Err(no_such());
        };
        drop(store);
        let (window, first, total) = windowed(log.lines().map(str::to_string), A_READING, MOST);
        Ok(ipc::CheckOutput {
            attempt: 1,
            name: kept.name,
            path: format!("session-check/{run}"),
            from_line: first,
            total_lines: total,
            bytes: log.len() as u64,
            whole: first == 1,
            lines: window.into(),
        })
    }

    /// `observe_session_check`: a run's log, live while it is out and whole once it is over.
    pub(crate) async fn session_check_observed(&self, run: u64) -> Result<api::SessionCheckLive, Refusal> {
        let no_such = || self.session_check_refusal(NO_SUCH_RUN, "no Job ran a Check under that id");
        let id = i64::try_from(run).map_err(|_| no_such())?;
        let store = self.store().lock().await;
        let Some(kept) = store.session_check(id).map_err(|why| self.ledger_fault(why))? else {
            return Err(no_such());
        };
        let log = store.session_check_log(id).map_err(|why| self.ledger_fault(why))?;
        drop(store);
        let follow: std::sync::Arc<dyn api::Follow> = match (kept.state.as_str(), self.session_check_live().find(id)) {
            ("running", Some(held)) => std::sync::Arc::new(crate::session_check_live::Live(held)),
            _ => std::sync::Arc::new(crate::session_check_live::Kept(log.unwrap_or_default())),
        };
        Ok(api::SessionCheckLive { run, name: kept.name, follow })
    }

    /// `stop_session_check` and `force_stop_session_check`: held for the run's own `armada check`,
    /// which asks on its next `chunk`.
    pub(crate) async fn session_check_stopped(&self, run: u64, force: bool) -> Result<SessionCheckAnswer, Refusal> {
        let not_running = || self.session_check_refusal(NOT_RUNNING, "that Check is not running");
        let id = i64::try_from(run).map_err(|_| not_running())?;
        let running = self
            .store()
            .lock()
            .await
            .session_check(id)
            .map_err(|why| self.ledger_fault(why))?
            .is_some_and(|kept| kept.state == SessionCheckState::Running.as_wire());
        if !running {
            return Err(not_running());
        }
        let asked = if force { SessionCheckStop::Force } else { SessionCheckStop::Graceful };
        let stop = self.session_check_live().run(id).asked(asked);
        Ok(SessionCheckAnswer { run: Some(run), stop })
    }

}
