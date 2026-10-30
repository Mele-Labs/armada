//! A piece as the file syntax `config`'s parsers read, written from the DTO a read answered.
//!
//! **Shaped like the file and not like the wire.** The wire tags an act `{"act": "tracker_move",
//! "to": ..}` and the file writes `tracker_write: {move: ..}`; each type below is one place the
//! two differ. A closed set the wire owns is matched here with no catch-all, so a variant added to
//! it fails to compile until the file can say it. **What this makes is only text**: `piece_saving`
//! parses it with the piece's own parser before it is kept, and the round trip is tested against
//! the parsers' own fixtures.
//!
//! An enum whose variants differ in their key is `untagged` with the key as a field. The tagged
//! form is written as a YAML tag, which no parser here reads.

use std::collections::BTreeMap;

use ipc::{
    AutonomyToSave, LifecycleDefinition, LifecycleRule, PieceScope, RuleAct, RuleAmong, RuleCondition, RuleTarget,
    RuleWhen, StateReadFrom, WorkspaceToSave,
};
use serde::Serialize;

const VERSION: u8 = 1;

/// `scope:` is left out for every repository, which is what the parsers read as absent.
fn scope_of(scope: &PieceScope) -> Option<&str> {
    match scope {
        PieceScope::Everywhere => None,
        PieceScope::Within { id } => Some(id),
    }
}

#[derive(Serialize)]
struct LifecycleText<'a> {
    version: u8,
    lifecycle_id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'a str>,
    states: Vec<StateText<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    transitions: Vec<TransitionText<'a>>,
    ends: &'a [String],
}

