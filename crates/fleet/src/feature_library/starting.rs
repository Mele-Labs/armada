//! A project with no feature of its own starts its library here: the modules its build files name and the docs at the top of
//! its repositories, read from its files, and the owner's Draft as Pending, which keeps each module as a pending feature.
//! Plain file reads, no model. `docs/concepts/feature-catalog.md`, *Starting a library*.

use std::collections::BTreeSet;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DraftModules, LibraryStart, ModulesDrafted, StartModule, WireError};
use store::{feature_state, KeptArea, KeptFeature, Store, WriteError};

use super::modules;
use super::pending::PENDING_AREA;
use crate::daemon::Fleet;

/// The project holds a confirmed or seeded feature, so its library has started.
const STARTED: &str = "fleet.library_started";
/// No repository of the project is served, so there are no files to read.
const NO_REPOSITORY: &str = "fleet.start_no_repository";
/// The project's files name no module.
const NO_MODULE: &str = "fleet.no_module";

/// The area a press keeps where the project keeps none, so Confirm on a drafted row has a place to put it, and the one an ask
/// that drafts from the docs keeps.
const MODULES_AREA: (&str, &str) = ("modules", "Modules");
pub(super) const FEATURES_AREA: (&str, &str) = ("features", "Features");

/// The top files that are docs, by the start of their name in upper case, and the top folders that hold docs.
const DOC_FILES: &[&str] = &["README", "ARCHITECTURE", "CONTRIBUTING", "DESIGN", "OVERVIEW"];
const DOC_FOLDERS: &[&str] = &["docs", "doc", "documentation"];

/// The most characters of one doc an ask reads, and of all its docs together.
const MOST_DOC_CHARS: usize = 4_000;
const MOST_DOCS_CHARS: usize = 12_000;

/// A feature a press or an ask keeps as pending.
pub(super) struct Draft {
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) summary: String,
    /// The repository it is in, by Manifest id, and its one prefix there; empty where it has none.
    pub(super) manifest: String,
    pub(super) path: String,
}

/// A module as a draft keeps it: the repository it is in, by Manifest id, and what the wire says of it.
struct Found {
    manifest: String,
    module: StartModule,
}

/// A slug made from a module's name: lower-case letters and digits, words joined by one dash.
pub(super) fn slug_of(name: &str) -> String {
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

/// The name and text of each of `given` that is a doc at the top of one of `roots`, a folder's Markdown files one level down,
/// each name once and each cut at [`MOST_DOC_CHARS`], all of them at [`MOST_DOCS_CHARS`]. **Blocking**.
pub(super) fn docs_text(roots: &[PathBuf], given: &[String]) -> Vec<(String, String)> {
    let listed = docs_of(roots);
    let (mut read, mut left): (Vec<(String, String)>, usize) = (Vec::new(), MOST_DOCS_CHARS);
    for doc in given.iter().filter(|one| listed.contains(one)) {
        for root in roots {
            let files: Vec<(String, PathBuf)> = match doc.strip_suffix('/') {
                Some(folder) => {
                    let Ok(listing) = std::fs::read_dir(root.join(folder)) else { continue };
                    let mut pages: Vec<String> = listing.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).filter(|name| name.to_ascii_lowercase().ends_with(".md")).collect();
                    pages.sort();
                    pages.into_iter().map(|name| (format!("{doc}{name}"), root.join(folder).join(name))).collect()
                }
                None => vec![(doc.clone(), root.join(doc))],
            };
            for (name, path) in files {
                if left == 0 {
                    return read;
                }
                let Some(text) = std::fs::read_to_string(path).ok().filter(|_| !read.iter().any(|(one, _)| *one == name)) else { continue };
                let text: String = text.chars().take(MOST_DOC_CHARS.min(left)).collect();
                left -= text.chars().count();
                read.push((name, text));
            }
        }
    }
    read
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

    /// Each of `drafts` kept as a pending feature of `project`, with `area` kept where the project keeps none so Confirm has a
    /// place to put it: the slugs kept and the slugs passed over, as the library held them. `None`, and nothing kept, where the
    /// project holds a feature of its own.
    pub(super) fn pending_drafted(&self, store: &mut Store, project: &str, drafts: Vec<Draft>, area: (&str, &str)) -> Result<Option<(Vec<String>, Vec<String>)>, WriteError> {
        let kept = store.features_in(project)?;
        if kept.iter().any(|one| one.state != feature_state::PENDING) {
            return Ok(None);
        }
        if store.feature_areas(project)?.is_empty() {
            let (slug, title) = area;
            store.put_feature_area(&KeptArea { project: project.to_string(), slug: slug.to_string(), title: title.to_string(), position: 0 })?;
        }
        let now = self.now().as_str().to_string();
        let (mut drafted, mut held): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
        for (place, one) in drafts.into_iter().enumerate() {
            if kept.iter().any(|feature| feature.slug == one.slug) {
                held.push(one.slug);
                continue;
            }
            if drafted.contains(&one.slug) {
                continue;
            }
            let feature = KeptFeature {
                id: self.minted(),
                project: project.to_string(),
                area: PENDING_AREA.to_string(),
                slug: one.slug.clone(),
                title: one.title,
                summary: one.summary,
                state: feature_state::PENDING.to_string(),
                manifest_id: one.manifest,
                entry_points: Vec::new(),
                paths: if one.path.is_empty() { Vec::new() } else { vec![one.path] },
                position: place as u32,
                added_at: now.clone(),
            };
            store.put_feature(&feature)?;
            drafted.push(one.slug);
        }
        Ok(Some((drafted, held)))
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
        // The library is read again under the lock, so a slug kept since the modules were read is passed over too.
        let drafts = found.into_iter().map(|one| Draft { slug: one.module.slug, title: one.module.name, summary: String::new(), manifest: one.manifest, path: one.module.path }).collect();
        let mut store = self.store().lock().await;
        match self.pending_drafted(&mut store, &project, drafts, MODULES_AREA).map_err(|why| self.ledger_fault(why))? {
            Some((drafted, held)) => Ok(ModulesDrafted { project, drafted, held }),
            None => Err(Refusal::IllegalMove(WireError::raised(STARTED, "This project holds features of its own, so its modules are not drafted.", self.run_id()))),
        }
    }
}
