//! The Checks a Session's agent runs with `armada check` in the slot it holds. `ipc::session_checks`.
//!
//! **The caller is a bare CLI and writes nothing, so it tells Fleet twice.** Fleet places the run
//! by the directory it ran in, resolved to a pool slot and then to the Session standing in that
//! slot, never by anything the call claims. A directory no Session holds is a person's own run and
//! answers no run, which is not a refusal.
//!
//! A run is a thread row on the Session, replaced by id as it ends, and a row on the Checks page.
//! Each change publishes the Session whole, so a client replaces it as it does for any ledger fact.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ManifestCheckLog, ManifestCheckRow, SessionCheckAct, SessionCheckAnswer, SessionCheckCall,
    SessionCheckState, WireError,
};
use store::{AttachmentState, HolderKind, KeptSessionCheck};

use crate::check_output::{windowed, A_READING, MOST};
use crate::daemon::Fleet;
use crate::sessioning::placed;

/// A run that no Session kept, asked for by its id.
const NO_SUCH_RUN: &str = "fleet.session_check_unknown";
/// A call that cannot be a run's start or end.
const UNNAMED: &str = "fleet.session_check_unnamed";

/// The Checks page's rows for these runs: no Job, the Session in its place.
pub(crate) fn rows_of(runs: &[KeptSessionCheck]) -> Vec<ManifestCheckRow> {
    runs.iter()
        .map(|run| ManifestCheckRow {
            source: ipc::SOURCE_ASKED_RUN.to_string(),
            requester: ipc::Requester::session(&run.session_id, run.slot),
            job_id: None,
            job_handle: None,
            job_title: None,
            step: None,
            session_id: Some(run.session_id.clone()),
            attempt: 1,
            group: None,
            name: run.name.clone(),
            state: run.state.clone(),
            started_at: Some(ipc::Instant::carried(&run.started_at)),
            ended_at: run.ended_at.as_deref().map(ipc::Instant::carried),
            took_ms: run.took_ms,
            logs: vec![ManifestCheckLog {
                check: run.name.clone(),
                kept: format!("session.{}.{}.log", run.session_id, run.id),
            }],
            asked_run_id: Some(run.id),
        })
        .collect()
}

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
            SessionCheckAct::End => self.session_check_ended(call).await,
        }
    }

    async fn session_check_started(
        &self,
        call: SessionCheckCall,
    ) -> Result<SessionCheckAnswer, Refusal> {
        let name = call.name.trim();
        if name.is_empty() {
            return Err(self.session_check_refusal(UNNAMED, "a Check run needs the Check's name"));
        }
        let place = placed(&call.cwd, &self.ledger_roots());
        let (Some(manifest), Some(slot)) = (place.manifest, place.slot) else {
            return Ok(SessionCheckAnswer { run: None });
        };
        let at = self.clock().now().as_str().to_string();
        let mut store = self.store().lock().await;
        let holders = store
            .attachments_at("slot", &slot.to_string(), Some(&manifest))
            .map_err(|why| self.ledger_fault(why))?;
        let Some(session) = holders
            .iter()
            .find(|held| held.holder.kind == HolderKind::Session && held.state == AttachmentState::Standing)
            .map(|held| held.holder.id.clone())
        else {
            return Ok(SessionCheckAnswer { run: None });
        };
        let run = store
            .start_session_check(&session, &manifest, slot, name, &at)
            .map_err(|why| self.ledger_fault(why))?;
        drop(store);
        self.session_check_row(&session, name, run, &at, SessionCheckState::Running)
            .await?;
        Ok(SessionCheckAnswer {
            run: u64::try_from(run).ok(),
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
        let at = self.clock().now().as_str().to_string();
        let mut store = self.store().lock().await;
        let Some(kept) = store.session_check(run).map_err(|why| self.ledger_fault(why))? else {
            return Ok(SessionCheckAnswer { run: None });
        };
        let ended = store
            .end_session_check(
                run,
                outcome.as_wire(),
                &at,
                call.took_ms.unwrap_or_default(),
                tail(call.log.as_deref().unwrap_or_default()),
            )
            .map_err(|why| self.ledger_fault(why))?;
        drop(store);
        if ended {
            self.session_check_row(&kept.session_id, &kept.name, run, &kept.started_at, outcome)
                .await?;
        }
        Ok(SessionCheckAnswer {
            run: u64::try_from(run).ok(),
        })
    }

    /// Put the run's row on the thread and publish the Session. `at` is when the run began, so
    /// the row keeps its place in the thread as it ends.
    async fn session_check_row(
        &self,
        session: &str,
        name: &str,
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

    /// `get_session_check_output`: one run's log, as the Job's Check output read answers it.
    pub(crate) async fn session_check_output(&self, run: u64) -> Result<ipc::CheckOutput, Refusal> {
        let no_such = || self.session_check_refusal(NO_SUCH_RUN, "no Session ran a Check under that id");
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

    /// The Sessions' runs for the Checks page, narrowed to the named repository's.
    pub(crate) async fn session_check_rows(
        &self,
        manifest: Option<&str>,
    ) -> Result<Vec<ManifestCheckRow>, Refusal> {
        let runs = self
            .store()
            .lock()
            .await
            .session_checks(manifest, ipc::ManifestChecks::MOST as u32)
            .map_err(|why| self.ledger_fault(why))?;
        Ok(rows_of(&runs))
    }
}
