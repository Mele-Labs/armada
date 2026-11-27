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
/// about nothing at most 0.59. At 0.70 every feature meant is kept and each query names one or two. Read with the first
/// paragraph of its Runs section joined to the summary (40 of the 85 have one), the feature meant fell under 0.70 on 3 of the 13
/// queries (`alerts and banners` 0.91 to 0.68, `phone` 0.76 to 0.58, `my teammate's claims` 0.76 to under 0.64).
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

/// The longest first paragraph of a Runs section read, in characters, so one long paragraph does not outweigh the title.
const MOST_RUNS: usize = 600;

/// What a feature is read by: its title and what it does, and where it has a Runs section its title and the first paragraph of
/// it. **Each is read alone and the nearer counts**: a Runs paragraph is a note on the code, and joined to the summary it took a
/// feature the words describe out of reach (measured 26 Nov 2026, below), where read beside it, it only adds a way in.
fn texts_of(title: &str, summary: &str, runs: Option<&str>) -> Vec<String> {
    let mut texts = vec![match summary.trim() {
        "" => title.to_string(),
        summary => format!("{title}: {summary}"),
    }];
    let first = runs.and_then(|runs| runs.split("\n\n").map(str::trim).find(|paragraph| !paragraph.is_empty()));
    if let Some(first) = first {
        texts.push(format!("{title}: {}", first.chars().take(MOST_RUNS).collect::<String>()));
    }
    texts
}

/// The slugs of `features` (slug, texts as [`texts_of`] makes them) that `words` are about, nearest first, at most [`MOST`], none
/// under [`NEAR_AT`]. A feature scores as its nearest text does. Blocking: it embeds.
pub(crate) fn nearest(model: &dyn TextModel, vectors: &Vectors, words: &str, features: &[(String, Vec<String>)]) -> Vec<String> {
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if words.is_empty() || features.is_empty() {
        return Vec::new();
    }
    let missing: Vec<&str> = {
        let kept = vectors.kept.lock().expect("never poisoned");
        let mut missing: Vec<&str> = features.iter().flat_map(|(_, texts)| texts).filter(|text| !kept.contains_key(*text)).map(String::as_str).collect();
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
        .filter_map(|(slug, texts)| texts.iter().filter_map(|text| kept.get(text)).map(|one| similarity(&words, one)).max_by(f32::total_cmp).map(|score| (slug.as_str(), score)))
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
            let mut features = Vec::new();
            for feature in store.features_in(&chosen).map_err(|why| self.ledger_fault(why))?.into_iter().filter(|feature| feature.state != store::feature_state::PENDING) {
                let sections = store.feature_sections(&feature.id).map_err(|why| self.ledger_fault(why))?;
                let runs = sections.iter().find(|section| section.section == "runs").map(|section| section.text.as_str());
                features.push((feature.slug.clone(), texts_of(&feature.title, &feature.summary, runs)));
            }
            (chosen, features)
        };
        let none = FeaturesByMeaning { project: chosen.clone(), slugs: Vec::new() };
        let Some(model) = self.language_model().model() else { return Ok(none) };
        let vectors: Arc<Vectors> = self.language_model().feature_vectors();
        let slugs = tokio::task::spawn_blocking(move || nearest(&*model, &vectors, &words, &features)).await.unwrap_or_default();
        Ok(FeaturesByMeaning { project: chosen, slugs })
    }
}
