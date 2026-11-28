//! The library's operations, answered from Fleet's store.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Features, Refusal};
use ipc::{
    ChangeFeature, FeatureBrief, FeatureChanged, FeatureLibrary, FeatureOfPath, FeaturePage, FeatureSectionsWritten, FeaturesOfPath, FeaturesSeeded, FeaturesUpserted, FirstEditSearches,
    PathHow, SeedFeatures, UpsertFeatures, WriteFeatureSections,
};

use super::mapping::How;
use super::reading::Loaded;
use crate::daemon::Fleet;

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
    /// `get_features_of_path`.
    async fn features_of_path_read(
        &self,
        caller: Option<api::Caller>,
        project: Option<String>,
        manifest_id: Option<String>,
        path: String,
    ) -> Result<FeaturesOfPath, Refusal> {
        self.feature_writes_filed().await;
        let path = path.trim().trim_start_matches("./").to_string();
        let store = self.store().lock().await;
        let mut features = Vec::new();
        for one in self.projects_asked(&store, caller, project.as_deref())? {
            let loaded = Loaded::read(&store, &one).map_err(|why| self.ledger_fault(why))?;
            let on = self.jobs_on_features(&store, &loaded.project)?;
            for owner in loaded.map.owners(manifest_id.as_deref(), &path) {
                let area = loaded.areas.iter().find(|area| area.slug == owner.feature.area).map(|area| area.title.clone()).unwrap_or_else(|| owner.feature.area.clone());
                features.push(FeatureOfPath {
                    project: loaded.project.clone(),
                    slug: owner.feature.slug.clone(),
                    title: owner.feature.title.clone(),
                    area,
                    how: match owner.how {
                        How::Prefix => PathHow::Prefix,
                        How::Filed => PathHow::Filed,
                    },
                    prefix: owner.prefix.map(str::to_string),
                    jobs: on.get(&owner.feature.id).cloned().unwrap_or_default(),
                });
            }
        }
        Ok(FeaturesOfPath { path, features })
    }
}

