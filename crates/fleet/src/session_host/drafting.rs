//! `propose_draft`: a session puts a draft to the person for approval, kept as a `draft` row that
//! waits as `draft:<row id>` until `answer_waiting`, his Dismiss or `withdraw_draft` settles it.
//!
//! **It does not wait**, as `ask_in_window` does not: the row is kept and the call answers, and the
//! open draft is read back off the thread (`waiting.rs`).

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DraftHow, DraftSettled, ProposeDraft, SessionRecord, SessionRow, WithdrawDraft};

use crate::daemon::Fleet;

const DRAFT_UNNAMED: &str = "fleet.draft_unnamed";
const DRAFT_EMPTY: &str = "fleet.draft_empty";
const DRAFT_WITHDRAW_UNNAMED: &str = "fleet.draft_withdraw_unnamed";
const DRAFT_WITHDRAW_NOT_OPEN: &str = "fleet.draft_withdraw_not_open";
pub(crate) const FIXED_NOT_A_DRAFT: &str = "fleet.fixed_not_a_draft";

/// The id a draft's waiting item goes by, from its row's.
pub(crate) const DRAFT_PREFIX: &str = "draft:";

/// The person's message for each way a draft is settled. `title` is the item's text.
pub(crate) fn approved(title: &str, words: Option<&str>) -> String {
    let mut body = format!("Approved: {title}\nApply it as written.");
    if let Some(words) = words {
        body.push('\n');
        body.push_str(words);
    }
    body
}

pub(crate) fn fixed(title: &str, text: &str) -> String {
    format!("Approved with my fixes: {title}\nUse exactly this text, as written:\n\n{text}")
}

pub(crate) fn sent_back(title: &str, note: &str) -> String {
    format!("Sent back: {title}\n{note}\nRewrite it and propose it again with propose_draft.")
}

/// The person's Dismiss on a draft. A blank reason adds no line.
pub(crate) fn withdrawn(title: &str, reason: Option<&str>) -> String {
    let mut body = format!("Withdrawn: {title}\nDo not propose it again.");
    if let Some(reason) = reason.map(str::trim).filter(|why| !why.is_empty()) {
        body.push('\n');
        body.push_str(reason);
    }
    body
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
    pub(crate) async fn draft_proposed(
        &self,
        caller: Option<api::Caller>,
        draft: ProposeDraft,
    ) -> Result<SessionRecord, Refusal> {
        let title = draft.title.trim().to_string();
        let text = draft.text.trim().to_string();
        if title.is_empty() {
            return Err(self.hosted_refusal(DRAFT_EMPTY, "a draft needs a title, one line naming what it is"));
        }
        if text.is_empty() {
            return Err(self.hosted_refusal(DRAFT_EMPTY, "a draft needs its text"));
        }
        let before = draft
            .before
            .map(|said| said.trim().to_string())
            .filter(|said| !said.is_empty());
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| draft.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(DRAFT_UNNAMED, "a job to propose it for"))?;
        if self
            .store()
            .lock()
            .await
            .session(&id)
            .map_err(|why| self.ledger_fault(why))?
            .is_none()
        {
            return Err(self.hosted_refusal(DRAFT_UNNAMED, "a job Fleet knows"));
        }
        self.row_put(
            &id,
            SessionRow::Draft {
                id: self.row_id(&id),
                at: self.instant(),
                title,
                text,
                before,
                settled: None,
                change: None,
            },
        )
        .await;
        self.draft_record(&id).await
    }

    /// `withdraw_draft`: the session takes back an open draft of its own. The row settles `withdrawn`
    /// with no words and the session is sent nothing, since it withdrew it. **Only an open draft on
    /// this job's thread**, placed by the connection as `propose_draft` places it.
    pub(crate) async fn draft_withdrawn(
        &self,
        caller: Option<api::Caller>,
        withdraw: WithdrawDraft,
    ) -> Result<SessionRecord, Refusal> {
        let named = withdraw.draft.trim();
        let row = named.strip_prefix(DRAFT_PREFIX).unwrap_or(named).trim().to_string();
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| withdraw.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(DRAFT_WITHDRAW_UNNAMED, "a job to withdraw it for"))?;
        if self.store().lock().await.session(&id).map_err(|why| self.ledger_fault(why))?.is_none() {
            return Err(self.hosted_refusal(DRAFT_WITHDRAW_UNNAMED, "a job Fleet knows"));
        }
        let not_open = || {
            self.hosted_refusal(
                DRAFT_WITHDRAW_NOT_OPEN,
                &format!("this job has no open draft {named}. Name the draft id propose_draft gave you, for a draft not yet settled"),
            )
        };
        // Held as an answer holds it, so his answer and this cannot both settle the row.
        let _settling = self.hosts().settling(&id, &format!("{DRAFT_PREFIX}{row}")).ok_or_else(not_open)?;
        let open = self
            .rows_of(&id)
            .await?
            .iter()
            .any(|one| matches!(one, SessionRow::Draft { id, settled: None, .. } if *id == row));
        if row.is_empty() || !open {
            return Err(not_open());
        }
        self.draft_settled(&id, &row, DraftHow::Withdrawn, None).await
    }

    /// Keep how a draft was settled on its row, and publish the session so the item leaves the list.
    /// Called only once the person's message is sent, so a failed send leaves the draft standing.
    pub(crate) async fn draft_settled(
        &self,
        session: &str,
        row_id: &str,
        how: DraftHow,
        words: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        let found = self.rows_of(session).await?.into_iter().find_map(|row| match row {
            SessionRow::Draft { id, at, title, text, before, change, .. } if id == row_id => Some(SessionRow::Draft {
                id,
                at,
                title,
                text,
                before,
                change,
                settled: Some(DraftSettled { how, words: words.clone(), at: self.instant() }),
            }),
            _ => None,
        });
        if let Some(row) = found {
            self.row_put(session, row).await;
        }
        self.draft_record(session).await
    }

    /// A workflow a Job proposed waits on the owner as a draft that names its change. The item
    /// leaves the list when the change is no longer ready, whoever settled it.
    pub(crate) async fn proposal_waits(&self, session: &str, change: i64, workflow: &str, definition: &str, before: Option<String>) {
        self.row_put(
            session,
            SessionRow::Draft {
                id: self.row_id(session),
                at: self.instant(),
                title: format!("Workflow {workflow}"),
                text: definition.to_string(),
                before,
                settled: None,
                change: Some(change),
            },
        )
        .await;
        let _ = self.draft_record(session).await;
    }

    /// The change an open draft is the say on, where it is one.
    pub(crate) async fn draft_change(&self, session: &str, row: &str) -> Option<i64> {
        self.rows_of(session).await.ok()?.into_iter().find_map(|one| match one {
            SessionRow::Draft { id, change, .. } if id == row => change,
            _ => None,
        })
    }

    async fn draft_record(&self, id: &str) -> Result<SessionRecord, Refusal> {
        let record = {
            let store = self.store().lock().await;
            let session = store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(DRAFT_UNNAMED, "a job Fleet knows"))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }
}
