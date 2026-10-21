//! The language model's read of whether a reply asks the person, beside the rules of [`super::asks`].
//! `docs/spikes/reply-asks-classifier.md` has the readers measured on replies none had seen.
//!
//! **Read once per reply, off the async runtime.** A Session's standing is derived on every read of
//! it, so the model's read is kept by the reply's text; one not read yet is answered by the rules and
//! put to the model on a blocking thread, and its Session is published again once the read is in.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use adapter_traits::{NotEmbedded, TextModel, TextReading};

use super::asks;

/// Which reader decides whether a reply asks. Only [`SHIPPED`] is in force; the others are the
/// readers it was measured against, kept so the next measurement can put one in force.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reader {
    Rules,
    Model,
    /// An ask if either the rules or the model call it.
    Either,
}

impl Reader {
    /// Whether this reader needs the model, and so whether Fleet fetches and loads it.
    pub(crate) fn reads_model(self) -> bool {
        self != Reader::Rules
    }
}

/// The reader in force. The rules stand in for the model until it is loaded, whichever this is.
/// Either, the owner's choice of 20 Oct 2026: on the held-out replies it found 54 of 81 asks where
/// the rules found 21, for 20 wrong calls where they made 8.
pub(crate) const SHIPPED: Reader = Reader::Either;

/// The probability at and above which the model calls a reply an ask, alone or beside the rules: the
/// most recall at which the post-18-Oct training replies, read out of fold, kept precision at 0.90.
/// One figure for both, since the rules alone read only 0.82 precision on those replies, leaving no
/// room for the model to run looser beside them.
pub(crate) const ASKS_AT: f32 = 0.84;

/// The closing sentences scored for the one to put in front of the person.
const SENTENCES: usize = 6;
/// Reads kept; past this they are all forgotten at once, since a reply stands only until the next turn.
const KEPT: usize = 512;

/// What the model made of one reply: how likely it asks, and its closing sentence most like an ask.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModelRead {
    pub score: f32,
    pub line: Option<String>,
}

pub(crate) fn read(model: &dyn TextModel, reply: &str) -> Result<ModelRead, NotEmbedded> {
    let sentences = asks::closing_sentences(reply);
    if sentences.is_empty() {
        return Ok(ModelRead { score: 0.0, line: None });
    }
    let close = asks::closing_text(reply);
    let last = &sentences[sentences.len().saturating_sub(SENTENCES)..];
    let texts: Vec<&str> = std::iter::once(close.as_str()).chain(last.iter().map(String::as_str)).collect();
    let scores = model.read(TextReading::ReplyAsks, &texts)?;
    let line = last.iter().zip(&scores[1..]).max_by(|a, b| a.1.total_cmp(b.1)).map(|(sentence, _)| sentence.clone());
    Ok(ModelRead { score: scores[0], line })
}

/// The sentence that asks, as `reader` decides from the rules' sentence and the model's read. Where
/// both call it, the rules' sentence is shown: it is the one an agent wrote to ask.
pub(crate) fn decided(reader: Reader, rules: Option<String>, model: Option<&ModelRead>) -> Option<String> {
    let Some(model) = model else {
        return rules;
    };
    match reader {
        Reader::Rules => rules,
        Reader::Model => (model.score >= ASKS_AT).then(|| rules.or_else(|| model.line.clone())).flatten(),
        Reader::Either => rules.or_else(|| (model.score >= ASKS_AT).then(|| model.line.clone()).flatten()),
    }
}

/// The model's reads, by reply, and the Sessions whose read came in since the last look.
#[derive(Default)]
pub(crate) struct Reads {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    read: HashMap<u64, ModelRead>,
    going: HashSet<u64>,
    settled: Vec<String>,
}

impl Reads {
    /// The sentence of `reply` that asks, by `reader`. With no model, or a reply it has not read yet,
    /// the rules' answer; the read is started so the next derive has it.
    pub(crate) fn asking_line(
        self: &Arc<Self>,
        reader: Reader,
        model: Option<Arc<dyn TextModel>>,
        session: &str,
        reply: &str,
    ) -> Option<String> {
        let rules = asks::asking_line(reply);
        let Some(model) = model.filter(|_| reader.reads_model()) else {
            return rules;
        };
        match self.read_of(read, model, session, reply) {
            Some(read) => decided(reader, rules, Some(&read)),
            None => rules,
        }
    }

    /// The model's read of `reply` by `how`, if it has made one. Otherwise the read is started on a
    /// blocking thread, so the next call has it, and `session` is named by [`Self::settled`] once it is in.
    /// A read that failed stays in `going`, so it is not tried again on every derive.
    ///
    /// **One `Reads` serves one reading**, since a read is kept by the reply's text alone.
    pub(crate) fn read_of(
        self: &Arc<Self>,
        how: fn(&dyn TextModel, &str) -> Result<ModelRead, NotEmbedded>,
        model: Arc<dyn TextModel>,
        session: &str,
        reply: &str,
    ) -> Option<ModelRead> {
        let key = key(reply);
        let mut inner = self.inner.lock().expect("never poisoned");
        if let Some(read) = inner.read.get(&key) {
            return Some(read.clone());
        }
        let runtime = tokio::runtime::Handle::try_current().ok()?;
        if inner.going.insert(key) {
            let (reads, session, reply) = (Arc::clone(self), session.to_string(), reply.to_string());
            runtime.spawn_blocking(move || {
                let Ok(read) = how(&*model, &reply) else {
                    return;
                };
                let mut inner = reads.inner.lock().expect("never poisoned");
                if inner.read.len() >= KEPT {
                    inner.read.clear();
                }
                inner.going.remove(&key);
                inner.read.insert(key, read);
                inner.settled.push(session);
            });
        }
        None
    }

    /// The Sessions whose reply the model read since the last call, once each.
    pub(crate) fn settled(&self) -> Vec<String> {
        let mut settled = std::mem::take(&mut self.inner.lock().expect("never poisoned").settled);
        settled.sort();
        settled.dedup();
        settled
    }
}

fn key(reply: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    reply.hash(&mut hasher);
    hasher.finish()
}
