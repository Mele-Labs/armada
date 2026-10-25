//! The claim, written before the code: **a Session can be dispatched to wait on another, and Fleet starts
//! it when its condition is met, from what it observes.** It holds its brief and no process; the Session
//! it waits on is told what it needs and may change the condition; each change is a row in both threads.
//! `docs/concepts/session.md`, *A Session that waits on another*. The stand-in process is `session_host`'s.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    AttachmentNamed, AttachmentReport, AttachmentState, ChangeWaiting, CloseSession, SessionFact, SessionId, SessionRecord, SessionRow,
    SessionVoice, StartAfter, StartAnotherSession, StartBase, StartCondition, StartSession, StartWaitingNow, WaitBy, WaiterEvent,
    WaiterSide,
};
use store::KeptWait;
use testkit::FakeJudge;

use super::agent_starts::{begins, caller, kind, two_groups, Rig};
use crate::after_following::{judged, Verdict};

pub(super) const BRIEF: &str = "after @The agent lands its PR, let the Field expand into the overlay";

pub(super) fn brief_after(rig: &Rig, on: &SessionId, condition: StartCondition, base: StartBase) -> StartSession {
    StartSession {
        manifest_id: rig.manifest.clone(),
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: Some(BRIEF.into()),
        after: Some(StartAfter { session: on.clone(), condition, base, needs: "to name its pull request".into() }),
        held: false,
    }
}

impl Rig {
    /// The Session waited on, with the Fleet reading what waits on it.
    pub(super) async fn a_waited_on_agent(&self) -> SessionId {
        crate::after_following::keep_the_waiting(Arc::clone(&self.fleet));
        self.an_agent().await
    }

    pub(super) async fn waits(&self, on: &SessionId, condition: StartCondition, base: StartBase) -> SessionRecord {
        Arc::clone(&self.fleet).start_session_reading(brief_after(self, on, condition, base)).await.expect("made to wait")
    }

