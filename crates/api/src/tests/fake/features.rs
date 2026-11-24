//! The feature library, as the fake answers it: one feature named for the call that reached it, so a route test reads
//! what arrived. What Fleet keeps and works out is `fleet::feature_library`'s, against a real store.

use ipc::{
    ChangeFeature, CheckCommitFeature, CommitFeatureChecked, FeatureAreaRow, FeatureBrief, FeatureChanged, FeatureLibrary, FeaturePage, FeatureRow,
    FeatureSectionKind, FeatureSectionsWritten, FeatureState, FeaturesHandedOver, FeaturesOfPath, FeaturesSeeded, FeaturesUpserted, FirstEditSearches,
    HandOverFeatures, SeedFeatures, UpsertFeatures, WriteFeatureSections,
};

use super::FakeDaemon;
use crate::{Features, Refusal};

fn row(slug: &str) -> FeatureRow {
    FeatureRow {
        id: String::from("f-1"),
        slug: slug.to_string(),
        area: String::from("takes"),
        title: String::from("Voting on takes"),
        summary: String::from("Pick, rank or score a page's takes."),
        state: FeatureState::Seeded,
        jobs_now: Vec::new(),
        stale_since: None,
        health: ipc::FeatureHealth { mark: ipc::HealthMark::Good, reasons: Vec::new() },
        retire: None,
        planned: false,
    }
}

impl Features for FakeDaemon {
    async fn list_features(&self, _caller: Option<crate::Caller>, project: Option<String>) -> Result<FeatureLibrary, Refusal> {
        Ok(FeatureLibrary {
            project: project.unwrap_or_else(|| String::from("p1")),
            areas: vec![FeatureAreaRow { slug: String::from("takes"), title: String::from("Takes"), features: vec![row("voting-on-takes")] }],
            librarian: Default::default(),
        })
    }

    async fn get_feature_page(&self, _caller: Option<crate::Caller>, project: Option<String>, feature: String, _section: Option<String>, _files: bool) -> Result<FeaturePage, Refusal> {
        Ok(FeaturePage {
            project: project.unwrap_or_else(|| String::from("p1")),
            feature: row(&feature),
            manifest_id: None,
            paths: Vec::new(),
            entry_points: Vec::new(),
            sections: Vec::new(),
            links: Vec::new(),
            decisions: Vec::new(),
            files: Vec::new(),
            docs: Vec::new(),
            tickets: Vec::new(),
            floor_plan: None,
        })
    }

    async fn get_glossary(&self, _caller: Option<crate::Caller>, project: Option<String>) -> Result<ipc::Glossary, Refusal> {
        Ok(ipc::Glossary { project: project.unwrap_or_default(), terms: Vec::new() })
    }

    async fn get_project_overview(&self, _caller: Option<crate::Caller>, project: Option<String>) -> Result<ipc::ProjectOverview, Refusal> {
        Ok(ipc::ProjectOverview { project: project.unwrap_or_else(|| String::from("p1")), what_it_is: None, rooms: Vec::new(), busiest: Vec::new(), glossary: Vec::new() })
    }

    async fn get_since_you_looked(&self, _caller: Option<crate::Caller>, project: Option<String>, _since: String) -> Result<ipc::SinceYouLooked, Refusal> {
        Ok(ipc::SinceYouLooked { project: project.unwrap_or_else(|| String::from("p1")), paragraph: String::new(), features: Vec::new() })
    }

    async fn get_whos_on_what(&self, _caller: Option<crate::Caller>, project: Option<String>) -> Result<ipc::WhosOnWhat, Refusal> {
        Ok(ipc::WhosOnWhat { project: project.unwrap_or_else(|| String::from("p1")), people: Vec::new(), features: Vec::new() })
    }

    async fn get_feature_brief(&self, _caller: Option<crate::Caller>, _project: Option<String>, feature: String) -> Result<FeatureBrief, Refusal> {
        Ok(FeatureBrief { feature, text: String::from("Voting on takes"), tokens: 4 })
    }

