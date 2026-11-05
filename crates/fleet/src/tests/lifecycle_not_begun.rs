//! A Job that has not begun holds nothing: on its start card or waiting on another, it refuses
//! neither of its parent's `all_done` and `finish_up`, End still lists it, and ending the parent
//! neither ends it nor stops it starting. Once it runs it holds a parent still live.
//! `docs/concepts/session/the-rail.md`, *A live instance holds its Job open*. **No forge, tracker or
//! process is reached**; the stand-in process is `session_host`'s.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Lifecycles, Sessions};
use core_model::{Observed, ReadingState};
use ipc::{AllDone, ChangeWaiting, CloseSession, FinishUp, SessionId, SessionRow, SessionState, StartAfter, StartBase, StartCondition, StartSession, WaiterEvent};

use super::agent_starts::{begins, caller, rig, Rig};
use super::session_host::eventually;
use crate::lifecycles::ending::IN_FLIGHT;

const CHILD: &str = "Check the reader";

/// The parent, up and able to make the calls, with what waits and what it holds being read.
async fn a_parent(rig: &Rig) -> SessionId {
    crate::after_following::keep_the_waiting(Arc::clone(&rig.fleet));
    crate::lifecycles::keep_the_lifecycles(Arc::clone(&rig.fleet));
    rig.an_agent().await
}

/// A Job the parent's agent starts, on its start card.
async fn on_its_card(rig: &Rig, parent: &SessionId, title: &str) -> SessionId {
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(rig, title, "Read crates/reader.")).await.expect("held").session_id;
    reads_running(rig, parent, title).await;
    held
}

/// A Job the parent's agent starts to wait on `on`'s end. The person's approval has it start alone.
async fn waiting_on(rig: &Rig, parent: &SessionId, title: &str, on: &SessionId, approved: bool) -> SessionId {
    let mut start = begins(rig, title, "Read crates/reader.");
    start.after = Some(StartAfter { session: on.clone(), condition: StartCondition::Ends, base: StartBase::Main, needs: String::new() });
    let id = Arc::clone(&rig.fleet).start_another_session(caller(), start).await.expect("made to wait").session_id;
    if approved {
        let change = ChangeWaiting { waiting_session: id.clone(), condition: None, base: None, held: Some(false), workflow: None, model: None, effort: None };
        Arc::clone(&rig.fleet).change_waiting(None, change).await.expect("approved");
    }
    reads_running(rig, parent, title).await;
    id
}

/// A Job on a start card of its own, with no process and nothing to do with the parent, for another to wait on.
async fn a_job_with_no_process(rig: &Rig) -> SessionId {
    let start = StartSession {
        manifest_id: rig.manifest.clone(),
        title: Some("The other".into()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: true,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    rig.fleet.start_session(start).await.expect("started").id
}

/// `title`'s instance reads Running to `parent`, so what holds it is a question of whether it ran.
async fn reads_running(rig: &Rig, parent: &SessionId, title: &str) {
    eventually(|| async {
        let store = rig.fleet.store().lock().await;
        rig.fleet.lifecycles_flying(&store, parent.as_str()).iter().any(|one| one.label == title && one.state == "Running")
    })
    .await;
}

async fn runs(rig: &Rig, id: &SessionId) {
    eventually(|| async { rig.fleet.store().lock().await.hosting(id.as_str()).ok().flatten().is_some_and(|one| one.ran) }).await;
}

impl Rig {
    fn processes_up(&self) -> usize {
        self.stand_in.starts().len()
    }
}

/// What refuses `id`, as the refusal names it.
async fn holding(rig: &Rig, id: &SessionId) -> Vec<String> {
    let store = rig.fleet.store().lock().await;
    rig.fleet.lifecycles_holding(&store, id.as_str()).iter().map(|one| one.held()).collect()
}

async fn all_done(rig: &Rig, id: &SessionId) -> Result<ipc::SessionRecord, api::Refusal> {
    rig.fleet.all_done(None, AllDone { session_id: Some(id.clone()), summary: None }).await
}

async fn finish_up(rig: &Rig, id: &SessionId) -> Result<ipc::FinishedUp, api::Refusal> {
    rig.fleet.finish_up(None, FinishUp { session_id: Some(id.clone()) }).await
}

async fn end(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("ended");
}

/// What Fleet told `id` that nothing holds it.
async fn told(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice: Some(notice), text, .. } if notice == "lifecycles_clear" => Some(text),
            _ => None,
        })
        .collect()
}

