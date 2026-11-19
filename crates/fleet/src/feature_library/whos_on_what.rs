//! Who is on what: the open pull requests of the Reviews lists, teammates' included, each kept with its author and the
//! files it changes, and the files mapped to the features that own them. `docs/concepts/feature-catalog.md`,
//! *Who's on what*.
//!
//! **One forge read a pull request a head, at the moment the lists are read anyway.** A pull request whose head is the
//! one already kept is not read again, and one the lists no longer name is dropped. Nothing here calls a model, and
//! `get_whos_on_what` asks no forge at all: it reads what the lists left.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{PullReviewLists, PullReviewRow, WhoCell, WhoFeature, WhoPerson, WhoPull, WhosOnWhat};
use store::{feature_state, KeptFeature, KeptOpenPull, Store};

use super::reading::Loaded;
use crate::daemon::Fleet;
use crate::my_pulls::Mine;

/// The most files kept of a pull request. A pull request that changes more is kept with `files_capped`.
pub(crate) const MOST_FILES: usize = 100;
/// The most pull requests a feature's brief names as open elsewhere.
const MOST_ELSEWHERE: usize = 5;
/// The owner's login in a cell where nobody is known to be signed in to the forge.
const ME_UNKNOWN: &str = "me";
/// A pull request the forge named no author for.
const NO_AUTHOR: &str = "unknown";

/// The lines of a feature's brief for each open pull request of somebody else that changes a file the feature owns,
/// newest first. **None where the owner's own login is not known**, since then nobody can be called a teammate.
pub(super) fn elsewhere_of(store: &Store, loaded: &Loaded, feature: &KeptFeature, mine: &Mine) -> Vec<String> {
    if mine.login().is_none() {
        return Vec::new();
    }
    let Ok(mut pulls) = store.open_pulls_kept() else { return Vec::new() };
    pulls.retain(|pull| !mine.knows(Some(&pull.author)) && pull.files.iter().any(|file| loaded.map.feature_owns(&feature.id, Some(&pull.manifest_id), file)));
    pulls.sort_by_key(|pull| Reverse(pull.number));
    pulls
        .into_iter()
        .take(MOST_ELSEWHERE)
        .map(|pull| format!("Open elsewhere: #{} \"{}\" by {} touches this feature.", pull.number, pull.title, pull.author))
        .collect()
}

/// The Fleets with a pass of [`Fleet::open_pulls_filed`] going, by address.
fn filing() -> &'static Mutex<BTreeSet<usize>> {
    static GOING: OnceLock<Mutex<BTreeSet<usize>>> = OnceLock::new();
    GOING.get_or_init(Mutex::default)
}

/// A pass going, until it is dropped.
struct Pass(usize);

impl Pass {
    /// The pass, or `None` where one is going already: it reads the heads this one would, so this one has nothing to add.
    fn begin(fleet: usize) -> Option<Pass> {
        filing().lock().unwrap_or_else(PoisonError::into_inner).insert(fleet).then_some(Pass(fleet))
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        filing().lock().unwrap_or_else(PoisonError::into_inner).remove(&self.0);
    }
}

/// One person's cell as it is built.
#[derive(Default)]
struct Cell {
    pulls: Vec<WhoPull>,
    jobs: Vec<ipc::SessionId>,
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
    /// The owner's logins: who he is signed in to the forge as for the first repository served. None known is an empty
    /// [`Mine`], and `elsewhere_of` then says nothing.
    pub(super) async fn owner_logins(&self) -> Mine {
        match self.repositories().first().map(|served| served.root().to_string()) {
            Some(root) => self.mine_in(&root).await,
            None => Mine::of([]),
        }
    }

