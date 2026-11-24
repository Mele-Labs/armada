//! A project's feature library: its areas and features, the sections Jobs write and what Fleet saw. Its own surface, for
//! [`Projects`](super::Projects)' reason: a handler taking only this cannot reach for a Job.
//! `docs/concepts/feature-catalog.md`.
//!
//! **`caller` places the project when none is named**, as a hosted session is placed for `all_done`: the project the
//! Job was started for, else the projects that hold its repository. A call that places none and names none is
//! [`Refusal::Unacceptable`]. The library lives on this Mac.

use std::future::Future;

use crate::daemon::Refusal;

pub trait Features: Send + Sync + 'static {
    /// `list_features` — the project's areas and features in order, each with the live Jobs on it and the oldest merge
    /// a written section is stale since.
    fn list_features(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
    ) -> impl Future<Output = Result<ipc::FeatureLibrary, Refusal>> + Send;

    /// `get_feature_page` — one feature whole, or with `section` that one section alone, and with `files` the floor plan's
    /// rooms filled with their files. [`Refusal::NotFound`] (`fleet.feature_unknown`) for a slug the project does not hold,
    /// and [`Refusal::Unacceptable`] (`fleet.feature_section_unknown`) for a section that is none of the four.
    fn get_feature_page(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
        feature: String,
        section: Option<String>,
        files: bool,
    ) -> impl Future<Output = Result<ipc::FeaturePage, Refusal>> + Send;

    /// `get_glossary` — the project's terms, each with the words that should not be used for it. Placed as `list_features`
    /// is.
    fn get_glossary(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
    ) -> impl Future<Output = Result<ipc::Glossary, Refusal>> + Send;

    /// `get_project_overview` — the project's README sentence, code map, busiest features and glossary.
    fn get_project_overview(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
    ) -> impl Future<Output = Result<ipc::ProjectOverview, Refusal>> + Send;

    /// `get_since_you_looked` — the features that are new, changed or waiting on him since `since`. [`Refusal::Unacceptable`]
    /// (`fleet.feature_since_unreadable`) for a `since` that is no RFC 3339 instant.
    fn get_since_you_looked(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
        since: String,
    ) -> impl Future<Output = Result<ipc::SinceYouLooked, Refusal>> + Send;

    /// `get_whos_on_what` — who has an open pull request or a running Job on which feature of the project.
    fn get_whos_on_what(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
    ) -> impl Future<Output = Result<ipc::WhosOnWhat, Refusal>> + Send;

    /// `get_feature_brief` — the page condensed to about a thousand tokens. Refused as `get_feature_page` is.
    fn get_feature_brief(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
        feature: String,
    ) -> impl Future<Output = Result<ipc::FeatureBrief, Refusal>> + Send;

    /// `get_features_of_path` — the features a repository-relative path belongs to, the longest prefix first.
    fn get_features_of_path(
        &self,
        caller: Option<crate::Caller>,
        project: Option<String>,
        manifest_id: Option<String>,
        path: String,
    ) -> impl Future<Output = Result<ipc::FeaturesOfPath, Refusal>> + Send;

    /// `write_feature_sections` — the calling Job rewrites sections of a feature. [`Refusal::Unacceptable`] for a call
    /// that places no hosted session, an ended one, an unknown slug (`fleet.feature_unknown`) or a blank text
    /// (`fleet.feature_section_empty`).
    fn write_feature_sections(
        &self,
        caller: Option<crate::Caller>,
        write: ipc::WriteFeatureSections,
    ) -> impl Future<Output = Result<ipc::FeatureSectionsWritten, Refusal>> + Send;

    /// `upsert_features` — areas and features kept by slug, for the Operator's seeding and for the owner's confirming and
    /// splitting from Bridge. [`Refusal::Unacceptable`] for a call through the door from any session but the Operator's
    /// (`fleet.not_the_operator`), a feature in an area neither the call nor the project has (`fleet.feature_area_unknown`) or a
    /// blank part (`fleet.feature_blank`), with nothing kept.
    fn upsert_features(&self, caller: Option<crate::Caller>, upsert: ipc::UpsertFeatures) -> impl Future<Output = Result<ipc::FeaturesUpserted, Refusal>> + Send;

    /// `answer_retire` — the owner's Retire or Keep for a feature the librarian proposed to retire. Refused as `upsert_features` is
    /// to any session but the Operator's (`fleet.not_the_operator`); [`Refusal::NotFound`] for an unknown feature
    /// (`fleet.feature_unknown`), [`Refusal::IllegalMove`] for one not proposed (`fleet.retire_not_proposed`).
    fn answer_retire(&self, caller: Option<crate::Caller>, answer: ipc::AnswerRetire) -> impl Future<Output = Result<ipc::RetireAnswered, Refusal>> + Send;

    /// `check_docs` — each doc of a feature put to the model against the feature's code, on the owner's press of Check, spent
    /// from the librarian's night share. Refused as `answer_retire` is to any session but the Operator's; [`Refusal::NotFound`]
    /// for an unknown feature, [`Refusal::IllegalMove`] where the share is spent (`fleet.librarian_share_spent`),
    /// [`Refusal::Unacceptable`] where there is no repository or no doc to check, [`Refusal::Fault`] where no answer could be read.
    fn check_docs(&self, caller: Option<crate::Caller>, check: ipc::CheckDocs) -> impl Future<Output = Result<ipc::DocsChecked, Refusal>> + Send;

    /// `dismiss_doc_flag` — the owner's Dismiss on a doc a check found disagreeing with the code: quiet until either changes.
    /// [`Refusal::IllegalMove`] where no disagreement stands on it (`fleet.doc_not_flagged`).
    fn dismiss_doc_flag(&self, caller: Option<crate::Caller>, flag: ipc::DocFlag) -> impl Future<Output = Result<ipc::DocFlag, Refusal>> + Send;

    /// `start_doc_job` — the owner's Start a Job on a doc a check found disagreeing with the code: a Job in the project sent a
    /// brief that quotes both and asks him which is right. Refused as `dismiss_doc_flag` is.
    fn start_doc_job(self: std::sync::Arc<Self>, caller: Option<crate::Caller>, flag: ipc::DocFlag) -> impl Future<Output = Result<ipc::DocJobStarted, Refusal>> + Send;

    /// `seed_features` — the project's first areas and features from the seed Fleet carries, on the Operator's word.
    /// [`Refusal::Unacceptable`] for a caller that is not the Operator's session (`fleet.not_the_operator`) or a project none
    /// of whose repositories holds the seed's marker (`fleet.seed_not_for_project`), [`Refusal::IllegalMove`] for a project that
    /// holds features (`fleet.feature_seeded`).
    fn seed_features(&self, caller: Option<crate::Caller>, seed: ipc::SeedFeatures) -> impl Future<Output = Result<ipc::FeaturesSeeded, Refusal>> + Send;

    /// `run_librarian` — the project's librarian runs its enabled parts now, on a press of Catch up, and keeps a record of the run.
    /// [`Refusal::Unacceptable`] for a call through the door from any session but the Operator's (`fleet.not_the_operator`) or a
    /// project Fleet does not hold (`fleet.feature_project`).
    fn run_librarian(&self, caller: Option<crate::Caller>, run: ipc::RunLibrarian) -> impl Future<Output = Result<ipc::LibrarianRan, Refusal>> + Send;

    /// `stop_librarian` — the project's running librarian run stops before its next step, on a press of Stop. Answers whether
    /// one was running. Refused as `run_librarian` is.
    fn stop_librarian(&self, caller: Option<crate::Caller>, stop: ipc::StopLibrarian) -> impl Future<Output = Result<ipc::LibrarianStopping, Refusal>> + Send;

    /// `change_feature` — the Operator or the owner merges a feature into another, renames it or moves it to an area. [`Refusal::Unacceptable`]
    /// (`fleet.not_the_operator`, `fleet.feature_change`, `fleet.feature_slug`), [`Refusal::NotFound`] (`fleet.feature_unknown`),
    /// [`Refusal::IllegalMove`] (`fleet.feature_slug_taken`).
    fn change_feature(&self, caller: Option<crate::Caller>, change: ipc::ChangeFeature) -> impl Future<Output = Result<ipc::FeatureChanged, Refusal>> + Send;

    /// `hand_over_features` — which feature pages the Job, or its subagent, is owed for what it is about to read or be told,
    /// each once. The Job is `hand.session_id`: the mod's hook is the caller, not the Job's own connection.
    fn hand_over_features(&self, hand: ipc::HandOverFeatures) -> impl Future<Output = Result<ipc::FeaturesHandedOver, Refusal>> + Send;

    /// `check_commit_feature` — whether the Job's `git commit` names the feature it serves. **Only a Job started after its
    /// project's catalog was seeded is refused.** Allowed where the Job is unknown, the project has no feature, the commit
    /// gives no message in its line or names a feature, or there is nothing to commit.
    fn check_commit_feature(&self, check: ipc::CheckCommitFeature) -> impl Future<Output = Result<ipc::CommitFeatureChecked, Refusal>> + Send;

    /// `get_first_edit_searches` — the weekly figure the library is judged by, `weeks` back, 8 where absent.
    fn get_first_edit_searches(&self, weeks: Option<u32>) -> impl Future<Output = Result<ipc::FirstEditSearches, Refusal>> + Send;
}
