//! The follow-ups an agent keeps for itself, kept across its replies until they are settled.
//! `extracting` reads them off the close of one reply; here each is held a Session at a time, so the
//! next reply does not wipe it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DeferLater, IssueReference, LaterDeferred, LaterHow, LaterSettled, SettleLater, WireError};

use crate::daemon::Fleet;

const NOT_SETTLEABLE: &str = "fleet.later_not_settleable";
const NOT_DEFERRABLE: &str = "fleet.later_not_deferrable";
const NOT_OPEN: &str = "fleet.later_not_open";
const LATER_OPEN: &str = "fleet.later_open";

/// What `all_done` is told while follow-ups are open: each one, and the three ways to settle it.
fn open_text(open: &[store::KeptLater]) -> String {
    let listed: Vec<String> = open.iter().map(|one| format!("- {}: {}", one.id, one.sentence)).collect();
    format!(
        "all_done refused. Still open under Later:\n\n{}\n\nSettle each with settle_later: done, filed with an issue number, or dropped with a reason.",
        listed.join("\n")
    )
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
    /// Keep `sentences`, said when a reply ended at `said_at`. A sentence already kept, open or
    /// settled, is not kept again. Whether any was new.
    pub(crate) async fn later_kept(&self, id: &str, sentences: &[String], said_at: &str) -> bool {
        // A sentence near one he marked Not deferred is not kept, by the vectors the asks read; one not
        // embedded yet is kept this read.
        let misreads = self.store().lock().await.later_misreads().unwrap_or_default();
        let sentences = match (misreads.is_empty(), self.language_model().model()) {
            (false, Some(model)) => {
                let texts: Vec<&str> = sentences.iter().chain(misreads.iter()).map(String::as_str).collect();
                let known = self.item_vectors().known(model, id, &texts);
                super::dedup::left_off_as_misread_sentences(sentences.to_vec(), &misreads, |text| known.get(text).cloned(), super::dedup::FOLDS_AT)
            }
            _ => sentences.to_vec(),
        };
        let mut store = self.store().lock().await;
        sentences.iter().fold(false, |any, sentence| store.keep_later(id, sentence, said_at).unwrap_or(false) || any)
    }

    /// The follow-ups of the Session's last reply, once the model has read it. Nothing while the
    /// readers are off, the reply is not read yet, or the Session has ended.
    pub(crate) async fn later_read(&self, id: &str) {
        if !super::extracting::IN_FORCE {
            return;
        }
        let Some(model) = self.language_model().model() else {
            return;
        };
        let reply = {
            let store = self.store().lock().await;
            let live = store.session(id).ok().flatten().is_some_and(|session| session.state != store::SessionState::Ended);
            store.turn_end(id).ok().flatten().filter(|_| live)
        };
        let Some((at, said)) = reply else {
            return;
        };
        if let Some(read) = self.extract_reads().read_of(super::extracting::read, model, id, &said) {
            self.later_kept(id, &read.later, &at).await;
        }
    }

    /// `all_done` is refused while any follow-up is open, naming each. **Only the agent's call asks**:
    /// the person's End job is `close_session` and never reads this.
    pub(crate) fn later_holds_done(&self, store: &store::Store, id: &str) -> Result<(), Refusal> {
        let open = store.later_of(id).map_err(|why| self.ledger_fault(why))?;
        match open.is_empty() {
            true => Ok(()),
            false => Err(Refusal::IllegalMove(WireError::raised(LATER_OPEN, open_text(&open), self.run_id()))),
        }
    }

    /// `defer_later`: the job keeps one follow-up for itself. The same sentence is the item it was.
    pub(crate) async fn later_deferred(&self, _caller: Option<api::Caller>, _defer: DeferLater) -> Result<LaterDeferred, Refusal> {
        Err(self.hosted_refusal(NOT_DEFERRABLE, "not built yet"))
    }

    /// `settle_later`: one open follow-up is done, filed or dropped. **A job's own call must say
    /// enough**: an issue it can name for a filed, a reason for a drop. The person's Drop is a call
    /// no connection places, and says nothing more.
    pub(crate) async fn later_settled(&self, caller: Option<api::Caller>, settle: SettleLater) -> Result<LaterSettled, Refusal> {
        let by_agent = caller.as_ref().is_some_and(|caller| self.session_holding(caller).is_some());
        let id = self.waiting_session(caller, settle.session_id.as_ref())?;
        if settle.misread && (by_agent || !matches!(settle.how, LaterHow::Dropped)) {
            return Err(self.hosted_refusal(NOT_SETTLEABLE, "only the person's drop is marked as not deferred"));
        }
        let reason = settle.reason.as_deref().map(str::trim).filter(|reason| !reason.is_empty());
        let (how, note) = match settle.how {
            LaterHow::Done => ("done", None),
            LaterHow::Filed => {
                let issue = settle.issue.as_deref().and_then(IssueReference::read).ok_or_else(|| {
                    self.hosted_refusal(NOT_SETTLEABLE, "filed needs issue, the forge's number such as 7 or a tracker's key such as ABC-12")
                })?;
                ("filed", Some(issue.as_str().to_string()))
            }
            LaterHow::Dropped => {
                if by_agent && reason.is_none() {
                    return Err(self.hosted_refusal(NOT_SETTLEABLE, "dropped needs reason, one sentence on why it will not be done"));
                }
                ("dropped", reason.map(str::to_string))
            }
        };
        let now = self.clock().now().as_str().to_string();
        let (settled, record) = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NOT_SETTLEABLE, "a job Fleet knows"))?;
            if session.state == store::SessionState::Ended {
                return Err(self.closed(&id));
            }
            let sentence = store
                .settle_later(&id, &settle.id, how, note.as_deref(), &now)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    Refusal::IllegalMove(WireError::raised(
                        NOT_OPEN,
                        format!("nothing is open under {}. It was settled already, or the job never kept it", settle.id),
                        self.run_id(),
                    ))
                })?;
            if settle.misread {
                store.keep_later_misread(&id, &sentence, &now).map_err(|why| self.ledger_fault(why))?;
            }
            let record = self.ledger_row(&store, &session)?;
            (LaterSettled { id: settle.id.clone(), sentence, open: record.later.clone() }, record)
        };
        self.publish(ipc::Event::SessionChanged(record));
        Ok(settled)
    }

    /// `confirm_later`: thumbs up on an open follow-up. Its sentence is kept for good as one, the row
    /// stays open and reads confirmed. The agent is told nothing.
    pub(crate) async fn later_confirmed(&self, confirm: ipc::ConfirmLater) -> Result<ipc::SessionRecord, Refusal> {
        let id = self.waiting_session(None, confirm.session_id.as_ref())?;
        let now = self.clock().now().as_str().to_string();
        let record = {
            let mut store = self.store().lock().await;
            let session = store.session(&id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(NOT_SETTLEABLE, "a job Fleet knows"))?;
            let open = store.later_of(&id).map_err(|why| self.ledger_fault(why))?;
            let Some(one) = open.iter().find(|one| one.id == confirm.id) else {
                return Err(Refusal::IllegalMove(WireError::raised(
                    NOT_OPEN,
                    format!("nothing is open under {}. It was settled already, or the job never kept it", confirm.id),
                    self.run_id(),
                )));
            };
            store.keep_later_confirmed(&id, &one.sentence, &now).map_err(|why| self.ledger_fault(why))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }
}