    /// The open pull requests the lists name, each kept with its files. **A forge read only for a head not kept**; the
    /// title and draft mark of one whose head is kept are refreshed from the row at no cost. The ones the lists no
    /// longer name are dropped, but only from a read in which every list answered, since a list that did not leaves
    /// pull requests unnamed that are still open. A read the forge limited changes nothing.
    pub(crate) async fn open_pulls_filed(&self, project: Option<&str>, lists: &PullReviewLists) {
        if lists.rate_limited.is_some() {
            return;
        }
        let Some(_pass) = Pass::begin(std::ptr::from_ref(self) as usize) else { return };
        let mut lists = lists.clone();
        let mut named: BTreeMap<(String, u64), PullReviewRow> = BTreeMap::new();
        for row in lists.lists_mut().into_iter().flatten() {
            named.entry((row.manifest_id.as_str().to_string(), row.number)).or_insert_with(|| row.clone());
        }
        let kept: BTreeMap<(String, u64), KeptOpenPull> = match self.store().lock().await.open_pulls_kept() {
            Ok(kept) => kept.into_iter().map(|pull| ((pull.manifest_id.clone(), pull.number), pull)).collect(),
            Err(_) => return,
        };
        for ((manifest, number), row) in &named {
            let before = kept.get(&(manifest.clone(), *number));
            let files = match before {
                Some(before) if before.head_sha == row.head => {
                    if before.title == row.title && before.draft == row.draft && before.author == row.author {
                        continue;
                    }
                    (before.files.clone(), before.files_capped)
                }
                _ => {
                    // A repository no project with a catalog holds has nothing to map files to: no read.
                    let held: Vec<String> = self
                        .projects_held()
                        .holding(&ipc::ManifestId::carried(manifest).to_domain())
                        .into_iter()
                        .map(|one| one.id().as_str().to_string())
                        .collect();
                    let catalogued = {
                        let store = self.store().lock().await;
                        held.iter().any(|one| store.project_has_features(one).unwrap_or(false))
                    };
                    if !catalogued {
                        continue;
                    }
                    let Ok(served) = self.served_named(Some(&row.manifest_id)) else { continue };
                    let ask = number.to_string();
                    // The forge was silent or limited: stop, and the heads not read are read at the next refresh.
                    let Some(files) = self.forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_patches(root, &ask)).await else {
                        break;
                    };
                    let mut paths: Vec<String> = files.into_iter().map(|file| file.path).collect();
                    let capped = paths.len() > MOST_FILES;
                    paths.truncate(MOST_FILES);
                    (paths, capped)
                }
            };
            let pull = KeptOpenPull {
                manifest_id: manifest.clone(),
                number: *number,
                repository: row.repository.clone(),
                title: row.title.clone(),
                author: row.author.clone(),
                draft: row.draft,
                head_sha: row.head.clone(),
                files: files.0,
                files_capped: files.1,
                read_at: self.now().as_str().to_string(),
            };
            // Unkept, the next refresh reads the head again.
            let _ = self.store().lock().await.keep_open_pull(&pull);
        }
        if !lists.silent.is_empty() {
            return;
        }
        let scope: Vec<String> = match project {
            Some(id) => match self.project_repositories(id).await {
                Ok(members) => members.iter().map(|member| member.as_str().to_string()).collect(),
                Err(_) => return,
            },
            None => self.repositories().served().iter().map(|served| served.manifest().id().as_str().to_string()).collect(),
        };
        let mut store = self.store().lock().await;
        for manifest in scope {
            let open: Vec<u64> = named.keys().filter(|(one, _)| *one == manifest).map(|(_, number)| *number).collect();
            let _ = store.drop_open_pulls_except(&manifest, &open);
        }
    }

    /// `get_whos_on_what`.
    pub(super) async fn whos_on_what_read(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<WhosOnWhat, Refusal> {
        let (loaded, _) = self.project_library(caller, project).await?;
        // A project Fleet cannot list the members of has no pull requests here, and its Jobs still show.
        let members: Vec<String> = match self.project_repositories(&loaded.project).await {
            Ok(members) => members.iter().map(|member| member.as_str().to_string()).collect(),
            Err(_) => Vec::new(),
        };
        let mine = self.owner_logins().await;
        let me = mine.login().map(str::to_string);
        let me_key = me.clone().unwrap_or_else(|| ME_UNKNOWN.to_string());
        let store = self.store().lock().await;
        let mut pulls = store.open_pulls_kept().map_err(|why| self.ledger_fault(why))?;
        pulls.retain(|pull| members.contains(&pull.manifest_id));
        let on = self.jobs_on_features(&store, &loaded.project)?;
        let mut by_feature: BTreeMap<&str, (&KeptFeature, BTreeMap<String, Cell>)> = BTreeMap::new();
        for feature in loaded.map.features().iter().filter(|feature| feature.state != feature_state::PENDING) {
            let mut cells: BTreeMap<String, Cell> = BTreeMap::new();
            for pull in &pulls {
                if !pull.files.iter().any(|file| loaded.map.feature_owns(&feature.id, Some(&pull.manifest_id), file)) {
                    continue;
                }
                let login = match (pull.author.trim(), mine.knows(Some(&pull.author))) {
                    (_, true) => me_key.clone(),
                    ("", false) => NO_AUTHOR.to_string(),
                    (author, false) => author.to_string(),
                };
                cells.entry(login).or_default().pulls.push(WhoPull { number: pull.number, repository: pull.repository.clone(), title: pull.title.clone(), draft: pull.draft });
            }
            if let Some(jobs) = on.get(&feature.id).filter(|jobs| !jobs.is_empty()) {
                cells.entry(me_key.clone()).or_default().jobs.extend(jobs.iter().map(|job| job.session_id.clone()));
            }
            if !cells.is_empty() {
                by_feature.insert(feature.id.as_str(), (feature, cells));
            }
        }
        let mut logins: BTreeSet<String> = by_feature.values().flat_map(|(_, cells)| cells.keys().cloned()).collect();
        if me.is_some() {
            logins.insert(me_key.clone());
        }
        let mut people: Vec<WhoPerson> = logins.into_iter().map(|login| WhoPerson { me: login == me_key, login }).collect();
        people.sort_by_key(|person| (!person.me, person.login.to_lowercase(), person.login.clone()));
        let mut features: Vec<WhoFeature> = by_feature
            .into_values()
            .map(|(feature, cells)| {
                let mut cells: Vec<WhoCell> = cells
                    .into_iter()
                    .map(|(login, mut cell)| {
                        cell.pulls.sort_by_key(|pull| Reverse(pull.number));
                        WhoCell { login, pulls: cell.pulls, jobs: cell.jobs }
                    })
                    .collect();
                cells.sort_by_key(|cell| (cell.login != me_key, cell.login.to_lowercase(), cell.login.clone()));
                WhoFeature { slug: feature.slug.clone(), title: feature.title.clone(), cells }
            })
            .collect();
        features.sort_by(|a, b| b.cells.len().cmp(&a.cells.len()).then_with(|| a.title.cmp(&b.title)).then_with(|| a.slug.cmp(&b.slug)));
        Ok(WhosOnWhat { project: loaded.project, people, features })
    }
}
