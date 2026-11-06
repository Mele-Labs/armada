//! The language model's vectors of a Session's texts, read for what he marked as misread.
//! `docs/spikes/needs-you-dedup.md` has the cosines measured.
//!
//! Fleet does not fold two items into one: whether two wordings are one ask is a guess, and the
//! person sees both. What stays is the vectors, and the cosine that leaves a sentence he marked Not
//! a question, or a follow-up he marked Not deferred, off the list.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use adapter_traits::TextModel;
use ipc::{WaitingItem, WaitingSource};

/// The cosine at and above which a text is the one he marked: the lowest at which no `different` pair of
/// the 150 development pairs scored at or above it (the closest scored 0.8445), measured 21 Oct 2026
/// on the base encoder's vectors, where it caught 11 of 45 `same`.
pub const FOLDS_AT: f32 = 0.845;

/// Vectors kept; past this they are all forgotten at once, since an item stands only until it is settled.
const KEPT: usize = 512;

fn cosine(one: &[f32], other: &[f32]) -> f32 {
    one.iter().zip(other).map(|(a, b)| a * b).sum()
}

/// `items` with every sentence read from a reply that is within `at` of one he marked Not a question
/// left off: the reader called it once and he said it asks nothing. An item the agent stated stands, and
/// one with no vector yet is kept this read.
pub fn left_off_as_misread(items: Vec<WaitingItem>, misreads: &[String], vector: impl Fn(&str) -> Option<Vec<f32>>, at: f32) -> Vec<WaitingItem> {
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
pub struct Vectors {
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
    pub fn known(self: &Arc<Self>, model: Arc<dyn TextModel>, session: &str, texts: &[&str]) -> HashMap<String, Vec<f32>> {
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
            inner.keep(missing, embedded);
            inner.settled.push(session);
        });
        known
    }

    /// The Sessions whose items the model embedded since the last call, once each.
    pub fn settled(&self) -> Vec<String> {
        let mut settled = std::mem::take(&mut self.inner.lock().expect("never poisoned").settled);
        settled.sort();
        settled.dedup();
        settled
    }
}

impl Inner {
    /// Keep `embedded` by the keys of `missing`, which are no longer being embedded. The store is
    /// cleared where it would pass [`KEPT`].
    fn keep(&mut self, missing: Vec<(u64, String)>, embedded: Vec<Vec<f32>>) {
        if self.kept.len() + missing.len() > KEPT {
            self.kept.clear();
        }
        for ((key, _), vector) in missing.into_iter().zip(embedded) {
            self.going.remove(&key);
            self.kept.insert(key, vector);
        }
    }
}

fn key(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
