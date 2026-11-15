//! What a project's settings say beyond its file: a `delivery` Fleet learned from the person's first answer, what the
//! Operator may do there unasked (`operator_acts`), and what a value the project overrides would be without it. `docs/practices/protocol/projects.md`, *Values*.
//!
//! **The learned note is never in the project's file**: it is a row in the store, kept when the learning writes
//! the file and dropped as soon as a write by another path leaves the key absent or a different word.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Values;
use ipc::ManifestId;

use crate::daemon::Fleet;
use crate::values::resolved_wire;

/// The key of `delivery`, the value Fleet learns from a project's file.
pub(crate) const LEARNED_KEY: &str = "delivery";
/// The key of what the Operator may do unasked in a project. **Learned only**: no file of the project names it.
pub(crate) const OPERATOR_ACTS_KEY: &str = "operator_acts";

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

    /// Keep that `project`'s `key` was learned as `word`.
    pub(crate) async fn project_learned_kept(&self, project: &str, key: &str, word: &str) {
        let at = self.now();
        if let Err(why) = self.store().lock().await.keep_project_learned(project, key, word, at.as_str()) {
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
    /// What the Operator may do unasked in each of his projects, `acts` absent where he has not said.
    pub(crate) async fn operator_acts_listed(&self) -> Result<ipc::OperatorActsList, Refusal> {
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        let projects = self.projects_on_the_wire(kept).await.projects;
        let store = self.store().lock().await;
        let projects = projects
            .into_iter()
            .map(|one| {
                let acts = store.project_learned(&one.id, OPERATOR_ACTS_KEY).ok().flatten().and_then(|word| ipc::OperatorActs::from_wire(&word));
                ipc::ProjectOperatorActs { project: one.id, name: one.name, acts }
            })
            .collect();
        Ok(ipc::OperatorActsList { projects })
    }

    /// One project's value, or unset where `acts` is absent. Refused for a project he has none of.
    pub(crate) async fn operator_acts_set(&self, set: ipc::SetOperatorActs) -> Result<ipc::OperatorActsList, Refusal> {
        let known = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        if !self.projects_on_the_wire(known).await.projects.iter().any(|one| one.id == set.project) {
            return Err(Refusal::NotFound(ipc::WireError::raised(crate::projects::NO_SUCH_PROJECT, format!("there is no project {}", set.project), self.run_id())));
        }
        match set.acts {
            Some(acts) => self.project_learned_kept(&set.project, OPERATOR_ACTS_KEY, acts.as_wire()).await,
            None => {
                if let Err(why) = self.store().lock().await.clear_project_learned(&set.project, OPERATOR_ACTS_KEY) {
                    return Err(self.ledger_fault(why));
                }
            }
        }
        self.operator_page_changed();
        self.operator_acts_listed().await
    }
}
