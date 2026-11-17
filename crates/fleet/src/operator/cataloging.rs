//! What Fleet tells the Operator about a project's feature library, once each. `docs/concepts/feature-catalog.md`.
//!
//! **Fleet detects, the Operator decides.** A project that holds Armada's repository and no feature is told as a seed to
//! apply with `seed_features`. Nothing here keeps a feature or a card.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::{held, Arrival, Arrived, Source};
use crate::daemon::Fleet;
use crate::feature_library::{features_of_note, Seed};

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
    /// Each project's feature library told to the session where it needs the Operator, once. While the Operator is off or its
    /// session is not running nothing is told.
    pub(super) async fn catalog_handed(self: &Arc<Self>) {
        if !self.operator_on() {
            return;
        }
        for arrival in self.seeds_offered().await {
            self.hand_once(arrival).await;
        }
    }

    /// The features a note points to, one project to a line, for the words the Operator is told. **Read off the files its element
    /// was drawn from and the component's name, against the paths each feature keeps**: a note that names neither says so.
    pub(crate) async fn note_features(&self, manifest: &str, files: &[String], component: Option<&str>) -> String {
        let projects: Vec<String> =
            self.projects_held().holding(&ipc::ManifestId::carried(manifest).to_domain()).into_iter().map(|one| one.id().as_str().to_string()).collect();
        let store = self.store().lock().await;
        let mut lines = String::new();
        for project in projects.iter().filter(|project| store.project_has_features(project).unwrap_or(false)) {
            let slugs = features_of_note(&store, project, manifest, files, component).unwrap_or_default();
            let on = if slugs.is_empty() { "no feature's code is where it was left".to_string() } else { slugs.join(", ") };
            lines.push_str(&format!("\nFeatures it points to in project {project}: {on}"));
        }
        lines
    }

    /// The projects that hold Armada's repository and no feature, as arrivals.
    async fn seeds_offered(&self) -> Vec<Arrival> {
        let Ok(seed) = Seed::armada() else { return Vec::new() };
        let projects: Vec<(String, String)> = self.projects_held().running().map(|one| (one.id().as_str().to_string(), one.name().as_str().to_string())).collect();
        let mut offered = Vec::new();
        for (id, name) in projects {
            if held(&self.operator_seat().sources.unseedable).contains(&id) {
                continue;
            }
            if self.store().lock().await.project_has_features(&id).unwrap_or(true) {
                continue;
            }
            if !self.seed_fits(&id, &seed).await {
                held(&self.operator_seat().sources.unseedable).insert(id);
                continue;
            }
            offered.push(Arrival {
                id: format!("seed:{id}"),
                arrived: Arrived::Seed,
                source: Source::Fleet,
                session_id: String::new(),
                manifest_id: String::new(),
                subject: format!("{name} has no feature library"),
                words: format!(
                    "Project {name} (id {id}) holds Armada's own repository and no feature. Fleet carries the catalog the owner approved. \
                     Keep it with `seed_features` for project `{id}`, then say in the feed what was kept and what was left out."
                ),
            });
        }
        offered
    }
}
