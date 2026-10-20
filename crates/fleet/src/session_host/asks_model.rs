//! The language model's read of whether a reply asks the person, beside the rules of [`super::asks`].
//! `docs/spikes/reply-asks-classifier.md` has the three readers measured on replies none had seen.
//!
//! **Read once per reply, off the async runtime.** A Session's standing is derived on every read of
//! it, so the model's read is kept by the reply's text; one not read yet is answered by the rules and
//! put to the model on a blocking thread, and its Session is published again once the read is in.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use adapter_traits::{Embedder, NotEmbedded};

use super::{asks, asks_head};

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

/// The reader in force. The rules stand in for the model until it is loaded, whichever this is. The
/// rules, since no reader kept precision at 0.90 on the held-out replies of 19 Oct 2026.
pub(crate) const SHIPPED: Reader = Reader::Rules;

/// The closing sentences scored for the one to put in front of the person.
const SENTENCES: usize = 12;
/// How many sentences at the very end the verdict is read from: the head's training input.
const VERDICT_FROM: usize = 3;
/// Reads kept; past this they are all forgotten at once, since a reply stands only until the next turn.
const KEPT: usize = 512;

/// What the model made of one reply: how likely it asks, and its sentence most like an ask.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModelRead {
    pub score: f32,
    pub line: Option<String>,
}

pub(crate) fn read(model: &dyn Embedder, reply: &str) -> Result<ModelRead, NotEmbedded> {
    let close = asks::closing_sentences(reply);
    let sentences: Vec<&str> = close[close.len().saturating_sub(SENTENCES)..].iter().map(String::as_str).collect();
    if sentences.is_empty() {
        return Ok(ModelRead { score: 0.0, line: None });
    }
    let vectors = model.embed(&sentences)?;
    let last = &vectors[vectors.len().saturating_sub(VERDICT_FROM)..];
    let mean: Vec<f32> = (0..asks_head::WEIGHTS.len()).map(|at| last.iter().map(|one| one[at]).sum::<f32>() / last.len() as f32).collect();
    let line = sentences.iter().zip(&vectors).max_by(|a, b| scored(a.1).total_cmp(&scored(b.1))).map(|(sentence, _)| sentence.to_string());
    Ok(ModelRead { score: scored(&mean), line })
}

fn scored(vector: &[f32]) -> f32 {
    let sum: f32 = asks_head::BIAS + asks_head::WEIGHTS.iter().zip(vector).map(|(weight, value)| weight * value).sum::<f32>();
    1.0 / (1.0 + (-sum).exp())
}

/// The sentence that asks, as `reader` decides from the rules' sentence and the model's read. Where
/// both call it, the rules' sentence is shown: it is the one an agent wrote to ask.
pub(crate) fn decided(reader: Reader, rules: Option<String>, model: Option<&ModelRead>) -> Option<String> {
    let Some(model) = model else {
        return rules;
    };
    match reader {
        Reader::Rules => rules,
        Reader::Model => (model.score >= asks_head::ASKS_AT).then(|| rules.or_else(|| model.line.clone())).flatten(),
        Reader::Either => rules.or_else(|| (model.score >= asks_head::BESIDE_RULES_AT).then(|| model.line.clone()).flatten()),
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
        model: Option<Arc<dyn Embedder>>,
        session: &str,
        reply: &str,
    ) -> Option<String> {
        let rules = asks::asking_line(reply);
        let Some(model) = model.filter(|model| reader.reads_model() && model.model() == asks_head::TRAINED_ON) else {
            return rules;
        };
        let key = key(reply);
        let mut inner = self.inner.lock().expect("never poisoned");
        if let Some(read) = inner.read.get(&key) {
            return decided(reader, rules, Some(read));
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return rules;
        };
        // A read that failed stays in `going`, so it is not tried again on every derive, and the rules'
        // answer stands for that reply.
        if inner.going.insert(key) {
            let (reads, session, reply) = (Arc::clone(self), session.to_string(), reply.to_string());
            runtime.spawn_blocking(move || {
                let read = read(&*model, &reply);
                let Ok(read) = read else {
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
        rules
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
