//! Workspaces, lifecycles and rules as Bridge and the agent's tools read them, and where a Job's
//! tickets, pull requests, deploys and started Jobs stand. The design is "Workspaces, lifecycles
//! and rules".
//!
//! **Reads only.** Nothing here saves a piece or moves an instance: a state is read from the
//! world, and a piece is written by the change that edits it.
//!
//! **DTOs, never the model.** `core_model::Lifecycle`, `Rule` and `Workspace` hold their parts
//! behind constructors that refuse what no engine could use; what reaches the wire is chosen field
//! by field in Fleet. A closed set the model owns (a forge fact, an autonomy, a place) is carried as
//! the spelling the model writes with, so a variant added there is one more word here and the match
//! in Fleet's conversion fails to compile until it is handled.
//!
//! **A Workspace here is a set of repositories** (`WorkspaceOfRepositories`), and not the Manifest's
//! monorepo unit that `WorkspaceCommands` and `ScannedWorkspace` speak of.
//!
//! **A Job that holds nothing answers empty lists**, which is every Job that began before lifecycles
//! existed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Instant, ManifestId, SessionId};

fn is_false(value: &bool) -> bool {
    !*value
}

/// Where a lifecycle or a rule applies, as its file says it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PieceScope {
    /// Every repository, where the lifecycles Armada carries apply.
    Everywhere,
    /// One Workspace or one repository, by id.
    Within { id: String },
}

/// How a piece ranks against another of its id: the scope first, then the place, then whether it came
/// from the home of the Workspace it is scoped to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceRank {
    /// How narrowly its scope names the repository asked about: `everywhere`, `workspace` or `repository`.
    pub reach: String,
    /// The place it was kept: `armada`, `kit` or `repository`.
    pub source: String,
    /// It came from the home repository of the Workspace it is scoped to. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub home: bool,
}

/// Where a lifecycle or a rule was kept, and what became of it beside another of its id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceKept {
    pub rank: PieceRank,
    /// The file it was read from. Absent for what Armada carries, which is built and not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The rank of the one that runs instead of this. Absent on the one that runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overridden_by: Option<PieceRank>,
    /// The place whose file this one replaced because it says something else: `armada`, `kit` or
    /// `repository`. Absent where it replaced none, and where the two say the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hides: Option<String>,
}

/// A file set aside, and the sentence a person reads for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceLeftOut {
    /// The place it was kept: `armada`, `kit` or `repository`.
    pub source: String,
    pub path: String,
    pub said: String,
}

/// A set of repositories that build one product, as one repository sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOfRepositories {
    /// The id a lifecycle or a rule is scoped to it by.
    pub id: String,
    pub name: String,
    /// The repositories, in the order the file lists them.
    pub repositories: Vec<ManifestId>,
    /// The repository whose `.armada/workspaces/` holds the file, and which the others read it from.
    /// Absent for one that is Kit's, which has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<ManifestId>,
    /// The place it was kept: `kit` or `repository`.
    pub source: String,
    pub path: String,
    /// The place of the one that runs instead of this. Absent on the one that runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overridden_by: Option<String>,
    /// The place whose file this one replaced because it says something else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hides: Option<String>,
}

/// `list_repository_workspaces`: the Workspaces that hold one repository, the one that runs for each
/// id and each one it replaced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryWorkspaces {
    pub manifest_id: ManifestId,
    pub workspaces: Vec<WorkspaceOfRepositories>,
    /// Every Workspace file Fleet could not read: none of them can be placed in a repository.
    pub left_out: Vec<PieceLeftOut>,
}

/// How a state is read, by the source that knows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum StateReadFrom {
    /// The tracker's own words for it. None is allowed, and every value is then unmapped.
    Tracker { words: Vec<String> },
    /// What the forge shows of a pull request: `draft`, `ready`, `reviewers_on`, `in_review`,
    /// `approved`, `merged` or `closed`.
    Forge { fact: String },
    /// The state a watch's command printed: `waiting`, `running`, `passed` or `failed`.
    Watch { state: String },
    /// What Fleet knows without asking anyone: `started`, `step_entered:<step id>`,
    /// `pull_request_opened`, `landed` or `ended`.
    Fleet { fact: String },
}

