//! The library's operations, answered from Fleet's store.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Features, Refusal};
use ipc::{
    FeatureBrief, FeatureLibrary, FeatureOfPath, FeaturePage, FeatureSectionsWritten, FeaturesOfPath, FeaturesUpserted, FirstEditSearches, PathHow,
    UpsertFeatures, WriteFeatureSections,
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

    async fn get_feature_page(&self, caller: Option<api::Caller>, project: Option<String>, feature: String, section: Option<String>) -> Result<FeaturePage, Refusal> {
        self.feature_paged(caller, project, feature, section).await
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

    async fn upsert_features(&self, upsert: UpsertFeatures) -> Result<FeaturesUpserted, Refusal> {
        self.features_upserted(upsert).await
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
