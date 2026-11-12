//! A Job's thread as a screen draws it: the rows Fleet keeps, folded once for every client.
//!
//! Pure. The input is what `get_session` answers and the Job's timeline entries; the output is the
//! head and the items of the whole thread, which [`window`] cuts to whole items. This is a port of
//! what Bridge works out in TypeScript (`rowsOfThread`, `session-thread-rows`, `thread-cards`,
//! `withChanges`, `useJudgedRows`, the pinned sketch and `itemsOf`), with the rules kept as they are.
//! **No row is worded here**: a settled ask folds into a run of calls as its `Ask` row, where Bridge folds
//! a line of text. What Bridge reads off rows it never draws, or off other Jobs, stays the client's.

use std::collections::{BTreeMap, HashMap, HashSet};

use ipc::{
    Instant, JudgeState, PlanCardState, RawScene, SessionAskState, SessionRecord, SessionRow, SessionThread,
    SessionVoice, ThreadHead, ThreadItem, ThreadPlan, ThreadTrail, TimelineEvent,
};

mod cards;
mod head;

use cards::{file_card_of, form_of, reads, sketch_card_of, with_changes, with_documents, with_sketch_state};
use head::{ask_of, head_of, pinned_of, waiting_plan_of};

const FLEET: &str = "fleet";

/// Calls no screen draws: the agent's own task list and tool loading, and a shown window, whose own row
/// stands for it. Bridge's `UNDRAWN`.
const UNDRAWN: [&str; 6] = ["TaskCreate", "TaskUpdate", "TaskList", "TaskGet", "ToolSearch", "show_window"];

/// The scenes of the revisions Fleet kept, by the path a ledger row names each by. The fold reads none
/// itself: [`revision_paths`] says which it needs.
pub(crate) type Revisions = HashMap<String, RawScene>;

/// The thread of a Job: its head and all of its items, oldest first.
pub(crate) fn folded(thread: &SessionThread, timeline: &[TimelineEvent], revisions: &Revisions) -> (ThreadHead, Vec<ThreadItem>) {
    let record = &thread.session;
    let mut head = head_of(thread);
    head.plan = waiting_plan_of(record);
    head.sketch = pinned_of(record, revisions);
    // While the Job waits on him, the Fleet turn that led to it stays open.
    let asking = head.asked.is_some() || head.plan.is_some() || head.sketch.is_some();
    let items = items_of(thread, timeline, revisions, head.sketch.as_ref().map(|pin| pin.sketch.as_str()), asking);
    (head, items)
}

/// A thread cut at a row (`get_job_thread`'s `upto`), drawn as it stood there: **nothing that stands now is on
/// it.** The head is empty, no ask is open so the last Fleet turn closes as it would, and no sketch is pinned, so
/// a sketch card stays where its row is. An approved plan's card stays if its `plan` row is inside the cut. The
/// trail is empty but for `started`, which a trail cannot go without and which is a fact of the Job, not a state.
pub(crate) fn folded_cut(thread: &SessionThread, timeline: &[TimelineEvent], revisions: &Revisions) -> (ThreadHead, Vec<ThreadItem>) {
    let head = ThreadHead {
        asked: None,
        open_call: None,
        lapsed: None,
        fleet_acts: Vec::new(),
        trail: ThreadTrail { started: thread.session.started_at.clone(), bays: Vec::new(), opened: Vec::new() },
        plan: None,
        sketch: None,
    };
    (head, items_of(thread, timeline, revisions, None, false))
}

/// Every revision path the sketches of a record name, which [`folded`] draws once each is in [`Revisions`].
pub(crate) fn revision_paths(record: &SessionRecord) -> Vec<String> {
    record
        .attachments
        .iter()
        .filter(|one| form_of(one) == Some("sketch"))
        .filter_map(|one| one.detail.get("revisions"))
        .flat_map(|kept| ipc::revision_paths_of_detail(kept))
        .collect()
}

/// Whole items: the newest `limit` before the item that holds `before`, and the id to read the older
/// ones by. `None` is a `before` no item holds.
pub(crate) fn window(
    items: &[ThreadItem],
    before: Option<&str>,
    limit: Option<u32>,
) -> Option<(Vec<ThreadItem>, Option<String>)> {
    let end = match before {
        None => items.len(),
        Some(id) => items.iter().position(|item| holds(item, id))?,
    };
    let most = limit.unwrap_or(ipc::THREAD_ITEMS_SERVED).clamp(1, ipc::THREAD_ITEMS_MOST) as usize;
    let start = end.saturating_sub(most);
    let older = (start > 0).then(|| items[start].id().to_string());
    Some((items[start..end].to_vec(), older))
}

