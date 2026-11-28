//! What Fleet tells the Operator about a project's feature library, once each. `docs/concepts/feature-catalog.md`.
//!
//! **Fleet detects, the Operator decides.** A pending feature and an area past eight features are told. Nothing here keeps a
//! feature or a card. Armada's own seed is not told: Fleet keeps it itself, `feature_library::seeding`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::{Arrival, Arrived, Source};
use crate::daemon::Fleet;
use crate::feature_library::catalog_rules::rules_of;
use crate::feature_library::{features_of_note, pending_facts};
use store::feature_state;

/// An area holds this many features and no more before the Operator proposes a split.
const AREA_FEATURES: u32 = 8;

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
        for arrival in self.pending_offered().await.into_iter().chain(self.areas_offered().await) {
            self.hand_once(arrival).await;
        }
    }

    /// Every pending feature and crowded area standing now, kept as told without telling: what piled up while the Operator
    /// was off is not handed to it at once when it comes on.
    pub(super) async fn catalog_kept_as_handed(&self) {
        let now = self.now();
        let standing: Vec<_> = self.pending_offered().await.into_iter().chain(self.areas_offered().await).collect();
        let mut store = self.store().lock().await;
        for arrival in standing {
            if let Err(why) = store.keep_linked_told("operator", super::session::TOLD, &arrival.id, &now) {
                self.operator_warned("a feature standing when the Operator came on was not kept as handed", why);
            }
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

    /// The pending features, each as an arrival that names who named it, the files filed under it and the features whose
    /// prefixes cover those files. **The Operator brings the owner one card for each**: confirm it, merge it into an existing
    /// feature, or rename it.
    async fn pending_offered(&self) -> Vec<Arrival> {
        let store = self.store().lock().await;
        let pending = store.pending_features().unwrap_or_default();
        let mut offered = Vec::new();
        for feature in pending {
            let Ok(facts) = pending_facts(&store, &feature) else { continue };
            let project = self.projects_held().running().find(|one| one.id().as_str() == feature.project).map(|one| one.name().as_str().to_string()).unwrap_or_default();
            let named = match facts.jobs.is_empty() {
                true => String::from("No Job is on record as naming it."),
                false => facts.jobs.iter().map(|(job, title, via)| format!("Job {job} \"{title}\" named it through {via}.")).collect::<Vec<_>>().join(" "),
            };
            let files = match facts.files.is_empty() {
                true => String::from("No file is filed under it."),
                false => format!("Files filed under it: {}.", facts.files.join(", ")),
            };
            let owners = match facts.owners.is_empty() {
                true => String::from("No other feature's prefix covers them."),
                false => format!(
                    "Features whose prefixes cover them: {}.",
                    facts.owners.iter().map(|(slug, files)| format!("{slug} ({files})")).collect::<Vec<_>>().join(", ")
                ),
            };
            offered.push(Arrival {
                id: format!("feature:{}:{}", feature.project, feature.slug),
                arrived: Arrived::Feature,
                source: Source::Fleet,
                session_id: String::new(),
                manifest_id: String::new(),
                project: feature.project.clone(),
                subject: format!("`{}` is not in {project}'s library", feature.slug),
                words: format!(
                    "Feature `{slug}` in project {project} (id {id}) is pending. {named} {files} {owners} Bring him one card: confirm it, \
                     merge it into an existing feature, or rename it. After he chooses, confirm with `upsert_features` (state confirmed, with \
                     an area, a title and a line), merge with `change_feature` and `into`, or rename with `change_feature` and `slug`, \
                     and pass his reason as `why` where he gave one.{rules}",
                    slug = feature.slug,
                    id = feature.project,
                    rules = rules_of(&store, &feature.project),
                ),
            });
        }
        offered
    }

    /// The areas that hold more than eight features, each as an arrival that names the features it holds and the project's other areas.
    /// **The Operator proposes a split on one card and changes nothing until he answers.** An area is told again only when it
    /// has grown by another eight.
    async fn areas_offered(&self) -> Vec<Arrival> {
        let store = self.store().lock().await;
        let mut offered = Vec::new();
        for (project, area, count) in store.feature_counts().unwrap_or_default() {
            if count <= AREA_FEATURES {
                continue;
            }
            let name = self.projects_held().running().find(|one| one.id().as_str() == project).map(|one| one.name().as_str().to_string()).unwrap_or_default();
            let kept = store.feature_areas(&project).unwrap_or_default();
            let title = kept.iter().find(|one| one.slug == area).map(|one| one.title.clone()).unwrap_or_else(|| area.clone());
            let features = store.features_in(&project).unwrap_or_default();
            let held: Vec<String> =
                features.iter().filter(|one| one.area == area && one.state != feature_state::PENDING).map(|one| format!("`{}` ({})", one.slug, one.title)).collect();
            let others = kept.iter().filter(|one| one.slug != area).map(|one| format!("`{}`", one.slug)).collect::<Vec<_>>().join(", ");
            offered.push(Arrival {
                id: format!("split:{project}:{area}:{}", (count - 1) / AREA_FEATURES),
                arrived: Arrived::Area,
                source: Source::Fleet,
                session_id: String::new(),
                manifest_id: String::new(),
                project: project.clone(),
                subject: format!("{title} in {name} holds more than eight features"),
                words: format!(
                    "Area `{area}` ({title}) in project {name} (id {project}) holds {}. Other areas the project keeps: {others}. Bring him one card \
                     that proposes a split: the new areas, each with the features that go there. Nothing changes until he answers. After he \
                     does, split each new area off with `change_area` and the act `split`, naming its features, and pass his reason as `why` \
                     where he gave one.{}",
                    held.join(", "),
                    rules_of(&store, &project),
                ),
            });
        }
        offered
    }
}
