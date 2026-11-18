//! Which features own a file. Fleet does not read the code graph, so a file is mapped by what Fleet can see:
//! the path prefixes a feature owns, and the files a Job filed under it (a commit naming it, or a write while its
//! commits did). **Longest prefix wins; several features may own one file**, by a tie on the prefix or by a Job's filing.

use std::collections::{BTreeMap, BTreeSet};

use store::KeptFeature;

/// Whether `prefix` owns `path`: the path is the prefix, or is under it, on whole segments, so `crates/fleet` owns
/// `crates/fleet/src/a.rs` and not `crates/fleet-extra/a.rs`.
pub(crate) fn owns(prefix: &str, path: &str) -> bool {
    let prefix = prefix.trim_matches('/');
    let path = path.trim_start_matches("./").trim_start_matches('/');
    !prefix.is_empty() && (path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/')))
}

/// How a file came to belong to a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum How {
    Prefix,
    Filed,
}

/// One feature that owns a file, and the prefix that matched where one did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Owner<'a> {
    pub(crate) feature: &'a KeptFeature,
    pub(crate) how: How,
    pub(crate) prefix: Option<&'a str>,
}

/// A project's features and the files Jobs filed under them.
pub(crate) struct FeatureMap {
    features: Vec<KeptFeature>,
    filed: BTreeMap<String, BTreeSet<String>>,
}

impl FeatureMap {
    pub(crate) fn new(features: Vec<KeptFeature>, filed: impl IntoIterator<Item = (String, String)>) -> FeatureMap {
        let mut by_feature: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (feature, path) in filed {
            by_feature.entry(feature).or_default().insert(path);
        }
        FeatureMap { features, filed: by_feature }
    }

    pub(crate) fn features(&self) -> &[KeptFeature] {
        &self.features
    }

    /// The features that own `path` in `manifest`, those owning it by the longest prefix first and then those a Job
    /// filed it under. A feature naming a repository owns nothing in another; `None` reads as any.
    pub(crate) fn owners(&self, manifest: Option<&str>, path: &str) -> Vec<Owner<'_>> {
        let here = |feature: &KeptFeature| feature.manifest_id.is_empty() || manifest.is_none_or(|one| one.is_empty() || one == feature.manifest_id);
        let mut by_prefix: Vec<(usize, Owner<'_>)> = Vec::new();
        for feature in self.features.iter().filter(|feature| here(feature)) {
            let best = feature.paths.iter().filter(|prefix| owns(prefix, path)).max_by_key(|prefix| prefix.trim_matches('/').len());
            if let Some(prefix) = best {
                by_prefix.push((prefix.trim_matches('/').len(), Owner { feature, how: How::Prefix, prefix: Some(prefix.as_str()) }));
            }
        }
        let longest = by_prefix.iter().map(|(length, _)| *length).max();
        let mut found: Vec<Owner<'_>> =
            by_prefix.into_iter().filter(|(length, _)| Some(*length) == longest).map(|(_, owner)| owner).collect();
        for feature in self.features.iter().filter(|feature| here(feature)) {
            let filed = self.filed.get(&feature.id).is_some_and(|paths| paths.contains(path.trim_start_matches("./")));
            if filed && !found.iter().any(|owner| owner.feature.id == feature.id) {
                found.push(Owner { feature, how: How::Filed, prefix: None });
            }
        }
        found
    }

    /// The files Jobs filed under `feature`.
    pub(crate) fn filed_of(&self, feature: &str) -> impl Iterator<Item = &str> {
        self.filed.get(feature).into_iter().flatten().map(String::as_str)
    }

    /// Whether the feature owns `path` by either way.
    pub(crate) fn feature_owns(&self, feature: &str, manifest: Option<&str>, path: &str) -> bool {
        self.owners(manifest, path).iter().any(|owner| owner.feature.id == feature)
    }
}
