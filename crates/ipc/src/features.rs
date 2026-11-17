//! A project's feature library over the wire. `docs/concepts/feature-catalog.md`.
//!
//! **Read by project, written by the Job that did the work.** Areas hold features. A feature has four written
//! sections, and the links Fleet detected: Jobs, pull requests, merges and the artifacts made for it.

use serde::{Deserialize, Serialize};

use crate::{ids::Instant, SessionId};

/// Whether a feature was named by seeding or confirmed since.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    Seeded,
    Confirmed,
}

/// The four sections a Job writes, in the order a page shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureSectionKind {
    /// How to test it.
    Test,
    /// How it runs.
    Runs,
    /// How to observe it.
    Observe,
    /// Tech debt.
    Debt,
}

impl FeatureSectionKind {
    pub const ALL: [FeatureSectionKind; 4] =
        [FeatureSectionKind::Test, FeatureSectionKind::Runs, FeatureSectionKind::Observe, FeatureSectionKind::Debt];

    /// The name it is kept and served under.
    pub const fn name(self) -> &'static str {
        match self {
            FeatureSectionKind::Test => "test",
            FeatureSectionKind::Runs => "runs",
            FeatureSectionKind::Observe => "observe",
            FeatureSectionKind::Debt => "debt",
        }
    }

    pub fn from_name(name: &str) -> Option<FeatureSectionKind> {
        Self::ALL.into_iter().find(|one| one.name() == name)
    }
}

/// What a link ties a feature to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureLinkKind {
    Job,
    PullRequest,
    Merge,
    Sketch,
    Page,
    Walk,
    Draft,
    Commit,
    Bug,
}

impl FeatureLinkKind {
    pub const ALL: [FeatureLinkKind; 9] = [
        FeatureLinkKind::Job,
        FeatureLinkKind::PullRequest,
        FeatureLinkKind::Merge,
        FeatureLinkKind::Sketch,
        FeatureLinkKind::Page,
        FeatureLinkKind::Walk,
        FeatureLinkKind::Draft,
        FeatureLinkKind::Commit,
        FeatureLinkKind::Bug,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            FeatureLinkKind::Job => "job",
            FeatureLinkKind::PullRequest => "pull_request",
            FeatureLinkKind::Merge => "merge",
            FeatureLinkKind::Sketch => "sketch",
            FeatureLinkKind::Page => "page",
            FeatureLinkKind::Walk => "walk",
            FeatureLinkKind::Draft => "draft",
            FeatureLinkKind::Commit => "commit",
            FeatureLinkKind::Bug => "bug",
        }
    }

    pub fn from_name(name: &str) -> Option<FeatureLinkKind> {
        Self::ALL.into_iter().find(|one| one.name() == name)
    }
}

/// A Job that is working on a feature now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureJob {
    pub session_id: SessionId,
    pub title: String,
}

/// One feature in a list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureRow {
    pub id: String,
    pub slug: String,
    /// The slug of the area it is in.
    pub area: String,
    pub title: String,
    /// One line.
    pub summary: String,
    pub state: FeatureState,
    /// The live Jobs that wrote a file of it since they started.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jobs_now: Vec<FeatureJob>,
    /// The oldest merge a written section is stale since: a pull request's number. Absent where none is stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_since: Option<u64>,
}

/// An area and its features, in their order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureAreaRow {
    pub slug: String,
    pub title: String,
    pub features: Vec<FeatureRow>,
}

/// `list_features`: a project's areas and features, with who is on each now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureLibrary {
    pub project: String,
    pub areas: Vec<FeatureAreaRow>,
}

/// A written section as it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSection {
    pub section: FeatureSectionKind,
    pub text: String,
    /// The Job that wrote it.
    pub job: String,
    pub written_at: Instant,
    /// The commit the Job's checkout stood at. Absent where it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The first merge after it that changed a file of the feature: a pull request's number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_since: Option<u64>,
}

/// One link of a feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureLink {
    pub kind: FeatureLinkKind,
    /// What it names: a Job's id, a pull request's number, an artifact's path, a commit.
    pub target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    /// The Job it came through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    /// How Fleet found it: `write`, `trailer`, `artifact` or `detector`.
    pub via: String,
    pub at: Instant,
}

/// A file Jobs wrote under a feature, and how many Jobs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFile {
    pub path: String,
    pub jobs: u32,
}

