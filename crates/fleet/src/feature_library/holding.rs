//! Holding a Job's end. **`all_done` is refused while the Job's merged or pushed work changed files of a feature whose
//! sections it has not written since it started**, and the refusal names the features and sections owed. A Job that
//! changed no feature's files is never held, and neither is one with nothing pushed.

use std::collections::BTreeSet;

use ipc::FeatureSectionKind;
use store::{Holder, SessionState, Store, WriteError};

/// The code of the refusal.
pub(crate) const OWED: &str = "fleet.feature_sections_owed";

/// For each feature the Job filed files under, the sections it has not written since it started, in a page's order.
/// A feature the Job wrote every section of is not listed.
pub(super) fn owed_by(store: &Store, job: &str) -> Result<Vec<(String, Vec<FeatureSectionKind>)>, WriteError> {
    let Some(session) = store.session(job)? else {
        return Ok(Vec::new());
    };
    let features: BTreeSet<String> = store.feature_files_of_job(job)?.into_iter().map(|file| file.feature_id).collect();
    let mut owed = Vec::new();
    for id in features {
        let Some(feature) = store.feature_by_id(&id)? else {
            continue;
        };
        // Approving a plan is not consent to change a Job already running: only a Job started once the feature was in
        // the catalog owes it.
        if feature.added_at.as_str() > session.started_at.as_str() {
            continue;
        }
        let written: BTreeSet<String> = store
            .feature_sections(&id)?
            .into_iter()
            .filter(|section| section.job == job && section.written_at.as_str() >= session.started_at.as_str())
            .map(|section| section.section)
            .collect();
        let missing: Vec<FeatureSectionKind> = FeatureSectionKind::ALL.into_iter().filter(|kind| !written.contains(kind.name())).collect();
        if !missing.is_empty() {
            owed.push((feature.slug, missing));
        }
    }
    Ok(owed)
}

/// What `all_done` is told while the Job owes sections, naming each feature and its sections. `None` where nothing is
/// owed, the Job pushed nothing, or it has ended: the person's End job never asks.
pub(crate) fn sections_owed_text(store: &Store, job: &str) -> Result<Option<String>, WriteError> {
    let Some(session) = store.session(job)? else {
        return Ok(None);
    };
    if session.state == SessionState::Ended {
        return Ok(None);
    }
    let pushed = store.attachments_of(&Holder::session(job))?.iter().any(|one| one.kind == crate::pull_requesting::PR);
    if !pushed {
        return Ok(None);
    }
    let owed = owed_by(store, job)?;
    if owed.is_empty() {
        return Ok(None);
    }
    let lines: Vec<String> = owed
        .iter()
        .map(|(feature, sections)| format!("{feature}: {}", sections.iter().map(|one| one.name()).collect::<Vec<_>>().join(", ")))
        .collect();
    Ok(Some(format!(
        "Feature sections owed. Your merged or pushed work changed files of these features:\n\n{}\n\nWrite each with write_feature_sections: the test, runs, observe and debt text of one feature in one call. Rewrite a section that is still true as it stands.",
        lines.join("\n")
    )))
}
