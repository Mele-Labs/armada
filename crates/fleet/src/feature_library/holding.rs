//! Holding a Job's end. **`all_done` is refused while the Job has pushed work and its commits named a feature in a `Feature:`
//! line whose sections it has not written since it started**, and the refusal names the features and sections owed. A
//! feature whose files it changed without naming it is owed nothing. A Job whose commits named none is never held, and
//! neither is one with nothing pushed.

use std::collections::BTreeSet;

use ipc::FeatureSectionKind;
use store::{feature_gist_by, feature_link_kind, Holder, SessionState, Store, WriteError};

use super::consulting::READ;

/// The code of the refusal.
pub(crate) const OWED: &str = "fleet.feature_sections_owed";

/// For each feature the Job's commits named, the sections it has not written since it started, in a page's order.
/// A feature the Job wrote every section of is not listed.
pub(super) fn owed_by(store: &Store, job: &str) -> Result<Vec<(String, Vec<FeatureSectionKind>)>, WriteError> {
    let Some(session) = store.session(job)? else {
        return Ok(Vec::new());
    };
    // Only what its commits name: files it changed under another feature link the Job there but owe nothing.
    let mut features: BTreeSet<String> = store.feature_ids_named_by_job(job)?.into_iter().collect();
    features.extend(store.feature_links_of_job(job, feature_link_kind::COMMIT)?.into_iter().map(|link| link.feature_id));
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

/// The most docs named as welcome a gist, so a Job that read forty pages is not handed forty lines.
const MOST_GISTS_ASKED: usize = 8;

/// A doc or ticket this Job read that is linked under a feature it owes, and has no gist of its own: the feature's slug,
/// the address it was read by and its title. Each address once, under the first feature that holds it, at most eight.
pub(super) fn gists_welcome(store: &Store, job: &str, owed: &[(String, Vec<FeatureSectionKind>)]) -> Result<Vec<(String, String, String)>, WriteError> {
    let mut named: Vec<(String, String, String)> = Vec::new();
    for kind in [feature_link_kind::DOC, feature_link_kind::TICKET] {
        for link in store.feature_links_of_job(job, kind)? {
            if link.via != READ || link.gist_by == feature_gist_by::JOB || named.iter().any(|(_, target, _)| *target == link.target) {
                continue;
            }
            let Some(feature) = store.feature_by_id(&link.feature_id)? else {
                continue;
            };
            if owed.iter().any(|(slug, _)| *slug == feature.slug) {
                named.push((feature.slug, link.target, link.title));
            }
        }
    }
    named.truncate(MOST_GISTS_ASKED);
    Ok(named)
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
    let docs = gists_welcome(store, job, &owed)?;
    let welcome = if docs.is_empty() {
        String::new()
    } else {
        let lines: Vec<String> =
            docs.iter().map(|(_, target, title)| if title == target { format!("- {target}") } else { format!("- {target}: {title}") }).collect();
        format!("\n\nDocs you read, linked under these features. A gist is welcome, never required: give `gists`, `{{target, gist}}` with one line of what the doc says that bears on the feature, in the same call.\n\n{}", lines.join("\n"))
    };
    Ok(Some(format!(
        "Feature sections owed, because your commits named these features:\n\n{}\n\nWrite each with write_feature_sections: the test, runs, observe and debt text of one feature in one call. Rewrite a section that is still true as it stands.{welcome}",
        lines.join("\n")
    )))
}