/// One state of a lifecycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleStateDefinition {
    pub id: String,
    /// What a person reads for it.
    pub label: String,
    pub read: StateReadFrom,
}

/// A move a lifecycle expects. **Expected, not permitted**: a state is read and never set, so an
/// instance that went somewhere else is in the state it was read in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleTransition {
    pub from: String,
    pub to: String,
}

/// A lifecycle: the states one kind of thing passes through, the moves expected between them and
/// the states that end it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleDefinition {
    pub id: String,
    pub name: String,
    pub scope: PieceScope,
    pub states: Vec<LifecycleStateDefinition>,
    pub transitions: Vec<LifecycleTransition>,
    /// The states that end an instance. A Job stays open while one it holds is in none of them.
    pub ends: Vec<String>,
}

/// A lifecycle that applies to a repository, and where it was kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedLifecycle {
    pub lifecycle: LifecycleDefinition,
    pub kept: PieceKept,
}

/// `list_lifecycles`: what applies to one repository, the one that runs for each id and each one it
/// replaced, and the files set aside.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryLifecycles {
    pub manifest_id: ManifestId,
    pub lifecycles: Vec<AppliedLifecycle>,
    pub left_out: Vec<PieceLeftOut>,
}

/// Which instances a rule looks at or acts on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleAmong {
    /// The instance whose entering a state set the rule off.
    Entered,
    /// What the entered instance came from and the Job holds, of one lifecycle.
    LinkedFrom { lifecycle: String },
    /// What came from the entered instance and the Job holds, of one lifecycle.
    LinkingTo { lifecycle: String },
    /// Every instance of one lifecycle the Job holds.
    Held { lifecycle: String },
}

/// The instances an act is done to: those `among` selects, standing in `in_state` where one is given.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleTarget {
    pub among: RuleAmong,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_state: Option<String>,
}

/// The moment a rule is for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleWhen {
    /// An instance of `lifecycle` enters `state`.
    Enters { lifecycle: String, state: String },
    StepStarts { step: String },
    StepEnds { step: String },
    /// The Job does something as a whole: `started`, `pull_request_opened`, `landed` or `ended`.
    Job { event: String },
}

/// One thing a rule asks, and all of a rule's conditions must hold for it to do anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleCondition {
    /// Of the instances `among` selects, `quantity` (`every`, `any` or `no`) of them stand in `in_state`.
    Instances { among: RuleAmong, quantity: String, in_state: String },
    /// The Job follows this workflow, by the slug its file gives it.
    Workflow { workflow: String },
    /// The Job is on this step.
    AtStep { step: String },
    /// The Job has opened a pull request.
    PullRequestOpened,
}

/// What a rule does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "snake_case")]
pub enum RuleAct {
    TrackerAssign { to: String, target: RuleTarget },
    /// Move to a state of the ticket's lifecycle: the state, and not the tracker's word for it.
    TrackerMove { to: String, target: RuleTarget },
    TrackerClose { target: RuleTarget },
    ForgeRequestReviewers { from: Vec<String>, target: RuleTarget },
    ForgeMarkReady { target: RuleTarget },
    ForgeMerge { target: RuleTarget },
    ForgeAutoMerge { target: RuleTarget },
    /// A message from a template, left on `target`.
    Draft { template: String, target: RuleTarget },
    StartJob {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workflow: Option<String>,
        prompt: String,
    },
    StartWatch {
        recipe: String,
        values: BTreeMap<String, String>,
        /// The deploy the watch takes on for the Job, where it watches one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deploy: Option<String>,
    },
    Tell { message: String },
    Wake,
    RunCommand { command: String },
    RunSkill { skill: String },
    RunDrone { prompt: String },
}

/// One act of a rule, and how far it goes on its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleDoing {
    pub what: RuleAct,
    /// The kind of act an autonomy default is set for: `tracker_write`, `forge_write`, `draft`,
    /// `start_job`, `start_watch`, `tell_agent`, `run_command` or `run_agent`.
    pub kind: String,
    /// What the rule does with it: `acts`, `asks` or `off`. The rule's own word, else the kind's default.
    pub autonomy: String,
}

