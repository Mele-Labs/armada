//! Items on a Session's Needs you list that are one ask, folded to one row, read by the language model.
//! `docs/spikes/needs-you-dedup.md` has the cosines measured.
//!
//! **The same ask reaches the list by more than one door.** An agent lists an item with `waiting_for`
//! and its reply's close asks the same thing, which Fleet raises as `replied:<turn ended>`; or an agent
//! item restates a walk or the plan Fleet already lists in its own words. `waiting.rs` drops an agent
//! item that names the same act and target as Fleet's, and the rules know walks; the rest is worded a
//! dozen ways, so the model reads each pair of items by the cosine of their texts.
//!
//! **Fleet's own item stands, then the agent's, then the reply's.** Fleet's is the one with an act
//! behind it (a window, a plan, a card), the agent's is the one it chose to state, and the reply's
//! sentence is the fallback for an agent that called nothing. Within one of those, an item with an act
//! stands over one without, then the earlier. The kept item is kept as it was. An item the model has
//! not embedded yet is kept as is this read, and its Session is published again once it is.
//!
//! **Greedy by precedence.** Sorted by precedence, each item is dropped where it is within [`FOLDS_AT`]
//! of one already kept, so a chain of near items folds into its best member rather than into each other.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use adapter_traits::TextModel;
use ipc::{WaitingItem, WaitingSource};

/// Whether this reader is in force, and so folds with the model. It is: on the 150 held-out pairs
/// the threshold below folded 6 of the 44 that were one ask and merged 1 of the 106 that were not,
/// and the owner picked it on 22 Oct 2026.
pub(crate) const IN_FORCE: bool = true;

/// The cosine at and above which two items are one ask: the lowest at which no `different` pair of
/// the 150 development pairs scored at or above it (the closest scored 0.8445), measured 21 Oct 2026
/// on the base encoder's vectors, where it caught 11 of 45 `same`.
pub(crate) const FOLDS_AT: f32 = 0.845;

/// Vectors kept; past this they are all forgotten at once, since an item stands only until it is settled.
const KEPT: usize = 512;

/// Which item stands when two are one ask: lower is kept. Fleet's own, then the agent's, then the
/// reply's sentence; within each, one with an act before one without.
fn precedence(item: &WaitingItem) -> (u8, u8) {
    let class = match item.source {
        WaitingSource::Agent => 1,
        WaitingSource::Replied => 2,
        _ => 0,
    };
    (class, u8::from(item.act.is_none()))
}

fn cosine(one: &[f32], other: &[f32]) -> f32 {
    one.iter().zip(other).map(|(a, b)| a * b).sum()
}

/// `items` with every one that is one ask with a higher-precedence item left off, in the order given.
/// `vector` is each text's unit vector, or `None` for one not embedded yet, which is kept as is and
/// folds nothing. Two items fold where their cosine is `at` or above.
pub(crate) fn folded(items: Vec<WaitingItem>, vector: impl Fn(&str) -> Option<Vec<f32>>, at: f32) -> Vec<WaitingItem> {
    let vectors: Vec<Option<Vec<f32>>> = items.iter().map(|item| vector(&item.text)).collect();
    let mut order: Vec<usize> = (0..items.len()).collect();
    // Stable, so two items alike in every rank stay in the list's own order.
    order.sort_by(|&a, &b| precedence(&items[a]).cmp(&precedence(&items[b])).then_with(|| items[a].since.as_str().cmp(items[b].since.as_str())));
    let mut kept: Vec<usize> = Vec::with_capacity(items.len());
    for at_item in order {
        // One command listed twice is one ask whatever each says around it.
        let same_act = |other: usize| items[at_item].act.as_ref().is_some_and(|own| items[other].act.as_ref().is_some_and(|theirs| own.kind == theirs.kind && own.target == theirs.target && !own.target.is_empty()));
        let folds = kept.iter().any(|&other| same_act(other))
            || match &vectors[at_item] {
                Some(own) => kept.iter().any(|&other| vectors[other].as_ref().is_some_and(|theirs| cosine(own, theirs) >= at)),
                None => false,
            };
        if !folds {
            kept.push(at_item);
        }
    }
    kept.sort_unstable();
    let mut kept = kept.into_iter().peekable();
    items
        .into_iter()
        .enumerate()
        .filter(|(index, _)| {
            if kept.peek() == Some(index) {
                kept.next();
                true
            } else {
                false
            }
        })
        .map(|(_, item)| item)
        .collect()
}

/// `items` with every sentence read from a reply that is within `at` of one he marked Not a question
/// left off: the reader called it once and he said it asks nothing. An item the agent stated stands, and
/// one with no vector yet is kept this read.
pub(crate) fn left_off_as_misread(items: Vec<WaitingItem>, misreads: &[String], vector: impl Fn(&str) -> Option<Vec<f32>>, at: f32) -> Vec<WaitingItem> {
    let marked: Vec<Vec<f32>> = misreads.iter().filter_map(|one| vector(one)).collect();
    items
        .into_iter()
        .filter(|item| {
            item.source != WaitingSource::Replied
                || !vector(&item.text).is_some_and(|own| marked.iter().any(|theirs| cosine(&own, theirs) >= at))
        })
        .collect()
}

/// The model's vectors of items' texts, and the Sessions whose texts were embedded since the last look.
/// Modelled on `asks_model::Reads`: kept by the text's hash, embedded in one batch per read on a
/// blocking thread, and the Session published again once they are in.
#[derive(Default)]
pub(crate) struct Vectors {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    kept: HashMap<u64, Vec<f32>>,
    going: HashSet<u64>,
    settled: Vec<String>,
}

impl Vectors {
    /// The vectors of those of `texts` the model has embedded, by text. The rest are embedded in one
    /// batch on a blocking thread, so the next read has them, and `session` is named by
    /// [`Self::settled`] once they are in. A batch that failed stays in `going`, so it is not tried
    /// again on every read.
    pub(crate) fn known(self: &Arc<Self>, model: Arc<dyn TextModel>, session: &str, texts: &[&str]) -> HashMap<String, Vec<f32>> {
        let mut inner = self.inner.lock().expect("never poisoned");
        let mut known = HashMap::new();
        let mut missing: Vec<(u64, String)> = Vec::new();
        for text in texts {
            let key = key(text);
            match inner.kept.get(&key) {
                Some(vector) => {
                    known.insert((*text).to_string(), vector.clone());
                }
                None => {
                    if inner.going.insert(key) {
                        missing.push((key, (*text).to_string()));
                    }
                }
            }
        }
        drop(inner);
        if missing.is_empty() {
            return known;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return known;
        };
        let (vectors, session) = (Arc::clone(self), session.to_string());
        runtime.spawn_blocking(move || {
            let texts: Vec<&str> = missing.iter().map(|(_, text)| text.as_str()).collect();
            let Ok(embedded) = model.embed(&texts) else {
                return;
            };
            if embedded.len() != missing.len() {
                return;
            }
            let mut inner = vectors.inner.lock().expect("never poisoned");
            if inner.kept.len() + missing.len() > KEPT {
                inner.kept.clear();
            }
            for ((key, _), vector) in missing.into_iter().zip(embedded) {
                inner.going.remove(&key);
                inner.kept.insert(key, vector);
            }
            inner.settled.push(session);
        });
        known
    }

    /// The Sessions whose items the model embedded since the last call, once each.
    pub(crate) fn settled(&self) -> Vec<String> {
        let mut settled = std::mem::take(&mut self.inner.lock().expect("never poisoned").settled);
        settled.sort();
        settled.dedup();
        settled
    }
}

fn key(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