/// Cut the items after the one that holds the row `upto`, which stays. A document, a Judge line or a change
/// is placed by time among the rows, so one written after the cut row stands after it and is not on the old
/// thread, as Electron's `upTo` keeps nothing past the cut. **Where no item holds the row**, because it is
/// one no screen draws, the items stand as they are.
pub(crate) fn cut_after(items: &mut Vec<ThreadItem>, upto: &str) {
    if let Some(at) = items.iter().position(|item| holds(item, upto)) {
        items.truncate(at + 1);
    }
}

/// Whether the item is `id` or folds the row `id`.
fn holds(item: &ThreadItem, id: &str) -> bool {
    item.id() == id
        || match item {
            ThreadItem::Calls { rows, .. } | ThreadItem::Checks { rows, .. } => rows.iter().any(|row| row.id() == id),
            ThreadItem::FleetTurn { notice, rows, .. } => {
                notice.id() == id || rows.iter().any(|row| row.id() == id)
            }
            _ => false,
        }
}

/// What a client holding `before` must do to hold `after`: the first item of `before` that differs and
/// the items from the matching place of `after` to its end. `None` where nothing differs.
pub(crate) fn tail_of(before: &[ThreadItem], after: &[ThreadItem]) -> Option<(Option<String>, Vec<ThreadItem>)> {
    let same = before.iter().zip(after).take_while(|(held, now)| held == now).count();
    if same == before.len() && same == after.len() {
        return None;
    }
    let from = before.get(same).map(|item| item.id().to_string());
    Some((from, after[same..].to_vec()))
}

// ---- the items

/// A row on its way to the fold, and when it was written.
struct Entry<'a> {
    at: Instant,
    /// Whether a row of the thread wrote it, which is what the timeline places a change by.
    from_row: bool,
    piece: Piece<'a>,
}

enum Piece<'a> {
    Row(&'a SessionRow),
    Card(ThreadItem),
}

fn items_of(
    thread: &SessionThread,
    timeline: &[TimelineEvent],
    revisions: &Revisions,
    pinned: Option<&str>,
    asking: bool,
) -> Vec<ThreadItem> {
    let record = &thread.session;
    let ledger = &record.attachments;
    let mut entries = thread
        .rows
        .iter()
        .filter(|row| drawn(row))
        .map(|row| {
            let card = match row {
                SessionRow::Tool { id, at, text, .. } => {
                    sketch_card_of(id, at, text, ledger).or_else(|| file_card_of(id, at, text, ledger))
                }
                _ => None,
            };
            Entry {
                at: row.at().clone(),
                from_row: true,
                piece: card.map_or(Piece::Row(row), Piece::Card),
            }
        })
        .collect::<Vec<_>>();
    with_changes(&mut entries, &record.id, timeline);
    with_sketch_state(&mut entries, ledger, revisions);
    with_documents(&mut entries, ledger);
    if let Some(failed) = record.hosted.as_ref().and_then(|hosted| hosted.start_failed.as_ref()) {
        let at = entries
            .last()
            .map(|entry| entry.at.clone())
            .or_else(|| record.last_turn_at.clone())
            .unwrap_or_else(|| Instant::carried("1970-01-01T00:00:00.000Z"));
        let item = ThreadItem::NotStarted { id: String::from("not-started"), at: at.clone(), cause: failed.cause.clone(), log: failed.log.clone() };
        entries.push(Entry { at, from_row: false, piece: Piece::Card(item) });
    }
    let mut entries = with_judged(entries, record);
    // The pinned sketch is drawn after the thread, so its card is not drawn twice; an earlier card stays as the replaced one.
    entries.retain(|entry| match &entry.piece {
        Piece::Card(ThreadItem::Sketch(card)) => Some(card.sketch.as_str()) != pinned || card.replaced,
        Piece::Row(row) => !undrawn(row),
        Piece::Card(_) => true,
    });
    let mut keep: HashSet<&str> = thread
        .rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Disputed { notice, .. } => Some(notice.as_str()),
            _ => None,
        })
        .collect();
    let mut items = fold(&entries, &keep);
    if let Some(turn) = asking.then(|| last_turn(&items)).flatten() {
        keep.insert(turn);
        items = fold(&entries, &keep);
    }
    with_approved_plan(&mut items, thread);
    items
}