/// What a rule does when an act fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleOnFailure {
    /// How many times the act is tried again first. Capped.
    pub retries: u8,
    /// Whom a failure that retrying did not mend goes to: `tell_agent` or `ask_owner`.
    pub then: String,
}

/// A lifecycle rule: When, If and Do, with an autonomy and a way to fail. **Not `FleetRule`**, which is
/// a notice rule from `fleet.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRule {
    pub id: String,
    pub name: String,
    pub scope: PieceScope,
    pub when: RuleWhen,
    /// All of them must hold. None means the moment is enough.
    pub conditions: Vec<RuleCondition>,
    pub does: Vec<RuleDoing>,
    /// The rule's own word, `acts`, `asks` or `off`, over the default for each kind of act. Absent takes
    /// the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autonomy: Option<String>,
    pub on_failure: RuleOnFailure,
}

/// A rule that applies to a repository, and where it was kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedRule {
    pub rule: LifecycleRule,
    pub kept: PieceKept,
}

/// `list_lifecycle_rules`: what applies to one repository, the one that runs for each id and each one
/// it replaced, and the files set aside. **Armada carries none**, so with no file the list is empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryLifecycleRules {
    pub manifest_id: ManifestId,
    pub rules: Vec<AppliedRule>,
    pub left_out: Vec<PieceLeftOut>,
}

/// Where an instance stands, as a source's last reading left it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstanceStanding {
    /// Taken on and not yet read.
    Unread,
    In { state: String },
    /// A source said something no state claims, kept as it was said. Not an end.
    Unmapped { said: String },
}

/// An instance another is linked to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceLink {
    pub lifecycle: String,
    pub key: String,
}

/// One thing a Job holds: a ticket, a pull request, a deploy, or a Job it started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldInstance {
    pub lifecycle: String,
    /// What its source calls it: `ARM-41`, `812`, a Job's id.
    pub key: String,
    /// What a person reads for it: the ticket's key, `PR #812`, the Job's title.
    pub label: String,
    pub standing: InstanceStanding,
    /// What it came from, in the order the links were made. A link may name an instance the Job does
    /// not hold, as the Job that started this one.
    pub links: Vec<InstanceLink>,
}

/// One change of standing, kept when a read found the instance somewhere else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceMove {
    pub lifecycle: String,
    pub key: String,
    pub from: InstanceStanding,
    pub to: InstanceStanding,
    /// Whose word it was: `tracker`, `forge`, `watch` or `fleet`.
    pub source: String,
    pub at: Instant,
}

/// One act a Job's rules planned, and where it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleActReading {
    /// The id of the rule that planned it.
    pub rule: String,
    /// The one line the owner reads for it, as it was kept.
    pub text: String,
    /// How far it goes on its own: `acts` or `asks`.
    pub course: String,
    /// Where it stands: `planned`, `asked`, `stuck`, `done`, `skipped`, `told` or `not_carried`.
    pub standing: String,
    /// What failed, how it was settled or why nothing here carries it. Left out where there is
    /// nothing to say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
    /// When it was planned.
    pub at: Instant,
    /// When it was settled. Left out until it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<Instant>,
    /// The instance it is done to. Left out for an act done to none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<InstanceLink>,
    /// The instance whose entering a state set it off, where one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entered: Option<InstanceLink>,
}

/// `get_job_lifecycles`, and the `job_lifecycles.changed` event: what a Job holds and where it stands.
///
/// **Whole**, so a client replaces what it holds for the Job. `lifecycles` are those the Job's
/// instances are of, and `moves` are theirs, oldest first. `acts` are those its rules planned,
/// oldest first, and are left out where there are none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobLifecycles {
    pub session_id: SessionId,
    pub lifecycles: Vec<LifecycleDefinition>,
    pub instances: Vec<HeldInstance>,
    pub moves: Vec<InstanceMove>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<RuleActReading>,
    /// The rules the Job's copy holds, which are the ones that run for it. Left out for a Job that froze no
    /// copy, whose rules are those `list_lifecycle_rules` serves for its repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<AppliedRule>,
    /// What was changed in the Job's own copy, for the lanes to mark. Left out where nothing was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<crate::JobCopyChange>,
}