    /// The first thing the record satisfies, within five seconds.
    pub(super) async fn eventually<F: Fn(&SessionRecord) -> bool>(&self, id: &SessionId, what: &str, held: F) -> SessionRecord {
        for _ in 0..500 {
            let record = self.record(id).await;
            if held(&record) {
                return record;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("{what} did not hold within five seconds");
    }

    pub(super) async fn thread_has(&self, id: &SessionId, what: &str, held: impl Fn(&SessionRow) -> bool) -> SessionRow {
        for _ in 0..500 {
            if let Some(row) = self.rows(id).await.into_iter().find(|one| held(one)) {
                return row;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("{what} was not in the thread within five seconds");
    }

    async fn first_message(&self, id: &SessionId) -> (SessionVoice, String) {
        let row = self.thread_has(id, "a first message", |row| matches!(row, SessionRow::Message { .. })).await;
        match row {
            SessionRow::Message { from, text, .. } => (from, text),
            _ => unreachable!(),
        }
    }

    fn processes(&self) -> usize {
        self.stand_in.starts().len()
    }

    /// Processes started, once there are `count`.
    async fn processes_reach(&self, count: usize) {
        for _ in 0..500 {
            if self.processes() >= count {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("{count} processes did not start within five seconds; {} did", self.processes());
    }
}

fn opened(number: &str, branch: &str) -> SessionFact {
    SessionFact::Attached {
        attachment: AttachmentReport {
            kind: "pr".into(),
            target: number.into(),
            detail: [("branch".to_string(), branch.to_string())].into(),
        },
    }
}

fn landed(number: &str) -> SessionFact {
    SessionFact::Settled { attachment: AttachmentNamed { kind: "pr".into(), target: number.into() }, state: AttachmentState::Spent }
}

// ---- The wait is a fact on a Session that runs nothing ----

#[tokio::test]
async fn a_session_dispatched_to_wait_holds_its_brief_and_runs_nothing() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let before = rig.processes();

    let waiting = rig.waits(&agent, StartCondition::PrLands, StartBase::Main).await;

    let wait = waiting.waits.expect("the record carries the wait");
    assert_eq!((wait.on.id.as_str(), wait.on.title.as_str()), (agent.as_str(), "The agent"));
    assert_eq!((wait.condition, wait.base, wait.by), (StartCondition::PrLands, StartBase::Main, WaitBy::You));
    assert_eq!(wait.brief, BRIEF);
    assert_eq!(wait.needs, "to name its pull request");
    assert_eq!(rig.processes(), before, "no process starts for a Session that waits");
    assert!(rig.rows(&waiting.id).await.is_empty(), "nothing is sent to it");
    assert!(waiting.attachments.iter().all(|one| one.kind != "slot"), "and it holds no slot");
}

#[tokio::test]
async fn the_session_waited_on_is_told_what_the_new_one_needs() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;

    let waiting = rig.waits(&agent, StartCondition::PrLands, StartBase::Main).await;

    let row = rig
        .thread_has(&agent, "the row that tells it", |row| matches!(row, SessionRow::Waiter { event: WaiterEvent::Waits, .. }))
        .await;
    let SessionRow::Waiter { session, side, says, .. } = row else { unreachable!() };
    assert_eq!((session.id, side), (waiting.id.clone(), WaiterSide::Waited));
    assert_eq!(says.as_deref(), Some("to name its pull request"));
    for _ in 0..500 {
        if rig.told("now waits on this Job") == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(rig.told("now waits on this Job"), 1, "its agent is sent it, once");
    assert_eq!(rig.told("It needs: to name its pull request."), 1);
    assert_eq!(rig.told("change_waiting"), 1, "and told it may change the start");
}

// ---- Each condition is met from what Fleet observes ----

#[tokio::test]
async fn ends_starts_it_with_the_handoff_then_the_brief() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;
    rig.thread_has(&agent, "the row that tells it", |row| matches!(row, SessionRow::Waiter { .. })).await;
    assert!(rig.record(&waiting.id).await.waits.is_some(), "still waiting while the other runs");

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: agent.clone() }).await.expect("closed");

    let started = rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;
    assert!(started.hosted.is_some());
    let (from, text) = rig.first_message(&waiting.id).await;
    assert!(matches!(from, SessionVoice::Session { ref id, ref title } if id == agent.as_str() && title == "The agent"), "{from:?}");
    assert!(text.starts_with("Handoff from The agent"), "the handoff first: {text}");
    assert!(text.contains("The agent ended"), "and why it starts: {text}");
    assert!(text.contains("You start from main"), "{text}");
    assert!(text.trim_end().ends_with(BRIEF), "then the brief: {text}");
    rig.processes_reach(2).await;
    rig.thread_has(&agent, "the row that says it started", |row| matches!(row, SessionRow::Waiter { event: WaiterEvent::Started, .. })).await;
}

#[tokio::test]
async fn a_condition_already_met_starts_it_at_once() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    rig.report(&agent, opened("2412", "armada/cockpit-map")).await;

    let waiting = rig.waits(&agent, StartCondition::PrPushed, StartBase::Branch).await;

    rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;
    rig.processes_reach(2).await;
}

#[tokio::test]
async fn a_session_that_has_ended_cannot_be_waited_on() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: agent.clone() }).await.expect("closed");

    let refused = Arc::clone(&rig.fleet).start_session_reading(brief_after(&rig, &agent, StartCondition::Ends, StartBase::Main)).await;

    assert!(format!("{refused:?}").contains("fleet.session_closed"), "{refused:?}");
}

#[tokio::test]
async fn pushed_starts_it_on_the_pull_requests_branch() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::PrPushed, StartBase::Branch).await;
    assert!(rig.record(&waiting.id).await.waits.is_some());

    rig.report(&agent, opened("2412", "armada/cockpit-map")).await;

    rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;
    let (_, text) = rig.first_message(&waiting.id).await;
    assert!(text.contains("pull request #2412 is pushed"), "{text}");
    assert!(text.contains("You start from the branch armada/cockpit-map (pull request #2412)"), "{text}");
}

