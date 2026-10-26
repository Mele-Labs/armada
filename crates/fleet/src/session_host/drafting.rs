//! `propose_draft`: a session puts a draft to the person for approval, kept as a `draft` row that
//! waits as `draft:<row id>` until `answer_waiting` settles it.
//!
//! **It does not wait**, as `ask_in_window` does not: the row is kept and the call answers, and the
//! open draft is read back off the thread (`waiting.rs`).

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DraftHow, DraftSettled, ProposeDraft, SessionRecord, SessionRow};

use crate::daemon::Fleet;

const DRAFT_UNNAMED: &str = "fleet.draft_unnamed";
const DRAFT_EMPTY: &str = "fleet.draft_empty";
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
            },
        )
        .await;
        self.draft_record(&id).await
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
            SessionRow::Draft { id, at, title, text, before, .. } if id == row_id => Some(SessionRow::Draft {
                id,
                at,
                title,
                text,
                before,
                settled: Some(DraftSettled { how, words: words.clone(), at: self.instant() }),
            }),
            _ => None,
        });
        if let Some(row) = found {
            self.row_put(session, row).await;
        }
        self.draft_record(session).await
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
