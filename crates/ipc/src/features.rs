//! A project's feature library over the wire. `docs/concepts/feature-catalog.md`.
//!
//! **Read by project, written by the Job that did the work.** Areas hold features. A feature has four written
//! sections, and the links Fleet detected: Jobs, pull requests, merges and the artifacts made for it.

use serde::{Deserialize, Serialize};

use crate::{ids::Instant, GlossaryTermWrite, SessionId};

/// Whether a feature was named by seeding, confirmed since, or is waiting on his word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    Seeded,
    Confirmed,
    /// Named by a Job and not in the catalog: the Operator brings it to the owner on a card.
    Pending,
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
    /// A Job used a word the project's glossary says not to. The target is the word and the title the term to use.
    GlossaryFlag,
}

impl FeatureLinkKind {
    pub const ALL: [FeatureLinkKind; 10] = [
        FeatureLinkKind::Job,
        FeatureLinkKind::PullRequest,
        FeatureLinkKind::Merge,
        FeatureLinkKind::Sketch,
        FeatureLinkKind::Page,
        FeatureLinkKind::Walk,
        FeatureLinkKind::Draft,
        FeatureLinkKind::Commit,
        FeatureLinkKind::Bug,
        FeatureLinkKind::GlossaryFlag,
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
            FeatureLinkKind::GlossaryFlag => "glossary_flag",
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

/// Why the librarian proposes retiring a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetireWhy {
    /// Its paths match no file now, and matched some once.
    CodeGone,
    /// It has files, and nothing has touched them, and no Job has been on it, for months.
    Unused,
}

/// A proposal to retire a feature, with what the librarian saw. Only the owner retires: `answer_retire`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetireProposal {
    pub why: RetireWhy,
    /// The fact it rests on, one line: `Its paths match no file since abc1234, 14 May`.
    pub evidence: String,
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
    /// How it stands and why, read from what Fleet keeps.
    pub health: crate::feature_health::FeatureHealth,
    /// The librarian proposes retiring it. Absent where it has none, or where the owner kept it on this evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retire: Option<RetireProposal>,
    /// Its paths match no file at `HEAD` and never did: the code is not written yet. It is a normal feature from its first file.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub planned: bool,
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
    /// Where the project's librarian stands: when it last ran and what has merged since.
    #[serde(default)]
    pub librarian: crate::LibrarianState,
}

/// `get_features_by_meaning`: the slugs of a project's features that what was typed is about, the nearest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturesByMeaning {
    pub project: String,
    /// The features it is about, by slug, nearest first. Empty where nothing is near enough or Fleet's language model is not loaded.
    pub slugs: Vec<String>,
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

/// One room of a floor plan: a module of the project's code, and whether the feature owns files in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloorRoom {
    /// The package's name where its manifest gives one, otherwise its folder.
    pub module: String,
    /// Where its folder is in its repository, `.` for the repository's own root.
    pub path: String,
    /// Which manifest named it: `cargo`, `npm`, `go`, `python`, `gradle`, `maven`, `dotnet`, `swift`, or `folder` where the tree has none.
    pub kind: String,
    /// The feature owns a file in it, by prefix or by a Job's filing.
    pub lit: bool,
    /// The repository, where the feature is not tied to one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
    /// The files tracked in it at the checkout's `HEAD`, largest first, on a read that asks with `files=true`; empty on any
    /// other. Lock files and binary assets are left out.
    #[serde(default)]
    pub files: Vec<FloorFile>,
}

/// One file of a room, or the small files of one of its folders taken together. **A plan carries about 1500**: every
/// file the feature owns or a Job is changing stands alone, then the largest, and the rest are taken together by folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloorFile {
    /// Repository-relative. A folder, ending in `/`, where `grouped` is more than 0.
    pub path: String,
    /// Its size as git keeps it, or the sum of the files taken together.
    pub bytes: u64,
    /// The feature owns it, by the rule that lights a room.
    pub lit: bool,
    /// A live Job has written it.
    pub changing: bool,
    /// How many files this entry stands for; 0 for one file.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub grouped: u32,
}

fn is_zero(count: &u32) -> bool {
    *count == 0
}

