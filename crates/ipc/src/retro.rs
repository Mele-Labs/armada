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

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize};

use crate::enums::{Actor, LandsIn, LessonState, Via, Whose};
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
/// **`title`, `what` and `fix` are absent on an item kept before 23.26**, which
/// has `statement` alone. On one written since, `statement` repeats `what`, so
/// a reader that predates the three still has a sentence to draw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroItem {
    /// Names this item in `agree_lesson` and `disagree_lesson`. Stable: it is
    /// the Job's id and the item's place in its retro, and a retro is written
    /// once. Since 23.26.
    pub id: String,
    /// Whose way it got in.
    pub who: Whose,
    /// A headline of about eight words. Since 23.26.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// One or two short sentences: what happened, and to whom. Since 23.26.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,
    /// One sentence naming what to change. Since 23.26.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    /// What got in the way, as one piece of prose. On an item written since
    /// 23.26 it is `what`.
    pub statement: String,
    /// The [`RetroRecord`] rows that show it, by `cite`. Never empty.
    pub evidence: Vec<String>,
    /// Where its fix lands. Since 23.15, and on every item written since:
    /// **absent on an item kept before**, never defaulted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands_in: Option<LandsIn>,
    /// Where it stands with the person, as on [`Lesson`]. **Absent only on an
    /// item whose row has no answer record**, read as before. Since 23.27.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<LessonState>,
    /// The Job proposed for it, as on [`Lesson`]. Since 23.27.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_proposed: Option<JobId>,
    /// What Accept would change in Kit, as on [`Lesson`]. Since 23.35.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<RetroChange>,
    /// What Accept applied, as on [`Lesson`]. Since 23.35.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<RetroChange>,
}

/// A change to Kit a retro item carries, which `agree_lesson` applies. Since
/// 23.35.
///
/// **Copied by Fleet off a refusal the record shows, never written by the
/// model.** The model only names which refusal; the command is the one that
/// refusal's own row holds, or one of its leading cuts. A person reads the
/// command here before pressing Accept, so it crosses whole.
///
/// **Tagged on `kind`**, so a surface matches one field and a second kind is a
/// new arm rather than a second shape. The models list is not a kind: it is
/// resolved from the harness at start and has no stored tier to change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RetroChange {
    /// Allow this command, in every Job on this machine, in Kit's allowlist.
    AllowCommand { command: String },
}

impl From<&core_model::Change> for RetroChange {
    fn from(change: &core_model::Change) -> RetroChange {
        match change {
            core_model::Change::AllowCommand { command } => RetroChange::AllowCommand {
                command: command.clone(),
            },
        }
    }
}

impl RetroChange {
    pub fn domain(&self) -> core_model::Change {
        match self {
            RetroChange::AllowCommand { command } => core_model::Change::AllowCommand {
                command: command.clone(),
            },
        }
    }
}

/// The change an item's author asks for, before Fleet has held it to the
/// record. **The model names a refusal and never writes a command**: `refusal`
/// is a `cite` on [`RetroRecord::refusals`], and `command` is optional and,
/// where present, must be one of that refusal's leading cuts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RetroChangeAnswered {
    AllowCommand {
        refusal: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
    },
}

/// One item of what the retro call answers with. **Not [`RetroItem`]**: the
/// model names no `id` and no `statement`, and an answer lacking a `title`,
/// `what` or `fix` is not an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroAnswered {
    pub who: Whose,
    /// Absent is an item that is dropped, the way a place spelt wrong is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands_in: Option<LandsIn>,
    pub title: String,
    pub what: String,
    pub fix: String,
    pub evidence: Vec<String>,
    /// A change to Kit, on a `kit` item whose fix is a command a refusal in the
    /// record names. **Read leniently**: one that will not read is left off and
    /// the item stays. Since 23.35.
    #[serde(
        default,
        deserialize_with = "leniently",
        skip_serializing_if = "Option::is_none"
    )]
    pub change: Option<RetroChangeAnswered>,
}

/// A `change` that reads, or `None` for whatever the model wrote there that
/// does not. It costs the change and never the item.
fn leniently<'de, D: Deserializer<'de>>(input: D) -> Result<Option<RetroChangeAnswered>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Change(RetroChangeAnswered),
        Unread(IgnoredAny),
    }
    Ok(match Option::<Written>::deserialize(input)? {
        Some(Written::Change(change)) => Some(change),
        Some(Written::Unread(_)) | None => None,
    })
}

/// What the retro call answers with. Read through [`crate::decode`] and
/// nowhere else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroWritten {
    /// **An item that will not read is left out**, rather than costing the
    /// items beside it: one `lands_in` the model spelled wrong, or one with no
    /// `title`, drops that item, the way one citing nothing the record holds is
    /// dropped.
    #[serde(deserialize_with = "readable")]
    pub items: Vec<RetroAnswered>,
}

/// One item of a model's answer, or whatever it wrote in that place instead.
#[derive(Deserialize)]
#[serde(untagged)]
enum Answered {
    Item(RetroAnswered),
    Unread(IgnoredAny),
}

fn readable<'de, D: Deserializer<'de>>(input: D) -> Result<Vec<RetroAnswered>, D::Error> {
    Ok(Vec::<Answered>::deserialize(input)?
        .into_iter()
        .filter_map(|answered| match answered {
            Answered::Item(item) => Some(item),
            Answered::Unread(_) => None,
        })
        .collect())
}

/// `list_lessons`: retro items across Jobs, newest retro first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lessons {
    pub lessons: Vec<Lesson>,
}

/// One retro item, with the Job it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    /// `RetroItem::id`. Since 23.26.
    pub id: String,
    pub job_id: JobId,
    pub handle: String,
    /// When the retro this item is in was written.
    pub at: Instant,
    pub who: Whose,
    /// As on [`RetroItem`]: absent on an item kept before 23.26.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    pub statement: String,
    /// `cite` values on the Job's own retro record: `get_job_retro` resolves
    /// them.
    pub evidence: Vec<String>,
    /// Where its fix lands. Absent on an item kept before 23.15, which
    /// `?lands_in=` never matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands_in: Option<LandsIn>,
    /// Where it stands with the person. Every item starts `open`. Since 23.26.
    pub state: LessonState,
    /// The Job proposed for it, once `agree_lesson` has proposed one. Since
    /// 23.26.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_proposed: Option<JobId>,
    /// What Accept would change in Kit. **Only on a `kit` item whose change
    /// Fleet copied off a refusal in the record**; absent on every other, and
    /// on one kept before 23.35. Since 23.35.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<RetroChange>,
    /// What `agree_lesson` applied. **Absent when nothing was applied**: an
    /// item with no `change`, one still open, and one disagreed with. Present
    /// is the change, exactly as it was applied, and it stays so on every later
    /// read of the item. Since 23.35.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<RetroChange>,
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
    /// Each file the gate's failure names, and whether the step's Drones
    /// named it in a tool call of their own. Since 23.26, on a gate failure
    /// that names a file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<RecordPath>,
}

/// A file a failed Check names, set against what the Drone did. **A fact read
/// off the transcript, and never a verdict**: `false` says no tool call of the
/// Drone's names the file, and `true` says one does, which may be a read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordPath {
    pub path: String,
    pub named_in_drone_calls: bool,
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
