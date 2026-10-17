//! A Session's retro: what got in the way while it ran, and the record it was
//! read from. `get_session_retro` answers one Session's and `list_lessons`
//! every item across Sessions, newest first. `docs/concepts/retro.md`.
//!
//! **The record is assembled on every read and never stored.** Each row is a
//! fact already on the Session's thread, so a read made later says what it
//! says, and the retro's items cite the rows by `cite` rather than repeating
//! them.

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize};

use crate::enums::{Actor, LandsIn, LessonState, Via, Whose};
use crate::ids::Instant;

/// `get_session_retro`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retro {
    pub session: RetroSession,
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
    /// written retro is a Session nothing got in the way of.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<RetroItem>,
    pub record: RetroRecord,
}

/// The Session a retro or an item was written for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroSession {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Where a Session's retro stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetroState {
    /// The retro is not written yet.
    Pending,
    Written,
    /// The call failed or its answer would not read. Not tried again.
    Failed,
    /// None is owed: nothing happened on the Session since the last retro.
    Skipped,
}

/// One thing that got in the way.
///
/// **`title`, `what` and `fix` are absent on an item kept before 23.26**, which
/// has `statement` alone. On one written since, `statement` repeats `what`, so
/// a reader that predates the three still has a sentence to draw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroItem {
    /// Names this item in `start_lesson_session` and `disagree_lesson`. Stable:
    /// it is the Session's id, the retro's number and the item's place in it.
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
    /// The Session started for it. Since 23.77.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_proposed: Option<String>,
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
    /// Allow this command, in every Session on this machine, in Kit's allowlist.
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

/// `list_lessons`: retro items across Sessions, newest retro first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lessons {
    pub lessons: Vec<Lesson>,
}

/// One retro item, with the Session it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    /// `RetroItem::id`. Since 23.26.
    pub id: String,
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
    /// `cite` values on the Session's own retro record: `get_session_retro`
    /// resolves them.
    pub evidence: Vec<String>,
    /// Where its fix lands. Absent on an item kept before 23.15, which
    /// `?lands_in=` never matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lands_in: Option<LandsIn>,
    /// Where it stands with the person. Every item starts `open`. Since 23.26.
    pub state: LessonState,
    /// The Session `start_lesson_session` started for it, state `agreed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_proposed: Option<String>,
    /// The Session whose retro the item is in; `handle` is its address.
    pub session: RetroSession,
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

/// Everything on a Session's thread a retro is read from. Every row carries a
/// `cite`, `refusal:1` and the like, unique within the record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroRecord {
    /// The gate's denials of a tool call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusals: Vec<RecordRefusal>,
    /// Resumes and reattaches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub restarts: Vec<RecordAct>,
    /// What the Session said got in its way.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<RecordSaid>,
    /// Its asks and how long each waited.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<RecordAsked>,
    /// The owner's messages to the Session, unfiltered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corrections: Vec<RecordSaid>,
    /// Tool results that errored: `tried` is the call, `because` the error.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_tools: Vec<RecordRefusal>,
    /// What each subagent's own transcript shows went wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagents: Vec<RecordSaid>,
}

/// A tool call that was refused or errored, with what it tried.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRefusal {
    pub cite: String,
    pub at: Instant,
    pub tool: String,
    /// The argument, off the call the refusal answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tried: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,
}

/// Something said, with when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSaid {
    pub cite: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
    pub said: String,
}

/// A restart of the Session's process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordAct {
    pub cite: String,
    pub at: Instant,
    pub actor: Actor,
    /// The door it came through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<Via>,
    /// What moved, in a few words.
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
    /// What was asked, and what it was about.
    pub about: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    /// From the ask to the answer. Absent where none came.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waited_ms: Option<u64>,
}
