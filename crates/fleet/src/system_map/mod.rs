//! A project's system map: what it is made of and how the parts reach each other, read from its own files with no model.
//! `docs/practices/protocol/projects.md`, *The system map*.
//!
//! **Detection is table-driven and read-only**: `detect` finds the parts, `pieces` the pieces of each, `edges` what reaches
//! what, `levels` draws the three levels, `answers` the nine sections of a part. A repository's map is held per `HEAD`, so a
//! read after the first costs nothing until a commit lands.

mod answers;
mod detect;
mod edges;
mod folder;
mod levels;
mod librarian;
mod model;
mod pieces;
mod repo;
mod scanning;
mod tables;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SystemMap, WireError};

use self::levels::{Found, Ground};
use self::model::{Mapped, Part};
use self::repo::{KitServer, Repo};
use crate::daemon::Fleet;
use crate::feature_library::mapping::FeatureMap;

const NO_PART: &str = "fleet.system_part_unknown";
const NO_FEATURE: &str = "fleet.feature_unknown";

/// One repository of a project, as it is read.
pub(crate) struct Member {
    pub(crate) manifest: String,
    pub(crate) root: PathBuf,
    kit: Vec<KitServer>,
}

fn held() -> &'static Mutex<BTreeMap<String, (String, Arc<Mapped>)>> {
    static HELD: OnceLock<Mutex<BTreeMap<String, (String, Arc<Mapped>)>>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

