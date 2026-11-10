//! What stands over a thread rather than in it: the ask, the open call, the lapsed questions, Fleet's acts,
//! the trail, and the plan and sketch drawn after the last item while they wait on the person.

use std::collections::{BTreeMap, HashSet, VecDeque};

use ipc::{
    AttachmentState, CallInFlight, FleetAct, FleetActKind, FleetActState, Instant, Lapsed, OpenCall, PlanCardState,
    SessionAskState, SessionRecord, SessionRow, SessionState, SessionThread, SessionVoice, ThreadAsk, ThreadHead,
    ThreadPlan, ThreadSketch, ThreadTrail, TrailBay, TrailOpened, WaitingSource,
};

use super::cards::{form_of, revisions_of};
use super::Revisions;

/// What a person says in place of an answer they did not wait for; a question it answers is no longer lapsed.
const LAPSE_ANSWER: &str = "Answering your earlier question";

/// The plan put to him, or set aside, which is drawn after the thread. **A pending plan is drawn only
/// while Fleet puts it to him**: recorded and held, it is not there yet.
pub(super) fn waiting_plan_of(record: &SessionRecord) -> Option<ThreadPlan> {
    let plan = record.plan.as_ref().filter(|plan| !plan.approved)?;
    let asked = record.waiting_for.iter().any(|one| one.source == WaitingSource::Plan);
    let state = match (plan.set_aside_at.is_some(), asked) {
        (true, _) => PlanCardState::SetAside,
        (false, true) => PlanCardState::Waiting,
        (false, false) => return None,
    };
    Some(ThreadPlan { id: format!("plan:v{}", plan.version), version: plan.version, state })
}

/// The newest sketch put to him and neither answered nor taken back, as the ledger kept it: what is drawn
/// after the plan while it waits. Fleet's own waiting item names which, so the two never disagree.
pub(super) fn pinned_of(record: &SessionRecord, revisions: &Revisions) -> Option<ThreadSketch> {
    let item = crate::session_host::sketching::sketch_items(&record.attachments, record.id.as_str()).next()?;
    let held = record.attachments.iter().find(|one| form_of(one) == Some("sketch") && one.target == item.id)?;
    // Fleet refuses a sketch whose scene will not draw, so a kept one always reads.
    let drawing = held.detail.get("scene").and_then(|scene| ipc::scene_of_detail(scene))?;
    let earlier = held.detail.get("against").and_then(|against| ipc::against_of_detail(against));
    Some(ThreadSketch {
        id: held.target.clone(),
        at: held.changed_at.clone(),
        sketch: held.target.clone(),
        title: held.detail.get("title").filter(|title| !title.is_empty()).cloned().unwrap_or(item.text),
        drawing,
        against: earlier.as_ref().map(|(title, _)| title.clone()),
        against_drawing: earlier.map(|(_, scene)| scene),
        replaced: false,
        asked: true,
        withdrawn: false,
        settled: None,
        revisions: revisions_of(held, revisions),
    })
}


pub(super) fn head_of(thread: &SessionThread) -> ThreadHead {
    let record = &thread.session;
    let rows = &thread.rows;
    let asked = record
        .hosted
        .as_ref()
        .and_then(|hosted| hosted.asked.as_ref())
        .or_else(|| record.terminal.as_ref().and_then(|terminal| terminal.asked.as_ref()))
        .map(ask_of);
    let open_call = match (&record.turn_since, rows.last()) {
        (Some(_), Some(SessionRow::Tool { text, at, .. })) => Some(OpenCall { text: text.clone(), since: at.clone() }),
        _ => None,
    };
    ThreadHead {
        asked,
        open_call,
        lapsed: if record.state == SessionState::Ended { None } else { lapsed_of(rows) },
        fleet_acts: fleet_acts_of(rows, record),
        trail: trail_of(record),
        plan: None,
        sketch: None,
    }
}

/// The call the Job waits on, with the command a person reads: the argument whole where `detail` was cut.
pub(super) fn ask_of(ask: &CallInFlight) -> ThreadAsk {
    let command = match (&ask.whole, ask.detail.is_empty()) {
        (Some(whole), _) => whole.clone(),
        (None, false) => ask.detail.clone(),
        (None, true) => ask.tool.clone(),
    };
    ThreadAsk { command, call: ask.clone() }
}

/// The questions of every ask that nobody answered in time, that was refused, or that was kept (because he
/// was walking a page, or nobody answered inside the hold), oldest first, under the newest one's id.
/// **A kept one outlasts what he says**: only a message that answers it clears it. An unanswered or refused
/// one is cleared by anything he says after it, and a question asked again later is left to the later ask.
pub(super) fn lapsed_of(rows: &[SessionRow]) -> Option<Lapsed> {
    let mut said = false;
    let mut later: HashSet<&str> = HashSet::new();
    let mut id: Option<&str> = None;
    let mut lapsed = VecDeque::new();
    for row in rows.iter().rev() {
        match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => {
                if text.starts_with(LAPSE_ANSWER) {
                    break;
                }
                said = true;
            }
            SessionRow::Ask { id: row_id, ask, state, .. } if !ask.questions.is_empty() => {
                let kept = *state == SessionAskState::Kept
                    || (!said && matches!(state, SessionAskState::Unanswered | SessionAskState::Refused));
                for asked in &ask.questions {
                    if kept && !later.contains(asked.question.as_str()) {
                        lapsed.push_front(asked.clone());
                        id = id.or(Some(row_id.as_str()));
                    }
                    later.insert(&asked.question);
                }
            }
            _ => {}
        }
    }
    Some(Lapsed { id: id?.to_string(), questions: lapsed.into() })
}

