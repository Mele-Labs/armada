//! Triggers on the wire: the ones a repository runs, saving one, and what a Job
//! did with each. `docs/concepts/trigger.md`. **Since 23.58.**
//!
//! **DTOs, never `core_model::Trigger`.** The file's path and its text are what
//! Bridge needs to draw a struck-through copy and to edit one, and neither is a
//! thing the domain type holds. `fleet::wire` is where the conversion is.
//!
//! **A definition is YAML text, whole**, as `WorkflowDefinition`'s is JSON text:
//! what an editor changes and sends to `save_trigger` is the same string, and
//! Fleet checks it with the loader's own rules.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId, StepId, WorkflowId};

/// The moment in a Job a Trigger runs at. `pr_opened` is the delivering step's
/// entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMoment {
    StepStarts,
    StepPasses,
    PrOpened,
}

/// Where a Trigger was read from, least specific first. A machine's replaces a
/// repository's of the same identity, whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerLevel {
    Armada,
    Repository,
    Machine,
}

/// Where a saved Trigger goes: the two places a person writes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerScope {
    /// `<repository>/.armada/triggers/`. **Fleet reads these from the base
    /// branch**, so a saved one runs once it is on `main`.
    Repository,
    /// `~/.armada/machine/triggers/`: every repository on this machine, and
    /// not Kit's, which travels.
    Machine,
}

/// What a Trigger runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerRuns {
    /// A Command the repository's `armada.yml` declares.
    Command { name: String },
    /// A skill a side Drone runs, on a branch of its own.
    Skill { name: String },
    /// A Drone sent with this prompt, on a branch of its own. 
    Drone { brief: String },
}

/// Why a Trigger is not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSkipReason {
    /// It names a Command this repository does not declare.
    NotInThisRepo,
    /// **No longer produced** (23.73): a skill runs on a side Drone. Kept so
    /// an older row still reads.
    SkillNotRun,
    /// It failed and held the Job, and the owner skipped it. Since 23.68.
    ByOwner,
}

/// A Trigger that will not run, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerSkip {
    pub reason: TriggerSkipReason,
    /// The Command or skill the Trigger names.
    pub name: String,
    /// The sentence Fleet writes in the Job's log, rendered and never matched on.
    pub said: String,
}

/// One Trigger in force in a repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerSummary {
    /// Part of its identity, which is `when`, `step` and `name`.
    pub name: String,
    pub when: TriggerMoment,
    /// Absent is every workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowId>,
    /// Absent is every step, and always absent on `pr_opened`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    pub runs: TriggerRuns,
    /// The Job waits on a failure: it is held until the owner reruns the
    /// Command or skips it, or a repair fixes it.
    #[serde(default)]
    pub block: bool,
    /// A repair Drone is sent on a failure.
    #[serde(default)]
    pub repair: bool,
    /// The place this copy was read from.
    pub level: TriggerLevel,
    /// The file as Fleet read it: a path, relative to the repository for a
    /// repository's own.
    pub file: String,
    /// Where it does not run in this repository. Absent where it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<TriggerSkip>,
    /// **The copies of this identity a more specific level replaced**, one per
    /// level, so a list can draw each struck through beside the one that runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<OverriddenTrigger>,
}

/// A copy a more specific level replaced. It does not run; `get_trigger` can
/// still read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverriddenTrigger {
    pub level: TriggerLevel,
    pub file: String,
}

/// A file Fleet runs without, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOutTrigger {
    pub level: TriggerLevel,
    pub file: String,
    /// The whole sentence, rendered and never matched on.
    pub said: String,
}

/// `list_triggers`' answer: what a repository runs and what it left out.
///
/// **One object, where `list_workflows` and `list_left_out_workflows` are two
/// routes.** That split was forced by a list that could not grow a sibling
/// without a major bump; this one starts as an object.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerList {
    pub triggers: Vec<TriggerSummary>,
    pub left_out: Vec<LeftOutTrigger>,
}

/// One definition as `get_trigger` answers it: the file's text, in the shape
/// `save_trigger` takes back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerDefinition {
    pub name: String,
    pub when: TriggerMoment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    pub level: TriggerLevel,
    pub file: String,
    /// The file whole, as YAML text and not a tree.
    pub definition: String,
    /// The level whose copy runs instead, where this one is replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overridden_by: Option<TriggerLevel>,
}

