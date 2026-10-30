//! `answer_document`: the person approves the newest revision of a page a Job wrote under
//! `.armada/documents/`, or sends it back. `crates/ipc/operations/answer_document.toml` is the contract.
//!
//! **Asked off the page Fleet kept**, as `documents` writes `asked` and `revision` on each new
//! revision; never off the agent's tool call. It waits in Needs you as `document:<path>`
//! ([`document_items`]) until he answers it or dismisses it, and the rail reads an approval off the
//! kept detail ([`approved_on_newest`]), so approving an older revision ends no step.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    AnswerDocument, AttachmentReport, DocumentHow, SendSessionMessage, SessionFact, SessionId, SessionRecord, SessionReport,
    WaitingAct, WaitingActKind, WaitingItem, WaitingSource, WireError, DOCUMENT_ASKED, DOCUMENT_ITEM_PREFIX, DOCUMENT_REVISION,
    DOCUMENT_SETTLED, DOCUMENT_SETTLED_AT, DOCUMENT_WORDS,
};

use crate::daemon::Fleet;

const DOCUMENT_NOT_OPEN: &str = "fleet.document_not_open";
const DOCUMENT_REVISED: &str = "fleet.document_revised";
const DOCUMENT_ANSWER_EMPTY: &str = "fleet.document_answer_empty";
const DOCUMENT_ANSWER_UNNAMED: &str = "fleet.document_answer_unnamed";

/// The person's message for each answer to a document. Words with an approval go on a line after it.
pub(crate) fn approved(title: &str, revision: &str, words: Option<&str>) -> String {
    let mut body = format!("Approved document: {title}, revision {revision}\nGo on from it as written.");
    if let Some(words) = words {
        body.push('\n');
        body.push_str(words);
    }
    body
}

pub(crate) fn sent_back(title: &str, revision: &str, note: &str, path: &str) -> String {
    format!("Sent back document: {title}, revision {revision}\n{note}\nRevise the page at {path} and write it again.")
}

fn is_document(detail: &BTreeMap<String, String>) -> bool {
    detail.get("form").map(String::as_str) == Some("document")
}

/// Whether a document's kept detail says he approved its newest revision. A new revision drops
/// `settled`, and `revision` is checked against the revisions kept besides.
pub(crate) fn approved_on_newest(detail: &BTreeMap<String, String>) -> bool {
    let newest = detail
        .get("revisions")
        .and_then(|kept| ipc::decode::<Vec<String>>("a document's revisions", kept.as_bytes()).ok())
        .map(|kept| kept.len().to_string());
    is_document(detail)
        && detail.get(DOCUMENT_ASKED).map(String::as_str) == Some("true")
        && detail.get(DOCUMENT_SETTLED).map(String::as_str) == Some(DocumentHow::Approved.word())
        && newest.is_some()
        && detail.get(DOCUMENT_REVISION) == newest.as_ref()
}

