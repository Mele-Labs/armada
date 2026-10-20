//! The local language model reading a thought before Claude is asked, and routing it at once when one
//! live Session is a clear match. `docs/spikes/thought-routing-by-model.md` has the two figures
//! below measured.
//!
//! **It only ever says yes.** A thought no Session clearly matches, a question, a model not loaded
//! yet and any failure all come back `None`, and the call goes to Claude as it always did. So the
//! model can make a read faster; it cannot send words anywhere Claude would not have.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::TextModel;
use ipc::ThoughtTarget;

use super::Candidate;
use crate::language_model::LanguageModel;

/// Whether this reader is in force, and so whether Fleet fetches and loads the model for it. In
/// force because no direct route disagreed with Claude on the 19 reads it was measured on.
pub(crate) const IN_FORCE: bool = true;

/// The cosine similarity at and above which the best Session is a clear match: the lowest score of a
/// route that agreed with Claude, with the best one that did not at 0.74.
pub(crate) const ROUTES_AT: f32 = 0.78;

/// How far the best Session must lead the second to be the one. A single Session has no second, so
/// it leads by its whole score.
pub(crate) const LEADS_BY: f32 = 0.10;

/// The longest words read here, in characters. A longer message is a paste or several thoughts, and
/// a vector of its start says little of where all of it belongs.
const MOST_WORDS: usize = 300;

/// Each live Session's vector by (id, title), so a read embeds only the Sessions it has not seen. The
/// title is the key, so a Session that is renamed is read again.
#[derive(Default)]
pub(crate) struct Vectors {
    kept: Mutex<HashMap<(String, String), Vec<f32>>>,
}

impl Vectors {
    /// The vector of each of `candidates`, in order, and of `words` first, embedding what is not kept.
    /// What is kept afterwards is these candidates and no others, which bounds it by the twenty offered.
    fn of(&self, model: &dyn TextModel, words: &str, candidates: &[Candidate]) -> Option<(Vec<f32>, Vec<Vec<f32>>)> {
        let keys: Vec<(String, String)> = candidates.iter().map(|one| (one.id.clone(), one.title.clone())).collect();
        let mut vectors: Vec<Option<Vec<f32>>> = {
            let kept = self.kept.lock().expect("never poisoned");
            keys.iter().map(|key| kept.get(key).cloned()).collect()
        };
        let missing: Vec<usize> = (0..keys.len()).filter(|&at| vectors[at].is_none()).collect();
        let texts: Vec<&str> =
            std::iter::once(words).chain(missing.iter().map(|&at| candidates[at].title.as_str())).collect();
        let mut embedded = model.embed(&texts).ok()?.into_iter();
        if embedded.len() != texts.len() {
            return None;
        }
        let thought = embedded.next()?;
        for (&at, vector) in missing.iter().zip(embedded) {
            vectors[at] = Some(vector);
        }
        let vectors: Vec<Vec<f32>> = vectors.into_iter().collect::<Option<_>>()?;
        let mut kept = self.kept.lock().expect("never poisoned");
        kept.clear();
        kept.extend(keys.into_iter().zip(vectors.iter().cloned()));
        Some((thought, vectors))
    }
}

fn similarity(one: &[f32], other: &[f32]) -> f32 {
    one.iter().zip(other).map(|(a, b)| a * b).sum()
}

/// The index of the one `scores` name as a clear match: best at `ROUTES_AT` and ahead of the second by
/// `LEADS_BY`.
pub(crate) fn clear_match(scores: &[f32]) -> Option<usize> {
    let (best, top) = scores.iter().copied().enumerate().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let second = scores.iter().copied().enumerate().filter(|&(at, _)| at != best).map(|(_, score)| score).max_by(f32::total_cmp);
    (top >= ROUTES_AT && top - second.unwrap_or(0.0) >= LEADS_BY).then_some(best)
}

/// The Session `words` plainly belong to, if the model reads one. Blocking: it embeds.
///
/// **A message that ends in a question mark is not read here.** The model tells what a message is
/// about, not whether it asks or tells, and "is the walk variations Session done?" is about that
/// Session and is Helm's to answer.
pub(crate) fn matched(
    model: &dyn TextModel,
    vectors: &Vectors,
    words: &str,
    candidates: &[Candidate],
) -> Option<ThoughtTarget> {
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if candidates.is_empty() || words.ends_with('?') || words.chars().count() > MOST_WORDS {
        return None;
    }
    let (thought, vectors) = vectors.of(model, &words, candidates)?;
    let scores: Vec<f32> = vectors.iter().map(|one| similarity(&thought, one)).collect();
    clear_match(&scores).map(|at| candidates[at].target.clone())
}

/// [`matched`] on a thread of its own, with the model once it is loaded. `None` while it is not.
pub(crate) async fn matched_by(
    language_model: &LanguageModel,
    words: &str,
    candidates: &[Candidate],
) -> Option<ThoughtTarget> {
    if !IN_FORCE {
        return None;
    }
    let model = language_model.model()?;
    let vectors: Arc<Vectors> = language_model.thought_vectors();
    let (words, candidates) = (words.to_string(), candidates.to_vec());
    tokio::task::spawn_blocking(move || matched(&*model, &vectors, &words, &candidates)).await.ok().flatten()
}
