//! An act on a Job another of his Macs holds, and what waits while that Mac cannot be reached.
//! `docs/practices/protocol/his-other-macs-fleets-half.md`, *Acting across Macs*.
//!
//! **Each act wraps the body its own route already takes**, so the Mac that holds the Job runs it with the
//! code that runs it for the person at home.

use serde::{Deserialize, Serialize};

use crate::{
    AnswerDocument, AnswerSessionAsk, AnswerSketch, AnswerWaiting, AnswerWaitingParts, ApprovePlan, DispatchJob, SendPlanChanges,
    SendSessionMessageBody, SessionId, StartSession, StopSession,
};

/// One act on a Job. Adjacently tagged, since `DispatchJob` has a `kind` of its own that an internal tag would clash with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub enum JobAct {
    Message(SendSessionMessageBody),
    AskAnswer(AnswerSessionAsk),
    WaitingAnswer(AnswerWaiting),
    WaitingAnswerParts(AnswerWaitingParts),
    PlanApprove(ApprovePlan),
    PlanChanges(SendPlanChanges),
    SketchAnswer(AnswerSketch),
    DocumentAnswer(AnswerDocument),
    Stop(StopSession),
    /// The pull request `repository` `number`, which Job `session_id` owns.
    Merge { session_id: SessionId, repository: String, number: u64 },
    Start(StartSession),
    Dispatch(DispatchJob),
}

impl JobAct {
    /// The Job the act is on. A start or a dispatch is on none.
    pub fn session_id(&self) -> Option<&SessionId> {
        match self {
            JobAct::Message(one) => Some(&one.message.session_id),
            JobAct::AskAnswer(one) => Some(&one.session_id),
            JobAct::WaitingAnswer(one) => Some(&one.session_id),
            JobAct::WaitingAnswerParts(one) => Some(&one.session_id),
            JobAct::PlanApprove(one) => Some(&one.session_id),
            JobAct::PlanChanges(one) => Some(&one.session_id),
            JobAct::SketchAnswer(one) => Some(&one.session_id),
            JobAct::DocumentAnswer(one) => Some(&one.session_id),
            JobAct::Stop(one) => Some(&one.session_id),
            JobAct::Merge { session_id, .. } => Some(session_id),
            JobAct::Start(_) | JobAct::Dispatch(_) => None,
        }
    }

    /// The project a start or a dispatch names.
    pub fn starts_in(&self) -> Option<&str> {
        match self {
            JobAct::Start(one) => one.project.as_deref(),
            JobAct::Dispatch(one) => one.start.project.as_deref(),
            _ => None,
        }
    }

    /// The act's name on the wire, as its `kind` spells it.
    pub fn name(&self) -> &'static str {
        match self {
            JobAct::Message(_) => "message",
            JobAct::AskAnswer(_) => "ask_answer",
            JobAct::WaitingAnswer(_) => "waiting_answer",
            JobAct::WaitingAnswerParts(_) => "waiting_answer_parts",
            JobAct::PlanApprove(_) => "plan_approve",
            JobAct::PlanChanges(_) => "plan_changes",
            JobAct::SketchAnswer(_) => "sketch_answer",
            JobAct::DocumentAnswer(_) => "document_answer",
            JobAct::Stop(_) => "stop",
            JobAct::Merge { .. } => "merge",
            JobAct::Start(_) => "start",
            JobAct::Dispatch(_) => "dispatch",
        }
    }
}

/// `POST /other_mac/act` on the Mac that holds the Job: what crosses the link. **`id` is the sender's, made once
/// for the send**, so a retry after a dropped link is answered with the first answer and not run again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacAct {
    pub id: String,
    pub act: JobAct,
    /// When the person made it, in the sending Mac's milliseconds since the epoch.
    pub at: i64,
}

/// An act kept on the sending Mac while the Mac that holds its Job cannot be reached, oldest first. Answered
/// as the `202` body of the act, and listed by `list_queued`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queued {
    pub id: String,
    /// The Mac it waits for.
    pub mac: String,
    pub act: JobAct,
    /// When the person made it, in this Mac's milliseconds since the epoch.
    pub since: i64,
    /// What that Mac said when it refused the act on its turn: the act stays, and is no longer sent, until he takes it back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// `GET /queued` and `queued.changed`: every act waiting, for every Mac, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedList {
    pub queued: Vec<Queued>,
}

/// `POST /queued/take_back`: the act kept under `id`, taken back before it is sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeBack {
    pub id: String,
}

/// `POST /sessions/start`: a start, which says on which of his Macs where `mac` is present. **Absent is this
/// Mac**, so every body sent before this field is read as it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartOn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(flatten)]
    pub start: StartSession,
}

/// `POST /sessions/dispatch`: a dispatch, with `mac` as [`StartOn`]'s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchOn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    #[serde(flatten)]
    pub job: DispatchJob,
}