#[derive(Serialize)]
struct StateText<'a> {
    id: &'a str,
    label: &'a str,
    read: ReadText<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ReadText<'a> {
    Tracker { tracker: &'a [String] },
    Forge { forge: &'a str },
    Watch { watch: &'a str },
    Fleet { fleet: &'a str },
}

#[derive(Serialize)]
struct TransitionText<'a> {
    from: &'a str,
    to: &'a str,
}

pub(super) fn lifecycle(lifecycle: &LifecycleDefinition) -> Result<String, String> {
    config::piece_text(&LifecycleText {
        version: VERSION,
        lifecycle_id: &lifecycle.id,
        name: &lifecycle.name,
        scope: scope_of(&lifecycle.scope),
        states: lifecycle
            .states
            .iter()
            .map(|state| StateText {
                id: &state.id,
                label: &state.label,
                read: match &state.read {
                    StateReadFrom::Tracker { words } => ReadText::Tracker { tracker: words },
                    StateReadFrom::Forge { fact } => ReadText::Forge { forge: fact },
                    StateReadFrom::Watch { state } => ReadText::Watch { watch: state },
                    StateReadFrom::Fleet { fact } => ReadText::Fleet { fleet: fact },
                },
            })
            .collect(),
        transitions: lifecycle.transitions.iter().map(|one| TransitionText { from: &one.from, to: &one.to }).collect(),
        ends: &lifecycle.ends,
    })
}

#[derive(Serialize)]
struct RuleText<'a> {
    version: u8,
    rule_id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'a str>,
    when: WhenText<'a>,
    #[serde(rename = "if", skip_serializing_if = "Vec::is_empty")]
    conditions: Vec<ConditionText<'a>>,
    #[serde(rename = "do")]
    does: Vec<ActText<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autonomy: Option<&'a str>,
    on_failure: FailureText<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum WhenText<'a> {
    Enters { enters: EntersText<'a> },
    StepStarts { step_starts: &'a str },
    StepEnds { step_ends: &'a str },
    Job { job: &'a str },
}

#[derive(Serialize)]
struct EntersText<'a> {
    lifecycle: &'a str,
    state: &'a str,
}

/// Which instances, written as the one key of `entered`, `linked_from`, `linking_to` and `held` the
/// parsers take, with the state they stand in.
#[derive(Serialize, Default)]
struct TargetText<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    entered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linked_from: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linking_to: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    held: Option<&'a str>,
    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    in_state: Option<&'a str>,
}

fn target_of<'a>(among: &'a RuleAmong, in_state: Option<&'a str>) -> TargetText<'a> {
    let named = TargetText { in_state, ..TargetText::default() };
    match among {
        RuleAmong::Entered => TargetText { entered: Some(true), ..named },
        RuleAmong::LinkedFrom { lifecycle } => TargetText { linked_from: Some(lifecycle), ..named },
        RuleAmong::LinkingTo { lifecycle } => TargetText { linking_to: Some(lifecycle), ..named },
        RuleAmong::Held { lifecycle } => TargetText { held: Some(lifecycle), ..named },
    }
}

fn on_of(target: &RuleTarget) -> TargetText<'_> {
    target_of(&target.among, target.in_state.as_deref())
}

#[derive(Serialize)]
#[serde(untagged)]
enum ConditionText<'a> {
    /// `every`, `any` or `no` is the key, so it is a map of one. Any other word is the parser's to refuse.
    Instances(BTreeMap<&'a str, TargetText<'a>>),
    Job { job: JobFactText<'a> },
}

#[derive(Serialize)]
#[serde(untagged)]
enum JobFactText<'a> {
    Workflow { workflow: &'a str },
    Step { step: &'a str },
    PullRequestOpened { pull_request_opened: bool },
}

#[derive(Serialize)]
#[serde(untagged)]
enum ActText<'a> {
    Tracker { tracker_write: TrackerText<'a> },
    Forge { forge_write: ForgeText<'a> },
    Draft { draft: DraftText<'a> },
    StartJob { start_job: StartJobText<'a> },
    StartWatch { start_watch: StartWatchText<'a> },
    Tell { tell_agent: TellText<'a> },
    Command { run_command: CommandText<'a> },
    Agent { run_agent: AgentText<'a> },
}

#[derive(Serialize)]
struct TrackerText<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    assign: Option<&'a str>,
    #[serde(rename = "move", skip_serializing_if = "Option::is_none")]
    move_to: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    close: Option<bool>,
    on: TargetText<'a>,
}

#[derive(Serialize)]
struct ForgeText<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    request_reviewers: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mark_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_merge: Option<bool>,
    on: TargetText<'a>,
}

#[derive(Serialize)]
struct DraftText<'a> {
    template: &'a str,
    on: TargetText<'a>,
}

#[derive(Serialize)]
struct StartJobText<'a> {
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow: Option<&'a str>,
}

#[derive(Serialize)]
struct StartWatchText<'a> {
    recipe: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    values: &'a BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deploy: Option<&'a str>,
}

#[derive(Serialize)]
struct TellText<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wake: Option<bool>,
}

#[derive(Serialize)]
struct CommandText<'a> {
    command: &'a str,
}

#[derive(Serialize)]
struct AgentText<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    skill: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drone: Option<&'a str>,
}

#[derive(Serialize)]
struct FailureText<'a> {
    retries: u8,
    then: &'a str,
}

fn tracker<'a>(on: &'a RuleTarget) -> TrackerText<'a> {
    TrackerText { assign: None, move_to: None, close: None, on: on_of(on) }
}

fn forge<'a>(on: &'a RuleTarget) -> ForgeText<'a> {
    ForgeText { request_reviewers: None, mark_ready: None, merge: None, auto_merge: None, on: on_of(on) }
}

