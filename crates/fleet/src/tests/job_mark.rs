//! The mark a Job's record carries: the state, lamp and group its row in a Jobs list draws, read by
//! Fleet as Bridge's TypeScript reads them, so the native Bridge and Pocket do not. A stand-in process;
//! no forge, tracker or terminal is reached.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{CloseSession, JobGroup, JobLamp, JobMark, JobState, PauseSession, SessionId, SessionStanding, SessionTurn, ShowWindow, ViewSession};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_stops::{stopped_a_while_ago, tell_ripe};

async fn mark(rig: &Rig, id: &SessionId) -> JobMark {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session.mark.expect("a mark")
}

fn reads(state: JobState, lamp: JobLamp, group: JobGroup) -> JobMark {
    JobMark { state, lamp, group }
}

/// A Job with a turn running that has not written.
async fn working(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "think about it").await;
    rig.stand_in.init(0);
    eventually(|| async { matches!(rig.turn(&id).await, SessionTurn::Working { .. }) }).await;
    id
}

#[tokio::test]
async fn a_working_turn_reads_working_with_its_lamp_running_in_working() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Working, JobLamp::Running, JobGroup::Working));
}

#[tokio::test]
async fn a_paused_job_reads_paused_with_its_lamp_idle_in_held() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.fleet.pause_session(PauseSession { session_id: id.clone() }).await.expect("paused");
    assert_eq!(mark(&rig, &id).await, reads(JobState::Paused, JobLamp::Idle, JobGroup::Held));
}

const ONE_QUESTION: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t1","input":{"questions":[
  {"question":"Which clock?","header":"Clock","multiSelect":false,
   "options":[{"label":"Fake","description":""},{"label":"Frozen","description":""}]}]}}"#;

/// The question holds the turn, which is still open: what is asked of him reads over the turn.
#[tokio::test]
async fn a_job_asked_a_question_reads_waiting_and_needs_him() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let ask: ipc::AskingToRun = ipc::decode("an ask", ONE_QUESTION.as_bytes()).expect("an ask");
    let _held = tokio::spawn(async move { fleet.session_permission(&session, ask).await });
    eventually(|| async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked.is_some() }).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Waiting, JobLamp::Needs, JobGroup::Needs));
}

/// A Job whose turn ends with `said`, as he left it.
async fn replied(rig: &Rig, said: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, said);
    eventually(|| async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.standing == Some(SessionStanding::Replied) }).await;
    id
}

async fn view(rig: &Rig, id: &SessionId, on: bool) {
    rig.fleet.view_session(ViewSession { session_id: id.clone(), on }).await.expect("viewed");
}

/// He has not been in it since its turn ended, so the reply is unread; once he has, it reads as it
/// would with no reply to read.
#[tokio::test]
async fn a_replied_turn_reads_replied_until_he_has_been_in_it_since() {
    let rig = rig_placing(None);
    let id = replied(&rig, "The cap is five.").await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Replied, JobLamp::Done, JobGroup::Done));
    view(&rig, &id, true).await;
    view(&rig, &id, false).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Idle, JobLamp::Done, JobGroup::Done));
}

/// A turn that ends while he has the Job on screen is read as it ends, though his visit began before it.
#[tokio::test]
async fn a_turn_ending_while_he_has_the_job_on_screen_reads_read() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    view(&rig, &id, true).await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.");
    eventually(|| async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.standing == Some(SessionStanding::Replied) }).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Idle, JobLamp::Done, JobGroup::Done));
}

/// A reply that comes after he left is unread again.
#[tokio::test]
async fn a_turn_ending_after_his_visit_reads_replied_again() {
    let rig = rig_placing(None);
    let id = replied(&rig, "The cap is five.").await;
    view(&rig, &id, true).await;
    view(&rig, &id, false).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Idle, JobLamp::Done, JobGroup::Done), "read once he has been in it");
    let before = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.last_turn_at;
    rig.send(&id, "and the floor?").await;
    rig.stand_in.finishes(0, "The floor is one.");
    eventually(|| async {
        let now = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
        now.last_turn_at != before && now.standing == Some(SessionStanding::Replied)
    })
    .await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Replied, JobLamp::Done, JobGroup::Done));
}

fn walk_window(id: &SessionId) -> ShowWindow {
    ShowWindow { url: "http://localhost:5173/".into(), title: Some("Findings".into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() }
}

/// A walk he has not approved is built work to look at: the row is marked Ready, not Needs, though the turn runs on.
#[tokio::test]
async fn an_owed_walk_marks_a_working_job_ready_and_leaves_it_working() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.fleet.show_window(None, walk_window(&id)).await.expect("shown");
    assert_eq!(mark(&rig, &id).await, reads(JobState::Working, JobLamp::Ready, JobGroup::Ready));
}

