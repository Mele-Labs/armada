//! What the pure core of the claim cannot reach: **a Job reads the Jobs it started, and commands them,
//! against a store.** The ruling on values is `acceptance`'s; here the effects are watched: a withdraw
//! runs no agent and clears the Press Start item, an end tells the child and hands its issue back, a
//! wait is changed or dropped and the Job waited on hears of it, and a Job that did not start the other
//! is refused. `docs/concepts/session/a-job-that-started-jobs.md`.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Backlog;
use api::{Refusal, Sessions};
use ipc::{
    SessionHeld, SessionId, SessionRecord, SessionRow, SessionState, StartAfter, StartBase, StartCondition, StartedAct,
    StartedCommand, StartedJob, StartedSessions, WaiterEvent,
};
use testkit::{FakeBeads, FakeBeadsRemote, FakeTrackers};

use super::agent_starts::{begins, caller, rig, rigged, rigged_fitted, Rig};
use crate::operator::{Desk, FleetDesk};
use crate::tests::backlog::{actor, BEADS};
use crate::tests::peer::Placing;

const PORT: u16 = 12345;

/// The connection of the first agent the stand-in starts, which is pid 1000.
fn first_agent(peers: &Placing) {
    peers.holding(1000, 50999, PORT);
}

fn at(port: u16) -> Option<api::Caller> {
    Some(api::Caller::at(format!("127.0.0.1:{port}").parse().unwrap()))
}

fn command(id: &SessionId, act: StartedAct) -> StartedCommand {
    StartedCommand { session_id: id.clone(), act, why: None, condition: None, base: None }
}

fn ending(id: &SessionId, why: &str) -> StartedCommand {
    StartedCommand { why: Some(why.into()), ..command(id, StartedAct::End) }
}

fn code(refused: &Result<StartedJob, Refusal>) -> String {
    format!("{:?}", refused.as_ref().expect_err("refused"))
}

impl Rig {
    async fn started(&self, title: &str) -> SessionId {
        Arc::clone(&self.fleet).start_another_session(caller(), begins(self, title, "Read crates/reader.")).await.expect("held").session_id
    }

