//! The state a Jobs-list row draws, read by Fleet so a client does not work it out. **States only**:
//! the words a row says beside its mark stay with the client. Carried on `SessionRecord::mark`.

use serde::{Deserialize, Serialize};

use crate::{WaitingActKind, WaitingItem, WaitingSource, READ_ITEM_PREFIX};

/// What a Job is doing, as a row's mark draws it. Bridge's `SessionState`, value for value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Blank,
    Working,
    Running,
    Subagent,
    Stuck,
    Stopped,
    StoppedAsks,
    Waiting,
    /// Built work waits for him to look at; nothing blocks on him. Waiting is the Job that cannot go on without him.
    Ready,
    Replied,
    Done,
    Failing,
    Idle,
    Quiet,
    Ended,
    Limited,
    After,
    Draft,
    Held,
    Paused,
    Unstarted,
}

/// The row's lamp: needs him, ready for him, failed, running, done; anything else is idle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobLamp {
    Needs,
    Ready,
    Failed,
    Running,
    Done,
    Idle,
}

/// The group a list files the row under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobGroup {
    Needs,
    Ready,
    Working,
    Held,
    Done,
}

/// How a Job's row reads: its state, lamp and group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobMark {
    pub state: JobState,
    pub lamp: JobLamp,
    pub group: JobGroup,
}

/// What a waiting item asks of him: **Needs you** (the Job cannot go on without him) or **Ready for you** (built
/// work waits for him to look at it). Read by Fleet for the mark and the calls and by Pocket for a push, so a
/// row, a banner, a sound and a push never disagree. `docs/concepts/session/waiting-on-the-person.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwedKind {
    /// The Job cannot go on without him.
    Needs,
    /// Built work waits for him to look at. It is no call: nothing counts, sounds or pushes for it.
    Ready,
    /// Not owed at all: `done` says the Job finished.
    Neither,
}

/// The kind of one item. **Exhaustive on the source, with no fallback**, so a new `WaitingSource` is not built
/// until it is placed. An `agent` item is read by its act, a `document` by whether it is only to read, and a
/// `replied` item by whether it names a pull request to open.
pub fn owed_kind(item: &WaitingItem) -> OwedKind {
    let act = item.act.as_ref().map(|act| act.kind);
    match item.source {
        WaitingSource::ReadyToTry | WaitingSource::Walk => OwedKind::Ready,
        WaitingSource::Agent if act == Some(WaitingActKind::Walk) => OwedKind::Ready,
        WaitingSource::Agent => OwedKind::Needs,
        WaitingSource::Document if item.id.starts_with(READ_ITEM_PREFIX) => OwedKind::Ready,
        WaitingSource::Document => OwedKind::Needs,
        WaitingSource::Replied if act == Some(WaitingActKind::ApprovePr) => OwedKind::Ready,
        WaitingSource::Replied => OwedKind::Needs,
        WaitingSource::Done => OwedKind::Neither,
        WaitingSource::AskCard
        | WaitingSource::Permission
        | WaitingSource::WindowAsk
        | WaitingSource::Draft
        | WaitingSource::Handoff
        | WaitingSource::Sketch
        | WaitingSource::FirstMessage
        | WaitingSource::Plan
        | WaitingSource::SlotChoice
        | WaitingSource::Delivery
        | WaitingSource::WordsAddress
        | WaitingSource::FleetCall
        | WaitingSource::TaskDone
        | WaitingSource::WatchNotOurs
        | WaitingSource::HeldChild
        | WaitingSource::PreviewOffer
        | WaitingSource::RuleAct
        | WaitingSource::OperatorSuggestion
        | WaitingSource::BrowserAsk
        | WaitingSource::Gate
        | WaitingSource::OperatorUndone
        | WaitingSource::JobConflict
        | WaitingSource::ClashOrder
        | WaitingSource::JobHeld
        | WaitingSource::GoalMet => OwedKind::Needs,
    }
}
