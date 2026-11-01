//! Whether his reply to a Job approves the page it put to him, read once by a cheap model call.
//!
//! **A page waits on his card** (`document_answers`), but he often answers it in the thread: "Run the
//! plan and merge it". Each pass, every page still asking that has replies of his since it was put to
//! him is read: they are marked read before the model is asked, so a pass reads only what came after.
//! One read as approving settles the page as his card's Approve does, and the plan held behind it with
//! it. The Job is not told a second time: it read his reply.
//!
//! **Only his own words are read.** A Fleet notice, an answer to one of the Job's questions and the
//! message a card's Approve or Send back sends are not his reply to the page.
//!
//! **A model call, not the local model**: the `approves` head is trained and not in force, since his
//! approvals mostly come from the card and too few were typed to learn from
//! (`docs/spikes/reply-approves-reader.md`).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use ipc::{DocumentHow, SessionId, SessionRow, SessionVoice, DOCUMENT_ASKED, DOCUMENT_ITEM_PREFIX, DOCUMENT_KEPT_AT, DOCUMENT_REVISION, DOCUMENT_SETTLED};
use store::{Holder, SessionSearch, SessionState};

use super::asking::answers_a_question;
use super::document_answers::is_document;
use super::walking::approved_address;
use crate::daemon::Fleet;

/// How often the pages still asking are looked over.
pub(crate) const EVERY: Duration = Duration::from_secs(60);

/// The most of his replies after the page that are read in one pass, newest kept.
const REPLIES: usize = 3;

/// What a card's answer sends the Job, which settles the page itself (`document_answers::approved`, `sent_back`).
const CARD_SENT: [&str; 2] = ["Approved document:", "Sent back document:"];

pub(crate) const APPROVES: &str = "approves:";

/// When his newest reply read for each page's revision was sent, so a pass reads only later ones.
/// Kept while Fleet runs: after a restart his replies are read again, and a settled page is not read.
#[derive(Default)]
pub(crate) struct Reads {
    read: Mutex<BTreeMap<String, String>>,
}

impl Reads {
    fn since(&self, key: &str) -> Option<String> {
        self.read.lock().expect("never poisoned").get(key).cloned()
    }

    fn mark(&self, key: String, at: String) {
        self.read.lock().expect("never poisoned").insert(key, at);
    }
}

/// His replies sent after `since`, oldest first, with when each was sent.
pub(crate) fn replies_after<'a>(rows: &'a [SessionRow], since: &str) -> Vec<(&'a str, &'a str)> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { id, at, from: SessionVoice::You, text, notice: None, .. }
                if at.as_str() > since
                    && !answers_a_question(id)
                    && approved_address(text).is_none()
                    && !text.trim_start().starts_with("Fleet:")
                    && !CARD_SENT.iter().any(|sent| text.trim_start().starts_with(sent)) =>
            {
                Some((text.as_str(), at.as_str()))
            }
            _ => None,
        })
        .collect()
}

/// What the model is asked.
pub(crate) fn question(title: &str, replies: &[&str]) -> String {
    let said: Vec<String> = replies.iter().enumerate().map(|(n, text)| format!("Message {}:\n{}", n + 1, text.trim())).collect();
    format!(
        "A coding agent wrote a design page titled \"{title}\" with a plan, and put it to a person to \
         approve before building. These are the person's next messages to the agent, in order:\n\n{replies}\n\n\
         Do the messages approve the page as it stands and tell the agent to go on, for example \
         \"looks good\", \"go ahead\" or \"run the plan and merge it\"? Asking a question, asking for a \
         change first, taking an approval back, or talking about something else is not approving.\n\n\
         Answer in one line and nothing else:\n\
         {APPROVES} yes or no",
        replies = said.join("\n\n"),
    )
}

/// The model's answer, read. Anything but a `yes` approves nothing.
pub(crate) fn approves(answer: &str) -> bool {
    answer.lines().any(|line| {
        let line = line.trim().to_lowercase();
        line.strip_prefix(APPROVES).is_some_and(|said| said.trim().starts_with("yes"))
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
    /// Read his replies to every page a live Job put to him and he has not answered.
    pub(crate) async fn pages_read(self: &Arc<Self>) {
        let asking: Vec<(String, String)> = {
            let store = self.store().lock().await;
            let live = store.find_sessions(&SessionSearch { state: Some(SessionState::Live), ..Default::default() }).unwrap_or_default();
            live.iter()
                .flat_map(|session| {
                    store
                        .attachments_of(&Holder::session(&session.id))
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|one| one.kind == "artifact" && is_document(&one.detail))
                        .filter(|one| one.detail.get(DOCUMENT_ASKED).map(String::as_str) == Some("true") && !one.detail.contains_key(DOCUMENT_SETTLED))
                        .map(|one| (session.id.clone(), one.target))
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        for (id, path) in asking {
            if let Err(why) = Arc::clone(self).page_replies_read(&id, &path).await {
                eprintln!("job {id}: his replies to the page `{path}` were not read: {why}");
            }
        }
    }

    async fn page_replies_read(self: Arc<Self>, id: &str, path: &str) -> Result<(), String> {
        let item = format!("{DOCUMENT_ITEM_PREFIX}{path}");
        // Held as a card's answer is, so a press and a read cannot both settle it.
        let Some(_settling) = self.hosts().settling(id, &item) else { return Ok(()) };
        let (harness, held) = self.document_held(id, path).await.map_err(|why| format!("{why:?}"))?;
        let Some(document) = held.filter(|one| !one.detail.contains_key(DOCUMENT_SETTLED)) else { return Ok(()) };
        let Some(kept_at) = document.detail.get(DOCUMENT_KEPT_AT) else { return Ok(()) };
        let revision = document.detail.get(DOCUMENT_REVISION).cloned().unwrap_or_default();
        let key = format!("{id}\n{path}\n{revision}");
        let since = self.hosts().page_reads().since(&key).filter(|read| read > kept_at).unwrap_or_else(|| kept_at.clone());
        let rows = self.rows_of(id).await.map_err(|why| format!("{why:?}"))?;
        let replies = replies_after(&rows, &since);
        let Some((_, newest)) = replies.last() else { return Ok(()) };
        self.hosts().page_reads().mark(key, newest.to_string());
        let read: Vec<&str> = replies[replies.len().saturating_sub(REPLIES)..].iter().map(|(text, _)| *text).collect();
        let title = document.detail.get("title").cloned().unwrap_or_else(|| path.to_string());
        let model = self.machine_settings().now().get(keys::MODELS_DISPUTE);
        if !approves(&self.dispute_asked(&question(&title, &read), &model).await?) {
            return Ok(());
        }
        self.page_settled(SessionId::carried(id), harness, document, DocumentHow::Approved, None).await.map(|_| ()).map_err(|why| format!("{why:?}"))
    }
}
