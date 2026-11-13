//! `propose_draft`: a session puts a draft to the person for approval, kept as a `draft` row that
//! waits as `draft:<row id>` until `answer_waiting`, his Dismiss or `withdraw_draft` settles it.
//!
//! **It does not wait**, as `ask_in_window` does not: the row is kept and the call answers, and the
//! open draft is read back off the thread (`waiting.rs`).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DraftHow, DraftSettled, ProposeDraft, SendSessionMessage, SessionRecord, SessionRow, WithdrawDraft, WorkflowChangeRef};
use store::{AttachmentState, Holder, KeptAttachment, KeptSession};

use crate::daemon::Fleet;
use crate::pull_requesting::PR;

const DRAFT_UNNAMED: &str = "fleet.draft_unnamed";
const DRAFT_EMPTY: &str = "fleet.draft_empty";
const DRAFT_PULL_REQUEST_NOT_HELD: &str = "fleet.draft_pull_request_not_held";

/// The ledger row linking an open draft to the pull request it describes. Its target is the draft's
/// row id and its repository the pull request's; `detail` holds what `PullRequestDraft` serves, so
/// a record is drawn without reading the thread. **Given back when the draft is settled.**
pub(crate) const PR_DRAFT: &str = "pr_draft";
/// On a `pr_draft` row: the pull request's number, the draft's text, the text it replaces and its revision.
pub(crate) const PR_DRAFT_NUMBER: &str = "pull_request";
pub(crate) const PR_DRAFT_TEXT: &str = "text";
pub(crate) const PR_DRAFT_BEFORE: &str = "before";
pub(crate) const PR_DRAFT_REVISION: &str = "revision";

