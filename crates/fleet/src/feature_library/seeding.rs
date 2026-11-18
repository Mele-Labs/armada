//! Armada's own catalog, kept by Fleet itself. **The seed is a data file Fleet carries**, `seed/armada.toml`: the
//! areas and features the owner approved on 16 Nov 2026 with their path prefixes and entry points. Fleet applies it through
//! `features_upserted`, so each feature's `added_at` is the moment it was kept and is never rewritten; the earliest across a
//! project is when its library was seeded. `docs/concepts/feature-catalog.md`, *How the catalog grows*.
//!
//! **Once per project, and only for Armada's repository.** Fleet keeps it on its own tick, with the Operator on or off; the Operator's `seed_features` is the same act on his word. A project that holds any feature is refused, and so is one none of
//! whose repositories holds the seed's marker paths. A path the repositories do not hold is left out and named.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AreaSeed, FeatureSeed, FeaturesSeeded, SeedFeatures, UpsertFeatures, WireError};

use crate::daemon::Fleet;

const ARMADA: &str = include_str!("seed/armada.toml");

const SEEDED: &str = "fleet.feature_seeded";
const NOT_FOR: &str = "fleet.seed_not_for_project";
pub(crate) const PROJECT: &str = "fleet.feature_project";

/// A seed as its file holds it.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Seed {
    /// Paths that tell a repository is the one this seed describes. All of them are in one repository, or the seed is not for
    /// the project.
    pub(crate) marker: Vec<String>,
    #[serde(default)]
    pub(crate) areas: Vec<AreaSeed>,
    #[serde(default)]
    pub(crate) features: Vec<FeatureSeed>,
}

impl Seed {
    /// Armada's own, read from the file compiled in.
    pub(crate) fn armada() -> Result<Seed, toml::de::Error> {
        toml::from_str(ARMADA)
    }
}

/// The path of an entry point: `path` or `path:line`.
fn path_of(entry: &str) -> &str {
    entry.split(':').next().unwrap_or(entry)
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
    /// The roots of the repositories `project` holds that Fleet serves, or `None` for a project Fleet does not hold.
    fn seed_roots(&self, project: &str) -> Option<Vec<PathBuf>> {
        let held = self.projects_held();
        let one = held.running().find(|one| one.id().as_str() == project)?;
        Some(self.project_roots(one).into_iter().map(PathBuf::from).collect())
    }

    /// Whether a repository of the project holds the seed's marker paths, so the seed describes it.
    pub(crate) async fn seed_fits(&self, project: &str, seed: &Seed) -> bool {
        let Some(roots) = self.seed_roots(project) else { return false };
        let marker = seed.marker.clone();
        tokio::task::spawn_blocking(move || roots.iter().any(|root| marker.iter().all(|path| root.join(path).exists()))).await.unwrap_or(false)
    }

    /// `seed_features`. **Nothing is kept on a refusal.**
    pub(super) async fn features_seeded(&self, caller: Option<api::Caller>, seed: SeedFeatures) -> Result<FeaturesSeeded, Refusal> {
        self.operator_called(caller)?;
        self.seed_project(seed.project.trim()).await
    }

    /// The projects the seed does not fit, as this Fleet run found them.
    fn unseedable(&self) -> std::sync::MutexGuard<'_, std::collections::BTreeSet<String>> {
        self.operator_seat().sources.unseedable.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Each running project that holds no feature and that the seed fits, seeded by Fleet itself, whether or not the Operator is
    /// on. **A project the seed does not fit is remembered, so a tick does not look at its repositories again**; one that is
    /// seeded holds features and is passed over. What was left out is logged.
    pub(crate) async fn projects_seeded(&self) {
        let Ok(carried) = Seed::armada() else { return };
        let projects: Vec<String> = self.projects_held().running().map(|one| one.id().as_str().to_string()).collect();
        for id in projects {
            if self.unseedable().contains(&id) {
                continue;
            }
            if self.store().lock().await.project_has_features(&id).unwrap_or(true) {
                continue;
            }
            if !self.seed_fits(&id, &carried).await {
                self.unseedable().insert(id);
                continue;
            }
            match self.seed_project(&id).await {
                Ok(kept) => eprintln!(
                    "fleet: seeded project {id} with {} areas and {} features; left out {} paths the repository does not hold{}",
                    kept.areas,
                    kept.created.len(),
                    kept.dropped.len(),
                    if kept.dropped.is_empty() { String::new() } else { format!(": {}", kept.dropped.join(", ")) },
                ),
                Err(refused) => eprintln!("fleet: project {id} was not seeded: {}", refused.error().message),
            }
        }
    }

    /// Armada's seed kept for `project`, with no caller to check. **Nothing is kept on a refusal.**
    async fn seed_project(&self, project: &str) -> Result<FeaturesSeeded, Refusal> {
        let project = project.to_string();
        let carried = Seed::armada().map_err(|why| self.hosted_fault("fleet.feature_seed_unread", &format!("the seed Fleet carries does not read: {why}")))?;
        let Some(roots) = self.seed_roots(&project) else {
            return Err(self.hosted_refusal(PROJECT, &format!("no project is named `{project}`")));
        };
        if !self.seed_fits(&project, &carried).await {
            return Err(self.hosted_refusal(NOT_FOR, "none of this project's repositories holds the paths the seed describes"));
        }
        if self.store().lock().await.project_has_features(&project).map_err(|why| self.ledger_fault(why))? {
            return Err(Refusal::IllegalMove(WireError::raised(SEEDED, "this project holds features already, and a seed is kept once", self.run_id())));
        }
        let (areas, features, dropped) = tokio::task::spawn_blocking(move || {
            let held = |path: &str| roots.iter().any(|root| root.join(path).exists());
            let mut dropped = Vec::new();
            let features: Vec<FeatureSeed> = carried
                .features
                .into_iter()
                .map(|mut feature| {
                    let slug = feature.slug.clone();
                    feature.paths.retain(|path| held(path) || {
                        dropped.push(format!("{slug}: {path}"));
                        false
                    });
                    feature.entry_points.retain(|point| held(path_of(point)) || {
                        dropped.push(format!("{slug}: {point}"));
                        false
                    });
                    feature
                })
                .collect();
            (carried.areas, features, dropped)
        })
        .await
        .map_err(|why| self.hosted_fault("fleet.feature_seed_unread", &format!("the seed could not be checked: {why}")))?;
        let kept = self.features_upserted(UpsertFeatures { project, areas, features }).await?;
        Ok(FeaturesSeeded { created: kept.created, areas: kept.areas, dropped })
    }
}
