//! When a written section no longer matches the code. **A section is stale when a merge after it changed a file mapped to
//! its feature**, and it reads `stale since #<pr>`, the first such merge.
//!
//! The merge of a Job's own pull request does not make that Job's sections stale: the section was written inside
//! the work the merge carries. Another Job's merge does.

use store::{KeptFeatureMerge, KeptSection};

use super::mapping::FeatureMap;

/// The number of the first merge in `merges`, oldest first, after `section` was written and not the writer's own,
/// that changed a file `map` gives to `feature`.
pub(crate) fn stale_since(map: &FeatureMap, feature: &str, section: &KeptSection, merges: &[KeptFeatureMerge]) -> Option<u64> {
    merges
        .iter()
        .filter(|merge| merge.merged_at.as_str() > section.written_at.as_str())
        .filter(|merge| merge.job.is_empty() || merge.job != section.job)
        .find(|merge| merge.files.iter().any(|file| map.feature_owns(feature, Some(&merge.manifest_id), file)))
        .map(|merge| merge.number)
}