/// The commit the checkout stands at, empty where git will not say.
fn head_of(root: &Path) -> String {
    crate::files::git_in(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Kit's servers, each given to the part whose folder it runs in or whose module its command names.
fn kit_of(repo: &Repo, parts: &[Part]) -> Vec<(usize, KitServer)> {
    let mut out = Vec::new();
    for server in &repo.kit {
        let by_dir = server.dir.as_deref().and_then(|dir| {
            parts.iter().position(|part| part.wire.paths.iter().any(|path| model::holds(path, dir.trim_start_matches("./")) && path.trim_matches('/') != "."))
        });
        let by_text = || {
            parts.iter().position(|part| {
                part.modules.iter().any(|module| {
                    let module = &repo.layout.modules[*module];
                    server.serve.split_whitespace().any(|word| word.trim_matches('"') == module.name) || (!module.root.is_empty() && server.serve.contains(&module.root))
                })
            })
        };
        if let Some(part) = by_dir.or_else(by_text) {
            out.push((part, server.clone()));
        }
    }
    out
}

/// The modules of the HTTP servers nobody runs from here that no other part reaches by a route or by its crate's name.
fn unheard_apis(det: &detect::Detected, pieces: &[model::Piece], edges: &[model::Edge]) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for (at, part) in det.parts.iter().enumerate().filter(|(_, part)| part.api) {
        let heard = edges.iter().any(|edge| {
            let to = match edge.to {
                model::Node::Piece(piece) => pieces[piece].part,
                model::Node::Part(part) => part,
            };
            to == at && pieces[edge.from].part != at && !det.parts[pieces[edge.from].part].api
        });
        if !heard {
            out.extend(part.modules.iter().copied());
        }
    }
    out
}

/// The map of one repository, held per `HEAD`. **Blocking**: the caller runs it off the main thread.
fn mapped(member: &Member) -> Arc<Mapped> {
    let key = member.root.to_string_lossy().into_owned();
    let head = head_of(&member.root);
    if !head.is_empty() {
        if let Some((at, map)) = held().lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
            if *at == head {
                return Arc::clone(map);
            }
        }
    }
    let repo = Repo::read(&member.manifest, &member.root, member.kit.clone());
    let mut det = detect::detect(&repo, &BTreeSet::new());
    let mut pieces = pieces::pieces(&repo, &mut det);
    let mut reach = edges::reach(&repo, &det, &pieces);
    // An HTTP server no client names a route of is the code of whatever runs it, so it is read again as a library.
    let unheard = unheard_apis(&det, &pieces, &reach.edges);
    if !unheard.is_empty() {
        det = detect::detect(&repo, &unheard);
        pieces = pieces::pieces(&repo, &mut det);
        reach = edges::reach(&repo, &det, &pieces);
    }
    let kit = kit_of(&repo, &det.parts);
    let mut parts = det.parts;
    // A server nobody runs from here is named for the service that serves it.
    for link in reach.links.iter().filter(|link| link.label.as_deref() == Some("served by")) {
        let service = parts.iter().find(|part| part.wire.id == link.to).map(|part| part.wire.name.clone());
        if let (Some(service), Some(api)) = (service, parts.iter_mut().find(|part| part.api && part.wire.id == link.from)) {
            api.wire.name = format!("{service} API");
        }
    }
    let map = Arc::new(Mapped { manifest: member.manifest.clone(), parts, pieces, edges: reach.edges, links: reach.links, files: repo.layout.files, kit, routes: reach.routes });
    if !head.is_empty() {
        held().lock().unwrap_or_else(PoisonError::into_inner).insert(key, (head, Arc::clone(&map)));
    }
    map
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
    /// The repositories of `project` Fleet serves, with the servers each one's Manifest declares.
    pub(crate) async fn system_members(&self, project: &str) -> Result<Vec<Member>, Refusal> {
        let held = self.project_held(project).await?;
        let mut members = Vec::new();
        for id in held.repositories() {
            let Some(served) = self.repositories().serving(id.as_str()) else { continue };
            let manifest = served.manifest();
            let kit = manifest
                .server_names()
                .into_iter()
                .filter_map(|name| {
                    let server = manifest.server(&name)?;
                    Some(KitServer { serve: server.serve().to_string(), dir: server.dir().map(str::to_string), links: server.links().iter().map(|link| link.url().to_string()).collect(), name })
                })
                .collect();
            members.push(Member { manifest: id.as_str().to_string(), root: PathBuf::from(served.root()), kit });
        }
        Ok(members)
    }

    /// The repositories of `project` mapped, off the main thread.
    async fn system_mapped(&self, project: &str) -> Result<Vec<Arc<Mapped>>, Refusal> {
        let members = self.system_members(project).await?;
        tokio::task::spawn_blocking(move || members.iter().map(mapped).collect()).await.map_err(|why| self.hosted_refusal("fleet.system_map_unreadable", &format!("reading the system map stopped: {why}")))
    }

    /// `get_system_map`.
    pub(crate) async fn system_map_read(&self, project: String, part: Option<String>, feature: Option<String>) -> Result<SystemMap, Refusal> {
        let maps = self.system_mapped(&project).await?;
        let (features, filed, additions, librarian_at) = self.system_ground(&project, &maps).await?;
        let map = FeatureMap::new(features.clone(), filed);
        let in_map = |feature: &str, file: &str| map.filed_of(feature).any(|one| one == file);
        let ground = Ground { project: &project, maps: &maps, features: &features, filed: &in_map, additions: &additions, librarian_at };
        if let Some(slug) = feature {
            let found = features.iter().find(|one| one.slug == slug).ok_or_else(|| Refusal::NotFound(WireError::raised(NO_FEATURE, format!("the project holds no feature `{slug}`"), self.run_id())))?;
            return Ok(ground.level_three(found));
        }
        match part {
            None => Ok(ground.level_one()),
            Some(id) => match ground.find(&id) {
                Some(Found::Detected { repo, part }) => Ok(ground.level_two(repo, part)),
                Some(Found::Added(_)) => Ok(ground.level_one()).map(|one| SystemMap { part: Some(id), parts: Vec::new(), links: Vec::new(), ..one }),
                None => Err(Refusal::NotFound(WireError::raised(NO_PART, format!("the map holds no part `{id}`"), self.run_id()))),
            },
        }
    }
}

/// One level of the map of the checkout at `root`, for the tests: no store, no Fleet, features given.
#[cfg(test)]
pub(crate) fn read_for_test(root: &Path, manifest: &str, part: Option<&str>, feature: Option<&store::KeptFeature>) -> SystemMap {
    let maps = vec![mapped(&Member { manifest: manifest.to_string(), root: root.to_path_buf(), kit: Vec::new() })];
    let features: Vec<store::KeptFeature> = feature.cloned().into_iter().collect();
    let filed = |_: &str, _: &str| false;
    let ground = Ground { project: "p", maps: &maps, features: &features, filed: &filed, additions: &[], librarian_at: None };
    match (feature, part.and_then(|id| ground.find(id))) {
        (Some(feature), _) => ground.level_three(feature),
        (None, Some(Found::Detected { repo, part })) => ground.level_two(repo, part),
        _ => ground.level_one(),
    }
}