/// Whether a row reaches the thread. A waiting ask is the head's `asked`; a `plan` row is where its card
/// stands; a hand off told to the parent is drawn by the row that started the Job it replaced. A `reasoning`
/// row is held back until the native thread draws it (the owner, 10 Nov 2026): it would end a run of calls
/// with nothing drawn between the two halves.
fn drawn(row: &SessionRow) -> bool {
    !matches!(
        row,
        SessionRow::BrowserReminder { .. }
            | SessionRow::Reasoning { .. }
            | SessionRow::Ask { state: SessionAskState::Waiting, .. }
            | SessionRow::Plan { .. }
            | SessionRow::Delivery { .. }
            | SessionRow::Linked { replaces: Some(_), .. }
    )
}

/// A call of a run: a tool row, an ask settled, or a question asked in the walk window.
fn is_call(row: &SessionRow) -> bool {
    matches!(row, SessionRow::Tool { .. } | SessionRow::Ask { .. } | SessionRow::WindowAsk { .. })
}

/// A call no screen draws, by the first word Bridge reads it by: a tool's name, or a settled ask's command.
fn undrawn(row: &SessionRow) -> bool {
    let text = match row {
        SessionRow::Tool { text, .. } => text.clone(),
        SessionRow::Ask { ask, .. } => format!("{}: ", ask_of(ask).command),
        _ => return false,
    };
    let word = text.split(char::is_whitespace).next().unwrap_or("");
    UNDRAWN.contains(&word.rsplit("__").next().unwrap_or(word))
}

/// The Fleet turn the thread ends in: the last item, or the one before the agent's words it lifted out.
fn last_turn(items: &[ThreadItem]) -> Option<&str> {
    match items {
        [.., ThreadItem::FleetTurn { id, .. }] => Some(id),
        [.., ThreadItem::FleetTurn { id, .. }, ThreadItem::Row { row: SessionRow::Message { from: SessionVoice::Agent, .. } }] => {
            Some(id)
        }
        _ => None,
    }
}

