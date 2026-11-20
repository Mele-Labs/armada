//! The features a Job is on, for the head of its Story tab, each with how the Job touched it: the files it wrote under it,
//! the commits that named it and its asks filed under it. `docs/concepts/job-story.md`, *A Job's story*.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Ask, JobFeature};
use store::{feature_link_kind, Store, WriteError};

use super::reading::Loaded;
use crate::daemon::Fleet;

/// What a Job did to one feature, by the feature's id.
#[derive(Default)]
struct Touched {
    files: BTreeSet<String>,
    commits: BTreeSet<String>,
    named: bool,
    asks: BTreeSet<String>,
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
    /// The features `job` is on, most touched first. **A naming the commit gate kept with no commit read from git yet
    /// counts as one commit**: the gate keeps a naming once a Job, and the commits are linked when its checkout is read.
    pub(crate) fn features_of_job_in(&self, store: &Store, job: &str, asks: &[Ask]) -> Result<Vec<JobFeature>, WriteError> {
        let mut touched: BTreeMap<String, Touched> = BTreeMap::new();
        for file in store.feature_files_of_job(job)? {
            touched.entry(file.feature_id).or_default().files.insert(file.path);
        }
        for link in store.feature_links_of_job(job, feature_link_kind::COMMIT)? {
            touched.entry(link.feature_id).or_default().commits.insert(link.target);
        }
        for id in store.feature_ids_named_by_job(job)? {
            touched.entry(id).or_default().named = true;
        }
        let libraries: Vec<Loaded> = self.projects_of_job_in(store, job).iter().filter_map(|project| Loaded::read(store, project).ok()).collect();
        for ask in asks {
            for slug in &ask.features {
                if let Some(feature) = libraries.iter().find_map(|library| library.feature_named(slug)) {
                    touched.entry(feature.id.clone()).or_default().asks.insert(ask.ask_id.clone());
                }
            }
        }
        let mut features = Vec::new();
        for (id, touched) in touched {
            let Some(feature) = store.feature_by_id(&id)? else { continue };
            let commits = touched.commits.len().max(usize::from(touched.named));
            features.push(JobFeature {
                slug: feature.slug,
                title: feature.title,
                area: feature.area,
                files_written: touched.files.len() as u32,
                commits_named: commits as u32,
                asks: touched.asks.len() as u32,
            });
        }
        features.sort_by(|a, b| {
            let most = |one: &JobFeature| one.files_written + one.commits_named + one.asks;
            most(b).cmp(&most(a)).then_with(|| a.title.cmp(&b.title)).then_with(|| a.slug.cmp(&b.slug))
        });
        Ok(features)
    }
}