/// `save_trigger`'s body: one definition, and where it goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveTrigger {
    pub scope: TriggerScope,
    /// The definition, whole, as YAML text. Its `when`, `step` and `name` say
    /// which file in the scope it is.
    pub definition: String,
    /// **Required to replace a copy already in the scope.** Absent is `false`.
    #[serde(default)]
    pub overwrite: bool,
    /// **Keep a Job's added step for every Job**: the addition this save came
    /// from. Fleet writes the Trigger the definition says and records on the
    /// addition where it was kept. Since 23.68; a Drone step too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept_from: Option<crate::added_steps::KeptFrom>,
}

/// What `save_trigger` wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerSaved {
    pub name: String,
    pub when: TriggerMoment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    pub scope: TriggerScope,
    /// The file as written, absolute.
    pub file: String,
    /// Whether a copy was there and is now replaced.
    pub replaced: bool,
    /// Whose copy of this identity runs in this repository now. Absent where
    /// none does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs_from: Option<TriggerLevel>,
    /// **A repository's file is written in the checkout and read from the base
    /// branch**, so it waits for `main`. True until the base holds this text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub waits_for_main: bool,
    /// Where it does not run in this repository, which a machine save can
    /// reach and a repository save is refused for. Absent where it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<TriggerSkip>,
}

/// `remove_trigger`'s body: the identity, and the scope to remove it from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveTrigger {
    pub scope: TriggerScope,
    pub when: TriggerMoment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    pub name: String,
}

/// What `remove_trigger` deleted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerRemoved {
    pub scope: TriggerScope,
    /// The file that was deleted, absolute.
    pub file: String,
    /// Whose copy of this identity runs now, where another level holds one. A
    /// repository's removed from the checkout still runs from the base branch
    /// until the removal is on `main`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs_from: Option<TriggerLevel>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub waits_for_main: bool,
}

/// Where one of a Job's Triggers stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerFiringState {
    /// Frozen onto the Job and its moment has not come.
    Pending,
    Skipped,
    Running,
    Passed,
    Failed,
    /// A destructive Command, held for the owner. **Nothing asks him yet.**
    AwaitingOwner,
    /// Failed with `repair` on, and a repair Drone is working on a branch of
    /// its own. Since 23.68.
    Repairing,
    /// The Command is running again, on the repair branch or on the Job's.
    /// Since 23.68.
    Rerunning,
    /// The repair branch passes. **Held for the owner's choice**, which
    /// `choose_trigger_fix` makes. Since 23.68.
    FixReady,
    /// Failed with `block` on, and **the Job waits on it**: `rerun_trigger` or
    /// `skip_trigger` lets it go. Since 23.68.
    Held,
}

/// Where the owner has a repair's held fix go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerFixChoice {
    /// Merged onto the Job's branch and pushed, so it lands on the Job's open
    /// pull request.
    ThisBranch,
    /// The repair branch opens a pull request of its own.
    NewPr,
}

/// A pull request a repair opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerPullRequest {
    pub url: String,
    /// Read off the end of `url`. Absent where it does not end in one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
}

/// What a failed Trigger's repair has come to. Present from the first repair
/// Drone on. **Since 23.68.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerRepair {
    /// Which try this is, 1 or 2. A client draws no count from it: it is how a
    /// second Drone is told from the first.
    pub attempt: u32,
    /// The branch the repair Drone writes on, cut from the Job's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The files the fix changes, repository-relative. Empty until the fix is
    /// held.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// What the owner chose, once he has. A `this_branch` that had to wait for
    /// the Job's Drone shows here with `state` still `fix_ready`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<TriggerFixChoice>,
    /// For `new_pr`, once it is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<TriggerPullRequest>,
}

/// `choose_trigger_fix`'s body: which of a Job's Triggers or added steps holds
/// a fix, and where it goes. **Exactly one** of `trigger` and `addition` is
/// set, as `HoldAct`'s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChooseTriggerFix {
    /// The Trigger's name. The latest firing of it that holds a fix is the one.
    /// Always sent before 23.72, and still the only name a Bridge before it
    /// knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    /// An added step's id, where it is an added step whose fix waits. Since
    /// 23.72.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addition: Option<String>,
    pub choice: TriggerFixChoice,
}

