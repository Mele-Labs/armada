//! A Job's retro: what got in the way while it ran, and the record it was read
//! from. `get_job_retro` answers one Job's and `list_lessons` every item across
//! Jobs, newest first. Since 23.12. `docs/concepts/retro.md`.
//!
//! **The record is assembled on every read and never stored.** Each row is a
//! fact already on the Job's record, so a read made after the Job ended says
//! what it says, and the retro's items cite the rows by [`cite`] rather than
//! repeating them.
//!
//! [`cite`]: RecordRefusal::cite

use serde::{Deserialize, Serialize};

use crate::enums::{Actor, Via, Whose};
use crate::ids::{Instant, JobId, StepId};

/// `get_job_retro`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRetro {
    pub job_id: JobId,
    pub state: RetroState,
    /// When the retro was kept. Absent while it is `pending`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
    /// The model that wrote it, on a `written` retro alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Why there is none, on a `failed` or `skipped` retro alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// What got in the way, in the order the model wrote it. Empty on a
    /// written retro is a Job nothing got in the way of.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<RetroItem>,
    pub record: RetroRecord,
    /// Notes the owner left in Bridge while this Job's detail was open.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<LinkedAnnotation>,
}

/// Where a Job's retro stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetroState {
    /// The Job has not ended, or it has and the retro is not written yet.
    Pending,
    Written,
    /// The call failed or its answer would not read. Not tried again.
    Failed,
    /// None is owed: the Job ended before Fleet wrote retros, or no Drone ever
    /// ran on it.
    Skipped,
}

/// One thing that got in the way.
///
/// **Also the shape the model answers in**, inside [`RetroWritten`], so what
/// it wrote and what the wire carries cannot drift.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroItem {
    /// Whose way it got in.
    pub who: Whose,
    /// One sentence: what got in the way.
    pub statement: String,
    /// The [`RetroRecord`] rows that show it, by `cite`. Never empty.
    pub evidence: Vec<String>,
}

/// What the retro call answers with. Read through [`crate::decode`] and
/// nowhere else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroWritten {
    pub items: Vec<RetroItem>,
}

/// `list_lessons`: retro items across Jobs, newest retro first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lessons {
    pub lessons: Vec<Lesson>,
}

/// One retro item, with the Job it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    pub job_id: JobId,
    pub handle: String,
    /// When the retro this item is in was written.
    pub at: Instant,
    pub who: Whose,
    pub statement: String,
    /// `cite` values on the Job's own retro record: `get_job_retro` resolves
    /// them.
    pub evidence: Vec<String>,
}

/// Everything on a Job's record a retro is read from. Every row carries a
/// `cite`, `refusal:1` and the like, unique within the record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroRecord {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusals: Vec<RecordRefusal>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_checks: Vec<RecordCheck>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_met: Vec<RecordNotMet>,
    /// What each step's submission said it had not done.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_done: Vec<RecordSaid>,
    /// What a Drone said in prose right after each submission, and the last
    /// thing each one said: where a Drone tells a person what it could not get
    /// to, which no field of a submission asked it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub said_after: Vec<RecordSaid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub restarts: Vec<RecordAct>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<RecordAsked>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waited: Vec<RecordWaited>,
    /// Every move a person or an agent made, with the door it came through.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<RecordAct>,
    /// What a Drone said got in its way, on submitting.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<RecordSaid>,
}

/// A tool call the Drone was refused, with what it tried.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRefusal {
    pub cite: String,
    pub at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    pub tool: String,
    /// The argument, off the call the refusal answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tried: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,
}

/// Which run a failed Check was in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunBy {
    /// The gate's run, which decides.
    Gate,
    /// The Drone's own `run_checks`, which decides nothing.
    Drone,
}

/// A Check that did not pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordCheck {
    pub cite: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    pub name: String,
    pub run: CheckRunBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced: Option<String>,
}

/// A Judge criterion that was not met.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordNotMet {
    pub cite: String,
    pub step: StepId,
    pub attempt: u32,
    pub criterion: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced: Option<String>,
}

/// Something a Drone said, under the step it said it on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSaid {
    pub cite: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    pub said: String,
}

/// A move somebody other than Fleet made, or a restart of either kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordAct {
    pub cite: String,
    pub at: Instant,
    pub actor: Actor,
    /// The door it came through. Absent on a move older than 23.12.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<Via>,
    /// What moved, in a few words: `status queued`, `step plan running`.
    pub moved: String,
    /// What the person said, where they said something with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
}

/// A question put to a person, and how long it waited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordAsked {
    pub cite: String,
    pub asked_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    /// What was asked: Fleet's own line, and what it was about.
    pub about: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    /// From the ask to the answer. Absent where none came.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waited_ms: Option<u64>,
}

/// A stretch the Job spent at an `awaiting_*` status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordWaited {
    pub cite: String,
    pub status: crate::enums::JobStatus,
    pub from: Instant,
    /// Absent where the Job is still there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,
}

/// A note the owner left in Bridge with this Job's detail open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedAnnotation {
    pub id: String,
    pub at: Instant,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

/// One file under `.armada/annotations/`, as Bridge writes it. **Read, never
/// written, here**, and only the fields a retro uses; the file carries more.
///
/// `open_job_id` is the Job whose detail was open when the note was left.
/// Bridge does not write it yet, and a note without it is linked to no Job:
/// nothing here guesses one from a time window.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationFile {
    pub id: String,
    pub text: String,
    pub created_at: String,
    #[serde(default)]
    pub screen: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub open_job_id: Option<String>,
}