async fn told_of_what_waits(rig: &Rig, id: &SessionId) -> bool {
    rig.rows(id).await.iter().any(|row| matches!(row, SessionRow::Waiter { event: WaiterEvent::Waits, .. }))
}

/// The parent's End list holds `child`, as Bridge reads it off the wire.
async fn end_lists(rig: &Rig, parent: &SessionId, child: &SessionId) -> bool {
    let held = rig.fleet.get_job_lifecycles(None, Some(parent.clone())).await.expect("read");
    held.instances.iter().any(|one| one.key == child.as_str() && one.ended_at.is_none())
}

#[tokio::test]
async fn a_job_on_its_start_card_leaves_its_parent_free_to_finish_and_end_and_starts_after() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let child = on_its_card(&rig, &parent, CHILD).await;

    assert!(finish_up(&rig, &parent).await.is_ok(), "it has not begun, so it holds nothing");
    assert!(all_done(&rig, &parent).await.expect("nothing holds it").done_since.is_some());
    assert!(end_lists(&rig, &parent, &child).await, "End still lists it");

    end(&rig, &parent).await;
    assert_eq!(rig.record(&child).await.state, SessionState::Live, "ending the parent does not end it");

    rig.begin(&child, None).await;
    runs(&rig, &child).await;
    assert_eq!(rig.record(&child).await.state, SessionState::Live);
}

#[tokio::test]
async fn once_it_runs_it_holds_a_parent_still_live_and_once_it_ends_it_does_not() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let child = on_its_card(&rig, &parent, CHILD).await;
    assert_eq!(holding(&rig, &parent).await, Vec::<String>::new());

    rig.begin(&child, None).await;
    runs(&rig, &child).await;

    let refused = all_done(&rig, &parent).await.expect_err("it is running");
    assert_eq!((refused.status(), refused.error().code.as_str()), (409, IN_FLIGHT));
    assert!(refused.error().message.contains(&format!("- {CHILD} is Running\n")), "{}", refused.error().message);
    assert!(finish_up(&rig, &parent).await.is_err());

    end(&rig, &child).await;
    eventually(|| async { holding(&rig, &parent).await.is_empty() }).await;
    assert!(all_done(&rig, &parent).await.is_ok(), "a Job already ended is unaffected");
}

#[tokio::test]
async fn an_approved_job_waiting_on_another_holds_nothing_and_starts_when_that_one_ends_though_its_parent_has_gone() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let other = a_job_with_no_process(&rig).await;
    let child = waiting_on(&rig, &parent, CHILD, &other, true).await;

    assert!(finish_up(&rig, &parent).await.is_ok());
    assert!(all_done(&rig, &parent).await.is_ok(), "it waits, approved or not");
    assert!(end_lists(&rig, &parent, &child).await, "End still lists it");

    end(&rig, &parent).await;
    assert!(rig.record(&child).await.waits.is_some(), "ending its parent is not what it waits for");
    assert_eq!(rig.record(&child).await.state, SessionState::Live);

    end(&rig, &other).await;
    runs(&rig, &child).await;
}

#[tokio::test]
async fn a_chain_holds_its_parent_one_job_at_a_time_as_each_runs() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let other = a_job_with_no_process(&rig).await;
    let first = waiting_on(&rig, &parent, "First", &other, true).await;
    let second = waiting_on(&rig, &parent, "Second", &first, true).await;
    // The word would start it once the proposer has read it, so what shows at once is the row.
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!told_of_what_waits(&rig, &first).await, "being waited on does not tell the one that still waits, since a word would start it");
    assert_eq!(rig.processes_up(), 1);
    assert!(all_done(&rig, &parent).await.is_ok(), "neither has begun");

    end(&rig, &other).await;
    runs(&rig, &first).await;
    eventually(|| async { told_of_what_waits(&rig, &first).await }).await;
    assert_eq!(holding(&rig, &parent).await, ["First is Running"], "the one after it still waits");
    assert!(all_done(&rig, &parent).await.is_err());

    end(&rig, &first).await;
    runs(&rig, &second).await;
    eventually(|| async { holding(&rig, &parent).await == ["Second is Running"] }).await;

    end(&rig, &second).await;
    eventually(|| async { holding(&rig, &parent).await.is_empty() }).await;
    assert!(all_done(&rig, &parent).await.is_ok());
}