#[tokio::test]
async fn lands_waits_for_the_merge_and_not_for_the_push() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::PrLands, StartBase::Main).await;

    rig.report(&agent, opened("2412", "armada/cockpit-map")).await;
    let named = rig.eventually(&waiting.id, "the pull request named", |one| one.waits.as_ref().is_some_and(|wait| wait.pull.is_some())).await;
    let pull = named.waits.unwrap().pull.unwrap();
    assert_eq!((pull.number, pull.branch.as_str()), (2412, "armada/cockpit-map"));
    assert_eq!(rig.processes(), 1, "pushed is not landed");

    rig.report(&agent, landed("2412")).await;

    rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;
    let (_, text) = rig.first_message(&waiting.id).await;
    assert!(text.contains("pull request #2412 landed"), "{text}");
    assert!(text.contains("You start from main, after the merge"), "{text}");
}

#[tokio::test]
async fn it_starts_once_however_often_the_other_moves() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::PrPushed, StartBase::Main).await;
    rig.report(&agent, opened("2412", "armada/a")).await;
    rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;

    rig.report(&agent, opened("2413", "armada/b")).await;
    rig.report(&agent, landed("2412")).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    rig.processes_reach(2).await;
}

#[tokio::test]
async fn a_session_closed_while_it_waits_never_starts() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: waiting.id.clone() }).await.expect("closed");
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: agent.clone() }).await.expect("closed");
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(rig.processes(), 1);
}

// ---- The condition read against a record, which is all a plan part and a pull request are ----

fn wait(condition: &str, part: &str) -> KeptWait {
    KeptWait {
        session_id: "w".into(),
        on_session: "o".into(),
        condition: condition.into(),
        part: part.into(),
        base: "main".into(),
        needs: String::new(),
        brief: "b".into(),
        held: false,
        told: true,
        by: "you".into(),
        pull_number: None,
        pull_branch: None,
        created_at: "2026-10-16T10:00:00.000Z".into(),
        started_at: None,
    }
}

fn attachment(target: &str, state: AttachmentState, at: &str, branch: &str) -> ipc::Attachment {
    ipc::Attachment {
        kind: "pr".into(),
        manifest_id: None,
        target: target.into(),
        state,
        detail: [("branch".to_string(), branch.to_string())].into(),
        since: ipc::Instant::carried(at),
        changed_at: ipc::Instant::carried(at),
    }
}

/// A Session record with a plan whose first group's tasks are in the states given.
async fn record_with_plan(states: &[ipc::PlanTaskState]) -> SessionRecord {
    let rig = super::agent_starts::rig(None);
    let agent = rig.an_agent().await;
    let mut record = rig.record(&agent).await;
    let tasks = states
        .iter()
        .enumerate()
        .map(|(at, state)| ipc::SessionPlanTask { id: format!("T{at}"), text: format!("Task {at}"), state: *state, seen: true })
        .collect();
    record.plan = Some(ipc::SessionPlan {
        version: 1,
        summary: String::new(),
        recorded_at: ipc::Instant::carried("2026-10-16T10:00:00.000Z"),
        approved: true,
        approved_at: None,
        set_aside_at: None,
        scope: Vec::new(),
        groups: vec![ipc::SessionPlanGroup { id: "g1".into(), agent: "x".into(), title: "Add the overlay".into(), gives: None, paths: Vec::new(), tasks, issue: None, worked_at: None }],
        changes: None,
        sketch: None,
        held_by: Vec::new(),
    });
    record
}

#[tokio::test]
async fn a_group_is_done_when_every_task_of_it_is() {
    use ipc::PlanTaskState::{Done, Working};
    let some = record_with_plan(&[Done, Working]).await;
    assert_eq!(judged(&wait("group_done", "g1"), &some), Verdict::Waiting { pull: None });
    let all = record_with_plan(&[Done, Done]).await;
    let Verdict::Met { said, .. } = judged(&wait("group_done", "g1"), &all) else { panic!("met") };
    assert_eq!(said, "Add the overlay is done in The agent");
    let none = record_with_plan(&[]).await;
    assert_eq!(judged(&wait("group_done", "g1"), &none), Verdict::Waiting { pull: None }, "a group of no tasks is not done");
}

