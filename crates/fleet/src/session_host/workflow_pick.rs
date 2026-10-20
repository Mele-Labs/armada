//! The local language model reading which workflow a new Session's request is, before Claude is
//! asked. `docs/spikes/workflow-pick-by-model.md` has it measured on 38 real requests.
//!
//! **Not in force.** On those 38 no threshold picks one workflow without a disagreement with Claude,
//! and the call that proposes the workflow also gives the Session its title, model and effort, which
//! the model cannot (the model and effort did not agree with Claude on 23 of 38 by the best rule
//! tried). So a local pick would save no call. It is kept so the figures can be run again and so
//! the reader is named where readers are named; nothing calls it while [`IN_FORCE`] is false.
//!
//! **It only ever says yes.** `none`, a tie, a model not loaded yet and any failure all come back
//! `None`, and Claude is asked as it always was.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use adapter_traits::TextModel;
use config::ResolvedWorkflow;
use core_model::WorkflowId;

use crate::language_model::LanguageModel;

/// Whether this reader is in force, and so whether Fleet fetches and loads the model for it. Off:
/// no pick it makes saves the call.
pub(crate) const IN_FORCE: bool = false;

/// The cosine similarity at and above which the best workflow is a clear pick. Above every best score
/// of the 38 requests (the highest, 0.66, was a workflow Claude did not pick).
pub(crate) const PICKS_AT: f32 = 0.70;

/// How far the best workflow must lead the second. Twice the widest lead (0.05) the 38 showed.
pub(crate) const LEADS_BY: f32 = 0.10;

/// Each workflow text's vector, so a read embeds only the workflows it has not seen.
#[derive(Default)]
pub(crate) struct Vectors {
    kept: Mutex<HashMap<String, Vec<f32>>>,
}

/// What a workflow is read by: its name and what it is for.
fn text_of(workflow: &ResolvedWorkflow) -> String {
    match workflow.for_requests() {
        Some(asked) => format!("{}: {asked}", workflow.name()),
        None => workflow.name().to_string(),
    }
}

fn similarity(one: &[f32], other: &[f32]) -> f32 {
    one.iter().zip(other).map(|(a, b)| a * b).sum()
}

/// The index of the one `scores` name as a clear pick: best at `PICKS_AT` and ahead of the second by
/// `LEADS_BY`. One score has no second, so it leads by its whole.
fn clear_pick(scores: &[f32]) -> Option<usize> {
    let (best, top) = scores.iter().copied().enumerate().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let second = scores.iter().copied().enumerate().filter(|&(at, _)| at != best).map(|(_, score)| score).max_by(f32::total_cmp);
    (top >= PICKS_AT && top - second.unwrap_or(0.0) >= LEADS_BY).then_some(best)
}

/// The workflow `request` plainly is, if the model reads one. Blocking: it embeds.
pub(crate) fn picked(
    model: &dyn TextModel,
    vectors: &Vectors,
    request: &str,
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
) -> Option<WorkflowId> {
    let request = request.split_whitespace().collect::<Vec<_>>().join(" ");
    if request.is_empty() || workflows.is_empty() {
        return None;
    }
    let (ids, texts): (Vec<&WorkflowId>, Vec<String>) = workflows.iter().map(|(id, one)| (id, text_of(one))).unzip();
    let missing: Vec<&str> = {
        let kept = vectors.kept.lock().expect("never poisoned");
        texts.iter().filter(|text| !kept.contains_key(*text)).map(String::as_str).collect()
    };
    let asked: Vec<&str> = std::iter::once(request.as_str()).chain(missing.iter().copied()).collect();
    let mut embedded = model.embed(&asked).ok()?;
    if embedded.len() != asked.len() {
        return None;
    }
    let request = embedded.remove(0);
    let mut kept = vectors.kept.lock().expect("never poisoned");
    kept.extend(missing.into_iter().map(str::to_string).zip(embedded));
    let scores: Vec<f32> = texts.iter().map(|text| kept.get(text).map(|one| similarity(&request, one))).collect::<Option<_>>()?;
    // What is kept afterwards is these workflows and no others.
    kept.retain(|text, _| texts.contains(text));
    clear_pick(&scores).map(|at| ids[at].clone())
}

/// [`picked`] on a thread of its own, with the model once it is loaded. `None` while it is not, and
/// while the reader is not in force.
#[allow(dead_code)]
pub(crate) async fn picked_by(
    language_model: &LanguageModel,
    request: &str,
    workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
) -> Option<WorkflowId> {
    if !IN_FORCE {
        return None;
    }
    let model = language_model.model()?;
    let vectors: Arc<Vectors> = language_model.workflow_vectors();
    let (request, workflows) = (request.to_string(), workflows.clone());
    tokio::task::spawn_blocking(move || picked(&*model, &vectors, &request, &workflows)).await.ok().flatten()
}