#[tokio::test]
async fn a_job_waiting_on_its_parents_end_lets_the_parent_end_and_is_then_held_for_the_person() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let child = waiting_on(&rig, &parent, CHILD, &parent, false).await;

    assert!(finish_up(&rig, &parent).await.is_ok(), "what it waits for is this Job ending");
    assert!(all_done(&rig, &parent).await.is_ok());

    end(&rig, &parent).await;
    eventually(|| async { rig.record(&child).await.waits.is_none() }).await;
    let met = rig.record(&child).await;
    assert!(met.hosted.and_then(|hosted| hosted.start).and_then(|start| start.message).is_some(), "its first message waits for him");
}

#[tokio::test]
async fn a_job_approved_to_wait_on_its_parents_end_starts_alone_once_the_parent_has_ended() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let child = waiting_on(&rig, &parent, CHILD, &parent, true).await;

    assert!(all_done(&rig, &parent).await.is_ok());
    end(&rig, &parent).await;
    runs(&rig, &child).await;
}

#[tokio::test]
async fn a_parent_refused_for_what_is_over_is_told_it_may_finish_naming_what_has_not_begun() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    on_its_card(&rig, &parent, CHILD).await;
    rig.fleet.store().lock().await.take_on_lifecycle_instance(parent.as_str(), "deploy", "Deploy", &[], "t0").expect("taken on");
    rig.fleet.lifecycle_seen(parent.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Running)).await;

    let refused = all_done(&rig, &parent).await.expect_err("the deploy is rolling out");
    assert!(!refused.error().message.contains(CHILD), "{}", refused.error().message);

    rig.fleet.lifecycle_seen(parent.as_str(), "deploy", "Deploy", Observed::Watch(ReadingState::Passed)).await;
    eventually(|| async { !told(&rig, &parent).await.is_empty() }).await;
    assert_eq!(told(&rig, &parent).await, [format!("Deploy is Healthy. Nothing holds this Job open: {CHILD} has not begun, so you may finish.")]);
}

/// A parent up and told to wait, its child read, and no tap yet: Fleet has not started.
async fn refused_before_fleet_started(rig: &Rig) -> SessionId {
    let parent = rig.an_agent().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(rig, CHILD, "Read crates/reader.")).await.expect("held").session_id;
    rig.fleet.lifecycles_moved(rig.record(&held).await).await;
    rig.fleet.store().lock().await.hold_lifecycle_job(parent.as_str(), "t0").expect("kept");
    parent
}

#[tokio::test]
async fn a_parent_refused_before_this_is_told_at_fleets_start_though_no_move_comes() {
    let rig = rig(None);
    let parent = refused_before_fleet_started(&rig).await;
    assert_eq!(told(&rig, &parent).await, Vec::<String>::new());

    crate::lifecycles::keep_the_lifecycles(Arc::clone(&rig.fleet));

    eventually(|| async { !told(&rig, &parent).await.is_empty() }).await;
    assert_eq!(told(&rig, &parent).await, [format!("Nothing holds this Job open: {CHILD} has not begun, so you may finish.")], "not that it is Running");
    assert!(!rig.fleet.store().lock().await.lifecycle_job_held(parent.as_str()).expect("reads"), "told once");
}

#[tokio::test]
async fn a_parent_told_to_wait_hears_of_a_job_ended_on_its_card_and_of_the_one_that_has_not_begun() {
    let rig = rig(None);
    let parent = a_parent(&rig).await;
    let child = on_its_card(&rig, &parent, CHILD).await;
    on_its_card(&rig, &parent, "Still on its card").await;
    rig.fleet.store().lock().await.hold_lifecycle_job(parent.as_str(), "t0").expect("kept");

    end(&rig, &child).await;

    eventually(|| async { !told(&rig, &parent).await.is_empty() }).await;
    assert_eq!(told(&rig, &parent).await, [format!("{CHILD} is Ended. Nothing holds this Job open: Still on its card has not begun, so you may finish.")]);
}
