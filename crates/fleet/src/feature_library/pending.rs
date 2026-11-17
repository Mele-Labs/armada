//! A feature a Job named that the catalog does not hold. **It is filed under a pending feature**, which the Operator brings to
//! the owner on a card: confirm it, merge it into an existing one, or rename it. `docs/concepts/feature-catalog.md`,
//! *A feature a Job names that does not exist*.
//!
//! A pending feature exists only in a library that has been seeded or kept: in a project with none, a named slug files
//! nothing, so the earliest `added_at` of a project stays the moment its library was seeded.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{feature_link_kind, feature_state, KeptFeature, Store, WriteError};

use super::mapping::How;
use super::reading::Loaded;
use super::writing::slug_ok;
use crate::daemon::Fleet;

/// The area pending features are listed under until the owner places one.
pub(super) const PENDING_AREA: &str = "pending";

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
    /// The feature `slug` names in the project `loaded` reads, or a pending one made now. `None` for a slug a trailer cannot
    /// carry, and for a project that holds no confirmed or seeded feature.
    pub(super) fn feature_or_pending(&self, store: &mut Store, loaded: &Loaded, slug: &str, now: &str) -> Option<KeptFeature> {
        if let Some(held) = loaded.feature_named(slug) {
            return Some(held.clone());
        }
        let slug = slug.trim();
        if !slug_ok(slug) || !loaded.map.features().iter().any(|one| one.state != feature_state::PENDING) {
            return None;
        }
        // The library may have moved since `loaded` was read, by a pending feature made for an earlier commit of this call.
        if let Some(held) = store.feature_named(&loaded.project, slug).ok().flatten() {
            return Some(held);
        }
        let pending = KeptFeature {
            id: self.minted(),
            project: loaded.project.clone(),
            area: PENDING_AREA.to_string(),
            slug: slug.to_string(),
            title: slug.to_string(),
            summary: String::new(),
            state: feature_state::PENDING.to_string(),
            manifest_id: String::new(),
            entry_points: Vec::new(),
            paths: Vec::new(),
            position: 0,
            added_at: now.to_string(),
        };
        store.put_feature(&pending).ok().map(|id| KeptFeature { id, ..pending })
    }
}

/// What the Operator is told of a pending feature: who named it, the files filed under it, and the features whose prefixes cover those files.
pub(crate) struct PendingFacts {
    /// Each Job that named it, with its title and how: `trailer` or `artifact`.
    pub(crate) jobs: Vec<(String, String, String)>,
    /// The files the most Jobs wrote under it.
    pub(crate) files: Vec<String>,
    /// The features that own those files by a prefix, the most files first.
    pub(crate) owners: Vec<(String, usize)>,
}

/// The facts of one pending feature, read from what Fleet saw.
pub(crate) fn pending_facts(store: &Store, feature: &KeptFeature) -> Result<PendingFacts, WriteError> {
    let mut jobs: Vec<(String, String, String)> = Vec::new();
    for link in store.feature_links(&feature.id)? {
        if link.job.is_empty() || jobs.iter().any(|(job, _, _)| *job == link.job) {
            continue;
        }
        let title = store.session(&link.job)?.and_then(|session| session.title).unwrap_or_default();
        let via = match link.kind.as_str() {
            feature_link_kind::COMMIT | feature_link_kind::JOB => "a commit trailer",
            _ => "an act",
        };
        jobs.push((link.job, title, via.to_string()));
    }
    let top = store.feature_top_files(&feature.id, 20)?;
    let loaded = Loaded::read(store, &feature.project)?;
    let mut owners: BTreeMap<String, usize> = BTreeMap::new();
    for (path, _) in &top {
        for owner in loaded.map.owners(None, path) {
            if owner.how == How::Prefix && owner.feature.id != feature.id {
                *owners.entry(owner.feature.slug.clone()).or_default() += 1;
            }
        }
    }
    let mut owners: Vec<(String, usize)> = owners.into_iter().collect();
    owners.sort_by(|one, other| other.1.cmp(&one.1).then_with(|| one.0.cmp(&other.0)));
    Ok(PendingFacts { jobs, files: top.into_iter().take(5).map(|(path, _)| path).collect(), owners })
}