#[tokio::test]
async fn a_task_is_done_on_its_own() {
    use ipc::PlanTaskState::{Done, Working};
    let record = record_with_plan(&[Done, Working]).await;
    assert!(matches!(judged(&wait("task_done", "T0"), &record), Verdict::Met { .. }));
    assert_eq!(judged(&wait("task_done", "T1"), &record), Verdict::Waiting { pull: None });
    assert_eq!(judged(&wait("task_done", "T9"), &record), Verdict::Waiting { pull: None }, "a task the plan lacks never is");
}

#[tokio::test]
async fn a_task_handed_in_without_an_edit_is_done_to_a_wait() {
    use ipc::PlanTaskState::{Done, HandedIn, Working};
    let record = record_with_plan(&[HandedIn, Working]).await;
    assert!(matches!(judged(&wait("task_done", "T0"), &record), Verdict::Met { .. }));
    assert_eq!(judged(&wait("task_done", "T1"), &record), Verdict::Waiting { pull: None });
    let group = record_with_plan(&[Done, HandedIn]).await;
    assert!(matches!(judged(&wait("group_done", "g1"), &group), Verdict::Met { .. }));
    let open = record_with_plan(&[HandedIn, Working]).await;
    assert_eq!(judged(&wait("group_done", "g1"), &open), Verdict::Waiting { pull: None });
}

#[tokio::test]
async fn a_pull_request_that_merged_before_the_wait_is_not_the_one_it_waits_for() {
    let mut record = record_with_plan(&[]).await;
    record.attachments = vec![
        attachment("2000", AttachmentState::Spent, "2026-10-16T09:00:00.000Z", "old"),
        attachment("2412", AttachmentState::Standing, "2026-10-16T10:30:00.000Z", "armada/new"),
    ];
    let Verdict::Waiting { pull } = judged(&wait("pr_lands", ""), &record) else { panic!("not landed") };
    assert_eq!(pull.map(|one| one.number), Some(2412), "the open one is its pull request");
    record.attachments[1].state = AttachmentState::Spent;
    assert!(matches!(judged(&wait("pr_lands", ""), &record), Verdict::Met { .. }));
}

#[tokio::test]
async fn a_pull_request_once_named_stays_the_waits_own() {
    let mut record = record_with_plan(&[]).await;
    record.attachments = vec![
        attachment("2412", AttachmentState::Standing, "2026-10-16T10:30:00.000Z", "a"),
        attachment("2413", AttachmentState::Spent, "2026-10-16T10:40:00.000Z", "b"),
    ];
    let mut named = wait("pr_lands", "");
    named.pull_number = Some("2412".into());
    assert!(matches!(judged(&named, &record), Verdict::Waiting { .. }), "another one landing is not news to it");
}

// ---- The Session waited on changes when it starts ----

#[tokio::test]
async fn the_agent_changes_the_condition_and_both_threads_show_it() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;

    let changed = Arc::clone(&rig.fleet)
        .change_waiting(caller(), ChangeWaiting { waiting_session: waiting.id.clone(), condition: Some(StartCondition::PrPushed), base: Some(StartBase::Branch) })
        .await
        .expect("changed");

    assert_eq!((changed.condition, changed.base, changed.by), (StartCondition::PrPushed, StartBase::Branch, WaitBy::Agent));
    let record = rig.record(&waiting.id).await.waits.expect("still waits");
    assert_eq!((record.condition, record.base, record.by), (StartCondition::PrPushed, StartBase::Branch, WaitBy::Agent));
    let mine = rig.thread_has(&agent, "the row in the waited on thread", |row| matches!(row, SessionRow::Waiter { event: WaiterEvent::Changed, .. })).await;
    let SessionRow::Waiter { session, side, from, to, by, .. } = mine else { unreachable!() };
    assert_eq!((session.id, side, by), (waiting.id.clone(), WaiterSide::Waited, Some(WaitBy::Agent)));
    assert_eq!(from.as_deref(), Some("It ends, from main"));
    assert_eq!(to.as_deref(), Some("Its pull request is pushed, from its PR branch"));
    let theirs = rig.thread_has(&waiting.id, "the row in the waiting thread", |row| matches!(row, SessionRow::Waiter { event: WaiterEvent::Changed, .. })).await;
    let SessionRow::Waiter { session, side, .. } = theirs else { unreachable!() };
    assert_eq!((session.id, side), (agent.clone(), WaiterSide::Waiting));
    assert_eq!(rig.told("The person changed"), 0, "the agent that made the change is not told its own");
}

