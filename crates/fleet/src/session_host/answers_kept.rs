//! His answer to each ask, kept as one ledger row of kind `answer` per answer and served as `SessionRecord.answers`.
//! **Kept apart from what the Job is told**, so a thread card draws the question, how he answered and his words
//! with none of the Job's instruction. The answered item has already left `waiting_for`.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{AnswerExtras, AnswerHow, AnsweredAsk};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;

/// The ledger kind of a kept answer.
pub(crate) const ANSWER: &str = "answer";

const ITEM: &str = "item";
const QUESTION: &str = "question";
const HOW: &str = "how";
const WORDS: &str = "words";
const PICK: &str = "pick";
const ARTIFACT: &str = "artifact";
const PART: &str = "part";
const NOTES: &str = "notes";
const EXTRAS: &str = "extras";

/// What the person did, before Fleet stamps when.
pub(crate) struct Answered<'a> {
    pub item: &'a str,
    pub question: &'a str,
    pub how: AnswerHow,
    pub words: Option<&'a str>,
    pub pick: Option<&'a str>,
    pub artifact: Option<&'a str>,
    pub part: Option<&'a str>,
    pub notes: &'a BTreeMap<String, String>,
    pub extras: &'a AnswerExtras,
}

impl<'a> Answered<'a> {
    /// An answer with nothing but its item, question and how; the rest is set with struct update.
    pub(crate) fn to(item: &'a str, question: &'a str, how: AnswerHow) -> Answered<'a> {
        static NONE: BTreeMap<String, String> = BTreeMap::new();
        static NO_EXTRAS: AnswerExtras = AnswerExtras { attachments: Vec::new(), mentions: Vec::new() };
        Answered { item, question, how, words: None, pick: None, artifact: None, part: None, notes: &NONE, extras: &NO_EXTRAS }
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
    /// Keeps his answer on the Job's ledger. **Best effort**: the Job was already told and the item is gone, so a
    /// ledger that will not take the row costs the card its answer and nothing else.
    pub(crate) async fn answer_kept(&self, id: &str, said: Answered<'_>) {
        let at = self.now().as_str().to_string();
        let mut detail = BTreeMap::from([
            (ITEM.to_string(), said.item.to_string()),
            (QUESTION.to_string(), said.question.to_string()),
            (HOW.to_string(), said.how.as_str().to_string()),
        ]);
        let some = [(WORDS, said.words), (PICK, said.pick), (ARTIFACT, said.artifact), (PART, said.part)];
        for (key, value) in some {
            if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
                detail.insert(key.to_string(), value.to_string());
            }
        }
        if !said.notes.is_empty() {
            if let Ok(notes) = ipc::encode(said.notes) {
                detail.insert(NOTES.to_string(), notes);
            }
        }
        if !said.extras.is_empty() {
            if let Ok(extras) = ipc::encode(said.extras) {
                detail.insert(EXTRAS.to_string(), extras);
            }
        }
        let row = KeptAttachment {
            holder: Holder::session(id),
            kind: ANSWER.to_string(),
            manifest_id: String::new(),
            // One row per answer: the same item answered again, as a redrawn sketch is, keeps both.
            target: format!("{}@{at}", said.item),
            state: AttachmentState::Standing,
            detail,
            since: at.clone(),
            changed_at: at,
        };
        let _ = self.store().lock().await.attach(&row, false);
    }
}

/// The answers among a Job's ledger rows, oldest first.
pub(crate) fn answers_of(held: &[KeptAttachment]) -> Vec<AnsweredAsk> {
    let mut answers: Vec<AnsweredAsk> = held.iter().filter(|one| one.kind == ANSWER).filter_map(answer_of).collect();
    answers.sort_by(|a, b| a.at.as_str().cmp(b.at.as_str()));
    answers
}

fn answer_of(row: &KeptAttachment) -> Option<AnsweredAsk> {
    let said = |key: &str| row.detail.get(key).cloned();
    Some(AnsweredAsk {
        item: said(ITEM)?,
        question: said(QUESTION).unwrap_or_default(),
        how: AnswerHow::of(row.detail.get(HOW)?)?,
        words: said(WORDS),
        pick: said(PICK),
        artifact: said(ARTIFACT),
        part: said(PART),
        notes: row.detail.get(NOTES).and_then(|notes| ipc::decode("his notes", notes.as_bytes()).ok()).unwrap_or_default(),
        extras: row.detail.get(EXTRAS).and_then(|extras| ipc::decode("his attachments and tags", extras.as_bytes()).ok()).unwrap_or_default(),
        at: ipc::Instant::carried(&row.since),
    })
}
