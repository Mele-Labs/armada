//! What Fleet sees tie work to features, and files without asking anyone. **Nothing here is a Job's word**: a file is
//! filed under a feature because the write gate or the forge saw it written, a pull request because Fleet attached it
//! to a Job, a merge because the forge says it merged, and an artifact because its act named the feature.
//!
//! The files a Job wrote are kept in memory by the write table, and filed from there every few seconds and before any
//! read of the library, so a restart loses at most a few seconds of writes.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::ManifestId;
use store::{feature_link_kind, KeptFeatureLink, KeptFeature, KeptFeatureMerge, Store, WriteError};

use super::mapping::FeatureMap;
use super::reading::Loaded;
use crate::clashing::{base_of, Who};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// How many paths of each writer were filed last, so a tick that finds nothing new files nothing.
fn filed_before() -> &'static Mutex<BTreeMap<String, usize>> {
    static FILED: OnceLock<Mutex<BTreeMap<String, usize>>> = OnceLock::new();
    FILED.get_or_init(Mutex::default)
}

/// Keep that `job` wrote `paths` in `manifest` under each feature of `loaded` that owns one, and that the Job is on it.
fn filed_under(store: &mut Store, loaded: &Loaded, job: &str, manifest: &str, paths: &[String], via: &str, now: &str) -> Result<(), WriteError> {
    let title = store.session(job)?.and_then(|session| session.title).unwrap_or_default();
    for path in paths {
        for owner in loaded.map.owners(Some(manifest), path) {
            store.keep_feature_write(&owner.feature.id, job, path, now)?;
            store.put_feature_link(&KeptFeatureLink {
                feature_id: owner.feature.id.clone(),
                kind: feature_link_kind::JOB.to_string(),
                target: job.to_string(),
                job: job.to_string(),
                title: title.clone(),
                state: String::new(),
                via: via.to_string(),
                at: now.to_string(),
                gist: String::new(),
                gist_by: String::new(),
            })?;
        }
    }
    Ok(())
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
    /// The files each Job wrote since the last time, filed under the features that own them. Cheap where nothing was
    /// written or nothing grew.
    pub(crate) async fn feature_writes_filed(&self) {
        let holders = self.writes().holders();
        if holders.is_empty() {
            return;
        }
        let me = std::ptr::from_ref(self) as usize;
        let grown: Vec<_> = {
            let mut before = filed_before().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            holders
                .into_iter()
                .filter(|(who, _, paths)| {
                    let Who::Session(job) = who;
                    before.insert(format!("{me}:{job}"), paths.len()) != Some(paths.len())
                })
                .collect()
        };
        if grown.is_empty() {
            return;
        }
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        for (who, manifest, paths) in grown {
            let Who::Session(job) = who;
            let paths: Vec<String> = paths.into_iter().collect();
            for project in self.projects_of_job_in(&store, &job) {
                if !store.project_has_features(&project).unwrap_or(false) {
                    continue;
                }
                if let Ok(loaded) = Loaded::read(&store, &project) {
                    let _ = filed_under(&mut store, &loaded, &job, &manifest, &paths, "write", &now);
                }
            }
        }
    }

    /// A pull request Fleet attached to a Job, or one that merged: its files filed under the features that own them,
    /// the Job linked to those features, and the pull request linked beside it. A merge is kept with its files, which is
    /// what makes a section stale. **One forge call, and none where the project has no library.**
    pub(crate) async fn feature_pull_filed(&self, served: &Served, number: u64, merged: Option<(&str, Option<&str>, &str)>) {
        let manifest = served.manifest().id().as_str().to_string();
        let (job, projects) = {
            let store = self.store().lock().await;
            if merged.is_some() && store.feature_merge_kept(&manifest, number).unwrap_or(true) {
                return;
            }
            let holder = store
                .attachments_at(crate::pull_requesting::PR, &number.to_string(), Some(&manifest))
                .unwrap_or_default()
                .into_iter()
                .find(|row| row.holder.kind == store::HolderKind::Session)
                .map(|row| row.holder.id);
            let mut projects = holder.as_deref().map(|job| self.projects_of_job_in(&store, job)).unwrap_or_default();
            if projects.is_empty() {
                projects = self.projects_held().holding(&ManifestId::carried(&manifest).to_domain()).into_iter().map(|one| one.id().as_str().to_string()).collect();
            }
            projects.retain(|project| store.project_has_features(project).unwrap_or(false));
            (holder, projects)
        };
        if projects.is_empty() {
            return;
        }
        let ask = number.to_string();
        let Some(files) = self.forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_patches(root, &ask)).await else {
            return;
        };
        let paths: Vec<String> = files.into_iter().map(|file| file.path).collect();
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        if let Some((at, commit, _)) = merged {
            let _ = store.keep_feature_merge(&KeptFeatureMerge {
                manifest_id: manifest.clone(),
                number,
                merged_at: at.to_string(),
                at_commit: commit.unwrap_or_default().to_string(),
                job: job.clone().unwrap_or_default(),
                files: paths.clone(),
            });
        }
        let (kind, at) = match merged {
            Some((at, _, _)) => (feature_link_kind::MERGE, at),
            None => (feature_link_kind::PULL_REQUEST, now.as_str()),
        };
        let title = merged.map(|(_, _, title)| title).unwrap_or_default();
        for project in projects {
            let Ok(loaded) = Loaded::read(&store, &project) else {
                continue;
            };
            if let Some(job) = &job {
                let _ = filed_under(&mut store, &loaded, job, &manifest, &paths, "detector", &now);
            }
            for feature in features_of_files(&loaded.map, &manifest, &paths) {
                let _ = store.put_feature_link(&KeptFeatureLink {
                    feature_id: feature,
                    kind: kind.to_string(),
                    target: number.to_string(),
                    job: job.clone().unwrap_or_default(),
                    title: title.to_string(),
                    state: String::new(),
                    via: "detector".to_string(),
                    at: at.to_string(),
                    gist: String::new(),
                    gist_by: String::new(),
                });
            }
        }
    }

    /// The commits of a Job's checkout that name features, their files filed under what they name and each commit linked.
    /// **Read from git, never from the Job's account of itself.** A checkout already gone files nothing.
    pub(crate) async fn feature_commits_filed(&self, served: &Served, job: &str) {
        let Some(standing) = self.writing_in(&Who::Session(job.to_string())).await else {
            return;
        };
        let base = base_of(served);
        let mut named = Vec::new();
        for worktree in standing.worktrees {
            let base = base.clone();
            if let Ok(Ok(found)) = tokio::task::spawn_blocking(move || adapters::feature_commits::commits_naming(&worktree, &base)).await {
                named.extend(found);
            }
        }
        if named.is_empty() {
            return;
        }
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        self.commits_filed_in(&mut store, job, &named, &now);
    }

    /// `named`, the commits of `job` that name features, filed. **A slug the library lacks files under a pending feature**,
    /// made now for the Operator to bring to the owner.
    pub(crate) fn commits_filed_in(&self, store: &mut Store, job: &str, named: &[adapters::feature_commits::NamingCommit], now: &str) {
        for project in self.projects_of_job_in(store, job) {
            let Ok(loaded) = Loaded::read(store, &project) else {
                continue;
            };
            for commit in named {
                for slug in &commit.features {
                    let Some(feature) = self.feature_or_pending(store, &loaded, slug, now) else {
                        continue;
                    };
                    for path in &commit.files {
                        let _ = store.keep_feature_write(&feature.id, job, path, now);
                    }
                    for (kind, target) in [(feature_link_kind::COMMIT, commit.sha.as_str()), (feature_link_kind::JOB, job)] {
                        let _ = store.put_feature_link(&KeptFeatureLink {
                            feature_id: feature.id.clone(),
                            kind: kind.to_string(),
                            target: target.to_string(),
                            job: job.to_string(),
                            title: String::new(),
                            state: String::new(),
                            via: "trailer".to_string(),
                            at: now.to_string(),
                            gist: String::new(),
                            gist_by: String::new(),
                        });
                    }
                }
            }
        }
    }

    /// An artifact a Job made, linked to the features its act named. **Only an act that names a feature links**: the
    /// attachment's `feature` detail is slugs, comma separated, and `form` says whether it is a sketch, a page, a draft or a
    /// walk. **A slug the project does not hold is kept for the Operator to confirm**, never refused and never linked.
    pub(crate) fn artifact_filed(&self, store: &mut Store, job: &str, target: &str, detail: &BTreeMap<String, String>, now: &str) {
        let Some(named) = detail.get(ipc::ARTIFACT_FEATURE) else {
            return;
        };
        let slugs = ipc::FeatureNames::One(named.clone()).slugs();
        let kind = match detail.get("form").map(String::as_str) {
            Some("sketch") => feature_link_kind::SKETCH,
            Some("document") => feature_link_kind::PAGE,
            Some("draft") => feature_link_kind::DRAFT,
            // A walk is kept as a `window` and a page of a walk's takes is still one walk.
            Some("walk" | "window") => feature_link_kind::WALK,
            _ => return,
        };
        let title = detail.get("title").cloned().unwrap_or_default();
        let projects = self.projects_of_job_in(store, job);
        for slug in slugs {
            let mut held = false;
            for project in &projects {
                let Ok(loaded) = Loaded::read(store, project) else {
                    continue;
                };
                let Some(feature) = loaded.feature_named(&slug).cloned() else {
                    continue;
                };
                held = true;
                self.artifact_linked(store, &feature, kind, job, target, &title, now);
            }
            if held {
                continue;
            }
            // Named where no project holds it: a pending feature in the first project that keeps a library, for the
            // Operator to bring the owner on a card, as a trailer's unknown slug is.
            for project in &projects {
                let Ok(loaded) = Loaded::read(store, project) else {
                    continue;
                };
                if let Some(feature) = self.feature_or_pending(store, &loaded, &slug, now) {
                    self.artifact_linked(store, &feature, kind, job, target, &title, now);
                    break;
                }
            }
        }
    }

    /// One artifact linked to one feature it named.
    #[allow(clippy::too_many_arguments)]
    fn artifact_linked(&self, store: &mut Store, feature: &KeptFeature, kind: &str, job: &str, target: &str, title: &str, now: &str) {
        let _ = store.put_feature_link(&KeptFeatureLink {
            feature_id: feature.id.clone(),
            kind: kind.to_string(),
            target: target.to_string(),
            job: job.to_string(),
            title: title.to_string(),
            state: String::new(),
            via: "artifact".to_string(),
            at: now.to_string(),
            gist: String::new(),
            gist_by: String::new(),
        });
    }
}

/// The ids of the features that own any of `paths` in `manifest`, each once.
fn features_of_files(map: &FeatureMap, manifest: &str, paths: &[String]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for path in paths {
        for owner in map.owners(Some(manifest), path) {
            if !found.contains(&owner.feature.id) {
                found.push(owner.feature.id.clone());
            }
        }
    }
    found
}