/// Only built work waits: nothing blocks on him, so the Job reads Ready in lamp and group too.
#[tokio::test]
async fn a_resting_job_owing_only_a_walk_reads_ready() {
    let rig = rig_placing(None);
    let id = replied(&rig, "Built it.").await;
    rig.fleet.show_window(None, walk_window(&id)).await.expect("shown");
    assert_eq!(mark(&rig, &id).await, reads(JobState::Ready, JobLamp::Ready, JobGroup::Ready));
}

/// One thing the Job cannot go on without is enough: the built work beside it does not soften it.
#[tokio::test]
async fn a_resting_job_owing_a_walk_and_a_question_reads_waiting_and_needs_him() {
    let rig = rig_placing(None);
    let id = replied(&rig, "Built it.").await;
    rig.fleet.show_window(None, walk_window(&id)).await.expect("shown");
    let ask = ipc::WaitingInput { id: "which".into(), text: "Which branch?".into(), act: None, options: Vec::new(), approves: false, steps: Vec::new(), artifact: None };
    rig.fleet.waiting_for(None, ipc::SetWaitingFor { items: vec![ask], session_id: Some(id.clone()) }).await.expect("stated");
    assert_eq!(mark(&rig, &id).await, reads(JobState::Waiting, JobLamp::Needs, JobGroup::Needs));
}

/// A Job whose turn ends with `said`, which may ask: a reply that asks stands Idle, not Replied.
async fn ended(rig: &Rig, said: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, said);
    eventually(|| async {
        let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
        record.last_turn_at.is_some() && record.hosted.is_some_and(|one| one.turn == SessionTurn::Idle)
    })
    .await;
    id
}

/// A Ready to try card of the Job's branch, open in its thread.
async fn a_card_open(rig: &Rig, id: &SessionId) {
    let at = rig.fleet.now();
    rig.fleet.store().lock().await.keep_preview_offered(id.as_str(), "armada/one", "abc1234", "Keeps one thing", None, &[], at.as_str()).expect("offered");
}

/// The card asks him itself: a closing line that only says the Job waits on it is no new ask, so the Job
/// stays Ready and out of Needs you.
#[tokio::test]
async fn a_reply_that_only_points_at_the_open_ready_to_try_card_leaves_the_job_ready() {
    let rig = rig_placing(None);
    let id = ended(&rig, "Built it.\n\nStill waiting for your Looks good on the Ready to try card.").await;
    a_card_open(&rig, &id).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Ready, JobLamp::Ready, JobGroup::Ready));
}

/// A question of its own beside the open card still needs him.
#[tokio::test]
async fn a_question_beside_the_open_ready_to_try_card_still_needs_him() {
    let rig = rig_placing(None);
    let id = ended(&rig, "Built it.\n\nShould I also change the Pocket list once you've tried it?").await;
    a_card_open(&rig, &id).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::Waiting, JobLamp::Needs, JobGroup::Needs));
}

#[tokio::test]
async fn a_stopped_job_fleet_gave_up_on_reads_stopped_asks_and_needs_him() {
    let rig = rig_placing(None);
    std::fs::write(rig.repair_state.join("answer"), "NOT REPAIRED: it waits on nothing").expect("an answer");
    let id = stopped_a_while_ago(&rig).await;
    tell_ripe(&rig, &id).await;
    rig.stand_in.finishes(0, "Still written.");
    eventually(|| async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.standing == Some(ipc::SessionStanding::Stopped) }).await;
    rig.fleet.stops_told().await;
    eventually(|| async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.stopped.is_some_and(|one| one.gave_up) }).await;
    assert_eq!(mark(&rig, &id).await, reads(JobState::StoppedAsks, JobLamp::Needs, JobGroup::Needs));
}

/// A Job that called `all_done` waits only for him to end it: it reads Done in the Done group, and asks nothing.
#[tokio::test]
async fn a_job_that_said_it_is_done_reads_done_in_done_and_does_not_need_him() {
    let rig = rig_placing(None);
    let id = replied(&rig, "It merged.").await;
    rig.fleet.all_done(None, ipc::AllDone { session_id: Some(id.clone()), summary: None }).await.expect("done");
    assert_eq!(mark(&rig, &id).await, reads(JobState::Done, JobLamp::Done, JobGroup::Done));
}

#[tokio::test]
async fn an_ended_job_reads_ended_and_done() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("ended");
    assert_eq!(mark(&rig, &id).await, reads(JobState::Ended, JobLamp::Done, JobGroup::Done));
}