/// Every document whose newest revision was put to the person and not answered, as the item that
/// waits on him: id `document:<path>`, text its title, opening the Job.
pub(crate) fn document_items<'a>(attachments: &'a [ipc::Attachment], session: &'a str) -> impl Iterator<Item = WaitingItem> + 'a {
    attachments
        .iter()
        .filter(|one| one.kind == "artifact" && is_document(&one.detail))
        .filter(|one| one.detail.get(DOCUMENT_ASKED).map(String::as_str) == Some("true") && !one.detail.contains_key(DOCUMENT_SETTLED))
        .map(move |one| WaitingItem {
            id: format!("{DOCUMENT_ITEM_PREFIX}{}", one.target),
            text: one.detail.get("title").cloned().unwrap_or_else(|| one.target.clone()),
            since: one.changed_at.clone(),
            source: WaitingSource::Document,
            act: Some(WaitingAct { kind: WaitingActKind::Message, target: session.to_string() }),
            options: Vec::new(),
            steps: Vec::new(),
            place: None,
            concern: None,
            cites: Vec::new(),
            confirmed: false,
        })
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
    /// `answer_document`: the Job is told as his message first, so a failed send leaves the revision
    /// open, then `detail` keeps how it ended.
    pub(crate) async fn document_answered(self: Arc<Self>, said: AnswerDocument) -> Result<SessionRecord, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let named = said.document.trim();
        let path = named.strip_prefix(DOCUMENT_ITEM_PREFIX).unwrap_or(named).to_string();
        let item = format!("{DOCUMENT_ITEM_PREFIX}{path}");
        let revision = said.revision.trim().to_string();
        let words = said.words.as_deref().map(str::trim).filter(|words| !words.is_empty());
        if said.how == DocumentHow::SentBack && words.is_none() {
            return Err(self.hosted_refusal(DOCUMENT_ANSWER_EMPTY, "a document is sent back with a note saying what to change"));
        }
        let terminal = self.terminal_session(&id).await?;
        let not_open = |why: String| self.hosted_refusal(DOCUMENT_NOT_OPEN, &why);
        // Held as a sketch's answer is, so two presses cannot both settle it.
        let _settling = self
            .hosts()
            .settling(&id, &item)
            .ok_or_else(|| not_open(format!("the document `{path}` is being answered already")))?;
        let (harness, held) = self.document_held(&id, &path).await?;
        let document = held.ok_or_else(|| not_open(format!("this job holds no document at `{path}`")))?;
        let title = document.detail.get("title").cloned().unwrap_or_else(|| path.clone());
        if document.detail.get(DOCUMENT_ASKED).map(String::as_str) != Some("true") {
            return Err(not_open(format!("the document `{title}` was kept before documents asked, and is not put to the person")));
        }
        if document.detail.get(DOCUMENT_REVISION) != Some(&revision) {
            return Err(self.revised(&title, &revision));
        }
        if let Some(how) = document.detail.get(DOCUMENT_SETTLED) {
            return Err(not_open(format!("revision {revision} of `{title}` was {} already", how.replace('_', " "))));
        }
        let body = match said.how {
            DocumentHow::Approved => approved(&title, &revision, words),
            DocumentHow::SentBack => sent_back(&title, &revision, words.unwrap_or_default(), &path),
        };
        let message = SendSessionMessage {
            session_id: said.session_id.clone(),
            text: body,
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        Arc::clone(&self).send_known(terminal, message).await?;
        // Read again after the send: a revision the Job kept meanwhile is not the one he answered.
        let (_, now) = self.document_held(&id, &path).await?;
        let Some(mut document) = now.filter(|one| one.detail.get(DOCUMENT_REVISION) == Some(&revision)) else {
            return self.published_hosted(&id).await;
        };
        document.detail.insert(DOCUMENT_SETTLED.into(), said.how.word().into());
        document.detail.insert(DOCUMENT_SETTLED_AT.into(), self.instant().as_str().to_string());
        if let Some(words) = words {
            document.detail.insert(DOCUMENT_WORDS.into(), words.to_string());
        }
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached {
                attachment: AttachmentReport { kind: document.kind, target: document.target, detail: document.detail },
            },
        })
        .await
    }

    /// The Job's harness, and the document it holds at `path`.
    async fn document_held(&self, id: &str, path: &str) -> Result<(String, Option<ipc::Attachment>), Refusal> {
        let store = self.store().lock().await;
        let session = store
            .session(id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(DOCUMENT_ANSWER_UNNAMED, "a job Fleet knows"))?;
        let held = self
            .ledger_row(&store, &session)?
            .attachments
            .into_iter()
            .find(|one| one.kind == "artifact" && one.target == path && is_document(&one.detail));
        Ok((session.harness, held))
    }

    fn revised(&self, title: &str, read: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            DOCUMENT_REVISED,
            format!("revision {read} of `{title}` is not the newest: the job has revised the page since"),
            self.run_id(),
        ))
    }
}