fn act_of(act: &RuleAct) -> ActText<'_> {
    match act {
        RuleAct::TrackerAssign { to, target } => ActText::Tracker { tracker_write: TrackerText { assign: Some(to), ..tracker(target) } },
        RuleAct::TrackerMove { to, target } => ActText::Tracker { tracker_write: TrackerText { move_to: Some(to), ..tracker(target) } },
        RuleAct::TrackerClose { target } => ActText::Tracker { tracker_write: TrackerText { close: Some(true), ..tracker(target) } },
        RuleAct::ForgeRequestReviewers { from, target } => {
            ActText::Forge { forge_write: ForgeText { request_reviewers: Some(from), ..forge(target) } }
        }
        RuleAct::ForgeMarkReady { target } => ActText::Forge { forge_write: ForgeText { mark_ready: Some(true), ..forge(target) } },
        RuleAct::ForgeMerge { target } => ActText::Forge { forge_write: ForgeText { merge: Some(true), ..forge(target) } },
        RuleAct::ForgeAutoMerge { target } => ActText::Forge { forge_write: ForgeText { auto_merge: Some(true), ..forge(target) } },
        RuleAct::Draft { template, target } => ActText::Draft { draft: DraftText { template, on: on_of(target) } },
        RuleAct::StartJob { workflow, prompt } => ActText::StartJob { start_job: StartJobText { prompt, workflow: workflow.as_deref() } },
        RuleAct::StartWatch { recipe, values, deploy } => {
            ActText::StartWatch { start_watch: StartWatchText { recipe, values, deploy: deploy.as_deref() } }
        }
        RuleAct::Tell { message } => ActText::Tell { tell_agent: TellText { message: Some(message), wake: None } },
        RuleAct::Wake => ActText::Tell { tell_agent: TellText { message: None, wake: Some(true) } },
        RuleAct::RunCommand { command } => ActText::Command { run_command: CommandText { command } },
        RuleAct::RunSkill { skill } => ActText::Agent { run_agent: AgentText { skill: Some(skill), drone: None } },
        RuleAct::RunDrone { prompt } => ActText::Agent { run_agent: AgentText { skill: None, drone: Some(prompt) } },
    }
}

pub(super) fn rule(rule: &LifecycleRule) -> Result<String, String> {
    config::piece_text(&RuleText {
        version: VERSION,
        rule_id: &rule.id,
        name: &rule.name,
        scope: scope_of(&rule.scope),
        when: match &rule.when {
            RuleWhen::Enters { lifecycle, state } => WhenText::Enters { enters: EntersText { lifecycle, state } },
            RuleWhen::StepStarts { step } => WhenText::StepStarts { step_starts: step },
            RuleWhen::StepEnds { step } => WhenText::StepEnds { step_ends: step },
            RuleWhen::Job { event } => WhenText::Job { job: event },
        },
        conditions: rule
            .conditions
            .iter()
            .map(|condition| match condition {
                RuleCondition::Instances { among, quantity, in_state } => {
                    ConditionText::Instances(BTreeMap::from([(quantity.as_str(), target_of(among, Some(in_state)))]))
                }
                RuleCondition::Workflow { workflow } => ConditionText::Job { job: JobFactText::Workflow { workflow } },
                RuleCondition::AtStep { step } => ConditionText::Job { job: JobFactText::Step { step } },
                RuleCondition::PullRequestOpened => {
                    ConditionText::Job { job: JobFactText::PullRequestOpened { pull_request_opened: true } }
                }
            })
            .collect(),
        does: rule.does.iter().map(|doing| act_of(&doing.what)).collect(),
        autonomy: rule.autonomy.as_deref(),
        on_failure: FailureText { retries: rule.on_failure.retries, then: &rule.on_failure.then },
    })
}

#[derive(Serialize)]
struct WorkspaceText<'a> {
    version: u8,
    workspace_id: &'a str,
    name: &'a str,
    repositories: Vec<&'a str>,
}

pub(super) fn workspace(workspace: &WorkspaceToSave) -> Result<String, String> {
    config::piece_text(&WorkspaceText {
        version: VERSION,
        workspace_id: &workspace.id,
        name: &workspace.name,
        repositories: workspace.repositories.iter().map(|one| one.as_str()).collect(),
    })
}

#[derive(Serialize)]
struct AutonomyText<'a> {
    version: u8,
    autonomy_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'a str>,
    defaults: &'a BTreeMap<String, String>,
}

pub(super) fn autonomy(autonomy: &AutonomyToSave) -> Result<String, String> {
    config::piece_text(&AutonomyText {
        version: VERSION,
        autonomy_id: &autonomy.id,
        scope: scope_of(&autonomy.scope),
        defaults: &autonomy.defaults,
    })
}
