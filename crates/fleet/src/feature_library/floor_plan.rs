//! A feature's floor plan: the project's code map with the feature's rooms lit. **Rooms are modules**
//! (`modules`), **a room is lit** where the feature owns a file in it by the rules of `mapping`, and **a door is a
//! dependency from a lit module to another module of its repository**, drawn as a contract where the module it leads
//! to carries a wire or API contract. **Each room holds its files** with git's sizes, lit where the feature owns one and
//! marked where a live Job wrote one (`floor_files`). Read on every page read, from the repository's files, git and the
//! library's own rows; no model and no code graph. `docs/concepts/feature-catalog.md`, *Floor plan*.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{FloorDoor, FloorPlan, FloorRoom};
use store::KeptFeature;

use super::floor_files::{self, capped, Placed, MOST_FILES};
use super::mapping::FeatureMap;
use super::modules::{self, Layout, Module};
use super::reading::Loaded;
use crate::daemon::Fleet;

/// Words that say a module is a wire or API contract, in its name or in the last segment of its path.
const CONTRACT_WORDS: &[&str] = &["api", "apis", "ipc", "proto", "protos", "protobuf", "contract", "contracts", "schema", "schemas", "openapi"];

/// Whether `module` is where a contract lives: its name or its folder says so, or a `.proto` or OpenAPI file lies in it.
pub(crate) fn is_contract(module: &Module) -> bool {
    let last = module.root.rsplit('/').next().unwrap_or("");
    module.spec_files
        || [module.name.as_str(), last]
            .iter()
            .flat_map(|text| text.split(|c: char| !c.is_alphanumeric()))
            .any(|word| CONTRACT_WORDS.contains(&word.to_ascii_lowercase().as_str()))
}

/// The rooms of `layout` the feature owns files in, by index.
fn lit_rooms(layout: &Layout, map: &FeatureMap, feature: &KeptFeature, manifest: &str) -> Vec<bool> {
    let owned = |path: &str| map.feature_owns(&feature.id, Some(manifest), path);
    let interest: Vec<&str> = feature.paths.iter().map(|path| path.trim_matches('/')).chain(map.filed_of(&feature.id)).filter(|path| !path.is_empty()).collect();
    let mut lit: Vec<bool> = layout.modules.iter().map(|module| !module.root.is_empty() && owned(&module.root)).collect();
    for path in interest {
        if let Some(room) = layout.holding(path) {
            lit[room] = lit[room] || owned(path);
        }
    }
    lit
}

/// The layout of the repository at `root` with its tracked files' sizes. **Blocking**: the caller runs it off the main thread.
pub(crate) fn read(root: &Path) -> Layout {
    Layout { files: floor_files::tracked(root), ..modules::read(root) }
}

/// The plan for `feature` over the layouts of its project's repositories, each with its manifest's id, with `changing` the
/// files live Jobs wrote by manifest and at most `most` file entries. A feature that names a repository is planned in that
/// one alone.
pub(crate) fn plan(layouts: &[(String, Layout)], map: &FeatureMap, feature: &KeptFeature, changing: &BTreeMap<String, BTreeSet<String>>, most: usize) -> FloorPlan {
    let held: Vec<&(String, Layout)> = layouts.iter().filter(|(manifest, _)| feature.manifest_id.is_empty() || *manifest == feature.manifest_id).collect();
    let several = held.len() > 1;
    let mut plan = FloorPlan::default();
    let (mut roots, mut placed): (Vec<&str>, Vec<Placed>) = (Vec::new(), Vec::new());
    for (manifest, layout) in held {
        let lit = lit_rooms(layout, map, feature, manifest);
        let first = plan.rooms.len();
        for (at, module) in layout.modules.iter().enumerate() {
            plan.rooms.push(FloorRoom {
                module: module.name.clone(),
                path: if module.root.is_empty() { ".".to_string() } else { module.root.clone() },
                kind: module.kind.label().to_string(),
                lit: lit[at],
                manifest_id: several.then(|| manifest.clone()),
                files: Vec::new(),
            });
            roots.push(&module.root);
        }
        let written = changing.get(manifest);
        for (path, bytes) in &layout.files {
            if let Some(room) = layout.holding(path) {
                let lit = map.feature_owns(&feature.id, Some(manifest), path);
                placed.push(Placed { room: first + room, path: path.clone(), bytes: *bytes, lit, changing: written.is_some_and(|paths| paths.contains(path)) });
            }
        }
        for (from, to) in layout.uses.iter().filter(|(from, _)| lit[*from]) {
            plan.doors.push(FloorDoor { from: layout.modules[*from].name.clone(), to: layout.modules[*to].name.clone(), contract: is_contract(&layout.modules[*to]) });
        }
    }
    for (room, files) in plan.rooms.iter_mut().zip(capped(placed, &roots, most)) {
        room.files = files;
    }
    plan
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
    /// The floor plan of `feature`, read off the main thread with the files live Jobs wrote. `None` where Fleet does not
    /// hold the project or none of its repositories reads.
    pub(super) async fn floor_plan_of(&self, loaded: &Loaded, feature: &KeptFeature) -> Option<FloorPlan> {
        let held = self.projects_held();
        let project = held.running().find(|one| one.id().as_str() == loaded.project)?;
        let repositories: Vec<(String, PathBuf)> = self
            .repositories()
            .served()
            .iter()
            .filter(|one| project.holds(one.manifest().id()))
            .map(|one| (one.manifest().id().as_str().to_string(), PathBuf::from(one.root())))
            .collect();
        let mut changing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (_, manifest, paths) in self.writes().holders() {
            changing.entry(manifest).or_default().extend(paths);
        }
        let (map, feature) = (loaded.map.clone(), feature.clone());
        let plan = tokio::task::spawn_blocking(move || {
            let layouts: Vec<(String, Layout)> = repositories.into_iter().map(|(manifest, root)| (manifest, read(&root))).collect();
            plan(&layouts, &map, &feature, &changing, MOST_FILES)
        });
        Some(plan.await.ok()?).filter(|plan| !plan.rooms.is_empty())
    }
}