fn none_kept(count: &usize) -> bool {
    *count == 0
}

/// One door of a floor plan: a lit module depends on another module of its repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloorDoor {
    pub from: String,
    pub to: String,
    /// The module it leads to carries a wire or API contract.
    pub contract: bool,
}

/// The project's code map with one feature's rooms lit.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloorPlan {
    pub rooms: Vec<FloorRoom>,
    pub doors: Vec<FloorDoor>,
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
    /// What the owner settled in the Jobs that shaped it, newest first. With `section`, none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<FeatureDecision>,
    /// The files the most Jobs wrote, most first.
    pub files: Vec<FeatureFile>,
    /// The documents a Job read before it changed a file of the feature, newest first, each once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<FeatureRead>,
    /// The tickets a Job read before it changed a file of the feature, newest first, each once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tickets: Vec<FeatureRead>,
    /// The code map with this feature's rooms lit. Absent on a page read by `section`, and where no repository could be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_plan: Option<FloorPlan>,
}

/// One thing the owner settled in a Job: his answer to a question, his pick on takes, his Approve or Ask for changes on a
/// sketch, a draft, a page or a plan, or his Looks good on a Ready to try card. Fleet files it; no Job writes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureDecision {
    /// The question he was asked, or the title of what he answered.
    pub question: String,
    /// The option's label, or `Approved`, `Asked for changes`, `Looks good`.
    pub choice: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The Job he settled it in.
    pub job: String,
    pub at: Instant,
}

/// A document or ticket a Job read before it changed the feature's files. Fleet saw the order; it does not know the Job
/// quoted or acted on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureRead {
    /// The address, path or id it was read by.
    pub target: String,
    /// The title the result held, else what it was read by.
    pub title: String,
    /// The result's first sentence. Empty where the result was not visible.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gist: String,
    /// Whether the Job that read it wrote `gist` at its end, in place of the result's first sentence.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gist_by_job: bool,
    /// The Job that read it.
    pub job: String,
    pub at: Instant,
    /// `gone` where the address no longer answers. Empty until a check sets it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    /// The names the document gives that the code had when it was last changed and no longer has. Empty for a ticket, an
    /// address, and a document the librarian's drift part has not found any in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drifted: Vec<String>,
    /// What the model found when he last pressed Check on the feature's docs. Absent for a ticket, and for a doc no press
    /// has checked against this feature's code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<crate::DocCheck>,
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
    /// May be empty where `glossary` is not.
    #[serde(default)]
    pub sections: Vec<FeatureSectionText>,
    /// Terms this Job introduced, kept in the project's glossary. A term the project holds is replaced, whatever its case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glossary: Vec<GlossaryTermWrite>,
    /// What this Job says of a doc or ticket it read, one line each. Replaces the first sentence kept for it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gists: Vec<DocGist>,
    /// The project's id. **Read only where the Job's repository is in more than one project and the slug is in
    /// each.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// One doc or ticket's gist, in the Job's own words: what it says that bears on the features the Job changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocGist {
    /// The address, path or id the Job read it by, as the owed answer names it.
    pub target: String,
    pub gist: String,
}

/// What a feature still owes this Job before it ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureOwed {
    pub feature: String,
    pub sections: Vec<FeatureSectionKind>,
    /// The docs and tickets this Job read that are linked under the feature and have no gist of its own, at most eight
    /// across features. A gist is welcome and never required.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<String>,
}

/// What `write_feature_sections` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSectionsWritten {
    pub feature: String,
    pub written: Vec<FeatureSectionKind>,
    /// The terms kept, as written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glossary: Vec<String>,
    /// How many gists were kept.
    #[serde(default, skip_serializing_if = "none_kept")]
    pub gists: usize,
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

/// `answer_retire`: the owner's answer to a proposal to retire a feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerRetire {
    /// A project's id.
    pub project: String,
    pub slug: String,
    /// True retires the feature. False keeps it, and the proposal stays quiet until its evidence changes.
    pub retire: bool,
}

/// What `answer_retire` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetireAnswered {
    pub project: String,
    pub slug: String,
    /// The feature left the catalog. False where the owner kept it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub retired: bool,
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