/// A turn Fleet started folds into one item: its notice, then the agent's replies and calls until the next
/// row of any other kind. The person's message is theirs, not Fleet's turn, so it ends the fold. A notice in
/// `keep` is a row as it came. A run of calls, or of one Check, folds into one; a card ends either.
fn fold(entries: &[Entry<'_>], keep: &HashSet<&str>) -> Vec<ThreadItem> {
    let mut items: Vec<ThreadItem> = Vec::new();
    let mut entries = entries.iter().peekable();
    while let Some(entry) = entries.next() {
        let row = match &entry.piece {
            Piece::Card(card) => {
                items.push(card.clone());
                continue;
            }
            Piece::Row(row) => *row,
        };
        if is_fleets(row) && !keep.contains(row.id()) {
            let mut reply: Vec<&SessionRow> = Vec::new();
            while let Some(Entry { piece: Piece::Row(next), .. }) = entries.peek() {
                if is_call(next) || matches!(next, SessionRow::Message { from: SessionVoice::Agent, .. }) {
                    reply.push(next);
                    entries.next();
                } else {
                    break;
                }
            }
            // A turn ends in words meant for the person: its last agent message is drawn outside the fold.
            let spoken = reply.iter().rposition(|one| matches!(one, SessionRow::Message { .. }));
            let handled = !reply.is_empty();
            let said = spoken.map(|at| reply.remove(at));
            items.push(ThreadItem::FleetTurn {
                id: row.id().to_string(),
                notice: row.clone(),
                rows: reply.into_iter().cloned().collect(),
                handled,
            });
            if let Some(said) = said {
                items.push(ThreadItem::Row { row: said.clone() });
            }
            continue;
        }
        match row {
            SessionRow::Check { id, name, .. } => match items.last_mut() {
                Some(ThreadItem::Checks { rows, .. })
                    if matches!(rows.first(), Some(SessionRow::Check { name: first, .. }) if first == name) =>
                {
                    rows.push(row.clone());
                }
                _ => items.push(ThreadItem::Checks { id: id.clone(), rows: vec![row.clone()] }),
            },
            SessionRow::Tool { id, at, subagent: Some(subagent), .. } => {
                items.push(ThreadItem::Subagent { id: id.clone(), at: at.clone(), subagent: subagent.clone() });
            }
            call if is_call(call) => match items.last_mut() {
                Some(ThreadItem::Calls { rows, .. }) => rows.push(row.clone()),
                _ => items.push(ThreadItem::Calls { id: row.id().to_string(), names: Vec::new(), rows: vec![row.clone()] }),
            },
            other => items.push(ThreadItem::Row { row: other.clone() }),
        }
    }
    for item in &mut items {
        if let ThreadItem::Calls { names, rows, .. } = item {
            *names = names_of(rows);
        }
    }
    items
}

/// A notice Fleet wrote: a message from the session named `fleet`.
fn is_fleets(row: &SessionRow) -> bool {
    matches!(row, SessionRow::Message { from: SessionVoice::Session { id, .. }, .. } if id == FLEET)
}

/// The tools a run holds, each named once, in the order they were first called. A settled ask names the
/// tool it asked about; a question in the walk window names none.
fn names_of(rows: &[SessionRow]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for row in rows {
        let name = match row {
            SessionRow::Tool { text, .. } => text.split(char::is_whitespace).next().unwrap_or(""),
            SessionRow::Ask { ask, .. } => ask.tool.as_str(),
            _ => continue,
        };
        if !name.is_empty() && !names.iter().any(|held| held == name) {
            names.push(name.to_string());
        }
    }
    names
}

/// A Judge's finished reading, after the last row written by then: before them all where it finished first,
/// and after the last where no row's time reads. Several after one row keep the order they were asked in.
fn with_judged<'a>(entries: Vec<Entry<'a>>, record: &SessionRecord) -> Vec<Entry<'a>> {
    let done = record.judge_readings.iter().filter(|one| one.state == JudgeState::Done);
    let mut after: BTreeMap<Option<usize>, Vec<Entry<'a>>> = BTreeMap::new();
    for one in done {
        let Some(finished) = one.finished_at.as_ref() else { continue };
        let timed = entries.iter().enumerate().filter(|(_, entry)| reads(&entry.at));
        let place = match timed.clone().next() {
            None => entries.len().checked_sub(1),
            Some(_) => timed.filter(|(_, entry)| entry.at <= *finished).map(|(at, _)| at).last(),
        };
        let item = ThreadItem::Judged {
            id: format!("judged:{}", one.id),
            at: finished.clone(),
            artifact: one.artifact.clone(),
            clean: one.notes.is_empty(),
        };
        after.entry(place).or_default().push(Entry { at: finished.clone(), from_row: false, piece: Piece::Card(item) });
    }
    if after.is_empty() {
        return entries;
    }
    let mut out = after.remove(&None).unwrap_or_default();
    for (at, entry) in entries.into_iter().enumerate() {
        out.push(entry);
        out.extend(after.remove(&Some(at)).unwrap_or_default());
    }
    out
}

/// The approved plan, after the item of the last row drawn before its `plan` row, where that item is a row
/// as it came. **Elsewhere it is not drawn**, as Bridge draws it nowhere.
fn with_approved_plan(items: &mut Vec<ThreadItem>, thread: &SessionThread) {
    let Some(plan) = thread.session.plan.as_ref().filter(|plan| plan.approved) else { return };
    let rows = &thread.rows;
    let Some(at) = rows.iter().position(|row| matches!(row, SessionRow::Plan { version, .. } if *version == plan.version)) else {
        return;
    };
    // A `browser_reminder` counts among what Bridge reads this by, though it is drawn nowhere.
    let after = rows[..at].iter().rev().find(|row| drawn(row) || matches!(row, SessionRow::BrowserReminder { .. }));
    let Some(after) = after.map(SessionRow::id) else { return };
    let one_row = |item: &ThreadItem| {
        !matches!(item, ThreadItem::Calls { .. } | ThreadItem::Checks { .. } | ThreadItem::FleetTurn { .. } | ThreadItem::Subagent { .. })
    };
    if let Some(found) = items.iter().position(|item| item.id() == after && one_row(item)) {
        let card = ThreadPlan { id: format!("plan:v{}", plan.version), version: plan.version, state: PlanCardState::Approved };
        items.insert(found + 1, ThreadItem::Plan(card));
    }
}