/// A description as it is compared with a draft's `before`: whitespace runs as one space, none at either end.
fn normalised(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The pull request a draft describes, of the `pr` rows a Job has not given back: the one it names,
/// else the newest whose body is its `before`. **`Err` with the number where it names one not held.**
fn described(held: &[KeptAttachment], named: Option<u64>, before: Option<&str>) -> Result<Option<KeptAttachment>, u64> {
    let mut pulls = held.iter().filter(|one| one.kind == PR && one.state != AttachmentState::GivenBack);
    if let Some(number) = named {
        return pulls
            .find(|one| one.target.trim_start_matches('#').parse::<u64>() == Ok(number))
            .cloned()
            .map(Some)
            .ok_or(number);
    }
    let Some(before) = before.map(normalised).filter(|said| !said.is_empty()) else {
        return Ok(None);
    };
    Ok(pulls.find(|one| one.detail.get("body").is_some_and(|body| normalised(body) == before)).cloned())
}

/// The `pr_draft` row for draft `row` on `pull`: its revision one more than the Job's earlier
/// drafts for the same pull request, settled ones too.
fn link_of(session: &str, row: &str, pull: &KeptAttachment, held: &[KeptAttachment], draft: (&str, Option<&str>), now: &str) -> KeptAttachment {
    let number = pull.target.trim_start_matches('#').to_string();
    let earlier = held
        .iter()
        .filter(|one| one.kind == PR_DRAFT && one.manifest_id == pull.manifest_id)
        .filter(|one| one.detail.get(PR_DRAFT_NUMBER) == Some(&number))
        .count();
    let mut detail = std::collections::BTreeMap::from([
        (PR_DRAFT_NUMBER.to_string(), number),
        (PR_DRAFT_TEXT.to_string(), draft.0.to_string()),
        (PR_DRAFT_REVISION.to_string(), (earlier + 1).to_string()),
    ]);
    if let Some(before) = draft.1 {
        detail.insert(PR_DRAFT_BEFORE.to_string(), before.to_string());
    }
    KeptAttachment {
        holder: Holder::session(session),
        kind: PR_DRAFT.to_string(),
        manifest_id: pull.manifest_id.clone(),
        target: row.to_string(),
        state: AttachmentState::Standing,
        detail,
        since: now.to_string(),
        changed_at: now.to_string(),
    }
}

/// A `pr_draft` row as the wire carries it. None where it names no pull request.
pub(crate) fn pull_request_draft(one: &KeptAttachment) -> Option<ipc::PullRequestDraft> {
    let detail = |key: &str| one.detail.get(key);
    Some(ipc::PullRequestDraft {
        manifest_id: ipc::ManifestId::carried(&one.manifest_id),
        number: detail(PR_DRAFT_NUMBER)?.parse().ok()?,
        draft: one.target.clone(),
        text: detail(PR_DRAFT_TEXT).cloned().unwrap_or_default(),
        before: detail(PR_DRAFT_BEFORE).cloned(),
        revision: detail(PR_DRAFT_REVISION).and_then(|said| said.parse().ok()).unwrap_or(1),
    })
}
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

/// His notes on a proposed workflow: it is proposed again with `propose_new_workflow`.
pub(crate) fn workflow_sent_back(title: &str, note: &str) -> String {
    format!("Sent back: {title}\n{note}\nRewrite it and propose it again with propose_new_workflow.")
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
        let held = {
            let store = self.store().lock().await;
            if store.session(&id).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(self.hosted_refusal(DRAFT_UNNAMED, "a job Fleet knows"));
            }
            store.attachments_of(&Holder::session(&id)).map_err(|why| self.ledger_fault(why))?
        };
        let pull = described(&held, draft.pull_request, before.as_deref()).map_err(|number| {
            self.hosted_refusal(
                DRAFT_PULL_REQUEST_NOT_HELD,
                &format!("this job holds no pull request #{number}. Name one it opened or claimed, or leave pull_request out"),
            )
        })?;
        let row = self.row_id(&id);
        let link = pull.map(|pull| link_of(&id, &row, &pull, &held, (&text, before.as_deref()), self.instant().as_str()));
        self.row_put(
            &id,
            SessionRow::Draft {
                id: row,
                at: self.instant(),
                title,
                text,
                before,
                settled: None,
                change: None,
            },
        )
        .await;
        if let Some(link) = link {
            let _ = self.store().lock().await.attach(&link, false);
        }
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
        // A settled draft waits on nobody, so it is no longer any pull request's.
        {
            let mut store = self.store().lock().await;
            let link = store.attachments_of(&Holder::session(session)).ok().and_then(|held| {
                held.into_iter()
                    .find(|one| one.kind == PR_DRAFT && one.target == row_id && one.state == AttachmentState::Standing)
            });
            if let Some(link) = link {
                let now = self.instant().as_str().to_string();
                let _ = store.attach(&KeptAttachment { state: AttachmentState::GivenBack, changed_at: now, ..link }, false);
            }
        }
        self.draft_record(session).await
    }

    /// A workflow a Job proposed waits on the owner as a draft that names its change. The item
    /// leaves the list when the change is no longer ready, whoever settled it.
    pub(crate) async fn proposal_waits(&self, session: &str, change: i64, workflow: &str, definition: &str, before: Option<String>) {
        self.proposal_titled(session, change, format!("Workflow {workflow}"), definition, before).await;
    }

    /// A proposal that waits as a draft named `title`: a rule or a lifecycle is named for what it is, as a
    /// workflow is.
    pub(crate) async fn proposal_titled(&self, session: &str, change: i64, title: String, definition: &str, before: Option<String>) {
        self.row_put(
            session,
            SessionRow::Draft {
                id: self.row_id(session),
                at: self.instant(),
                title,
                text: definition.to_string(),
                before,
                settled: None,
                change: Some(change),
            },
        )
        .await;
        let _ = self.draft_record(session).await;
    }

    /// His answer on a proposed workflow: the change saved where `how` approves it, put aside otherwise, then the Job told.
    /// Boxed here: built inline, its futures put `settle_answer`'s debug frame past a test thread's 2 MiB stack.
    pub(crate) fn proposal_answered<'a>(
        self: Arc<Self>,
        session: &'a str,
        row: &'a str,
        change: i64,
        how: DraftHow,
        words: Option<String>,
        (terminal, sent): (Option<KeptSession>, SendSessionMessage),
    ) -> Pin<Box<impl Future<Output = Result<SessionRecord, Refusal>> + 'a>> {
        Box::pin(async move {
            let change = WorkflowChangeRef { change };
            match how {
                DraftHow::Approved => self.proposal_approved(change).await?,
                _ => self.proposal_dismissed(change).await?,
            }
            Arc::clone(&self).send_known(terminal, sent).await?;
            self.draft_settled(session, row, how, words).await
        })
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
