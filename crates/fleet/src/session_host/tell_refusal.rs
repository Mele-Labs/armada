//! The language model's read of whether a Job's reply turns down a Fleet tell, and the sentence that does.
//! `docs/spikes/tell-refusal-reader.md` has the head measured, and why it is not in force.
//!
//! **Only the first reply after a tell is read** (`store::KeptTell::first_end`), and only for the tells
//! whose refusal he would want to rule on: a stranded slot, the context handoff and the subagent burn.
//! The behind-main notices are left out; refusing those is almost always right. A refusal raises a
//! `refused:<tell sent at>` item, so one tell is one item however many replies follow.
//!
//! Reads are kept by the reply's text in their own [`Reads`], apart from the asks reader's, since the
//! same text is read for two things.

use adapter_traits::{AgentHarness, Delivery, NotEmbedded, TextModel, TextReading, Vcs, WorkProduct};

use super::asks;
use super::asks_model::ModelRead;
use crate::daemon::Fleet;

/// Whether the refusal reader is in force. **It is not**: out of fold, on the in-scope tells, it
/// caught no refusal, so a call from it would be a guess. The reader and its record are built, so
/// putting it in force is this one line, once `docs/spikes/tell-refusal-reader.md` shows precision
/// of 0.75 or more with a refusal caught.
pub(crate) const IN_FORCE: bool = false;

/// The probability at and above which a reply reads as turning the tell down: the lowest at which
/// no wrong call was made out of fold over every tell kind, 213 replies.
pub(crate) const REFUSED_AT: f32 = 0.95;

/// The closing sentences scored for the one to put in front of the person.
const SENTENCES: usize = 6;

/// The tells that are read, as `store::KeptTell::kind` spells them.
pub(crate) const STRANDED_SLOT: &str = "stranded_slot";
pub(crate) const CONTEXT: &str = "context";
pub(crate) const SUBAGENTS: &str = "subagents";

/// What the model made of one reply: how likely it refuses, and its closing sentence most like a refusal.
pub(crate) fn read(model: &dyn TextModel, reply: &str) -> Result<ModelRead, NotEmbedded> {
    let sentences = asks::closing_sentences(reply);
    let last = &sentences[sentences.len().saturating_sub(SENTENCES)..];
    let texts: Vec<&str> = std::iter::once(reply).chain(last.iter().map(String::as_str)).collect();
    let scores = model.read(TextReading::TellRefused, &texts)?;
    let line = last.iter().zip(&scores[1..]).max_by(|a, b| a.1.total_cmp(b.1)).map(|(sentence, _)| sentence.clone());
    Ok(ModelRead { score: scores[0], line })
}

/// The sentence that says no, if the model reads the reply as refusing the tell. A refusal with no
/// sentence to point at is shown by the reply's first.
pub(crate) fn refusal(read: &ModelRead, reply: &str) -> Option<String> {
    (read.score >= REFUSED_AT).then(|| read.line.clone().unwrap_or_else(|| reply.to_string()))
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
    /// Keep that `session` was just sent a tell of `kind` (one of the constants above), after it took the words.
    pub(crate) async fn tell_kept(&self, session: &str, kind: &str) {
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.keep_tell(session, kind, &at);
    }
}