/// `change_feature`: the Operator merges a feature into another, or renames it, or moves it to another area. Give `into`, or any of
/// `slug`, `title` and `area`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeFeature {
    pub project: String,
    /// The slug of the feature to change, or one it was known by.
    pub feature: String,
    /// Fold it into the feature of this slug: what it holds moves there and its slug becomes an alias of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<String>,
    /// Its new slug. The old one becomes an alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The slug of an area the project keeps: it moves there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

/// What `change_feature` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureChanged {
    /// The slug the feature has now: the target's after a merge.
    pub feature: String,
    /// Where it was merged, where it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged_into: Option<String>,
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

/// `hand_over_features`: the mod's hook asks which feature pages a Job is owed for what it is about to read, search or
/// be told. **Fleet answers once per feature per holder**, and keeps the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandOverFeatures {
    /// The Job.
    pub session_id: SessionId,
    /// The subagent the read is the turn of, or the spawn call's id where the subagent is being told its brief. Absent for
    /// the Job's own thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// The file or folder a Read, Grep or Glob is about to open, as the call spelled it. Absolute paths under the Job's
    /// checkout are read as relative to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// A brief or a prompt: the features it names are handed over. A feature is named by a `Feature: <slug>` line, as a
    /// commit names one, or by its slug between backticks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
}

/// What `hand_over_features` hands over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturesHandedOver {
    /// The slugs handed over now, in the order of their pages. Empty where every feature asked about was handed over before.
    pub features: Vec<String>,
    /// Their condensed pages as plain text for the agent, empty where none.
    pub text: String,
}

/// `check_commit_feature`: the mod's hook asks, before an agent's `git commit` runs, whether the commit names the
/// feature it serves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckCommitFeature {
    /// The Job.
    pub session_id: SessionId,
    /// The shell line, whole, with the message in it.
    pub command: String,
    /// The files the commit is about to take, relative to the repository: what is staged, and what a `git add` or `-a` in
    /// the same line would stage. Empty where the hook could not say.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// What `check_commit_feature` says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitFeatureChecked {
    /// Whether the commit may run.
    pub allowed: bool,
    /// What the agent is told where it may not: the features its files belong to, each as the line to copy. Empty where allowed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub says: String,
    /// The slugs of the features the files belong to, the most files first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// The key an artifact's `detail` keeps the features it was made for under: slugs, comma separated. `detail` holds strings only.
pub const ARTIFACT_FEATURE: &str = "feature";

/// The feature, or features, a sketch, page, walk or draft is made for: one slug, or a list. **Both are read**, so an agent
/// may write `"feature": "voting-on-takes"` or `"feature": ["voting-on-takes", "page-actions"]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureNames {
    One(String),
    Many(Vec<String>),
}

impl FeatureNames {
    /// The slugs, trimmed and in lower case, none blank and each once, in the order written.
    pub fn slugs(&self) -> Vec<String> {
        let all: Vec<&str> = match self {
            FeatureNames::One(slug) => vec![slug.as_str()],
            FeatureNames::Many(slugs) => slugs.iter().map(String::as_str).collect(),
        };
        let mut kept: Vec<String> = Vec::new();
        for slug in all.into_iter().flat_map(|one| one.split(',')).map(|one| one.trim().to_lowercase()) {
            if !slug.is_empty() && !kept.contains(&slug) {
                kept.push(slug);
            }
        }
        kept
    }

    /// The slugs as an artifact's `detail` keeps them, comma separated. `None` where the call named none.
    pub fn as_detail(&self) -> Option<String> {
        Some(self.slugs().join(",")).filter(|slugs| !slugs.is_empty())
    }
}

/// Keep what an act named in the `detail` of the artifact it made. Nothing is kept where the act named none.
pub fn feature_in_detail(detail: &mut std::collections::BTreeMap<String, String>, named: Option<&FeatureNames>) {
    if let Some(slugs) = named.and_then(FeatureNames::as_detail) {
        detail.insert(ARTIFACT_FEATURE.to_string(), slugs);
    }
}