/// The kind of act a Fleet word is, by the notice it was sent for. Fleet's background model reads write
/// no row, so they are none of these.
pub(super) fn fleet_act_kind_of(notice: Option<&str>) -> FleetActKind {
    match notice {
        Some("stop_nudge" | "rail_nudge") => FleetActKind::Nudge,
        Some("rail_step") => FleetActKind::Step,
        Some("slot_free" | "slot_kept" | "slot_stranded") => FleetActKind::Slot,
        Some("limit_wake" | "sign_in_wake") => FleetActKind::Restart,
        _ => FleetActKind::Told,
    }
}

/// Everything Fleet did, does or will do for a Job, read from what is kept and never from the agent: each
/// word and note of its own in the thread is over, the nudge coming and the repair running are read off
/// the record. **The two sentences of a nudge and a repair are Fleet's own words.**
pub(super) fn fleet_acts_of(rows: &[SessionRow], record: &ipc::SessionRecord) -> Vec<FleetAct> {
    let mut acts: Vec<FleetAct> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Message { id, at, from: SessionVoice::Session { title, .. }, text, notice, .. }
                if title == "Fleet" =>
            {
                let line = text.lines().next().unwrap_or("");
                Some(FleetAct {
                    id: id.clone(),
                    at: at.clone(),
                    state: FleetActState::Over,
                    kind: fleet_act_kind_of(notice.as_deref()),
                    said: line.strip_prefix("Fleet: ").unwrap_or(line).to_string(),
                    why: None,
                    until: None,
                })
            }
            _ => None,
        })
        .collect();
    if let Some((start, words)) = repairing_of(rows) {
        acts.retain(|act| act.id != start.0);
        acts.push(FleetAct {
            id: String::from("repair"),
            at: start.1,
            state: FleetActState::UnderWay,
            kind: FleetActKind::Restart,
            said: format!("Repairing: {words}"),
            why: None,
            until: None,
        });
    }
    if let Some(told_by) = &record.stop_told_by {
        acts.push(FleetAct {
            id: String::from("stop-nudge"),
            at: told_by.clone(),
            state: FleetActState::Coming,
            kind: FleetActKind::Nudge,
            said: String::from("Tell it to carry on or name what it waits on"),
            why: Some(String::from("Its turn ended with nothing running and nothing asked")),
            until: Some(told_by.clone()),
        });
    }
    acts
}

/// The repair under way, from Fleet's own notes: the latest start note with no end note after it, as
/// the id and time of that note and what broke in its words. **Read from the notes and not from the
/// record**, which carries no repair of any kind but a dead Job's.
fn repairing_of(rows: &[SessionRow]) -> Option<((String, Instant), String)> {
    rows.iter().rev().find_map(|row| match row {
        SessionRow::Message { id, at, from: SessionVoice::Session { title, .. }, text, .. } if title == "Fleet" => {
            let line = text.lines().next().unwrap_or("");
            if line.starts_with(crate::repair::REPAIRED) || line.starts_with(crate::repair::NOT_REPAIRED) {
                Some(None)
            } else {
                line.strip_prefix(crate::repair::STARTED).map(|words| Some(((id.clone(), at.clone()), words.to_string())))
            }
        }
        _ => None,
    })?
}

/// Where the work has been, off the ledger rows including the ones the ledger drops: a slot no longer
/// standing is a stay that ended (`changed_at`), and a pull request let go was still opened at its `since`.
pub(super) fn trail_of(record: &ipc::SessionRecord) -> ThreadTrail {
    let mut bays = record
        .attachments
        .iter()
        .filter(|one| one.kind == "slot")
        .filter_map(|one| {
            Some(TrailBay {
                slot: one.target.parse().ok()?,
                from: one.since.clone(),
                until: (one.state != AttachmentState::Standing).then(|| one.changed_at.clone()),
            })
        })
        .collect::<Vec<_>>();
    bays.sort_by(|a, b| a.from.cmp(&b.from));
    let mut opened: BTreeMap<u64, Instant> = BTreeMap::new();
    for one in record.attachments.iter().filter(|one| one.kind == "pr") {
        if let Ok(number) = one.target.parse() {
            opened.insert(number, one.since.clone());
        }
    }
    ThreadTrail {
        started: record.started_at.clone(),
        bays,
        opened: opened.into_iter().map(|(number, at)| TrailOpened { number, at }).collect(),
    }
}

