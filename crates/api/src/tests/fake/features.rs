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
        cover: None,
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
            pictures: Vec::new(),
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

    async fn add_feature_picture(&self, _caller: Option<crate::Caller>, add: ipc::AddFeaturePicture) -> Result<ipc::FeaturePictureAdded, Refusal> {
        let picture = ipc::FeaturePicture {
            id: String::from("pic-1"),
            url: add.url,
            screen: add.screen,
            commit: None,
            taken_at: ipc::Instant::carried("2026-11-28T09:00:00.000Z"),
            width: 1,
            height: 1,
            bytes: 1,
            by: ipc::PictureBy::Job,
            job: None,
        };
        Ok(ipc::FeaturePictureAdded { feature: add.feature, picture })
    }

    async fn remove_feature_picture(&self, _caller: Option<crate::Caller>, remove: ipc::RemoveFeaturePicture) -> Result<ipc::FeaturePictureRemoved, Refusal> {
        Ok(ipc::FeaturePictureRemoved { feature: String::from("voting-on-takes"), picture: remove.picture, cover: None })
    }

    async fn get_feature_picture(&self, id: String) -> Result<Vec<u8>, Refusal> {
        Ok(id.into_bytes())
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

    async fn get_features_by_meaning(&self, _caller: Option<crate::Caller>, project: Option<String>, _words: String) -> Result<ipc::FeaturesByMeaning, Refusal> {
        Ok(ipc::FeaturesByMeaning { project: project.unwrap_or_else(|| String::from("p1")), slugs: Vec::new() })
    }

    async fn get_library_start(&self, _caller: Option<crate::Caller>, project: Option<String>) -> Result<ipc::LibraryStart, Refusal> {
        Ok(ipc::LibraryStart { project: project.unwrap_or_else(|| String::from("p1")), modules: Vec::new(), docs: Vec::new() })
    }

    async fn draft_module_features(&self, _caller: Option<crate::Caller>, draft: ipc::DraftModules) -> Result<ipc::ModulesDrafted, Refusal> {
        Ok(ipc::ModulesDrafted { project: draft.project, drafted: Vec::new(), held: Vec::new() })
    }

    async fn start_sub_project(self: std::sync::Arc<Self>, start: ipc::StartSubProject) -> Result<ipc::SubProject, Refusal> {
        Ok(ipc::SubProject {
            id: String::from("sp-1"),
            project: start.project,
            name: start.name,
            goal: start.goal,
            jobs: start.jobs,
            features: Vec::new(),
            orchestrator: None,
            started_at: ipc::Instant::carried("2026-11-26T12:00:00Z"),
            ended_at: None,
        })
    }

    async fn list_sub_projects(self: std::sync::Arc<Self>, _project: Option<String>, _ended: bool) -> Result<ipc::SubProjects, Refusal> {
        Ok(ipc::SubProjects { sub_projects: Vec::new() })
    }

    async fn turn_into_orchestrator(self: std::sync::Arc<Self>, turn: ipc::TurnIntoOrchestrator) -> Result<ipc::SubProject, Refusal> {
        Ok(ipc::SubProject {
            id: String::from("sp-1"),
            project: turn.project,
            name: String::new(),
            goal: turn.goal,
            jobs: Vec::new(),
            features: Vec::new(),
            orchestrator: Some(turn.session_id),
            started_at: ipc::Instant::carried("2026-11-26T12:00:00Z"),
            ended_at: None,
        })
    }

    async fn ungroup_sub_project(&self, _ungroup: ipc::UngroupSubProject) -> Result<(), Refusal> {
        Ok(())
    }

    async fn stop_librarian(&self, _caller: Option<crate::Caller>, stop: ipc::StopLibrarian) -> Result<ipc::LibrarianStopping, Refusal> {
        Ok(ipc::LibrarianStopping { project: stop.project, stopping: false })
    }

    async fn ask_librarian(&self, _caller: Option<crate::Caller>, ask: ipc::AskLibrarian) -> Result<ipc::LibrarianAnswered, Refusal> {
        Ok(ipc::LibrarianAnswered { project: ask.project, asked_at: ipc::Instant::carried("2026-11-24T09:00:00.000Z"), reply: ask.words, act: None, calls: 1, limit: 40 })
    }

    async fn undo_librarian_ask(&self, _caller: Option<crate::Caller>, undo: ipc::UndoLibrarianAsk) -> Result<ipc::LibrarianAskUndone, Refusal> {
        Ok(ipc::LibrarianAskUndone { project: undo.project, asked_at: undo.asked_at, reply: String::from("Undone") })
    }

    async fn refile_librarian_ask(&self, _caller: Option<crate::Caller>, refile: ipc::RefileLibrarianAsk) -> Result<ipc::LibrarianAnswered, Refusal> {
        let act = ipc::LibrarianAct::FiledDoc { target: String::from("docs/guide.md"), title: String::from("Guide"), feature: refile.feature.clone(), feature_title: refile.feature };
        Ok(ipc::LibrarianAnswered { project: refile.project, asked_at: refile.asked_at, reply: String::from("Moved"), act: Some(act), calls: 1, limit: 40 })
    }

    async fn rank_with_librarian(&self, _caller: Option<crate::Caller>, rank: ipc::RankWithLibrarian) -> Result<ipc::RankedIssues, Refusal> {
        Ok(ipc::RankedIssues { project: rank.project, days: 30, fetched_at: None, might_pick_up: Vec::new(), might_be_interested: Vec::new(), by_librarian: Some(ipc::Instant::carried("2026-11-24T09:00:00.000Z")) })
    }

    async fn forget_librarian_ranking(&self, _caller: Option<crate::Caller>, forget: ipc::RankWithLibrarian) -> Result<ipc::RankedIssues, Refusal> {
        Ok(ipc::RankedIssues { project: forget.project, days: 30, fetched_at: None, might_pick_up: Vec::new(), might_be_interested: Vec::new(), by_librarian: None })
    }

    async fn seed_features(&self, _caller: Option<crate::Caller>, _seed: SeedFeatures) -> Result<FeaturesSeeded, Refusal> {
        Ok(FeaturesSeeded { created: Vec::new(), areas: 0, dropped: Vec::new() })
    }

    async fn change_feature(&self, _caller: Option<crate::Caller>, change: ChangeFeature) -> Result<FeatureChanged, Refusal> {
        Ok(FeatureChanged { feature: change.slug.unwrap_or_else(|| change.feature.clone()), merged_into: change.into, rule: None })
    }

    async fn change_area(&self, _caller: Option<crate::Caller>, change: ipc::ChangeArea) -> Result<ipc::AreaChanged, Refusal> {
        Ok(ipc::AreaChanged { project: change.project, area: change.area, rule: None })
    }

    async fn reject_in_catalog(&self, _caller: Option<crate::Caller>, reject: ipc::RejectInCatalog) -> Result<ipc::CatalogRuleKept, Refusal> {
        let rule = ipc::CatalogRule {
            id: String::from("r-1"),
            text: reject.why.unwrap_or_default(),
            act: ipc::CatalogRuleAct::NotTogether,
            features: reject.features,
            areas: reject.areas,
            paths: reject.paths,
            at: ipc::Instant::carried("2026-11-28T12:00:00Z"),
        };
        Ok(ipc::CatalogRuleKept { project: reject.project, rule })
    }

    async fn list_catalog_rules(&self, project: Option<String>) -> Result<ipc::CatalogRules, Refusal> {
        Ok(ipc::CatalogRules { project: project.unwrap_or_default(), rules: Vec::new() })
    }

    async fn drop_catalog_rule(&self, _caller: Option<crate::Caller>, drop: ipc::DropCatalogRule) -> Result<ipc::CatalogRules, Refusal> {
        Ok(ipc::CatalogRules { project: drop.project, rules: Vec::new() })
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
