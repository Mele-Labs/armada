//! The state a Jobs-list row draws, read off the record the ledger built and what the thread says
//! beside it. **A port of Bridge's TypeScript, check for check**: `jobStateOf` in
//! `packages/screens/src/session-state.ts`, `bucketOf` in `SessionFrame.tsx`, `sessionMark` and the
//! row in `cockpit/deck/ship.ts`, and `toneOf`, `lampOf` and `groupOf` in `cockpit/deck/order.ts`.
//! The order of the checks is the rule, so a change to one side is a change to the other until
//! Electron reads this too.

use ipc::{AttachmentState, JobGroup, JobLamp, JobMark, JobState, SessionRecord, SessionStanding, SessionTurn, WaitingActKind, WaitingSource};

/// What the thread says that the record does not.
pub(super) struct Thread {
    /// A question that lapsed still stands for him: the ledger's own reading, the one `ThreadHead` draws.
    pub lapsed: bool,
    /// A Check of its own is running.
    pub checks_running: bool,
    /// The thread holds a row.
    pub spoken: bool,
}

/// The row's mark. `on_screen` is whether a client has the Job on screen now.
pub(super) fn mark_of(record: &SessionRecord, thread: &Thread, on_screen: bool, now: Option<i64>) -> JobMark {
    reading_of(record, state_of(record, thread, read_since(record, on_screen)), now)
}

/// `readSince`: he has been in the Job since its last turn ended, or has it on screen now.
fn read_since(record: &SessionRecord, on_screen: bool) -> bool {
    let millis = |at: &ipc::Instant| core_model::Timestamp::from_rfc3339(at.as_str()).epoch_millis();
    let Some(ended) = record.last_turn_at.as_ref().and_then(millis) else {
        return false;
    };
    on_screen || record.viewed_at.as_ref().and_then(millis).is_some_and(|viewed| viewed >= ended)
}

/// `jobStateOf`. `read` is whether he has read the reply: [`read_since`].
fn state_of(record: &SessionRecord, thread: &Thread, read: bool) -> JobState {
    let ended = record.state == ipc::SessionState::Ended;
    let hosted = record.hosted.as_ref().filter(|_| !ended);
    if hosted.is_some_and(|one| one.paused.is_some()) {
        return JobState::Paused;
    }
    if ended {
        return JobState::Ended;
    }
    if quiet(record) {
        return JobState::Quiet;
    }
    if hosted.is_some_and(|one| one.start_failed.is_some()) {
        return JobState::Unstarted;
    }
    // Amber only once Fleet has given up on bringing it back.
    if let Some(stopped) = &record.stopped {
        return if stopped.gave_up { JobState::StoppedAsks } else { JobState::Stopped };
    }
    if record.waits.is_some() {
        return JobState::After;
    }
    if record.proposal.as_ref().is_some_and(|one| one.reading) {
        return JobState::Working;
    }
    if hosted.is_some_and(|one| one.start.is_some()) {
        return JobState::Draft;
    }
    if record.held_until.is_some() {
        return JobState::Limited;
    }
    let asked = hosted.and_then(|one| one.asked.as_ref()).or(record.terminal.as_ref().and_then(|one| one.asked.as_ref()));
    if asked.is_some() {
        return JobState::Waiting;
    }
    let owes: Vec<&ipc::WaitingItem> = record.waiting_for.iter().filter(|one| one.source != WaitingSource::Done).collect();
    let holds_him = |one: &&ipc::WaitingItem| match one.source {
        WaitingSource::SlotChoice => one.creating_since.is_none(),
        WaitingSource::Delivery | WaitingSource::Plan | WaitingSource::TaskDone | WaitingSource::Sketch | WaitingSource::Document => true,
        _ => false,
    };
    if owes.iter().any(holds_him) {
        return JobState::Waiting;
    }
    let standing = record.standing.unwrap_or(if working(record) {
        SessionStanding::Working
    } else if record.done_since.is_some() {
        SessionStanding::Done
    } else {
        SessionStanding::Idle
    });
    if standing == SessionStanding::Stuck {
        return JobState::Stuck;
    }
    if working(record) {
        return JobState::Working;
    }
    if standing == SessionStanding::Working {
        if record.attachments.iter().any(|one| one.kind == "subagent" && one.state == AttachmentState::Standing) {
            return JobState::Subagent;
        }
        let command = record.attachments.iter().any(|one| {
            one.kind == "background"
                && one.state == AttachmentState::Standing
                && matches!(one.detail.get("form").map(String::as_str), Some("command" | "monitor"))
        });
        if command || thread.checks_running {
            // A reply that asks beside a command it left running is his to answer first.
            return if owes.is_empty() { JobState::Running } else { JobState::Waiting };
        }
        return JobState::Working;
    }
    if blank(record, thread) {
        return JobState::Blank;
    }
    if !owes.is_empty() || thread.lapsed {
        return JobState::Waiting;
    }
    if !record.held_open_by.is_empty() {
        return JobState::Held;
    }
    if standing == SessionStanding::Replied && !read {
        return JobState::Replied;
    }
    if standing == SessionStanding::Done {
        return JobState::Done;
    }
    if pulls(record).any(|(merged, failed)| !merged && failed) {
        return JobState::Failing;
    }
    JobState::Idle
}

/// Nothing listens: no host, and no terminal that does.
fn quiet(record: &SessionRecord) -> bool {
    record.hosted.is_none() && !record.terminal.as_ref().is_some_and(|one| one.listening)
}

fn working(record: &SessionRecord) -> bool {
    record.hosted.as_ref().is_some_and(|one| matches!(one.turn, SessionTurn::Working { .. }))
}

