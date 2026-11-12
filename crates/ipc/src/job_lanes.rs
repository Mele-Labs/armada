//! A Job's lanes as a screen draws them, worked out by Fleet so a client does not. Carried on
//! `SessionRecord::lanes`. `docs/practices/protocol/a-jobs-lanes-fleet-works-out.md`.
//!
//! **Two layers.** What a Job holds and what its rules did (`job`), and the lanes and edges drawn from
//! it (`lanes`, `rules`), so a client draws the second and opens a panel from the first. Bridge's
//! TypeScript worked both out (`lanesOfJob` and `jobLanes`) and keeps doing so for now.
//!
//! **A time is an instant, never a clock.** Fleet knows no time zone, so `LaneFired::at` is the instant
//! and the client words it.

use serde::{Deserialize, Serialize};

use crate::Instant;

fn is_false(value: &bool) -> bool {
    !*value
}

/// What a node is: its shape and its tint. A kind is never a status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneNodeKind {
    Step,
    Tracker,
    Forge,
    Watch,
    Job,
    /// Where the workflow starts: a small circle, no step.
    Start,
    /// Where the workflow ends.
    End,
}

/// Whether a node is a step of the workflow or a state of a lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneNodeRole {
    Step,
    State,
}

/// What reads a lifecycle's states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneLifecycleKind {
    Tracker,
    Forge,
    Watch,
    Job,
}

/// Whether a lane is the workflow's or an instance's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneKind {
    Workflow,
    Lifecycle,
}

/// How far a rule goes on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneAutonomy {
    Acts,
    Asks,
    Off,
}

/// Whose rule it is: Armada's, the person's own (Kit's) or the repository's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneKept {
    Armada,
    Yours,
    Repository,
}

/// The moment a rule is for. Fleet sets `starts`, `ends` and `enters`; `leaves` is never set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneMoment {
    Starts,
    Ends,
    Enters,
    Leaves,
}

/// How a rule that fired ended: its acts done, waiting on the person, still to be done, or set aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneOutcome {
    Acted,
    Asked,
    Waiting,
    Skipped,
}

/// A ticket a started Job holds for the Job that started it. **`state` and `state_id` are empty** where it
/// stands in no state: not read yet, or a status no state lists, which `state` then keeps as it was said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneCarriedTicket {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub state: String,
    pub state_id: String,
}

/// One state of a lifecycle as a Job draws it, with the column it stands in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneLifecycleState {
    pub id: String,
    pub label: String,
    pub col: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub end: bool,
}

/// A lifecycle the Job holds an instance of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneLifecycle {
    pub id: String,
    pub label: String,
    /// Where its states are read from, in a word under the lane's title: `Tracker`, `Forge`, `Watch` or `Fleet`.
    pub from: String,
    pub kind: LaneLifecycleKind,
    pub states: Vec<LaneLifecycleState>,
    /// The ways a state is left, as pairs of state ids.
    pub next: Vec<(String, String)>,
}

/// A step of the Job's workflow, in the column its order gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneStep {
    pub id: String,
    pub label: String,
    pub col: u32,
}

/// Either end of a rule's edge: a lane (`workflow` or a lifecycle's id) and the step or state in it.
/// `$step` as the item is the step the Job was on when the rule fired, else the one it is on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneRef {
    pub lane: String,
    pub item: String,
}

/// A rule as a Job's lanes draw it. A rule a Job event or a gate sets off is not drawn, so it is not here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneRule {
    pub id: String,
    pub label: String,
    pub from: LaneRef,
    pub at: LaneMoment,
    pub to: LaneRef,
    pub autonomy: LaneAutonomy,
    pub kept: LaneKept,
}

/// One thing a Job holds: a ticket, its pull request, a deploy, a Job it started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneInstance {
    /// `<lifecycle>:<key, percent-encoded>`: a key can hold a slash, which a node id cannot.
    pub id: String,
    pub lifecycle: String,
    pub label: String,
    /// A ticket's own title; `Unmapped: <word>` for a status no state lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// For a Job this one started: the ticket it was started for, read from there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<LaneCarriedTicket>,
    /// Empty where it stands in no state.
    pub state: String,
    /// The word its source used, for a status no state lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmapped: Option<String>,
    /// Ending the Job ended it, standing where it was.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ended: bool,
    /// The states it went through, in order, the current one last.
    pub visited: Vec<String>,
}

/// One act of a rule as the Job's rules kept it. `standing` is Fleet's word: `planned`, `asked`, `stuck`,
/// `done`, `skipped`, `told` or `not_carried`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneFiredAct {
    pub text: String,
    /// `acts` or `asks`.
    pub course: LaneAutonomy,
    pub standing: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
    /// When it was settled, else when it was planned.
    pub at: Instant,
}

/// A rule that fired for an instance, and how it ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneFired {
    pub rule: String,
    /// The instance it was done to, by its lane id; empty where a step set it off and it handed to a step.
    pub instance: String,
    pub outcome: LaneOutcome,
    /// When its last act was settled, else planned.
    pub at: Instant,
    /// What it did, a line for each act.
    pub did: Vec<String>,
    /// The id of the step the Job was on when it was planned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    pub acts: Vec<LaneFiredAct>,
}