/// What `choose_trigger_fix` came to. **`state` is where the firing stands
/// now**: `passed` for a placed fix, `failed` where the Command still fails on
/// the Job's branch, and `fix_ready` where the choice is kept until the Job's
/// Drone is done or a worktree slot is free.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerFixChosen {
    pub state: TriggerFiringState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<TriggerPullRequest>,
}

/// One of a Job's Triggers: frozen at approval, and each firing of it. A step
/// run again fires again, so one frozen Trigger can be several of these.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTrigger {
    pub name: String,
    pub when: TriggerMoment,
    /// The step it fires on. For `pr_opened`, the delivering one.
    pub step: StepId,
    pub level: TriggerLevel,
    pub state: TriggerFiringState,
    /// Why, where `state` is `skipped`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<TriggerSkip>,
    /// Absent for a signal, a timeout and a program that never started, and
    /// before it ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Absent while `pending`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// **The Job's log line for this firing**: the note in `get_job_log` whose
    /// `at` is this and whose `trigger` field is `name`. Its `stdout` and
    /// `stderr` fields are what the Command printed. Absent until the line is
    /// written, which is when the firing ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_at: Option<Instant>,
    /// Absent until a repair Drone has been put on it, and where `repair` is
    /// off. Since 23.68.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<TriggerRepair>,
    /// The Trigger blocks, so a failure holds the Job. Left out where it does
    /// not. It is what tells `repairing` on a Trigger that holds the Job from
    /// one that does not. Since 23.68.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub blocks: bool,
    /// A side Drone runs it, because it names a skill, so it has a branch of
    /// its own while it works and a fix to place when it commits. Left out for
    /// a Command. Since 23.73.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub drone: bool,
}

/// `job.trigger_changed`: one of a Job's Triggers moved, carried whole so a
/// client replaces its row without a read. **One message per state change**, and
/// none where nobody is watching.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTriggerChanged {
    pub job_id: JobId,
    pub trigger: JobTrigger,
    pub at: Instant,
}

impl From<core_model::TriggerWhen> for TriggerMoment {
    fn from(when: core_model::TriggerWhen) -> TriggerMoment {
        match when {
            core_model::TriggerWhen::StepStarts => TriggerMoment::StepStarts,
            core_model::TriggerWhen::StepPasses => TriggerMoment::StepPasses,
            core_model::TriggerWhen::PrOpened => TriggerMoment::PrOpened,
        }
    }
}

impl From<TriggerMoment> for core_model::TriggerWhen {
    fn from(moment: TriggerMoment) -> core_model::TriggerWhen {
        match moment {
            TriggerMoment::StepStarts => core_model::TriggerWhen::StepStarts,
            TriggerMoment::StepPasses => core_model::TriggerWhen::StepPasses,
            TriggerMoment::PrOpened => core_model::TriggerWhen::PrOpened,
        }
    }
}

impl From<core_model::TriggerSource> for TriggerLevel {
    fn from(source: core_model::TriggerSource) -> TriggerLevel {
        match source {
            core_model::TriggerSource::Armada => TriggerLevel::Armada,
            core_model::TriggerSource::Repository => TriggerLevel::Repository,
            core_model::TriggerSource::Machine => TriggerLevel::Machine,
        }
    }
}

impl From<core_model::FixChoice> for TriggerFixChoice {
    fn from(choice: core_model::FixChoice) -> TriggerFixChoice {
        match choice {
            core_model::FixChoice::ThisBranch => TriggerFixChoice::ThisBranch,
            core_model::FixChoice::NewPr => TriggerFixChoice::NewPr,
        }
    }
}

impl From<TriggerFixChoice> for core_model::FixChoice {
    fn from(choice: TriggerFixChoice) -> core_model::FixChoice {
        match choice {
            TriggerFixChoice::ThisBranch => core_model::FixChoice::ThisBranch,
            TriggerFixChoice::NewPr => core_model::FixChoice::NewPr,
        }
    }
}

