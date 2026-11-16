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
    /// A Job's `SendMessage` to a Job the receiving Mac holds.
    JobMessage(JobMessage),
    /// Move Job `session_id` to the Mac that sent this, once its turn ends.
    Move { session_id: SessionId },
    /// A Job the sending Mac is moving here, its turn ended and its branch pushed. Answered at once; the
    /// receiving Mac says how it went with a `move_settled` once it has the conversation.
    Arrive(ArrivingJob),
    /// The Mac a Job moved to took it, or refused it with why, so the Mac it left ends it or resumes it.
    MoveSettled(MoveSettled),
}

/// How a move ended on the Mac the Job went to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveSettled {
    pub session_id: SessionId,
    /// Why that Mac would not take it. Absent where it holds the Job now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// What a Job moving between Macs takes with it: its row, the branch its work is on, and the length and hash of
/// its conversation, which the receiving Mac reads in pieces from the sending one before it resumes the Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrivingJob {
    /// The row as the sending Mac had it. Its `id` is kept, so the Job has the same id on both Macs.
    pub record: crate::SessionRecord,
    /// Pushed to `origin` before this was sent. Absent for a Job that never held a slot, which is given a
    /// fresh branch where it lands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub conversation: CarriedConversation,
}

/// A Job's conversation file, as the Mac sending it has it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarriedConversation {
    pub bytes: u64,
    /// SHA-256 of the whole file, lowercase hex.
    pub sha256: String,
}

/// `move_job_to_mac`: move a hosted Job to another of his Macs. Its turn finishes, its branch is pushed, and the
/// other Mac resumes the same conversation in a fresh slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveJobToMac {
    pub session_id: SessionId,
    /// The Mac to move it to, for a Job this Mac holds. Absent for a Job held from another Mac, which comes here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
}

/// A message from Job `from` on the sending Mac to Job `to` on the Mac that holds it, written into `to`'s thread
/// in `from`'s voice, with the sending Mac named.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobMessage {
    pub from: SessionId,
    /// `from`'s title, as the receiving thread names it.
    pub title: String,
    pub to: SessionId,
    pub text: String,
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
            JobAct::JobMessage(one) => Some(&one.to),
            JobAct::Move { session_id } => Some(session_id),
            JobAct::MoveSettled(one) => Some(&one.session_id),
            JobAct::Start(_) | JobAct::Dispatch(_) | JobAct::Arrive(_) => None,
        }
    }

    /// The project a start, a dispatch or an arriving Job names.
    pub fn starts_in(&self) -> Option<&str> {
        match self {
            JobAct::Start(one) => one.project.as_deref(),
            JobAct::Dispatch(one) => one.start.project.as_deref(),
            JobAct::Arrive(one) => one.record.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()),
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
            JobAct::JobMessage(_) => "job_message",
            JobAct::Move { .. } => "move",
            JobAct::Arrive(_) => "arrive",
            JobAct::MoveSettled(_) => "move_settled",
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