    async fn get_features_of_path(
        &self,
        _caller: Option<crate::Caller>,
        _project: Option<String>,
        _manifest_id: Option<String>,
        path: String,
    ) -> Result<FeaturesOfPath, Refusal> {
        Ok(FeaturesOfPath { path, features: Vec::new() })
    }

    async fn write_feature_sections(&self, _caller: Option<crate::Caller>, write: WriteFeatureSections) -> Result<FeatureSectionsWritten, Refusal> {
        Ok(FeatureSectionsWritten { feature: write.feature, written: vec![FeatureSectionKind::Test], glossary: Vec::new(), gists: 0, owed: Vec::new() })
    }

    async fn upsert_features(&self, _caller: Option<crate::Caller>, upsert: UpsertFeatures) -> Result<FeaturesUpserted, Refusal> {
        Ok(FeaturesUpserted { created: upsert.features.iter().map(|one| one.slug.clone()).collect(), updated: Vec::new(), areas: upsert.areas.len() as u32 })
    }

    async fn run_librarian(&self, _caller: Option<crate::Caller>, run: ipc::RunLibrarian) -> Result<ipc::LibrarianRan, Refusal> {
        Ok(ipc::LibrarianRan { project: run.project, ran_at: ipc::Instant::carried("2026-11-18T09:00:00.000Z"), ran: Vec::new(), read_through: None, skipped_for_share: Vec::new(), stopped: false })
    }

    async fn answer_retire(&self, _caller: Option<crate::Caller>, answer: ipc::AnswerRetire) -> Result<ipc::RetireAnswered, Refusal> {
        Ok(ipc::RetireAnswered { project: answer.project, slug: answer.slug, retired: answer.retire })
    }

    async fn check_docs(&self, _caller: Option<crate::Caller>, check: ipc::CheckDocs) -> Result<ipc::DocsChecked, Refusal> {
        Ok(ipc::DocsChecked { project: check.project, slug: check.slug, checked: Vec::new(), unchecked: Vec::new(), calls: 0 })
    }

    async fn dismiss_doc_flag(&self, _caller: Option<crate::Caller>, flag: ipc::DocFlag) -> Result<ipc::DocFlag, Refusal> {
        Ok(flag)
    }

    async fn start_doc_job(self: std::sync::Arc<Self>, _caller: Option<crate::Caller>, flag: ipc::DocFlag) -> Result<ipc::DocJobStarted, Refusal> {
        Ok(ipc::DocJobStarted { project: flag.project, slug: flag.slug, target: flag.target, session_id: ipc::SessionId::carried("s-1"), title: String::from("Doc or code: Voting on takes"), brief: String::new(), send_failed: None })
    }

    async fn stop_librarian(&self, _caller: Option<crate::Caller>, stop: ipc::StopLibrarian) -> Result<ipc::LibrarianStopping, Refusal> {
        Ok(ipc::LibrarianStopping { project: stop.project, stopping: false })
    }

    async fn seed_features(&self, _caller: Option<crate::Caller>, _seed: SeedFeatures) -> Result<FeaturesSeeded, Refusal> {
        Ok(FeaturesSeeded { created: Vec::new(), areas: 0, dropped: Vec::new() })
    }

    async fn change_feature(&self, _caller: Option<crate::Caller>, change: ChangeFeature) -> Result<FeatureChanged, Refusal> {
        Ok(FeatureChanged { feature: change.slug.unwrap_or_else(|| change.feature.clone()), merged_into: change.into })
    }

    async fn hand_over_features(&self, hand: HandOverFeatures) -> Result<FeaturesHandedOver, Refusal> {
        Ok(FeaturesHandedOver { features: hand.paths, text: String::new() })
    }

    async fn check_commit_feature(&self, check: CheckCommitFeature) -> Result<CommitFeatureChecked, Refusal> {
        let allowed = check.command.contains("Feature:");
        Ok(CommitFeatureChecked { allowed, says: if allowed { String::new() } else { String::from("Add a Feature: line.") }, features: Vec::new() })
    }

    async fn get_first_edit_searches(&self, _weeks: Option<u32>) -> Result<FirstEditSearches, Refusal> {
        Ok(FirstEditSearches { weeks: Vec::new(), unmeasured: 0 })
    }
}
