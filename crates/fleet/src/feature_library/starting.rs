//! A project with no feature of its own starts its library here: the modules its build files name and the docs at the top of
//! its repositories, read from its files, and the owner's Draft as Pending, which keeps each module as a pending feature.
//! Plain file reads, no model. `docs/concepts/feature-catalog.md`, *Starting a library*.

use std::collections::BTreeSet;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DraftModules, LibraryStart, ModulesDrafted, StartModule, WireError};
use store::{feature_state, KeptArea, KeptFeature};

use super::modules;
use super::pending::PENDING_AREA;
use crate::daemon::Fleet;

/// The project holds a confirmed or seeded feature, so its library has started.
const STARTED: &str = "fleet.library_started";
/// No repository of the project is served, so there are no files to read.
const NO_REPOSITORY: &str = "fleet.start_no_repository";
/// The project's files name no module.
const NO_MODULE: &str = "fleet.no_module";

/// The area a press keeps where the project keeps none, so Confirm on a drafted row has a place to put it.
const MODULES_AREA: (&str, &str) = ("modules", "Modules");

/// The top files that are docs, by the start of their name in upper case, and the top folders that hold docs.
const DOC_FILES: &[&str] = &["README", "ARCHITECTURE", "CONTRIBUTING", "DESIGN", "OVERVIEW"];
const DOC_FOLDERS: &[&str] = &["docs", "doc", "documentation"];

/// A module as a draft keeps it: the repository it is in, by Manifest id, and what the wire says of it.
struct Found {
    manifest: String,
    module: StartModule,
}

/// A slug made from a module's name: lower-case letters and digits, words joined by one dash.
fn slug_of(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// The modules of each repository, each slug once, the first repository's first. **Blocking**.
fn modules_of(repositories: &[(String, PathBuf)]) -> Vec<Found> {
    let mut taken = BTreeSet::new();
    let mut found = Vec::new();
    for (manifest, root) in repositories {
        for module in modules::read(root).modules {
            let slug = slug_of(&module.name);
            if slug.is_empty() || !taken.insert(slug.clone()) {
                continue;
            }
            found.push(Found { manifest: manifest.clone(), module: StartModule { name: module.name, slug, path: module.root, held: false } });
        }
    }
    found
}

/// The docs at the top of each repository, each name once. **Blocking**.
fn docs_of(roots: &[PathBuf]) -> Vec<String> {
    let mut docs: Vec<String> = Vec::new();
    for root in roots {
        let Ok(listing) = std::fs::read_dir(root) else { continue };
        let mut here: Vec<String> = listing
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let kind = entry.file_type().ok()?;
                if kind.is_dir() && DOC_FOLDERS.contains(&name.as_str()) {
                    Some(format!("{name}/"))
                } else if kind.is_file() && DOC_FILES.iter().any(|stem| name.to_ascii_uppercase().starts_with(stem)) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        here.sort();
        for one in here {
            if !docs.contains(&one) {
                docs.push(one);
            }
        }
    }
    docs
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
    /// The served repositories of `project`, refused where Fleet holds no such project or serves none of them.
    fn start_repositories(&self, project: &str) -> Result<Vec<(String, PathBuf)>, Refusal> {
        let Some(repositories) = self.served_members(project) else {
            return Err(self.hosted_refusal(super::seeding::PROJECT, &format!("no project is named `{project}`")));
        };
        if repositories.is_empty() {
            return Err(self.hosted_refusal(NO_REPOSITORY, &format!("No repository of project `{project}` is served here, so there are no files to read.")));
        }
        Ok(repositories)
    }

    /// The modules of `project`, each marked where the library holds its slug.
    async fn start_modules(&self, project: &str, repositories: Vec<(String, PathBuf)>) -> Result<Vec<Found>, Refusal> {
        let mut found = tokio::task::spawn_blocking(move || modules_of(&repositories)).await.unwrap_or_default();
        let store = self.store().lock().await;
        let held: BTreeSet<String> = store.features_in(project).map_err(|why| self.ledger_fault(why))?.into_iter().map(|one| one.slug).collect();
        for one in &mut found {
            one.module.held = held.contains(&one.module.slug);
        }
        Ok(found)
    }

    /// `get_library_start`.
    pub(super) async fn library_start_read(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<LibraryStart, Refusal> {
        let project = {
            let store = self.store().lock().await;
            self.projects_asked(&store, caller, project.as_deref())?.remove(0)
        };
        let repositories = self.start_repositories(&project)?;
        let roots: Vec<PathBuf> = repositories.iter().map(|(_, root)| root.clone()).collect();
        let modules = self.start_modules(&project, repositories).await?;
        let docs = tokio::task::spawn_blocking(move || docs_of(&roots)).await.unwrap_or_default();
        Ok(LibraryStart { project, modules: modules.into_iter().map(|one| one.module).collect(), docs })
    }

    /// `draft_module_features`. **Nothing is kept on a refusal**, and a slug the library holds is passed over.
    pub(super) async fn modules_drafted(&self, caller: Option<api::Caller>, draft: DraftModules) -> Result<ModulesDrafted, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = draft.project.trim().to_string();
        let repositories = self.start_repositories(&project)?;
        let found = self.start_modules(&project, repositories).await?;
        if found.is_empty() {
            return Err(self.hosted_refusal(NO_MODULE, "No build file of this project names a module, and it has no folders to draft."));
        }
        let mut store = self.store().lock().await;
        let kept = store.features_in(&project).map_err(|why| self.ledger_fault(why))?;
        if kept.iter().any(|one| one.state != feature_state::PENDING) {
            return Err(Refusal::IllegalMove(WireError::raised(STARTED, "This project holds features of its own, so its modules are not drafted.", self.run_id())));
        }
        if store.feature_areas(&project).map_err(|why| self.ledger_fault(why))?.is_empty() {
            let (slug, title) = MODULES_AREA;
            store.put_feature_area(&KeptArea { project: project.clone(), slug: slug.to_string(), title: title.to_string(), position: 0 }).map_err(|why| self.ledger_fault(why))?;
        }
        let now = self.now().as_str().to_string();
        let (mut drafted, mut held) = (Vec::new(), Vec::new());
        for (place, one) in found.into_iter().enumerate() {
            // A slug kept since the modules were read is passed over too.
            if one.module.held || kept.iter().any(|feature| feature.slug == one.module.slug) {
                held.push(one.module.slug);
                continue;
            }
            let feature = KeptFeature {
                id: self.minted(),
                project: project.clone(),
                area: PENDING_AREA.to_string(),
                slug: one.module.slug.clone(),
                title: one.module.name,
                summary: String::new(),
                state: feature_state::PENDING.to_string(),
                manifest_id: one.manifest,
                entry_points: Vec::new(),
                paths: if one.module.path.is_empty() { Vec::new() } else { vec![one.module.path] },
                position: place as u32,
                added_at: now.clone(),
            };
            store.put_feature(&feature).map_err(|why| self.ledger_fault(why))?;
            drafted.push(one.module.slug);
        }
        Ok(ModulesDrafted { project, drafted, held })
    }
}