#[tokio::test]
async fn the_person_changes_it_and_the_agent_waited_on_is_told() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;

    let changed = Arc::clone(&rig.fleet)
        .change_waiting(None, ChangeWaiting { waiting_session: waiting.id.clone(), condition: Some(StartCondition::PrLands), base: None })
        .await
        .expect("changed");

    assert_eq!(changed.by, WaitBy::You);
    for _ in 0..500 {
        if rig.told("The person changed when job") == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(rig.told("It ends -> Its pull request lands"), 1);
}

#[tokio::test]
async fn a_change_to_a_condition_already_met_starts_it() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::PrLands, StartBase::Main).await;
    rig.report(&agent, opened("2412", "armada/a")).await;
    rig.eventually(&waiting.id, "the pull request named", |one| one.waits.as_ref().is_some_and(|wait| wait.pull.is_some())).await;

    Arc::clone(&rig.fleet)
        .change_waiting(caller(), ChangeWaiting { waiting_session: waiting.id.clone(), condition: Some(StartCondition::PrPushed), base: None })
        .await
        .expect("changed");

    rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;
}

#[tokio::test]
async fn a_plan_part_the_other_session_does_not_hold_is_refused() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    rig.fleet.record_plan(None, two_groups(&agent)).await.expect("recorded");
    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;

    let refused = Arc::clone(&rig.fleet)
        .change_waiting(caller(), ChangeWaiting { waiting_session: waiting.id.clone(), condition: Some(StartCondition::GroupDone { group: "g9".into() }), base: None })
        .await;
    assert!(format!("{refused:?}").contains("fleet.wait_part_unknown"), "{refused:?}");

    let held = Arc::clone(&rig.fleet)
        .change_waiting(caller(), ChangeWaiting { waiting_session: waiting.id.clone(), condition: Some(StartCondition::GroupDone { group: "g1".into() }), base: None })
        .await
        .expect("a group it holds");
    assert_eq!(held.condition, StartCondition::GroupDone { group: "g1".into() });
}

#[tokio::test]
async fn a_session_that_has_started_cannot_be_changed() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;
    Arc::clone(&rig.fleet).start_waiting_now(StartWaitingNow { session_id: waiting.id.clone() }).await.expect("started");

    let refused = Arc::clone(&rig.fleet)
        .change_waiting(None, ChangeWaiting { waiting_session: waiting.id.clone(), condition: Some(StartCondition::PrLands), base: None })
        .await;
    assert!(format!("{refused:?}").contains("fleet.wait_over"), "{refused:?}");
}

#[tokio::test]
async fn start_now_ignores_the_condition() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = rig.waits(&agent, StartCondition::PrLands, StartBase::Main).await;

    Arc::clone(&rig.fleet).start_waiting_now(StartWaitingNow { session_id: waiting.id.clone() }).await.expect("started");

    let (_, text) = rig.first_message(&waiting.id).await;
    assert!(text.contains("You started it"), "{text}");
    assert!(rig.record(&waiting.id).await.waits.is_none());
}

// ---- Refused before anything is made ----

#[tokio::test]
async fn a_wait_on_a_session_nothing_hosts_makes_no_session() {
    let rig = super::agent_starts::rig(None);
    rig.a_waited_on_agent().await;
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len();

    let refused = Arc::clone(&rig.fleet)
        .start_session_reading(brief_after(&rig, &SessionId::carried("nobody"), StartCondition::Ends, StartBase::Main))
        .await;
    assert!(format!("{refused:?}").contains("fleet.wait_session_unknown"), "{refused:?}");
    assert_eq!(rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len(), listed);
}

