//! A Job reads and commands the Jobs it started. `docs/concepts/session/a-job-that-started-jobs.md`.
//!
//! **Neither call names the Job it is for.** Fleet places the caller by its connection, as
//! `start_another_session` does, and a Job's children are the Jobs its `started_to` attachments
//! name. **A read answers the whole tree below the caller**, each Job with `started_by`, the Job that
//! started it; **a command is for a direct child only**, so a call that places no hosted Job is refused,
//! and so is an entry naming a Job the caller did not start, a grandchild included.

use serde::{Deserialize, Serialize};

use crate::session_summary::SessionSummary;
use crate::session_waits::{SessionWait, StartBase, StartCondition};
use crate::sessions::{SessionId, SessionRecord, SessionStanding, SessionState, WaitingItem};

/// The refusal of a read or a command whose call places no hosted Job. Here because the route
/// answers a whole batch with it.
pub const STARTED_UNPLACED: &str = "fleet.started_unplaced";

/// The workflow step a child is on, and what that step waits for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedStep {
    /// The step's index in the workflow.
    pub index: u32,
    /// The step's label, as the rail draws it.
    pub label: String,
    /// What the step waits for, one sentence for each end that has not come: `Waiting on PR #812: Approved`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_on: Vec<String>,
}

/// How a child ended, and what it said it achieved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedResult {
    /// Why it ended, as the ledger has it: `closed` for an end by Fleet or a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    /// What the agent said the Job achieved with `all_done`. Absent where it never said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// One Job below the caller, in one answer: who started it, where it stands, what it waits on and who
/// for, its pull requests, and its result once it ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedJob {
    /// Its id, title, state, branch and pull requests, as `list_sessions` gives them.
    #[serde(flatten)]
    pub summary: SessionSummary,
    /// The Job that started it: the caller for a child, the child for a grandchild.
    pub started_by: SessionId,
    /// What it is doing. Absent for a Job Fleet has read nothing of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<SessionStanding>,
    /// It waits on its start card for the person to press Start.
    pub held: bool,
    /// The person or a Job paused it: it holds no slot and runs no turn.
    pub paused: bool,
    /// The step of its workflow it is on. Absent for a Job on no workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StartedStep>,
    /// What it waits on the person for, the items Needs you shows for it, Fleet's own among them
    /// (`start:<id>`, `unsent:<id>`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_for: Vec<WaitingItem>,
    /// The Job it waits on before it starts, and the condition. Absent once it has started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<SessionWait>,
    /// Present once it ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<StartedResult>,
}

impl StartedJob {
    /// `record` as a line, started by `started_by`. `step` is the label of the step it is on, which the record
    /// carries as an index; `said` is its `all_done` summary, which a record of an ended Job does not.
    pub fn of(record: &SessionRecord, started_by: SessionId, step: Option<String>, said: Option<String>) -> StartedJob {
        let ended = record.state == SessionState::Ended;
        StartedJob {
            summary: SessionSummary::of(record),
            started_by,
            standing: record.standing,
            held: record.hosted.as_ref().is_some_and(|hosted| hosted.start.is_some()),
            paused: record.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()),
            step: record.workflow_step.zip(step).map(|(index, label)| StartedStep {
                index,
                label,
                waiting_on: record.workflow_waiting_on.clone(),
            }),
            waiting_for: record.waiting_for.clone(),
            waits: record.waits.clone(),
            result: ended.then(|| StartedResult {
                end_reason: record.end_reason.clone(),
                summary: said.or_else(|| record.done_summary.clone()),
            }),
        }
    }
}

/// `list_started_sessions`: every Job below the caller, each child followed by its own descendants,
/// oldest first at each level. Answered with this.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedSessions {
    pub jobs: Vec<StartedJob>,
}

/// What a Job does to a child.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartedAct {
    /// End a child still held on its start card or waiting on another Job, with no agent ever run.
    /// Refused for one that has begun.
    Withdraw,
    /// End a live child. Needs `why`, which the child is told.
    End,
    /// Pause a child: it gives up its slot and keeps its context.
    Pause,
    /// Resume a paused child.
    Resume,
    /// Change the condition a waiting child starts on, or where it starts from.
    Change,
    /// Drop a waiting child's wait: its card stays held for the person and waits on nothing.
    Drop,
}

impl StartedAct {
    /// The word the act goes by on the wire and in a refusal.
    pub fn word(self) -> &'static str {
        match self {
            StartedAct::Withdraw => "withdraw",
            StartedAct::End => "end",
            StartedAct::Pause => "pause",
            StartedAct::Resume => "resume",
            StartedAct::Change => "change",
            StartedAct::Drop => "drop",
        }
    }
}

/// One entry of `command_started_sessions`: an act on one child.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedCommand {
    /// The child. Refused where the caller did not start it.
    pub session_id: SessionId,
    pub act: StartedAct,
    /// Why, for `end`: the child is told it. Required there and ignored elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// For `change`: when the child starts. As `change_waiting`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<StartCondition>,
    /// For `change`: where the child starts from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<StartBase>,
}

/// `command_started_sessions`' body: the entries, each done as a call of its own would be, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartedCommands {
    pub commands: Vec<StartedCommand>,
}

/// What a call with `commands` answers: one entry per command, in the order given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StartedAnswers {
    pub commands: Vec<StartedAnswer>,
}

/// One entry of [`StartedAnswers`]: `done` with the child as it now stands, or `refused` with the
/// reason. Exactly one is present.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StartedAnswer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<StartedJob>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<crate::WireError>,
}