/// No slot, no branch, no title, no turn and no row. An attachment counts as Bridge draws one: what is
/// given back is gone unless it is a subagent, an artifact or a background task, and a slot or branch
/// only while it stands. A title of only space is none, and so is a quick terminal's, which Fleet gives
/// it at start: it is not started until he writes.
fn blank(record: &SessionRecord, thread: &Thread) -> bool {
    let drawn = record.attachments.iter().any(|one| match one.kind.as_str() {
        "slot" | "branch" => one.state == AttachmentState::Standing,
        "subagent" | "artifact" | "background" => true,
        _ => one.state != AttachmentState::GivenBack,
    });
    let titled = !record.quick_terminal && record.title.as_deref().is_some_and(|title| !title.trim().is_empty());
    !drawn && !titled && record.last_turn_at.is_none() && !thread.spoken
}

/// Each pull request it holds, in order: whether it merged, and whether its Checks failed.
fn pulls(record: &SessionRecord) -> impl Iterator<Item = (bool, bool)> + '_ {
    record.attachments.iter().filter(|one| one.kind == crate::pull_requesting::PR && one.state != AttachmentState::GivenBack).map(|one| {
        let merged = one.state == AttachmentState::Spent || one.detail.get("state").map(String::as_str) == Some("merged");
        (merged, one.detail.get("checks").map(String::as_str) == Some("failed"))
    })
}

/// `bucketOf`, with only the buckets the row reads.
#[derive(PartialEq)]
enum Bucket {
    Needs,
    Working,
    Other,
}

fn bucket_of(state: JobState) -> Bucket {
    use JobState::*;
    match state {
        Waiting | Done | Unstarted | StoppedAsks | Stuck | Failing => Bucket::Needs,
        Working | Running | Subagent => Bucket::Working,
        _ => Bucket::Other,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Ask,
    Calm,
    /// `sessionMark` never says it, so `failed` and the quiet rule in `tone_of` are not reached for a Job
    /// today. Kept, as Bridge keeps them, for the day a mark does.
    #[allow(dead_code)]
    Bad,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Needs,
    Working,
    Held,
    Done,
}

/// A failure nobody has touched for this many minutes is old news, and drawn quiet.
const QUIET_AFTER_MINUTES: i64 = 15;

/// Whether the Job needs him, as the Cockpit's ship reads it (`phase` is `needs`): its row asks, or a
/// failing pull request of its own alarms on an idle Job. The call it raises is `standing_calls`'.
pub(crate) fn needs_him(record: &SessionRecord, state: JobState) -> bool {
    tone_and_phase(record, state).1 == Phase::Needs
}

/// The row's own tone, and the phase `shipOf` files the Job under.
fn tone_and_phase(record: &SessionRecord, state: JobState) -> (Tone, Phase) {
    let bucket = bucket_of(state);
    let dead = matches!(state, JobState::Ended | JobState::Quiet);
    // A paused Job is calm: nothing is asked of him.
    let own = if state != JobState::Paused && bucket == Bucket::Needs { Tone::Ask } else { Tone::Calm };
    // A walk he has not approved calls on the row as its own question does. A Custom tab's Mod Job that
    // asks does too in Bridge; mods come after the switch to the native app.
    let walk = record.waiting_for.iter().any(|one| one.source == WaitingSource::Walk && one.act.as_ref().is_some_and(|act| act.kind == WaitingActKind::Walk));
    let tone = if own == Tone::Calm && walk && !dead { Tone::Ask } else { own };
    let phase = if dead {
        Phase::Done
    } else {
        let needs = tone == Tone::Ask;
        let stuck = state == JobState::Stuck;
        // Its own pull request: the first that has not merged, else the first.
        let red_pr = pulls(record).find(|(merged, _)| !merged).is_some_and(|(_, failed)| failed);
        let paused = state == JobState::Paused;
        // A paused Job is parked by him: a red pull request on it alarms nothing until he resumes it.
        let alarm = !paused && (tone == Tone::Bad || (red_pr && !working(record)));
        if needs || (alarm && !stuck) {
            Phase::Needs
        } else if stuck || paused || record.waits.is_some() {
            Phase::Held
        } else if bucket == Bucket::Working {
            Phase::Working
        } else {
            Phase::Done
        }
    };
    (tone, phase)
}

/// The row's lamp and group for `state`: `sessionMark`, then the row in `shipOf`, then `toneOf`,
/// `lampOf` and `groupOf`.
fn reading_of(record: &SessionRecord, state: JobState, now: Option<i64>) -> JobMark {
    let bucket = bucket_of(state);
    let (tone, phase) = tone_and_phase(record, state);
    let idle_minutes = record
        .last_turn_at
        .as_ref()
        .and_then(|at| core_model::Timestamp::from_rfc3339(at.as_str()).epoch_millis())
        .zip(now)
        .map(|(at, now)| ((now - at) as f64 / 60_000.0).round().max(0.0) as i64);
    let tone = match tone {
        Tone::Bad if phase == Phase::Held || idle_minutes.unwrap_or(0) >= QUIET_AFTER_MINUTES => Tone::Calm,
        tone => tone,
    };
    let group = match (tone, phase) {
        (Tone::Ask | Tone::Bad, _) => JobGroup::Needs,
        (_, Phase::Needs | Phase::Done) => JobGroup::Done,
        (_, Phase::Working) => JobGroup::Working,
        (_, Phase::Held) => JobGroup::Held,
    };
    let lamp = match (tone, group) {
        (Tone::Ask, _) => JobLamp::Needs,
        (Tone::Bad, _) => JobLamp::Failed,
        (_, JobGroup::Working) => JobLamp::Running,
        (_, JobGroup::Done) => JobLamp::Done,
        _ => JobLamp::Idle,
    };
    JobMark { state, lamp, group }
}