// ---- A Session an agent started waits too, and its first message is still the person's ----

#[tokio::test]
async fn a_session_an_agent_started_to_wait_holds_its_first_message_for_the_person_once_met() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let mut start: StartAnotherSession = begins(&rig, "Check the reader", "Read crates/reader.");
    // The agent starts a Session to follow its own end: the one it waits on is the one that asks.
    start.after = Some(StartAfter { session: agent.clone(), condition: StartCondition::Ends, base: StartBase::Main, needs: String::new() });

    let held = Arc::clone(&rig.fleet).start_another_session(caller(), start).await.expect("held");

    let record = rig.record(&held.session_id).await;
    assert!(record.waits.as_ref().is_some_and(|wait| wait.by == WaitBy::Agent), "it waits on the one the agent named");
    assert!(record.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).is_none(), "not held while it waits");
    assert!(kind(&record, "started_by").is_some(), "and the agent that started it still follows it");

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: agent.clone() }).await.expect("closed");

    let met = rig.eventually(&held.session_id, "the condition met", |one| one.waits.is_none()).await;
    let unsent = met.hosted.and_then(|hosted| hosted.start).and_then(|start| start.message).expect("held, its first message waiting for him");
    assert!(unsent.starts_with("Handoff from The agent"), "{unsent}");
    assert!(unsent.trim_end().ends_with("Read crates/reader."));
    assert!(rig.rows(&held.session_id).await.iter().all(|row| !matches!(row, SessionRow::Message { .. })), "off the thread");
    assert_eq!(rig.processes(), 1, "and nothing runs until he sends it");
}

// ---- The proposer reads the brief ----

#[tokio::test]
async fn the_proposer_reads_the_condition_and_the_need_out_of_the_brief() {
    let answer = "condition: pr_lands\npart: none\nbase: main\nneeds: to name its pull request";
    let rig = super::session_host::rig_fitted(|fittings| fittings.judge = Arc::new(FakeJudge::saying(answer)));
    let on = rig.start_titled(Some("Cockpit Map")).await;

    let reading = Arc::clone(&rig.fleet)
        .read_session_wait(ipc::ReadSessionWait { session_id: on, request: BRIEF.into() })
        .await
        .expect("read");

    assert_eq!((reading.condition, reading.base, reading.needs.as_str()), (StartCondition::PrLands, StartBase::Main, "to name its pull request"));
}

#[tokio::test]
async fn a_brief_that_cannot_be_read_is_the_defaults() {
    let rig = super::session_host::rig_fitted(|fittings| fittings.judge = Arc::new(FakeJudge::saying("I could not say")));
    let on = rig.start_titled(Some("Cockpit Map")).await;

    let reading = Arc::clone(&rig.fleet)
        .read_session_wait(ipc::ReadSessionWait { session_id: on, request: BRIEF.into() })
        .await
        .expect("read");

    assert_eq!((reading.condition, reading.base, reading.needs.as_str()), (StartCondition::Ends, StartBase::Main, ""));
}

#[test]
fn the_answer_reads_into_a_wait_the_wire_holds() {
    use crate::session_proposer::{PlanPart, WaitBrief};
    let parts = [
        PlanPart { id: "g1".into(), title: "Add the overlay".into(), group: true },
        PlanPart { id: "T3".into(), title: "Expand the Field".into(), group: false },
    ];
    let brief = WaitBrief::about(BRIEF, "Cockpit Map", &parts);
    assert!(brief.question().contains("g1 group - Add the overlay"));
    let read = |answer: &str| brief.read(answer);

    assert_eq!(read("condition: group_done\npart: g1\nbase: main\nneeds: none").condition, StartCondition::GroupDone { group: "g1".into() });
    assert_eq!(read("condition: task_done\npart: T3\nbase: main\nneeds: none").condition, StartCondition::TaskDone { task: "T3".into() });
    assert_eq!(read("condition: group_done\npart: T3").condition, StartCondition::Ends, "a task named as a group is not a group");
    assert_eq!(read("condition: group_done\npart: g9").condition, StartCondition::Ends, "a part the plan lacks is not one");
    assert_eq!(read("condition: pr_pushed").base, StartBase::Branch, "a push starts on the branch unless the words say main");
    assert_eq!(read("condition: pr_pushed\nbase: main").base, StartBase::Main);
    assert_eq!(read("condition: pr_lands\nbase: branch").base, StartBase::Branch);
    assert_eq!(read("condition: nonsense").condition, StartCondition::Ends);
    assert_eq!(read("condition: ends\nneeds: none").needs, "");
    assert_eq!(read("condition: ends\nneeds: its overlay API").needs, "its overlay API");
}