/// `get_feature_page`: one feature, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturePage {
    pub project: String,
    pub feature: FeatureRow,
    /// The repository the prefixes are in. Absent where any member of the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
    /// The path prefixes it owns.
    pub paths: Vec<String>,
    /// `path` or `path:line`, as seeded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry_points: Vec<String>,
    /// Only the sections that were written, in the order a page shows them.
    pub sections: Vec<FeatureSection>,
    /// Oldest first.
    pub links: Vec<FeatureLink>,
    /// The files the most Jobs wrote, most first.
    pub files: Vec<FeatureFile>,
}

/// `get_feature_brief`: the page condensed for an agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureBrief {
    pub feature: String,
    /// Plain text, about a thousand tokens at most.
    pub text: String,
    /// Characters over four.
    pub tokens: u32,
}

/// One section a Job writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSectionText {
    pub section: FeatureSectionKind,
    pub text: String,
}

/// `write_feature_sections`: this Job rewrites sections of one feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteFeatureSections {
    /// The feature's slug.
    pub feature: String,
    pub sections: Vec<FeatureSectionText>,
    /// The project's id. **Read only where the Job's repository is in more than one project and the slug is in
    /// each.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What a feature still owes this Job before it ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureOwed {
    pub feature: String,
    pub sections: Vec<FeatureSectionKind>,
}

/// What `write_feature_sections` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSectionsWritten {
    pub feature: String,
    pub written: Vec<FeatureSectionKind>,
    /// What this Job still owes after the write, across features. Empty where nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owed: Vec<FeatureOwed>,
}

/// An area to keep.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaSeed {
    pub slug: String,
    pub title: String,
}

/// A feature to keep.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSeed {
    pub slug: String,
    /// An area's slug, in this call or already kept.
    pub area: String,
    pub title: String,
    /// One line.
    pub summary: String,
    /// Where absent: `seeded` for a new one, and a feature already kept keeps its state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<FeatureState>,
    /// The path prefixes it owns, whole: a feature already kept has them replaced.
    #[serde(default)]
    pub paths: Vec<String>,
    /// `path` or `path:line`.
    #[serde(default)]
    pub entry_points: Vec<String>,
    /// The repository the prefixes are in. Absent for any member of the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
}

/// `upsert_features`: the Operator keeps areas and features, matched by slug.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpsertFeatures {
    pub project: String,
    #[serde(default)]
    pub areas: Vec<AreaSeed>,
    #[serde(default)]
    pub features: Vec<FeatureSeed>,
}

/// What `upsert_features` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturesUpserted {
    /// The slugs of the features that did not exist.
    pub created: Vec<String>,
    /// The slugs of the features that did and were rewritten.
    pub updated: Vec<String>,
    /// How many areas were kept.
    pub areas: u32,
}

/// `seed_features`: Fleet keeps a project's first areas and features from the seed it carries, on the Operator's word.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedFeatures {
    /// A project's id.
    pub project: String,
}

/// What `seed_features` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturesSeeded {
    /// The slugs of the features kept.
    pub created: Vec<String>,
    /// How many areas were kept.
    pub areas: u32,
    /// A prefix or entry point the project's repositories do not hold, left out, as `<slug>: <path>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<String>,
}

/// How a path came to belong to a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathHow {
    /// A path prefix the feature owns.
    Prefix,
    /// A Job filed the file under the feature: its commit named it, or it wrote the file while its commits did.
    Filed,
}

/// A feature a path belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureOfPath {
    pub project: String,
    pub slug: String,
    pub title: String,
    pub area: String,
    pub how: PathHow,
    /// The prefix that matched, where `how` is `prefix`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

/// `get_features_of_path`: the features a file belongs to, the longest prefix first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturesOfPath {
    pub path: String,
    pub features: Vec<FeatureOfPath>,
}

/// One week of Jobs, counted by the searches they ran before their first edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchWeek {
    /// The Monday the week starts, `YYYY-MM-DD`, UTC.
    pub starts: String,
    /// Jobs that edited or wrote a file that week and were measured.
    pub jobs: u32,
    /// Their code searches before that first edit, added up.
    pub searches: u32,
    /// `searches` over `jobs`, to one place. Zero where no Job.
    pub per_job: f64,
}

/// `get_first_edit_searches`: how many code searches Jobs ran before their first edit, a week at a time, newest
/// week last.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FirstEditSearches {
    pub weeks: Vec<SearchWeek>,
    /// Jobs in those weeks whose turns were read before searches were counted, so they are in no week.
    pub unmeasured: u32,
}
