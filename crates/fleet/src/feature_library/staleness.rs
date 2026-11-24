//! When a written section no longer matches the code. **A section is stale when a merge after it changed a file mapped to
//! its feature**, and it reads `stale since #<pr>`, the first such merge.
//!
//! The merge of a Job's own pull request does not make that Job's sections stale: the section was written inside
//! the work the merge carries. Another Job's merge does.

use std::collections::BTreeSet;

use store::{KeptFeatureMerge, KeptSection};

use super::mapping::FeatureMap;

/// A merge with the ids of the features that own one of its files.
pub(crate) struct OwnedMerge {
    pub(crate) merge: KeptFeatureMerge,
    owners: BTreeSet<String>,
}

/// Each of `merges` with the features `map` gives one of its files to. **Worked out once for every section read against
/// them**: asking each section's every merged file which features own it went through every feature's prefixes each time.
pub(crate) fn owned(map: &FeatureMap, merges: Vec<KeptFeatureMerge>) -> Vec<OwnedMerge> {
    merges
        .into_iter()
        .map(|merge| {
            let files: BTreeSet<String> = merge.files.iter().map(|file| file.trim_start_matches("./").to_string()).collect();
            let owners = map.owning_any(Some(&merge.manifest_id), &files);
            OwnedMerge { merge, owners }
        })
        .collect()
}

/// The number of the first merge in `merges`, oldest first, after `section` was written and not the writer's own,
/// that changed a file of `feature`.
pub(crate) fn stale_since<'a>(feature: &str, section: &KeptSection, merges: impl IntoIterator<Item = &'a OwnedMerge>) -> Option<u64> {
    merges
        .into_iter()
        .filter(|one| one.merge.merged_at.as_str() > section.written_at.as_str())
        .filter(|one| one.merge.job.is_empty() || one.merge.job != section.job)
        .find(|one| one.owners.contains(feature))
        .map(|one| one.merge.number)
}