/// A change made to the Job's own copy, marked on what it changes. `lane` is an instance's id, `workflow`,
/// `rule`, or a lifecycle's id with no `item`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneChange {
    pub id: String,
    pub lane: String,
    pub item: String,
    pub text: String,
    pub at: Instant,
    #[serde(default, skip_serializing_if = "is_false")]
    pub dropped: bool,
    /// A changed rule's name, from any rule the Job or its repository holds, drawn or not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The definitions the Job's lanes are drawn from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneCopy {
    pub steps: Vec<LaneStep>,
    pub lifecycles: Vec<LaneLifecycle>,
    pub rules: Vec<LaneRule>,
}

/// What a Job holds and what its rules did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobLanesHeld {
    pub copy: LaneCopy,
    pub instances: Vec<LaneInstance>,
    pub fired: Vec<LaneFired>,
    pub changes: Vec<LaneChange>,
    /// What the step the Job is on waits for, a sentence for each end that has not come.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting: Vec<String>,
}

/// A node of a lane: a step or a state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneNode {
    /// `<lane id>/<item>`.
    pub id: String,
    pub label: String,
    pub col: u32,
    pub role: LaneNodeRole,
    pub kind: LaneNodeKind,
    /// The step the agent is working on.
    #[serde(default, skip_serializing_if = "is_false")]
    pub lit: bool,
    /// Where an instance stands, or the step a Job is at with nobody working on it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub here: bool,
    /// An earlier state the instance went through, or the start of a workflow that has begun.
    #[serde(default, skip_serializing_if = "is_false")]
    pub passed: bool,
    /// One sentence for each end the step has not had.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub end: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub skipped: bool,
    /// Where an instance stood when ending the Job ended it. It takes the place of `here`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ended: bool,
    /// The ticket the Job on this lane was started for, drawn on the node the Job stands at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<LaneCarriedTicket>,
    /// The Job's own copy changed here, and how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed: Option<String>,
    /// A workflow's editor only: the Workspaces and repositories, by name, that name or read this state
    /// otherwise, add it, or leave it out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub differs: Vec<String>,
    /// A workflow's editor only: the Workspaces whose pieces tie for this repository, so it reads the
    /// default's: `Web and Mobile differ`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clash: Option<String>,
}

/// What a piece's mark waits on in a workflow's editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanePending {
    /// A pull request that removes it from the repository. Fleet sets none: the window that opened it knows.
    Removal,
    /// The owner's Approve of a draft that keeps it for every Job.
    Keep,
}

/// A lane: the workflow's steps, or one instance's states.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneView {
    pub id: String,
    pub kind: LaneKind,
    pub title: String,
    /// Under the title, quiet: where its states are read from.
    pub source: String,
    /// What kind of thing the lane is, in its lifecycle's own name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,
    /// The tooltip of the title: a ticket's own title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The tracker's lifecycle: this lane is a ticket.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ticket: bool,
    /// Ending the Job ended this instance.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ended: bool,
    /// The Job's own copy of this lane's lifecycle changed, and how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed: Option<String>,
    /// A workflow's editor only: what this lane's lifecycle waits on before it is kept or goes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<LanePending>,
    pub nodes: Vec<LaneNode>,
    /// The ways a state is left within the lane, as pairs of node ids.
    pub moves: Vec<(String, String)>,
}

/// An edge: a rule that fired, or one still waiting, between two nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneEdge {
    /// `<rule>:<source id>` and the target's after `>`; one rule has an edge for each instance.
    pub id: String,
    /// The rule this edge is drawn for.
    pub rule: String,
    pub from: String,
    pub to: String,
    pub label: String,
    /// What a rule still waiting waits for: `Plan starts`, `PR #812: Approved`. Absent once it fired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<String>,
    /// When it fired. Absent while it waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fired_at: Option<Instant>,
    pub autonomy: LaneAutonomy,
    pub status: LaneOutcome,
    /// What it did, a line for each act. Absent while it waits.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub did: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed: Option<String>,
    /// The same rule's second edge: drawn with its mark and no words.
    #[serde(default, skip_serializing_if = "is_false")]
    pub quiet: bool,
    /// A workflow's editor only: the Workspaces and repositories, by name, where the rule applies alone or
    /// is changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub differs: Vec<String>,
    /// A workflow's editor only: the Workspaces whose rules tie for this repository: `Web and Mobile differ`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clash: Option<String>,
    /// A workflow's editor only: a rule a draft waiting on Approve would add, which no place keeps yet.
    #[serde(default, skip_serializing_if = "is_false")]
    pub proposed: bool,
    /// A workflow's editor only: what this rule waits on before it is kept or goes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<LanePending>,
}

/// `SessionRecord::lanes`: where the Job stands in its workflow, what it holds, and the lanes and edges
/// drawn from both. **Absent** for a Job that has ended and for one that holds nothing and follows no
/// workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobLanesView {
    /// The workflow's name; empty for a Job on none.
    pub workflow: String,
    /// The step the Job is on: -1 before a step starts, the step count once every one has ended.
    pub at: i32,
    /// The agent is working on the step it is on.
    #[serde(default, skip_serializing_if = "is_false")]
    pub working: bool,
    /// The steps the person skipped, by index.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<u32>,
    pub job: JobLanesHeld,
    pub lanes: Vec<LaneView>,
    pub rules: Vec<LaneEdge>,
}