impl<H, V, W> Features for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_features(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<FeatureLibrary, Refusal> {
        self.features_listed(caller, project).await
    }

    async fn get_features_by_meaning(&self, caller: Option<api::Caller>, project: Option<String>, words: String) -> Result<ipc::FeaturesByMeaning, Refusal> {
        self.features_by_meaning(caller, project, words).await
    }

    async fn get_feature_page(&self, caller: Option<api::Caller>, project: Option<String>, feature: String, section: Option<String>, files: bool) -> Result<FeaturePage, Refusal> {
        self.feature_paged(caller, project, feature, section, files).await
    }

    async fn get_glossary(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<ipc::Glossary, Refusal> {
        self.glossary_read(caller, project).await
    }

    async fn get_project_overview(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<ipc::ProjectOverview, Refusal> {
        self.project_overview_read(caller, project).await
    }

    async fn get_since_you_looked(&self, caller: Option<api::Caller>, project: Option<String>, since: String) -> Result<ipc::SinceYouLooked, Refusal> {
        self.since_looked_read(caller, project, since).await
    }

    async fn get_whos_on_what(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<ipc::WhosOnWhat, Refusal> {
        self.whos_on_what_read(caller, project).await
    }

    async fn get_feature_brief(&self, caller: Option<api::Caller>, project: Option<String>, feature: String) -> Result<FeatureBrief, Refusal> {
        self.feature_briefed(caller, project, feature).await
    }

    async fn get_features_of_path(
        &self,
        caller: Option<api::Caller>,
        project: Option<String>,
        manifest_id: Option<String>,
        path: String,
    ) -> Result<FeaturesOfPath, Refusal> {
        self.features_of_path_read(caller, project, manifest_id, path).await
    }

    async fn write_feature_sections(&self, caller: Option<api::Caller>, write: WriteFeatureSections) -> Result<FeatureSectionsWritten, Refusal> {
        self.feature_sections_written(caller, write).await
    }

    async fn upsert_features(&self, caller: Option<api::Caller>, upsert: UpsertFeatures) -> Result<FeaturesUpserted, Refusal> {
        self.owner_or_operator_called(caller)?;
        self.features_upserted(upsert).await
    }

    async fn answer_retire(&self, caller: Option<api::Caller>, answer: ipc::AnswerRetire) -> Result<ipc::RetireAnswered, Refusal> {
        self.retire_answered(caller, answer).await
    }

    async fn check_docs(&self, caller: Option<api::Caller>, check: ipc::CheckDocs) -> Result<ipc::DocsChecked, Refusal> {
        self.docs_checked(caller, check).await
    }

    async fn dismiss_doc_flag(&self, caller: Option<api::Caller>, flag: ipc::DocFlag) -> Result<ipc::DocFlag, Refusal> {
        self.doc_flag_dismissed(caller, flag).await
    }

    async fn start_doc_job(self: std::sync::Arc<Self>, caller: Option<api::Caller>, flag: ipc::DocFlag) -> Result<ipc::DocJobStarted, Refusal> {
        self.doc_job_started(caller, flag).await
    }

    async fn get_library_start(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<ipc::LibraryStart, Refusal> {
        self.library_start_read(caller, project).await
    }

    async fn draft_module_features(&self, caller: Option<api::Caller>, draft: ipc::DraftModules) -> Result<ipc::ModulesDrafted, Refusal> {
        self.modules_drafted(caller, draft).await
    }

    async fn start_sub_project(self: std::sync::Arc<Self>, start: ipc::StartSubProject) -> Result<ipc::SubProject, Refusal> {
        self.sub_project_started(start).await
    }

    async fn list_sub_projects(self: std::sync::Arc<Self>, project: Option<String>, ended: bool) -> Result<ipc::SubProjects, Refusal> {
        self.sub_projects_listed(project, ended).await
    }

    async fn turn_into_orchestrator(self: std::sync::Arc<Self>, turn: ipc::TurnIntoOrchestrator) -> Result<ipc::SubProject, Refusal> {
        self.orchestrator_turned(turn).await
    }

    async fn ungroup_sub_project(&self, ungroup: ipc::UngroupSubProject) -> Result<(), Refusal> {
        self.sub_project_ungrouped(ungroup).await
    }

    async fn seed_features(&self, caller: Option<api::Caller>, seed: SeedFeatures) -> Result<FeaturesSeeded, Refusal> {
        self.features_seeded(caller, seed).await
    }

    async fn run_librarian(&self, caller: Option<api::Caller>, run: ipc::RunLibrarian) -> Result<ipc::LibrarianRan, Refusal> {
        self.librarian_pressed(caller, run).await
    }

    async fn stop_librarian(&self, caller: Option<api::Caller>, stop: ipc::StopLibrarian) -> Result<ipc::LibrarianStopping, Refusal> {
        self.librarian_stop_pressed(caller, stop)
    }

    async fn ask_librarian(&self, caller: Option<api::Caller>, ask: ipc::AskLibrarian) -> Result<ipc::LibrarianAnswered, Refusal> {
        self.librarian_asked(caller, ask).await
    }

    async fn undo_librarian_ask(&self, caller: Option<api::Caller>, undo: ipc::UndoLibrarianAsk) -> Result<ipc::LibrarianAskUndone, Refusal> {
        self.librarian_ask_undone(caller, undo).await
    }

    async fn refile_librarian_ask(&self, caller: Option<api::Caller>, refile: ipc::RefileLibrarianAsk) -> Result<ipc::LibrarianAnswered, Refusal> {
        self.librarian_ask_refiled(caller, refile).await
    }

    async fn rank_with_librarian(&self, caller: Option<api::Caller>, rank: ipc::RankWithLibrarian) -> Result<ipc::RankedIssues, Refusal> {
        self.librarian_rank_pressed(caller, rank).await
    }

    async fn forget_librarian_ranking(&self, caller: Option<api::Caller>, forget: ipc::RankWithLibrarian) -> Result<ipc::RankedIssues, Refusal> {
        self.librarian_ranking_forgotten(caller, forget).await
    }

    async fn change_feature(&self, caller: Option<api::Caller>, change: ChangeFeature) -> Result<FeatureChanged, Refusal> {
        self.feature_changed(caller, change).await
    }

    async fn hand_over_features(&self, hand: ipc::HandOverFeatures) -> Result<ipc::FeaturesHandedOver, Refusal> {
        self.features_handed_over(hand).await
    }

    async fn check_commit_feature(&self, check: ipc::CheckCommitFeature) -> Result<ipc::CommitFeatureChecked, Refusal> {
        self.commit_feature_checked(check).await
    }

    async fn get_first_edit_searches(&self, weeks: Option<u32>) -> Result<FirstEditSearches, Refusal> {
        self.first_edit_searches_read(weeks).await
    }
}