impl TriggerPullRequest {
    /// A pull request's address as Fleet recorded it.
    pub fn at(url: &str) -> TriggerPullRequest {
        let number = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .and_then(|last| last.parse().ok());
        TriggerPullRequest {
            url: url.to_string(),
            number,
        }
    }
}

impl From<&core_model::RepairRecord> for TriggerRepair {
    fn from(record: &core_model::RepairRecord) -> TriggerRepair {
        TriggerRepair {
            attempt: record.tries,
            branch: record.branch.clone(),
            files: record.files.clone(),
            choice: record.choice.map(TriggerFixChoice::from),
            pull_request: record.pull_request.as_deref().map(TriggerPullRequest::at),
        }
    }
}

impl From<core_model::TriggerState> for TriggerFiringState {
    fn from(state: core_model::TriggerState) -> TriggerFiringState {
        match state {
            core_model::TriggerState::Skipped => TriggerFiringState::Skipped,
            core_model::TriggerState::Running => TriggerFiringState::Running,
            core_model::TriggerState::Passed => TriggerFiringState::Passed,
            core_model::TriggerState::Failed => TriggerFiringState::Failed,
            core_model::TriggerState::AwaitingOwner => TriggerFiringState::AwaitingOwner,
            core_model::TriggerState::Repairing => TriggerFiringState::Repairing,
            core_model::TriggerState::Rerunning => TriggerFiringState::Rerunning,
            core_model::TriggerState::FixReady => TriggerFiringState::FixReady,
            core_model::TriggerState::Held => TriggerFiringState::Held,
        }
    }
}

impl From<&core_model::TriggerSkipped> for TriggerSkip {
    fn from(skipped: &core_model::TriggerSkipped) -> TriggerSkip {
        let (reason, name) = match skipped {
            core_model::TriggerSkipped::NotInThisRepo { command } => {
                (TriggerSkipReason::NotInThisRepo, command)
            }
            core_model::TriggerSkipped::SkillNotRun { skill } => {
                (TriggerSkipReason::SkillNotRun, skill)
            }
            core_model::TriggerSkipped::ByOwner => (TriggerSkipReason::ByOwner, &String::new()),
        };
        TriggerSkip {
            reason,
            name: name.clone(),
            said: skipped.to_string(),
        }
    }
}

impl From<&core_model::TriggerFiring> for JobTrigger {
    /// A firing as it stands. `log_at` is the instant the Job's log line for it
    /// carries, which `fleet::triggering` stamps from the firing's own end.
    fn from(firing: &core_model::TriggerFiring) -> JobTrigger {
        let ended = firing.ended_at.as_ref();
        JobTrigger {
            name: firing.name.clone(),
            when: firing.when.into(),
            step: StepId::from(&firing.step),
            level: firing.source.into(),
            state: firing.state.into(),
            skipped: firing.skipped.as_ref().map(TriggerSkip::from),
            exit_code: firing.exit_code,
            started_at: Some(Instant::from(&firing.started_at)),
            ended_at: ended.map(Instant::from),
            log_at: match firing.state {
                core_model::TriggerState::Running => None,
                _ => Some(Instant::from(ended.unwrap_or(&firing.started_at))),
            },
            repair: (firing.repair.tries > 0).then(|| TriggerRepair::from(&firing.repair)),
            blocks: firing.on_failure.block,
            drone: false,
        }
    }
}

impl From<&core_model::FrozenTrigger> for JobTrigger {
    /// A frozen Trigger whose moment has not come.
    fn from(frozen: &core_model::FrozenTrigger) -> JobTrigger {
        JobTrigger {
            name: frozen.name.clone(),
            when: frozen.when.into(),
            step: StepId::from(&frozen.step),
            level: frozen.source.into(),
            state: TriggerFiringState::Pending,
            skipped: None,
            exit_code: None,
            started_at: None,
            ended_at: None,
            log_at: None,
            repair: None,
            blocks: frozen.on_failure.block,
            drone: matches!(
                frozen.resolution,
                core_model::TriggerResolution::Skill { .. }
                    | core_model::TriggerResolution::Drone { .. }
            ),
        }
    }
}
