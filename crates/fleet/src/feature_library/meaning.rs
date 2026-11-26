//! Which features of a project some typed words are about, read by meaning with the local language model, so a feature is
//! found by what it does and not by its name. Bridge's sentence asks it beside its own match of the words.
//!
//! **It only ever adds.** The model not loaded yet, blank words and any failure all answer an empty list, and the word match
//! Bridge holds stands without it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, TextModel, Vcs, WorkProduct};
use api::Refusal;
use ipc::FeaturesByMeaning;

use crate::daemon::Fleet;

/// Whether this reader is in force, and so whether Fleet fetches and loads the model for it.
pub(crate) const IN_FORCE: bool = true;

/// The cosine similarity at and above which a feature is about the words. Measured on this library's 85 features (`title: summary`)
/// against 13 short queries, 26 Nov 2026: the feature meant scored 0.73 to 0.91, the nearest wrong one 0.58 to 0.74 and words
/// about nothing at most 0.59. At 0.70 every feature meant is kept and each query names one or two.
pub(crate) const NEAR_AT: f32 = 0.70;

/// The most features it names.
pub(crate) const MOST: usize = 8;

/// Each feature text's vector, so a read embeds only the texts it has not seen.
#[derive(Default)]
pub(crate) struct Vectors {
    kept: Mutex<HashMap<String, Vec<f32>>>,
}

fn similarity(one: &[f32], other: &[f32]) -> f32 {
    one.iter().zip(other).map(|(a, b)| a * b).sum()
}

/// What a feature is read by: its title and what it does.
fn text_of(title: &str, summary: &str) -> String {
    match summary.trim() {
        "" => title.to_string(),
        summary => format!("{title}: {summary}"),
    }
}

/// The slugs of `features` (slug, title, summary) that `words` are about, nearest first, at most [`MOST`], none under
/// [`NEAR_AT`]. Blocking: it embeds.
pub(crate) fn nearest(model: &dyn TextModel, vectors: &Vectors, words: &str, features: &[(String, String, String)]) -> Vec<String> {
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if words.is_empty() || features.is_empty() {
        return Vec::new();
    }
    let texts: Vec<String> = features.iter().map(|(_, title, summary)| text_of(title, summary)).collect();
    let missing: Vec<&str> = {
        let kept = vectors.kept.lock().expect("never poisoned");
        let mut missing: Vec<&str> = texts.iter().filter(|text| !kept.contains_key(*text)).map(String::as_str).collect();
        missing.sort_unstable();
        missing.dedup();
        missing
    };
    let asked: Vec<&str> = std::iter::once(words.as_str()).chain(missing.iter().copied()).collect();
    let Ok(mut embedded) = model.embed(&asked) else { return Vec::new() };
    if embedded.len() != asked.len() {
        return Vec::new();
    }
    let words = embedded.remove(0);
    let mut kept = vectors.kept.lock().expect("never poisoned");
    kept.extend(missing.into_iter().map(str::to_string).zip(embedded));
    let mut scored: Vec<(&str, f32)> = features
        .iter()
        .zip(&texts)
        .filter_map(|((slug, _, _), text)| kept.get(text).map(|one| (slug.as_str(), similarity(&words, one))))
        .filter(|(_, score)| *score >= NEAR_AT)
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.into_iter().take(MOST).map(|(slug, _)| slug.to_string()).collect()
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
    /// `get_features_by_meaning`.
    pub(super) async fn features_by_meaning(&self, caller: Option<api::Caller>, project: Option<String>, words: String) -> Result<FeaturesByMeaning, Refusal> {
        let (chosen, features) = {
            let store = self.store().lock().await;
            let projects = self.projects_asked(&store, caller, project.as_deref())?;
            // A Job in several projects reads the first that has a library, as `list_features` does.
            let mut chosen = projects[0].clone();
            for one in &projects {
                if store.project_has_features(one).map_err(|why| self.ledger_fault(why))? {
                    chosen = one.clone();
                    break;
                }
            }
            let features: Vec<(String, String, String)> = store
                .features_in(&chosen)
                .map_err(|why| self.ledger_fault(why))?
                .into_iter()
                .filter(|feature| feature.state != store::feature_state::PENDING)
                .map(|feature| (feature.slug, feature.title, feature.summary))
                .collect();
            (chosen, features)
        };
        let none = FeaturesByMeaning { project: chosen.clone(), slugs: Vec::new() };
        let Some(model) = self.language_model().model() else { return Ok(none) };
        let vectors: Arc<Vectors> = self.language_model().feature_vectors();
        let slugs = tokio::task::spawn_blocking(move || nearest(&*model, &vectors, &words, &features)).await.unwrap_or_default();
        Ok(FeaturesByMeaning { project: chosen, slugs })
    }
}
