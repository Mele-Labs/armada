//! Each message of his, sorted by a small model after it is delivered. `docs/concepts/job-story.md`, *Sorting a message*.
//!
//! **Never in the way of the message.** Delivery writes one row (`message_arrived`) and goes on. The notice loop takes
//! the rows waiting a few at a time and reads each on a task of its own, so a slow or failed read costs the thread
//! nothing: a failure drops the row and the message stays unsorted, with no chip, for him to sort with one press.
//! A row taken by a read that Fleet died under stays `sorting` and is never read again.
//!
//! **The model is `models.dispute`'s**, the cheapest on the roster, since the read is one short answer a message.
//! Naming its own dial is the owner's to decide.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Model, Vcs, WorkProduct};
use config::settings as keys;
use ipc::{SessionRow, SessionVoice};
use store::{message_kind, sorted_by, KeptMessage};
use tokio::task::JoinHandle;

use super::sorting::{self, OpenAsk, Turn};
use crate::daemon::Fleet;
use crate::judging::watched;

/// How many waiting messages one pass of the loop takes.
const AT_ONCE: u32 = 3;

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
    /// A message of his was delivered to `session`: queued to be sorted. **One quick write and nothing awaited on the model.**
    pub(crate) async fn message_arrived(&self, session: &str, row_id: &str, words: &str) {
        let at = self.now().as_str().to_string();
        let message = KeptMessage {
            session_id: session.to_string(),
            row_id: row_id.to_string(),
            kind: message_kind::UNSORTED.to_string(),
            ask_id: None,
            words: words.trim().to_string(),
            at: at.clone(),
            sorted_by: sorted_by::FLEET.to_string(),
            sorted_at: at,
        };
        if let Err(why) = self.store().lock().await.keep_message_unsorted(&message) {
            self.session_warned(session, "a message of his was not queued to be sorted", format!("{why:?}"));
        }
    }

    /// One pass of the notice loop: the messages waiting are read, each on its own task, and none is awaited.
    pub(crate) async fn messages_sorting(self: &Arc<Self>) {
        drop(self.messages_taken_to_sort().await);
    }

    /// The messages waiting, taken and each given a task. **Taken once**: a second pass finds none of them.
    pub(crate) async fn messages_taken_to_sort(self: &Arc<Self>) -> Vec<JoinHandle<()>> {
        let at = self.now().as_str().to_string();
        let Ok(taken) = self.store().lock().await.take_unsorted(AT_ONCE, &at) else { return Vec::new() };
        taken
            .into_iter()
            .map(|message| {
                let fleet = Arc::clone(self);
                tokio::spawn(async move { fleet.message_read(message).await })
            })
            .collect()
    }

    async fn message_read(self: Arc<Self>, message: KeptMessage) {
        let session = message.session_id.as_str();
        let (asks, turns, corrections) = {
            let asks = self.store().lock().await.asks_of_session(session);
            let corrections = self.store().lock().await.corrections(sorting::EXAMPLES);
            (asks, self.turns_before(session, &message.row_id).await, corrections)
        };
        let (Ok(asks), Ok(corrections)) = (asks, corrections) else {
            let _ = self.store().lock().await.drop_message_read(session, &message.row_id);
            return;
        };
        let open: Vec<OpenAsk> = asks.iter().filter_map(open_of).collect();
        let question = sorting::question(&message.words, &open, &turns, &corrections);
        let read = match self.story_asked(&question).await {
            Ok(reply) => sorting::read_of(&reply, &message.words, &open),
            Err(why) => {
                self.session_warned(session, "a message of his was not sorted", &why);
                None
            }
        };
        match read {
            Some(sorted) => {
                if let Err(refusal) = self.sorting_kept(&message, sorted, store::sorted_by::FLEET).await {
                    self.session_warned(session, "how a message was sorted was not kept", format!("{refusal:?}"));
                    let _ = self.store().lock().await.drop_message_read(session, &message.row_id);
                }
            }
            None => {
                let _ = self.store().lock().await.drop_message_read(session, &message.row_id);
            }
        }
    }

    /// The turns of the thread before `row_id`, the last few, his and the agent's.
    async fn turns_before(&self, session: &str, row_id: &str) -> Vec<Turn> {
        let Ok(rows) = self.rows_of(session).await else { return Vec::new() };
        let before = rows.iter().position(|row| row.id() == row_id).unwrap_or(rows.len());
        let turns: Vec<Turn> = rows[..before]
            .iter()
            .filter_map(|row| match row {
                SessionRow::Message { from: SessionVoice::You, text, notice: None, .. } => Some(Turn { person: true, text: text.clone() }),
                SessionRow::Message { from: SessionVoice::Agent, text, .. } => Some(Turn { person: false, text: text.clone() }),
                _ => None,
            })
            .collect();
        let skipped = turns.len().saturating_sub(sorting::TURNS);
        turns.into_iter().skip(skipped).collect()
    }

    /// `question` put once to the model, through the Judge's client and the proposer's budget. Whatever goes wrong is words for the log.
    pub(crate) async fn story_asked(&self, question: &str) -> Result<String, String> {
        let proposing = self.proposing().map_err(|why| format!("{why:?}"))?;
        let named = self.machine_settings().now().get(keys::MODELS_DISPUTE);
        let model = Model::named_at(&named, None).unwrap_or_else(|_| proposing.model.clone());
        let ask = Ask::put(model, question, proposing.environment.clone()).map_err(|_| String::from("there was nothing to ask"))?;
        watched(proposing.client.as_ref(), &ask, proposing.budget, &|_| {}, std::future::pending::<()>())
            .await
            .map_err(|why| why.to_string())
    }
}

/// An ask a message may still join: not built, not dropped.
pub(super) fn open_of(ask: &store::KeptAsk) -> Option<OpenAsk> {
    if ask.state == store::ask_state::BUILT || ask.state == store::ask_state::DROPPED {
        return None;
    }
    Some(OpenAsk { id: ask.ask_id.clone(), kind: kind_from(&ask.kind)?, title: ask.title.clone() })
}

pub(super) fn kind_from(word: &str) -> Option<ipc::AskKind> {
    sorting::ask_kind_of(word)
}
