//! What a project's settings say beyond its file: a `delivery` Fleet learned from the person's first answer, and
//! what a value the project overrides would be without it. `docs/practices/protocol/projects.md`, *Values*.
//!
//! **The learned note is never in the project's file**: it is a row in the store, kept when the learning writes
//! the file and dropped as soon as a write by another path leaves the key absent or a different word.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Values;
use ipc::ManifestId;

use crate::daemon::Fleet;
use crate::values::resolved_wire;

/// The key of the one value Fleet learns.
pub(crate) const LEARNED_KEY: &str = "delivery";

/// Where `value` came from the project, the answer below it: its word and layer, `default` where no layer below
/// sets a word.
fn below_of(value: &mut ipc::ResolvedValue, below: &ipc::ResolvedValue) {
    if value.from == ipc::ValueLayer::Project {
        value.inherited = below.value.clone();
        value.inherited_from = Some(below.from);
    }
}

/// The plain keys and review kinds of `wire` that the project sets, each told what the layers below it give.
fn inherited_over(wire: &mut ipc::ResolvedValues, below: &ipc::ResolvedValues) {
    below_of(&mut wire.pr_mode, &below.pr_mode);
    below_of(&mut wire.auto_merge, &below.auto_merge);
    below_of(&mut wire.review_model, &below.review_model);
    below_of(&mut wire.layout, &below.layout);
    below_of(&mut wire.theme, &below.theme);
    below_of(&mut wire.density, &below.density);
    if let (Some(value), Some(below)) = (wire.agent_starts.as_mut(), below.agent_starts.as_ref()) {
        below_of(value, below);
    }
    if let (Some(value), Some(below)) = (wire.delivery.as_mut(), below.delivery.as_ref()) {
        below_of(value, below);
    }
    if let (Some(value), Some(below)) = (wire.reviews.as_mut(), below.reviews.as_ref()) {
        reviews_inherited_over(value, below);
    }
}

/// The review kinds of `reviews` that the project sets, each told the word and layer below it. A kind always has a
/// word, so unset below is `off` from `default`.
fn reviews_inherited_over(reviews: &mut ipc::ResolvedReviews, below: &ipc::ResolvedReviews) {
    let project = Some(ipc::ValueLayer::Project);
    if reviews.plans_from == project {
        reviews.plans_inherited = Some(below.plans.clone());
        reviews.plans_inherited_from = below.plans_from;
    }
    if reviews.pull_requests_from == project {
        reviews.pull_requests_inherited = Some(below.pull_requests.clone());
        reviews.pull_requests_inherited_from = below.pull_requests_from;
    }
    if reviews.walks_from == project {
        reviews.walks_inherited = Some(below.walks.clone());
        reviews.walks_inherited_from = below.walks_from;
    }
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
    /// `wire`, the answer for `project`, told what a value the project sets would be without it, and whether its
    /// `delivery` was learned. **A Job's frozen values never come here.**
    pub(crate) async fn project_values_annotated(
        &self,
        mut wire: ipc::ResolvedValues,
        project: &str,
        repository: Option<&ManifestId>,
    ) -> Result<ipc::ResolvedValues, Refusal> {
        let below = resolved_wire(&self.values_resolved(None, repository).await?);
        inherited_over(&mut wire, &below);
        if let Some(delivery) = wire.delivery.as_mut() {
            if delivery.from == ipc::ValueLayer::Project {
                let note = self.store().lock().await.project_learned(project, LEARNED_KEY);
                delivery.learned = matches!(note, Ok(Some(word)) if delivery.value.as_deref() == Some(word.as_str()));
            }
        }
        Ok(wire)
    }

    /// Keep that `project`'s delivery was learned as `word`.
    pub(crate) async fn project_learned_kept(&self, project: &str, word: &str) {
        let at = self.now();
        if let Err(why) = self.store().lock().await.keep_project_learned(project, LEARNED_KEY, word, at.as_str()) {
            self.values_warned("what a project learned was not kept", Some(project), format!("{why:?}"));
        }
    }

    /// Drop the note that `project`'s delivery was learned where its file, as now written, leaves the key absent or a
    /// different word from it. **A rewrite of the same word keeps the note.**
    pub(crate) async fn project_learned_standing(&self, project: &str, written: &Values) {
        let mut store = self.store().lock().await;
        let Ok(Some(note)) = store.project_learned(project, LEARNED_KEY) else { return };
        if written.delivery.map(|word| word.as_wire()) == Some(note.as_str()) {
            return;
        }
        if let Err(why) = store.clear_project_learned(project, LEARNED_KEY) {
            self.values_warned("what a project learned was not cleared", Some(project), format!("{why:?}"));
        }
    }
}