#[test]
fn a_reading_is_plain_only_where_the_answer_says_so() {
    let waits: ipc::WaitReading = ipc::decode("a wait reading", br#"{"condition":{"kind":"ends"},"base":"main"}"#).unwrap();
    assert!(!waits.plain, "an answer from before the flag still waits");
    let plain: ipc::WaitReading = ipc::decode("a wait reading", br#"{"condition":{"kind":"ends"},"base":"main","plain":true}"#).unwrap();
    assert!(plain.plain);
    let said = ipc::encode(&waits).unwrap();
    assert!(!said.contains("plain"), "a reading that waits does not carry the flag: {said}");
}

#[test]
fn a_brief_that_names_no_condition_reads_plain() {
    use crate::session_proposer::WaitBrief;
    let brief = WaitBrief::about(BRIEF, "Cockpit Map", &[]);
    assert!(brief.question().contains("plain:"), "the question asks for it");
    let read = |answer: &str| brief.read(answer);

    assert!(read("plain: yes\ncondition: ends\nbase: main\nneeds: none").plain);
    assert!(!read("plain: no\ncondition: pr_lands").plain);
    assert!(!read("condition: ends").plain, "an answer that does not say is a wait");
    assert!(!read("plain: yes\ncondition: pr_lands").plain, "a brief that names a condition is not plain");
    assert!(!crate::session_proposer::unread_wait().plain, "a reading that could not be had waits, as it always did");
}

#[test]
fn a_wait_is_a_closed_set_on_the_wire() {
    let after: StartAfter = ipc::decode("a start after", br#"{"session":"s1"}"#).unwrap();
    assert_eq!((after.condition, after.base, after.needs.as_str()), (StartCondition::Ends, StartBase::Main, ""));
    let after: StartAfter = ipc::decode("a start after", br#"{"session":"s1","condition":{"kind":"group_done","group":"g1"},"base":"branch","needs":"x"}"#).unwrap();
    assert_eq!(after.condition, StartCondition::GroupDone { group: "g1".into() });
    assert!(ipc::decode::<StartAfter>("a start after", br#"{"session":"s1","condition":{"kind":"sometime"}}"#).is_err());
    let _: Option<Refusal> = None;
}

// ---- A Session waiting on a workflow is not prompted by the rail before it starts ----

#[tokio::test]
async fn a_session_waiting_on_a_workflow_gets_no_step_prompt_before_its_condition_is_met() {
    let rig = super::agent_starts::rig_on_the_rail();
    let agent = rig.a_waited_on_agent().await;
    let processes = rig.processes();

    let waiting = rig.waits(&agent, StartCondition::Ends, StartBase::Main).await;
    rig.eventually(&waiting.id, "the brief read", |one| one.workflow.as_deref() == Some("rail")).await;
    rig.rail_has_acted().await;

    assert_eq!(rig.processes(), processes, "no process starts for a Session that waits");
    assert!(rig.rows(&waiting.id).await.is_empty(), "and nothing is sent to it");

    // Once met it starts on one turn that holds the step, and the rail adds none.
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: agent.clone() }).await.expect("closed");
    rig.processes_reach(processes + 1).await;
    rig.rail_has_acted().await;
    let sent = rig.sent_to(processes);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].contains("Record the plan before editing.") && sent[0].contains(BRIEF), "{}", sent[0]);
}