    /// Start pressed on a held child, and its process up.
    async fn begun(&self, id: &SessionId) -> usize {
        let before = self.stand_in.starts().len();
        self.begin(id, None).await;
        for _ in 0..500 {
            if self.stand_in.starts().len() > before {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(self.stand_in.starts().len() > before, "its process started");
        self.stand_in.init(before);
        before
    }

    /// A child of the agent that waits on `on` to end.
    async fn waiting_on(&self, on: &SessionId, title: &str) -> SessionId {
        let mut start = begins(self, title, "Read crates/reader.");
        start.after = Some(StartAfter { session: on.clone(), condition: StartCondition::PrLands, base: StartBase::Main, needs: String::new() });
        Arc::clone(&self.fleet).start_another_session(caller(), start).await.expect("held").session_id
    }

    async fn commanded(&self, commands: Vec<StartedCommand>) -> Vec<Result<StartedJob, Refusal>> {
        Arc::clone(&self.fleet).command_started_sessions(caller(), commands).await.expect("placed")
    }

    async fn listed_children(&self) -> StartedSessions {
        Arc::clone(&self.fleet).list_started_sessions(caller()).await.expect("placed")
    }

    async fn items(&self, id: &SessionId) -> Vec<String> {
        self.record(id).await.waiting_for.into_iter().map(|one| one.id).collect()
    }

    async fn thread_has_row(&self, id: &SessionId, held: impl Fn(&SessionRow) -> bool) -> bool {
        for _ in 0..500 {
            if self.rows(id).await.iter().any(&held) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        false
    }
}

fn placing_rig() -> Rig {
    let peers = Placing::nothing();
    first_agent(&peers);
    rig(Some(peers))
}

/// A second child of the agent, which another child can wait on.
async fn a_sibling(rig: &Rig) -> SessionId {
    let held: SessionHeld = Arc::clone(&rig.fleet).start_another_session(caller(), begins(rig, "Another", "Read it.")).await.expect("held");
    held.session_id
}

#[tokio::test]
async fn a_parent_reads_each_child_with_where_it_stands_what_it_waits_on_and_the_items_fleet_raised() {
    let rig = placing_rig();
    let parent = rig.an_agent().await;
    let held = rig.started("Held").await;
    let waiting = rig.waiting_on(&parent, "Waiting").await;
    let running = rig.started("Running").await;
    rig.begun(&running).await;

    let read = rig.listed_children().await;
    let ids: Vec<&str> = read.jobs.iter().map(|one| one.summary.id.as_str()).collect();
    assert_eq!(ids, [held.as_str(), waiting.as_str(), running.as_str()], "oldest first");

    let [held_job, waiting_job, running_job] = &read.jobs[..] else { panic!("three: {:?}", read.jobs) };
    assert!(held_job.held && !held_job.paused && held_job.waits.is_none() && held_job.result.is_none());
    assert!(
        held_job.waiting_for.iter().any(|one| one.id == format!("unsent:{}", held.as_str())),
        "the first message it waits on him to send: {:?}",
        held_job.waiting_for
    );
    let wait = waiting_job.waits.as_ref().expect("it waits");
    assert_eq!((wait.on.id.as_str(), wait.condition.clone()), (parent.as_str(), StartCondition::PrLands));
    assert!(!running_job.held && running_job.waits.is_none());
    assert_eq!(running_job.summary.state, SessionState::Live);
}

#[tokio::test]
async fn an_ended_child_reads_with_why_it_ended_and_what_it_said_it_achieved() {
    let rig = placing_rig();
    rig.an_agent().await;
    let child = rig.started("Done").await;
    rig.begun(&child).await;
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.keep_done(child.as_str(), &at, Some("The reader keeps its last line.")).expect("said");
    rig.fleet.closed_and_parked(ipc::CloseSession { session_id: child.clone() }).await.expect("closed");

    let read = rig.listed_children().await;
    let [job] = &read.jobs[..] else { panic!("one: {:?}", read.jobs) };
    assert_eq!(job.summary.state, SessionState::Ended);
    let result = job.result.as_ref().expect("a result once ended");
    assert_eq!(result.summary.as_deref(), Some("The reader keeps its last line."));
    assert!(result.end_reason.is_some(), "{result:?}");
}

#[tokio::test]
async fn a_held_child_and_a_waiting_child_are_withdrawn_with_no_agent_run_and_the_start_item_leaves() {
    let rig = placing_rig();
    let parent = rig.an_agent().await;
    let held = rig.started("Held").await;
    let waiting = rig.waiting_on(&parent, "Waiting").await;
    let processes = rig.stand_in.starts().len();
    let start_item = format!("start:{}", held.as_str());
    assert!(rig.items(&parent).await.contains(&start_item), "{:?}", rig.items(&parent).await);

    let answers = rig.commanded(vec![command(&held, StartedAct::Withdraw), command(&waiting, StartedAct::Withdraw)]).await;

    for (answer, id) in answers.iter().zip([&held, &waiting]) {
        assert_eq!(answer.as_ref().expect("withdrawn").summary.state, SessionState::Ended, "{id:?}");
    }
    assert_eq!(rig.stand_in.starts().len(), processes, "no agent ever ran");
    assert!(!rig.items(&parent).await.contains(&start_item), "Press Start leaves his list");
    assert_eq!(rig.record(&held).await.state, SessionState::Ended);
}

#[tokio::test]
async fn a_child_that_has_begun_is_not_withdrawn_and_stays_live() {
    let rig = placing_rig();
    rig.an_agent().await;
    let child = rig.started("Begun").await;
    rig.begun(&child).await;

    let answers = rig.commanded(vec![command(&child, StartedAct::Withdraw)]).await;

    let refused = code(&answers[0]);
    assert!(refused.contains("fleet.started_begun"), "{refused}");
    assert_eq!(rig.record(&child).await.state, SessionState::Live);
}

#[tokio::test]
async fn ending_a_child_tells_it_why_closes_it_and_hands_back_the_issue_it_claimed() {
    let remote = FakeBeadsRemote::new();
    let copy = Arc::new(FakeBeads::on(&remote));
    let peers = Placing::nothing();
    first_agent(&peers);
    let planted = Arc::clone(&copy);
    let rig = rigged_fitted(Some(peers), None, None, move |fittings| {
        fittings.trackers = Arc::new(FakeTrackers::empty().beads_named("beads", planted));
        fittings.starting().manifest = config::Manifest::parse(std::path::Path::new("armada.yml"), BEADS).expect("loads");
    });
    let changes = [("machine.name".to_string(), Some(ipc::SettingValue::Text("mac-a".to_string())))].into_iter().collect();
    api::Settings::save_settings(&*rig.fleet, ipc::SaveSettings { changes }).await.expect("saved");
    let reference = copy.create("Groom the reader");
    copy.sync("/").expect("pushed");
    let parent = rig.an_agent().await;
    let child = rig.started("Grooming").await;
    let process = rig.begun(&child).await;
    let claim = ipc::ClaimIssue { issue: reference.clone(), session_id: Some(child.clone()), pull_requests: Vec::new() };
    rig.fleet.claim_issue(None, claim).await.expect("the child holds it");
    assert_eq!(remote.holder(&reference), Some(actor("mac-a", &child)));

    let answers = rig.commanded(vec![ending(&child, "the reader is out of scope")]).await;

    assert_eq!(answers[0].as_ref().expect("ended").summary.state, SessionState::Ended);
    let told = rig.sent_to(process).join("\n");
    assert!(told.contains("ended it: the reader is out of scope"), "{told}");
    assert!(told.contains("stays on your branch"), "{told}");
    assert_eq!(remote.holder(&reference), Some(String::new()), "handed back");
    let bead = remote.bead(&reference).expect("still filed");
    assert_eq!(bead.status, "open");
    assert!(bead.reason.contains(parent.as_str()) && bead.reason.contains(child.as_str()), "the note names both Jobs: {}", bead.reason);
}

#[tokio::test]
async fn an_end_without_a_why_is_refused_and_the_child_stays_live() {
    let rig = placing_rig();
    rig.an_agent().await;
    let child = rig.started("Held").await;

    let answers = rig.commanded(vec![command(&child, StartedAct::End)]).await;

    assert!(code(&answers[0]).contains("fleet.started_needs_why"));
    assert_eq!(rig.record(&child).await.state, SessionState::Live);
}

#[tokio::test]
async fn a_child_is_paused_and_resumed_by_the_job_that_started_it() {
    let rig = placing_rig();
    rig.an_agent().await;
    let child = rig.started("Running").await;
    rig.begun(&child).await;

    let paused = rig.commanded(vec![command(&child, StartedAct::Pause)]).await;
    assert!(paused[0].as_ref().expect("paused").paused);
    assert!(rig.record(&child).await.hosted.expect("hosted").paused.is_some());

    let resumed = rig.commanded(vec![command(&child, StartedAct::Resume)]).await;
    assert!(!resumed[0].as_ref().expect("resumed").paused);
}

#[tokio::test]
async fn a_wait_on_a_job_that_is_ending_is_changed_and_the_job_waited_on_is_told() {
    let rig = placing_rig();
    rig.an_agent().await;
    let other = a_sibling(&rig).await;
    rig.begun(&other).await;
    let waiting = rig.waiting_on(&other, "Waiting").await;
    rig.fleet.closed_and_parked(ipc::CloseSession { session_id: other.clone() }).await.expect("closed");

    let change = StartedCommand { condition: Some(StartCondition::Ends), ..command(&waiting, StartedAct::Change) };
    let answers = rig.commanded(vec![change]).await;

    let wait = answers[0].as_ref().expect("changed").waits.clone().expect("still waits");
    assert_eq!((wait.condition, wait.on.id.as_str()), (StartCondition::Ends, other.as_str()), "on the same Job, ended or not");
    let said = rig.thread_has_row(&other, |row| matches!(row, SessionRow::Waiter { event: WaiterEvent::Changed, .. })).await;
    assert!(said, "the Job waited on has the change in its thread");
}

#[tokio::test]
async fn a_change_tells_a_live_job_waited_on_which_job_changed_the_wait() {
    let rig = placing_rig();
    rig.an_agent().await;
    let other = a_sibling(&rig).await;
    let process = rig.begun(&other).await;
    let waiting = rig.waiting_on(&other, "Waiting").await;

    let change = StartedCommand { condition: Some(StartCondition::Ends), ..command(&waiting, StartedAct::Change) };
    rig.commanded(vec![change]).await[0].as_ref().expect("changed");

    for _ in 0..500 {
        if !rig.sent_to(process).iter().all(|line| !line.contains("which it started")) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let told = rig.sent_to(process).join("\n");
    assert!(told.contains("changed when job") && told.contains("Waiting") && told.contains("which it started"), "{told}");
}

#[tokio::test]
async fn a_dropped_wait_leaves_the_card_held_on_nothing_and_the_job_waited_on_is_told() {
    let rig = placing_rig();
    rig.an_agent().await;
    let other = a_sibling(&rig).await;
    let process = rig.begun(&other).await;
    let waiting = rig.waiting_on(&other, "Waiting").await;

    let dropped = rig.commanded(vec![command(&waiting, StartedAct::Drop)]).await;

    let job = dropped[0].as_ref().expect("dropped");
    assert!(job.waits.is_none() && job.held, "held for the person, waiting on nothing: {job:?}");
    let message = rig.card_message(&waiting).await.expect("the brief is the card's");
    assert_eq!(message.trim_end(), "Read crates/reader.", "no handoff from a Job it no longer waits on");
    for _ in 0..500 {
        if rig.sent_to(process).iter().any(|line| line.contains("dropped the wait")) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let told = rig.sent_to(process).join("\n");
    assert!(told.contains("dropped the wait of job") && told.contains("Waiting"), "{told}");
    let again = rig.commanded(vec![command(&waiting, StartedAct::Drop)]).await;
    assert!(again[0].is_err(), "a wait that is gone is not dropped twice");
}

#[tokio::test]
async fn a_job_that_did_not_start_another_is_refused_on_every_act_a_grandchild_and_an_unplaced_call_included() {
    let peers = Placing::nothing();
    first_agent(&peers);
    peers.holding(1001, 52001, PORT);
    let rig = rig(Some(Arc::clone(&peers)));
    rig.an_agent().await;
    let child = rig.started("Child").await;
    rig.begun(&child).await;
    // The child starts a Job of its own: the parent's grandchild.
    let grandchild: SessionHeld = Arc::clone(&rig.fleet)
        .start_another_session(at(52001), begins(&rig, "Grandchild", "Read it."))
        .await
        .expect("the child starts one");

    for act in [StartedAct::Withdraw, StartedAct::Pause, StartedAct::Resume, StartedAct::Drop] {
        let answers = rig.commanded(vec![command(&grandchild.session_id, act)]).await;
        assert!(code(&answers[0]).contains("fleet.started_not_yours"), "{act:?}: {}", code(&answers[0]));
    }
    let answers = rig.commanded(vec![ending(&grandchild.session_id, "no longer needed")]).await;
    let refused = code(&answers[0]);
    assert!(refused.contains("fleet.started_not_yours") && refused.contains("was not started by this job"), "{refused}");
    assert_eq!(rig.record(&grandchild.session_id).await.state, SessionState::Live, "nothing was done to it");
    let top = rig.listed_children().await;
    let below: Vec<(&str, &str)> = top.jobs.iter().map(|one| (one.summary.id.as_str(), one.started_by.as_str())).collect();
    assert_eq!(below.len(), 2, "it is read, though not commanded: {below:?}");
    assert_eq!(below[1], (grandchild.session_id.as_str(), child.as_str()), "started by the child");

    // The child is placed by its own connection and reads only its own.
    let read = Arc::clone(&rig.fleet).list_started_sessions(at(52001)).await.expect("placed");
    let ids: Vec<&str> = read.jobs.iter().map(|one| one.summary.id.as_str()).collect();
    assert_eq!(ids, [grandchild.session_id.as_str()]);
    assert_eq!(read.jobs[0].started_by, child, "the child started it");
    let upward = Arc::clone(&rig.fleet).command_started_sessions(at(52001), vec![command(&child, StartedAct::Pause)]).await.expect("placed");
    assert!(code(&upward[0]).contains("fleet.started_not_yours"), "a child cannot command its parent");

    // A call from no Job's process, or with no connection, places nobody.
    for unplaced in [at(40404), None] {
        let refused = Arc::clone(&rig.fleet).command_started_sessions(unplaced.clone(), vec![command(&child, StartedAct::Pause)]).await;
        assert!(format!("{:?}", refused.expect_err("refused")).contains("fleet.started_unplaced"));
        let refused = Arc::clone(&rig.fleet).list_started_sessions(unplaced).await;
        assert!(format!("{:?}", refused.expect_err("refused")).contains("fleet.started_unplaced"));
    }
}

#[tokio::test]
async fn a_batch_is_answered_in_order_and_a_refused_entry_leaves_the_others_done() {
    let rig = rigged(None, None, None);
    rig.an_agent().await;
    let first = rig.started("First").await;
    let second = rig.started("Second").await;
    let elsewhere = SessionId::carried("01NOTAJOB");

    let answers = rig
        .commanded(vec![command(&first, StartedAct::Withdraw), command(&elsewhere, StartedAct::Withdraw), command(&second, StartedAct::Withdraw)])
        .await;

    let [one, none, two] = &answers[..] else { panic!("three answers: {answers:?}") };
    assert_eq!(one.as_ref().expect("done").summary.id, first);
    assert!(code(none).contains("fleet.started_not_yours"));
    assert_eq!(two.as_ref().expect("done").summary.id, second);
    let ended = |record: SessionRecord| record.state == SessionState::Ended;
    assert!(ended(rig.record(&first).await) && ended(rig.record(&second).await));
}

/// A Job, its two children, and below the first a grandchild and a great-grandchild, each started by
/// the Job above it from its own connection. Returns the rig, the Job, its children, the grandchild
/// and the great-grandchild.
async fn a_tree() -> (Rig, SessionId, [SessionId; 2], SessionId, SessionId) {
    let peers = Placing::nothing();
    first_agent(&peers);
    peers.holding(1001, 52001, PORT);
    peers.holding(1002, 52002, PORT);
    let rig = rig(Some(Arc::clone(&peers)));
    let top = rig.an_agent().await;
    let first = rig.started("First").await;
    let second = rig.started("Second").await;
    rig.begun(&first).await;
    let grandchild = Arc::clone(&rig.fleet).start_another_session(at(52001), begins(&rig, "Grandchild", "Read it.")).await.expect("held").session_id;
    rig.begun(&grandchild).await;
    let great = Arc::clone(&rig.fleet).start_another_session(at(52002), begins(&rig, "Great", "Read it.")).await.expect("held").session_id;
    (rig, top, [first, second], grandchild, great)
}

#[tokio::test]
async fn a_job_reads_the_whole_tree_below_it_each_child_before_its_own_and_each_with_who_started_it_but_commands_only_its_children() {
    let (rig, top, [first, second], grandchild, great) = a_tree().await;

    let from_the_first = Arc::clone(&rig.fleet).list_started_sessions(at(52001)).await.expect("placed");
    let ids: Vec<&str> = from_the_first.jobs.iter().map(|one| one.summary.id.as_str()).collect();
    assert_eq!(ids, [grandchild.as_str(), great.as_str()], "a Job reads what is below it, not what is beside it");

    // Cycles in the links read once: the grandchild is said to have started the first, the great one the top.
    {
        let mut store = rig.fleet.store().lock().await;
        for (holder, target) in [(&grandchild, &first), (&great, &top)] {
            let at = rig.fleet.now().as_str().to_string();
            let link = store::KeptAttachment {
                holder: store::Holder::session(holder.as_str()),
                kind: "started_to".into(),
                manifest_id: rig.manifest.as_str().to_string(),
                target: target.as_str().into(),
                state: store::AttachmentState::Spent,
                detail: Default::default(),
                since: at.clone(),
                changed_at: at,
            };
            store.attach(&link, false).expect("kept");
        }
    }

    let read = rig.listed_children().await;
    let seen: Vec<(&str, &str)> = read.jobs.iter().map(|one| (one.summary.id.as_str(), one.started_by.as_str())).collect();
    assert_eq!(
        seen,
        [
            (first.as_str(), top.as_str()),
            (grandchild.as_str(), first.as_str()),
            (great.as_str(), grandchild.as_str()),
            (second.as_str(), top.as_str()),
        ],
        "each child followed by its own descendants, oldest first at each level"
    );

    for below in [&grandchild, &great] {
        let answers = rig.commanded(vec![ending(below, "no longer needed")]).await;
        assert!(code(&answers[0]).contains("fleet.started_not_yours"), "a command stays for a direct child: {}", code(&answers[0]));
    }
    assert_eq!(rig.record(&great).await.state, SessionState::Live);
}

#[tokio::test]
async fn the_operators_live_list_names_who_started_a_job_up_to_the_root_and_an_ended_starter_still_counts() {
    let (rig, top, [first, _second], grandchild, great) = a_tree().await;
    let desk = FleetDesk::on(&rig.fleet, |_| {});
    let titled = |record: SessionRecord| record.title.expect("titled");
    let (top_title, first_title, grandchild_title) = (
        titled(rig.record(&top).await),
        titled(rig.record(&first).await),
        titled(rig.record(&grandchild).await),
    );

    let live = desk.live().await;
    let started_by = |id: &SessionId| live.iter().find(|one| one.session_id == id.as_str()).unwrap_or_else(|| panic!("{id:?} is live")).started_by.clone();
    assert!(started_by(&top).is_empty(), "the person started it");
    assert_eq!(started_by(&first), [(top.as_str().to_string(), top_title.clone())]);
    assert_eq!(
        started_by(&great),
        [
            (grandchild.as_str().to_string(), grandchild_title.clone()),
            (first.as_str().to_string(), first_title.clone()),
            (top.as_str().to_string(), top_title.clone()),
        ],
        "nearest first, up to the root"
    );

    rig.commanded(vec![ending(&first, "no longer needed")]).await;
    let live = desk.live().await;
    assert!(live.iter().all(|one| one.session_id != first.as_str()), "it ended");
    let chain = live.iter().find(|one| one.session_id == great.as_str()).expect("live").started_by.clone();
    assert_eq!(chain.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), [grandchild.as_str(), first.as_str(), top.as_str()], "an ended starter is named too");
}

/// `message_reach`: a Job writes up its starter chain and down its tree, and is refused a sibling.
/// A caller that is no Job, and an address that is no Job, are allowed.
#[tokio::test]
async fn a_job_may_message_the_jobs_above_and_below_it_and_is_refused_its_siblings() {
    use api::HostedSessions;
    use crate::session_host::address_of;

    let (rig, top, [_, second], grandchild, great) = a_tree().await;
    let reach = |from: u16, to: &SessionId| {
        let fleet = Arc::clone(&rig.fleet);
        let to = ipc::MessageTo { to: address_of(to.as_str()) };
        async move {
            let caller = at(from).expect("an address");
            fleet.message_reach(caller, to).await.expect("answered")
        }
    };
    let allowed = ipc::MessageReach::Allowed {};

    assert_eq!(reach(52001, &top).await, allowed, "the Job that started it");
    assert_eq!(reach(52001, &grandchild).await, allowed, "a Job it started");
    assert_eq!(reach(52001, &great).await, allowed, "a Job below one it started");
    assert_eq!(reach(52002, &top).await, allowed, "the Job that started its starter");
    match reach(52001, &second).await {
        ipc::MessageReach::Refused { message } => assert_eq!(
            message,
            format!(
                "`SendMessage` to {} is refused: a Job writes only to the Jobs that started it, the Jobs it started and the Jobs the person names to it",
                address_of(second.as_str())
            )
        ),
        allowed => panic!("a sibling is refused, and was {allowed:?}"),
    }
    assert_eq!(reach(59999, &second).await, allowed, "a caller that is no Job");
    let nobody = ipc::MessageTo { to: String::from("s-0badc0de") };
    let caller = at(52001).expect("an address");
    assert_eq!(Arc::clone(&rig.fleet).message_reach(caller, nobody).await.expect("answered"), allowed, "an address that is no Job");
}
