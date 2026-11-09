//! The family of Jobs the Started Jobs claim is read against: a parent that started four Jobs, one
//! of those Jobs' own child, and another parent's Job. None of it reaches a store, a process or a
//! network; each Job is the record Fleet would have served for it.

use std::collections::BTreeMap;

use ipc::{
    Attachment, AttachmentState, HostedFacts, Instant, ManifestId, SessionId, SessionMode, SessionOrigin, SessionRecord, SessionState,
    SessionTurn, SessionUsage, SessionWait, StartBase, StartCondition, WaitBy, WaitingItem, WaitingSource,
};

pub const PARENT: &str = "01JOBGROOMING";
pub const OTHER_PARENT: &str = "01JOBSOMEONEELSE";

/// Started by the parent, and held on its start card.
pub const HELD: &str = "01JOBRAILONBEADS";
/// Started by the parent, chained to `RUNNING` with `pr_lands`.
pub const CHAINED: &str = "01JOBCHAINED";
/// Started by the parent, pressed, and building.
pub const RUNNING: &str = "01JOBBUILDING";
/// Started by the parent, ended, with something to say for itself.
pub const ENDED: &str = "01JOBLANDED";
/// Started by `RUNNING`, so the parent's grandchild.
pub const GRANDCHILD: &str = "01JOBGRANDCHILD";
/// Started by `OTHER_PARENT`.
pub const ANOTHERS: &str = "01JOBANOTHERS";

pub const SUMMARY: &str = "Moved the rail's reads onto beads; the pull request landed";

fn at(minute: u32) -> Instant {
    Instant::carried(format!("2026-11-04T10:{minute:02}:00.000Z"))
}

fn attachment(kind: &str, target: &str, state: AttachmentState, minute: u32, detail: &[(&str, &str)]) -> Attachment {
    Attachment {
        kind: kind.to_string(),
        manifest_id: Some(ManifestId::carried("armada")),
        target: target.to_string(),
        state,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect::<BTreeMap<_, _>>(),
        since: at(minute),
        changed_at: at(minute),
    }
}

/// A live hosted Job with nothing on it yet.
pub fn job(id: &str, title: &str, attachments: Vec<Attachment>) -> SessionRecord {
    SessionRecord {
        id: SessionId::carried(id),
        harness: "a-harness".to_string(),
        origin: SessionOrigin::Bridge,
        manifest_id: Some(ManifestId::carried("armada")),
        cwd: "/work/armada".to_string(),
        title: Some(title.to_string()),
        state: SessionState::Live,
        started_at: at(0),
        last_seen_at: at(30),
        last_turn_at: None,
        ended_at: None,
        end_reason: None,
        usage: SessionUsage::default(),
        attachments,
        hosted: Some(HostedFacts {
            turn: SessionTurn::Idle,
            asked: None,
            model: None,
            resolved_model: None,
            effort: None,
            mode: SessionMode::Auto,
            running: true,
            commands: Vec::new(),
            paused: None,
            start: None,
            start_failed: None,
        }),
        terminal: None,
        mod_out_of_date: false,
        mod_version: None,
        tools_missing: None,
        running: Vec::new(),
        waiting_for: Vec::new(),
        done_since: None,
        done_summary: None,
        so_far: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        on_your_words: false,
        r#mod: false,
        from_job: None,
        from_tab: None,
        mod_of_bay: false,
        auto_land: false,
        activity: None,
        build: None,
        workflow: None,
        proposal: None,
        plan: None,
        touches: Vec::new(),
        workflow_step: None,
        judge_readings: Vec::new(),
        closed: Vec::new(),
        waits: None,
        later: Vec::new(),
        held_open_by: Vec::new(),
        stopped: None,
        stop_told_by: None,
        turn_since: None,
        last_heard_at: None,
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
        in_flight: Vec::new(),
        mark: None,
        viewed_at: None,
    }
}

fn item(id: &str, text: &str, source: WaitingSource) -> WaitingItem {
    WaitingItem {
        id: id.to_string(),
        text: text.to_string(),
        since: at(10),
        source,
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
    }
}

/// The parent: it started `HELD`, `CHAINED`, `RUNNING` and `ENDED`, in that order, each link spent
/// and a fact that stays. It also forked a conversation, which is not a child.
pub fn parent() -> SessionRecord {
    let started = |target: &str, minute: u32| attachment("started_to", target, AttachmentState::Spent, minute, &[]);
    job(
        PARENT,
        "Groom the backlog",
        vec![
            started(HELD, 1),
            started(CHAINED, 2),
            started(RUNNING, 3),
            started(ENDED, 4),
            attachment("forked_to", "01JOBAFORK", AttachmentState::Spent, 5, &[]),
            attachment("branch", "armada/groom", AttachmentState::Standing, 0, &[]),
        ],
    )
}

pub fn held() -> SessionRecord {
    let mut record = job(HELD, "Move the rail onto beads", Vec::new());
    record.waiting_for = vec![item(&format!("unsent:{HELD}"), "First message", WaitingSource::FirstMessage)];
    record.hosted.as_mut().expect("hosted").start =
        Some(ipc::StartHeld { asks_delivery: false, picked: None, message: Some("Move the rail's reads onto beads.".into()), queued: false });
    record
}

pub fn chained() -> SessionRecord {
    let mut record = job(CHAINED, "Read the rail from beads", Vec::new());
    record.waits = Some(SessionWait {
        on: ipc::SessionLink { id: SessionId::carried(RUNNING), title: "Build the rail reader".into() },
        condition: StartCondition::PrLands,
        base: StartBase::Main,
        needs: String::new(),
        brief: "Read the rail from beads once the reader lands".into(),
        pull: None,
        part_title: None,
        by: WaitBy::Agent,
        held: true,
        workflow: None,
        model: None,
        effort: None,
    });
    record
}

/// Building, on the second step of its workflow, with a pull request open and a question out.
pub fn running() -> SessionRecord {
    let mut record = job(
        RUNNING,
        "Build the rail reader",
        vec![
            attachment("slot", "3", AttachmentState::Standing, 6, &[]),
            attachment("branch", "armada/rail-reader", AttachmentState::Standing, 6, &[]),
            attachment("pr", "41", AttachmentState::Standing, 7, &[("title", "Read the rail from beads"), ("state", "open")]),
            attachment("started_to", GRANDCHILD, AttachmentState::Spent, 8, &[]),
        ],
    );
    record.workflow = Some("feature".into());
    record.workflow_step = Some(1);
    record.workflow_waiting_on = vec!["Waiting on PR #41: Approved".into()];
    record.waiting_for = vec![item("which-store", "Which store does the reader open?", WaitingSource::Agent)];
    record
}

pub fn ended() -> SessionRecord {
    let mut record = job(ENDED, "Close the old rail", Vec::new());
    record.state = SessionState::Ended;
    record.ended_at = Some(at(20));
    record.end_reason = Some("closed".into());
    record
}

pub fn grandchild() -> SessionRecord {
    job(GRANDCHILD, "Open the store", Vec::new())
}

pub fn anothers() -> SessionRecord {
    job(ANOTHERS, "Someone else's Job", Vec::new())
}
