//! What a Job writes into the library, and what the Operator seeds. A section is written by the Job that did the work,
//! with the commit its checkout stood at; the shape of the library, its areas and features, is the Operator's.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FeatureOwed, FeatureSectionsWritten, FeaturesUpserted, UpsertFeatures, WriteFeatureSections};
use store::{KeptArea, KeptFeature, KeptSection, SessionState};

use super::reading::Loaded;
use crate::clashing::Who;
use crate::daemon::Fleet;

const SECTION_EMPTY: &str = "fleet.feature_section_empty";
pub(super) const AREA_UNKNOWN: &str = "fleet.feature_area_unknown";
const BLANK: &str = "fleet.feature_blank";
pub(super) const SLUG: &str = "fleet.feature_slug";

/// A slug is what a commit's `Feature:` trailer can carry: lowercase letters, digits, dashes, underscores, dots and slashes.
pub(super) fn slug_ok(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '/' | '.'))
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
    /// The commit the Job's checkout stands at, empty where it has none or git would not say.
    async fn commit_of_job(&self, job: &str) -> String {
        let Some(standing) = self.writing_in(&Who::Session(job.to_string())).await else {
            return String::new();
        };
        let Some(worktree) = standing.worktrees.first().cloned() else {
            return String::new();
        };
        tokio::task::spawn_blocking(move || adapters::feature_commits::head(&worktree)).await.ok().flatten().unwrap_or_default()
    }

    /// `write_feature_sections`.
    pub(super) async fn feature_sections_written(
        &self,
        caller: Option<api::Caller>,
        write: WriteFeatureSections,
    ) -> Result<FeatureSectionsWritten, Refusal> {
        let job = self.waiting_session(caller.clone(), write.session_id.as_ref())?;
        if (write.sections.is_empty() && write.glossary.is_empty() && write.gists.is_empty()) || write.sections.iter().any(|one| one.text.trim().is_empty()) {
            return Err(self.hosted_refusal(SECTION_EMPTY, "each section needs its text, one or more sentences"));
        }
        if write.glossary.iter().any(|one| one.term.trim().is_empty() || one.meaning.trim().is_empty()) {
            return Err(self.hosted_refusal(super::glossary::BLANK, "each glossary term needs the word and what it means"));
        }
        if write.gists.iter().any(|one| one.target.trim().is_empty() || one.gist.trim().is_empty()) {
            return Err(self.hosted_refusal(super::gisting::BLANK, "each gist needs the target it was read by and a line of what it says"));
        }
        self.feature_writes_filed().await;
        let commit = self.commit_of_job(&job).await;
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let session = store.session(&job).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(SECTION_EMPTY, "a job Fleet knows"))?;
        if session.state == SessionState::Ended {
            return Err(self.closed(&job));
        }
        let mut projects = self.projects_of_job_in(&store, &job);
        if let Some(named) = write.project.as_deref().map(str::trim).filter(|named| !named.is_empty()) {
            projects.insert(0, named.to_string());
        }
        let mut found = None;
        for project in &projects {
            let loaded = Loaded::read(&store, project).map_err(|why| self.ledger_fault(why))?;
            if let Some(feature) = loaded.feature_named(write.feature.trim()) {
                found = Some(feature.clone());
                break;
            }
        }
        let feature = found.ok_or_else(|| self.hosted_refusal(super::reading::UNKNOWN, &format!("no feature `{}` in this job's project", write.feature.trim())))?;
        if let Some(target) = super::gisting::unread(&store, &job, &write.gists).map_err(|why| self.ledger_fault(why))? {
            return Err(self.hosted_refusal(super::gisting::NOT_READ, &format!("you did not read and link `{target}`: give a gist only of a doc or ticket you read before a change to the feature's files")));
        }
        let mut written = Vec::new();
        for one in &write.sections {
            store
                .put_feature_section(&KeptSection {
                    feature_id: feature.id.clone(),
                    section: one.section.name().to_string(),
                    text: one.text.trim().to_string(),
                    job: job.clone(),
                    written_at: now.clone(),
                    at_commit: commit.clone(),
                })
                .map_err(|why| self.ledger_fault(why))?;
            if !written.contains(&one.section) {
                written.push(one.section);
            }
        }
        let glossary = super::glossary::terms_kept(&mut store, &feature.project, &job, &now, &write.glossary).map_err(|why| self.ledger_fault(why))?;
        let gists = super::gisting::gists_kept(&mut store, &job, &write.gists).map_err(|why| self.ledger_fault(why))?;
        let owed = super::holding::owed_by(&store, &job).map_err(|why| self.ledger_fault(why))?;
        let welcome = super::holding::gists_welcome(&store, &job, &owed).map_err(|why| self.ledger_fault(why))?;
        let owed = owed
            .into_iter()
            .map(|(feature, sections)| {
                let docs = welcome.iter().filter(|(slug, ..)| *slug == feature).map(|(_, target, _)| target.clone()).collect();
                FeatureOwed { feature, sections, docs }
            })
            .collect();
        Ok(FeatureSectionsWritten { feature: feature.slug, written, glossary, gists, owed })
    }

    /// `upsert_features`. **Nothing is kept unless every feature names an area**, one of the call's or one already kept.
    pub(super) async fn features_upserted(&self, upsert: UpsertFeatures) -> Result<FeaturesUpserted, Refusal> {
        let project = upsert.project.trim().to_string();
        if project.is_empty() {
            return Err(self.hosted_refusal(BLANK, "name the project"));
        }
        for area in &upsert.areas {
            if area.slug.trim().is_empty() || area.title.trim().is_empty() {
                return Err(self.hosted_refusal(BLANK, "an area needs a slug and a title"));
            }
            if !slug_ok(area.slug.trim()) {
                return Err(self.hosted_refusal(SLUG, &format!("`{}` is not a slug: lowercase letters, digits and dashes", area.slug.trim())));
            }
        }
        for feature in &upsert.features {
            if feature.slug.trim().is_empty() || feature.title.trim().is_empty() || feature.summary.trim().is_empty() {
                return Err(self.hosted_refusal(BLANK, "a feature needs a slug, a title and a summary"));
            }
            if !slug_ok(feature.slug.trim()) {
                return Err(self.hosted_refusal(SLUG, &format!("`{}` is not a slug: lowercase letters, digits and dashes", feature.slug.trim())));
            }
        }
        let mut store = self.store().lock().await;
        let kept_areas: BTreeSet<String> =
            store.feature_areas(&project).map_err(|why| self.ledger_fault(why))?.into_iter().map(|one| one.slug).collect();
        let known: BTreeSet<&str> = kept_areas.iter().map(String::as_str).chain(upsert.areas.iter().map(|one| one.slug.trim())).collect();
        if let Some(feature) = upsert.features.iter().find(|feature| !known.contains(feature.area.trim())) {
            return Err(self.hosted_refusal(AREA_UNKNOWN, &format!("feature `{}` is in area `{}`, which is neither in this call nor kept", feature.slug.trim(), feature.area.trim())));
        }
        let already = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        for (place, area) in upsert.areas.iter().enumerate() {
            store
                .put_feature_area(&KeptArea { project: project.clone(), slug: area.slug.trim().to_string(), title: area.title.trim().to_string(), position: place as u32 })
                .map_err(|why| self.ledger_fault(why))?;
        }
        let now = self.now().as_str().to_string();
        let (mut created, mut updated) = (Vec::new(), Vec::new());
        for (place, feature) in upsert.features.iter().enumerate() {
            let slug = feature.slug.trim().to_string();
            let before = already.feature(&slug);
            let state = match (feature.state, before) {
                (Some(ipc::FeatureState::Confirmed), _) => store::feature_state::CONFIRMED,
                (Some(ipc::FeatureState::Seeded), _) => store::feature_state::SEEDED,
                (Some(ipc::FeatureState::Pending), _) => store::feature_state::PENDING,
                (None, Some(held)) if held.state == store::feature_state::CONFIRMED => store::feature_state::CONFIRMED,
                (None, _) => store::feature_state::SEEDED,
            };
            store
                .put_feature(&KeptFeature {
                    id: self.minted(),
                    project: project.clone(),
                    area: feature.area.trim().to_string(),
                    slug: slug.clone(),
                    title: feature.title.trim().to_string(),
                    summary: feature.summary.trim().to_string(),
                    state: state.to_string(),
                    manifest_id: feature.manifest_id.clone().unwrap_or_default().trim().to_string(),
                    entry_points: feature.entry_points.iter().map(|point| point.trim().to_string()).filter(|point| !point.is_empty()).collect(),
                    paths: feature.paths.iter().map(|path| path.trim().to_string()).filter(|path| !path.is_empty()).collect(),
                    position: place as u32,
                    added_at: now.clone(),
                })
                .map_err(|why| self.ledger_fault(why))?;
            match before {
                Some(_) => updated.push(slug),
                None => created.push(slug),
            }
        }
        Ok(FeaturesUpserted { created, updated, areas: upsert.areas.len() as u32 })
    }
}
