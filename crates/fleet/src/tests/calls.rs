//! What calls the person, read by Fleet as Electron Bridge's TypeScript reads it so the native Bridge and
//! Pocket do not: which Jobs' items call him, which of it he has not opened, and what began to wait on him
//! since. The reading is checked on a Job's real row with the items and marks a stand-in cannot make
//! put on it; the wire is checked end to end through the follower. A stand-in process; no forge, tracker
//! or terminal is reached.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use api::HostedSessions;
use ipc::{
    Attachment, AttachmentState, CallArrival, CallArrivalKind, CallTone, Event, Instant, JobGroup, JobLamp, JobMark, JobState, MainMerge,
    MainRedJob, MainView, MergeLineView, MergeLineViews, OpenStandingCall, SessionId, SessionPaused, SessionRecord, SessionRepairing, SessionStopped, SessionTurn,
    StandingCallKind, StandingCalls, StopReason, StopRestart, ViewSession, WaitingItem, WaitingSource,
};

use super::session_host::{eventually, rig_placing, Rig};
use crate::standing_calls::Keeper;

fn at(second: u32) -> Instant {
    Instant::carried(format!("2026-11-09T10:00:{second:02}.000Z"))
}

/// Long after every instant here.
fn now() -> i64 {
    Instant::carried("2026-11-09T10:30:00.000Z").to_domain().epoch_millis().expect("an instant")
}

fn nowhere(_: &str) -> bool {
    false
}

fn item(id: &str, text: &str, source: WaitingSource, second: u32) -> WaitingItem {
    WaitingItem {
        approves: false,
        id: id.into(),
        text: text.into(),
        since: at(second),
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

/// A live Job's own row, whose `waiting_for` and mark the test then sets as it needs.
async fn row(rig: &Rig) -> SessionRecord {
    let id = rig.start().await;
    let mut record = Arc::clone(&rig.fleet).get_session(id).await.expect("a session").session;
    record.waiting_for.clear();
    if let Some(hosted) = record.hosted.as_mut() {
        hosted.start = None;
    }
    marked(record, JobState::Idle)
}

fn marked(mut record: SessionRecord, state: JobState) -> SessionRecord {
    record.mark = Some(JobMark { state, lamp: JobLamp::Idle, group: JobGroup::Held });
    record
}

fn arrival(record: &SessionRecord, text: &str, tone: CallTone, kind: CallArrivalKind) -> CallArrival {
    CallArrival { session_id: record.id.clone(), title: record.title.clone().unwrap_or_else(|| "Job".into()), text: text.into(), tone, kind, key: None }
}

/// Seeded on what stands.
fn seeded(record: &SessionRecord) -> Keeper {
    let mut keeper = Keeper::default();
    keeper.seed(std::slice::from_ref(record), MergeLineViews::default());
    keeper
}

#[tokio::test]
async fn what_stands_when_fleet_starts_is_not_news_and_a_new_item_arrives_once() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.waiting_for = vec![item("replied:1", "Land it?", WaitingSource::Replied, 1)];
    let mut keeper = seeded(&record);

    let first = keeper.changed(&nowhere, now()).expect("what stands is a call");
    assert!(first.arrived.is_empty(), "seeding tells nothing: {first:?}");
    assert_eq!(first.calls.len(), 1);
    assert_eq!(first.calls[0].key, format!("session:{}:replied:1", record.id.as_str()));

    record.waiting_for.push(item("ask:2", "Which clock?", WaitingSource::AskCard, 2));
    keeper.hear(&record);
    let second = keeper.changed(&nowhere, now()).expect("a new item");
    assert_eq!(second.arrived, [arrival(&record, "Which clock?", CallTone::Blocked, CallArrivalKind::Needs)]);

    keeper.hear(&record);
    assert_eq!(keeper.changed(&nowhere, now()), None, "a row heard again moves nothing");
}

#[tokio::test]
async fn an_item_posted_again_under_its_id_is_the_same_item() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.waiting_for = vec![item("ask:1", "Which clock?", WaitingSource::AskCard, 1)];
    let mut keeper = seeded(&record);
    keeper.changed(&nowhere, now());

    record.waiting_for = vec![item("ask:1", "Which clock, then?", WaitingSource::AskCard, 4)];
    keeper.hear(&record);
    let moved = keeper.changed(&nowhere, now()).expect("the words moved");
    assert!(moved.arrived.is_empty(), "{moved:?}");
    assert_eq!(moved.calls[0].text, "Which clock, then?");
}

#[tokio::test]
async fn a_paused_job_tells_nothing_and_resuming_it_does_not_tell_what_it_held() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    let mut keeper = seeded(&record);
    keeper.changed(&nowhere, now());

    record.hosted.as_mut().expect("hosted").paused = Some(SessionPaused { at: at(0), resuming: false });
    record.waiting_for = vec![item("ask:1", "Which clock?", WaitingSource::AskCard, 1)];
    keeper.hear(&record);
    assert_eq!(keeper.changed(&nowhere, now()), None, "nothing it waits on calls or is told while it is paused");

    record.hosted.as_mut().expect("hosted").paused = None;
    keeper.hear(&record);
    let resumed = keeper.changed(&nowhere, now()).expect("it calls again");
    assert!(resumed.arrived.is_empty(), "what was read while paused is not news: {resumed:?}");
    assert_eq!(resumed.calls.len(), 1);
}

#[tokio::test]
async fn a_source_that_holds_the_turn_is_blocked_and_any_other_is_waiting() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    let mut keeper = seeded(&record);

    record.waiting_for = vec![
        item("perm:1", "Run the tests?", WaitingSource::Permission, 1),
        item("replied:2", "Land it?", WaitingSource::Replied, 2),
        item("call:3", "A notice", WaitingSource::FleetCall, 3),
    ];
    keeper.hear(&record);
    let told = keeper.changed(&nowhere, now()).expect("three arrivals");
    assert_eq!(
        told.arrived,
        [
            arrival(&record, "Run the tests?", CallTone::Blocked, CallArrivalKind::Needs),
            arrival(&record, "Land it?", CallTone::Waiting, CallArrivalKind::Needs),
            // A fleet call holds the turn for a call, but a banner tells it as it tells any other.
            arrival(&record, "A notice", CallTone::Waiting, CallArrivalKind::Needs),
        ]
    );
}

#[tokio::test]
async fn a_job_held_on_its_start_card_is_news_as_not_started() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    let mut keeper = seeded(&record);

    record = marked(record, JobState::Draft);
    record.hosted.as_mut().expect("hosted").start = Some(ipc::StartHeld { asks_delivery: false, picked: None, message: None, queued: false });
    keeper.hear(&record);
    let told = keeper.changed(&nowhere, now()).expect("the start card");
    assert_eq!(told.arrived, [arrival(&record, "Not started", CallTone::Blocked, CallArrivalKind::Needs)]);
}

/// A stopped Job that asks him, with the repair as given, and the words its call carries.
async fn stopped_call_text(repair: Option<ipc::StopRepair>, repairing: Option<SessionRepairing>) -> String {
    let rig = rig_placing(None);
    let mut record = marked(row(&rig).await, JobState::Working);
    let mut keeper = seeded(&record);
    keeper.changed(&nowhere, now());
    record = marked(record, JobState::StoppedAsks);
    record.last_turn_at = Some(at(10));
    record.stopped = Some(SessionStopped {
        reason: StopReason::ProcessGone,
        at: at(12),
        tasks: Vec::new(),
        said: None,
        restart: StopRestart::Failed,
        repair,
        gave_up: true,
        down: None,
    });
    record.repairing = repairing;
    keeper.hear(&record);
    keeper.changed(&nowhere, now()).expect("it stopped").calls.remove(0).text
}

#[tokio::test]
async fn a_repair_under_way_is_one_clause_on_the_line_and_replaces_the_repair_job_words() {
    let running = Some(SessionRepairing { said: "its process died twice".into(), since: at(13) });
    assert_eq!(
        stopped_call_text(Some(ipc::StopRepair::Running), running).await,
        "Stopped: process ended · Repairing: its process died twice"
    );
    assert_eq!(stopped_call_text(Some(ipc::StopRepair::Failed), None).await, "Stopped: process ended · Not repaired");
    assert_eq!(stopped_call_text(Some(ipc::StopRepair::Held), None).await, "Stopped: process ended · No repair: Fleet is unhealthy");
}

#[tokio::test]
async fn turning_stopped_for_good_arrives_as_stopped_and_stays_a_call_he_has_not_opened_after_a_visit() {
    let rig = rig_placing(None);
    let mut record = marked(row(&rig).await, JobState::Working);
    let mut keeper = seeded(&record);
    keeper.changed(&nowhere, now());

    record = marked(record, JobState::StoppedAsks);
    record.last_turn_at = Some(at(10));
    record.stopped = Some(SessionStopped {
        reason: StopReason::ProcessGone,
        at: at(12),
        tasks: Vec::new(),
        said: None,
        restart: StopRestart::Failed,
        repair: None,
        gave_up: true,
        down: None,
    });
    keeper.hear(&record);
    let told = keeper.changed(&nowhere, now()).expect("it stopped");
    assert_eq!(told.arrived, [arrival(&record, "Stopped: process ended", CallTone::Waiting, CallArrivalKind::Stopped)]);
    assert_eq!(told.calls.len(), 1);
    assert_eq!(told.calls[0].kind, StandingCallKind::Job);
    assert_eq!(told.calls[0].key, format!("session:{}:needs:{}", record.id.as_str(), at(10).as_str()));
    assert_eq!(told.calls[0].text, "Stopped: process ended · Restart failed");
    assert_eq!(told.calls[0].since, at(10));
    assert!(!told.calls[0].opened);

    keeper.hear(&record);
    assert_eq!(keeper.changed(&nowhere, now()), None, "it is told once");

    // A Job that asks him is never opened by a visit: `needingOf`.
    record.viewed_at = Some(at(30));
    keeper.hear(&record);
    assert_eq!(keeper.changed(&nowhere, now()), None);
    assert!(!keeper.standing(&nowhere, now())[0].opened);
}

#[tokio::test]
async fn a_job_that_needs_him_without_asking_is_a_call_until_he_has_been_in_it() {
    let rig = rig_placing(None);
    let mut record = marked(row(&rig).await, JobState::Done);
    record.last_turn_at = Some(at(4));
    let key = format!("session:{}:needs:{}", record.id.as_str(), at(4).as_str());

    let unseen = seeded(&record).standing(&nowhere, now());
    assert_eq!((unseen.len(), unseen[0].key.as_str(), unseen[0].text.as_str(), unseen[0].since.clone(), unseen[0].opened), (1, key.as_str(), "Done", at(4), false));

    record.viewed_at = Some(at(5));
    let seen = seeded(&record).standing(&nowhere, now());
    assert!(seen[0].opened, "he has been in it since the turn ended");

    // The turn working on is no call at all.
    record.hosted.as_mut().expect("hosted").turn = SessionTurn::Working { woken_by: None };
    let working = seeded(&marked(record, JobState::Working)).standing(&nowhere, now());
    assert!(working.is_empty(), "{working:?}");
}

#[tokio::test]
async fn a_job_that_asks_stays_a_call_after_he_has_seen_its_item_and_a_paused_one_raises_nothing() {
    let rig = rig_placing(None);
    let mut record = marked(row(&rig).await, JobState::Waiting);
    record.viewed_at = Some(at(5));
    record.waiting_for = vec![item("ask:1", "Which clock?", WaitingSource::AskCard, 3)];
    let calls = seeded(&record).standing(&nowhere, now());
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].key.contains(":needs:") && !calls[0].opened && calls[0].text == "Waiting on you", "the item was seen in the Job; the Job's row still asks: {calls:?}");

    record = marked(record, JobState::Paused);
    record.hosted.as_mut().expect("hosted").paused = Some(SessionPaused { at: at(0), resuming: false });
    assert!(seeded(&record).standing(&nowhere, now()).is_empty());
}

#[tokio::test]
async fn an_item_a_job_held_before_it_was_paused_is_not_told_on_resume() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.waiting_for = vec![item("ask:1", "Which clock?", WaitingSource::AskCard, 1)];
    let mut keeper = seeded(&record);

    // Fleet lists nothing for a paused Job.
    record.hosted.as_mut().expect("hosted").paused = Some(SessionPaused { at: at(0), resuming: false });
    record.waiting_for.clear();
    keeper.hear(&record);
    keeper.changed(&nowhere, now());

    record.hosted.as_mut().expect("hosted").paused = None;
    record.waiting_for = vec![item("ask:1", "Which clock?", WaitingSource::AskCard, 1)];
    keeper.hear(&record);
    let resumed = keeper.changed(&nowhere, now()).expect("the call is back");
    assert!(resumed.arrived.is_empty(), "{resumed:?}");
}

#[tokio::test]
async fn an_item_that_does_not_hold_the_turn_is_hidden_while_the_turn_works() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.hosted.as_mut().expect("hosted").turn = SessionTurn::Working { woken_by: None };
    record.waiting_for = vec![item("replied:1", "Land it?", WaitingSource::Replied, 1)];
    let mut keeper = seeded(&record);
    assert_eq!(keeper.changed(&nowhere, now()), None, "a turn at work is not waiting on him");

    record.waiting_for.push(item("perm:2", "Run the tests?", WaitingSource::Permission, 2));
    keeper.hear(&record);
    let held = keeper.changed(&nowhere, now()).expect("a permission holds the turn");
    assert_eq!(held.calls.len(), 1);
    assert_eq!(held.calls[0].text, "Run the tests?");
}

#[tokio::test]
async fn an_item_older_than_his_last_visit_is_no_call_except_a_document() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.viewed_at = Some(at(5));
    record.waiting_for = vec![
        item("replied:1", "Land it?", WaitingSource::Replied, 3),
        item("replied:2", "And the floor?", WaitingSource::Replied, 7),
        item("document:a.html", "The design", WaitingSource::Document, 3),
    ];
    let keeper = seeded(&record);
    let calls = keeper.standing(&nowhere, now());
    let texts: Vec<&str> = calls.iter().map(|one| one.text.as_str()).collect();
    assert_eq!(texts, ["The design", "And the floor?"], "oldest first, the one he saw in the Job gone");

    // On screen is now, which is after everything.
    let here = |_: &str| true;
    let texts: Vec<String> = keeper.standing(&here, now()).into_iter().map(|one| one.text).collect();
    assert_eq!(texts, ["The design"]);
}

#[tokio::test]
async fn a_visited_jobs_call_is_opened_unless_the_job_asks_him() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.viewed_at = Some(at(5));
    record.waiting_for = vec![item("document:a.html", "The design", WaitingSource::Document, 3)];

    let resting = seeded(&marked(record.clone(), JobState::Idle));
    assert!(resting.standing(&nowhere, now())[0].opened, "he has been in it since it began");

    let asking = seeded(&marked(record.clone(), JobState::Waiting));
    assert!(!asking.standing(&nowhere, now())[0].opened, "a Job that asks him stays unopened once visited");

    let unseen = {
        let mut record = record;
        record.viewed_at = None;
        seeded(&marked(record, JobState::Idle))
    };
    assert!(!unseen.standing(&nowhere, now())[0].opened);
}

fn pull_row(number: &str, branch: &str) -> Attachment {
    Attachment {
        kind: "pr".into(),
        manifest_id: None,
        target: number.into(),
        state: AttachmentState::Standing,
        detail: BTreeMap::from([("branch".to_string(), branch.to_string())]),
        since: at(0),
        changed_at: at(0),
    }
}

fn red_main(root: &str, number: u64, branch: &str) -> MergeLineView {
    MergeLineView {
        root: root.into(),
        name: "app".into(),
        main: Some(MainView::Red {
            failed: MainRedJob { check: "unit".into(), unmapped: false, test: Some("it_holds".into()), log_url: None },
            also: Vec::new(),
            since: Some(at(20)),
            merge: Some(MainMerge { number, url: None, branch: Some(branch.into()), session: None }),
            taken_by: None,
            elsewhere: None,
            checking: Vec::new(),
        }),
        pulls: Vec::new(),
        landed: Vec::new(),
        checkout: None,
        read_at: None,
        quiet_since: None,
    }
}

fn lines(of: Vec<MergeLineView>) -> MergeLineViews {
    MergeLineViews { lines: of }
}

#[tokio::test]
async fn main_red_is_a_call_unless_the_job_that_broke_it_is_on_it() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.attachments = vec![pull_row("7", "fix-clock")];
    let id = record.id.as_str().to_string();
    let mut keeper = seeded(&record);
    keeper.lines = lines(vec![red_main("/r", 7, "fix-clock")]);

    let calls = keeper.standing(&nowhere, now());
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!((call.key.as_str(), call.kind, call.title.as_str(), call.text.as_str()), ("main:/r", StandingCallKind::Main, "main is red", "unit · it_holds"));
    assert_eq!(call.owner, format!("session:{id}"), "it is the Job's, who broke it");
    assert_eq!(call.session_id.as_ref(), Some(&record.id));
    assert_eq!(call.since, at(20));
    assert_eq!(call.tone, CallTone::Waiting);

    // Told of it after it arose: it is on it.
    keeper.told.insert(id.clone(), at(25).to_domain().epoch_millis().expect("an instant"));
    keeper.hear(&record);
    assert!(keeper.standing(&nowhere, now()).is_empty());

    // Told of it before it arose: not on it.
    keeper.told.insert(id, at(15).to_domain().epoch_millis().expect("an instant"));
    keeper.hear(&record);
    assert_eq!(keeper.standing(&nowhere, now()).len(), 1);

    // At work: on it.
    record.hosted.as_mut().expect("hosted").turn = SessionTurn::Working { woken_by: None };
    keeper.hear(&record);
    assert!(keeper.standing(&nowhere, now()).is_empty());
}

#[tokio::test]
async fn main_with_a_newer_run_still_going_is_no_call_and_a_failing_pull_request_is() {
    let rig = rig_placing(None);
    let record = row(&rig).await;
    let mut keeper = seeded(&record);
    let mut held = red_main("/r", 7, "fix-clock");
    if let Some(MainView::Red { checking, .. }) = held.main.as_mut() {
        checking.push(ipc::MainChecking { commit: "abc".into(), pull_request: None });
    }
    let mut failing = red_main("/s", 8, "other");
    failing.main = None;
    failing.pulls = vec![ipc::PullView { held_by: None, tone: ipc::PullTone::Bad, pull: ipc::HubPullRequest {
        number: 9,
        title: "Cap the retries".into(),
        branch: "cap".into(),
        url: "https://forge/9".into(),
        author: None,
        ci: Some(ipc::HubPullCi::Failed),
        queue: None,
        opened_at: None,
        updated_at: Some(at(12)),
        draft: false,
        mergeable: None,
        review: None,
        additions: None,
        deletions: None,
        owner: None,
        merge_by_hand: false,
        mine: true,
    } }];
    keeper.lines = lines(vec![held, failing]);
    let calls = keeper.standing(&nowhere, now());
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!((calls[0].key.as_str(), calls[0].kind, calls[0].title.as_str(), calls[0].text.as_str()), ("pull:9", StandingCallKind::Pull, "#9 cap", "checks failed"));
    assert_eq!(calls[0].owner, "pull:9", "nobody owns it, so the call is its own owner");
    assert_eq!(calls[0].session_id, None);
    assert_eq!(calls[0].since, at(12));
}

#[tokio::test]
async fn one_unopened_entry_stands_for_each_owner() {
    let rig = rig_placing(None);
    let mut record = row(&rig).await;
    record.attachments = vec![pull_row("7", "fix-clock")];
    record.waiting_for = vec![
        item("replied:1", "Land it?", WaitingSource::Replied, 1),
        item("ask:2", "Which clock?", WaitingSource::AskCard, 2),
    ];
    let mut keeper = seeded(&record);
    keeper.lines = lines(vec![red_main("/r", 7, "fix-clock"), red_main("/s", 8, "unowned")]);

    let all = StandingCalls { calls: keeper.standing(&nowhere, now()) };
    assert_eq!(all.calls.len(), 4, "two items, the Job's failure, and one nobody owns: {all:?}");
    let owners: Vec<&str> = all.unopened().iter().map(|one| one.owner.as_str()).collect();
    assert_eq!(owners, [format!("session:{}", record.id.as_str()).as_str(), "main:/s"], "the newest in front, one for the Job and its failure");
}

#[tokio::test]
async fn open_call_opens_a_main_call_and_nothing_else() {
    let rig = rig_placing(None);
    let record = row(&rig).await;
    let mut keeper = seeded(&record);
    keeper.lines = lines(vec![red_main("/r", 7, "fix-clock")]);
    let opened = |keeper: &Keeper| keeper.standing(&nowhere, now())[0].opened;
    assert!(!opened(&keeper));

    keeper.open("pull:99", &nowhere, now());
    keeper.open("session:nobody:ask:1", &nowhere, now());
    assert!(!opened(&keeper), "a key that names no call standing opens nothing");

    keeper.open("main:/r", &nowhere, now());
    assert!(opened(&keeper));
    let told = keeper.changed(&nowhere, now()).expect("openedness moved");
    assert!(told.arrived.is_empty() && told.calls[0].opened, "{told:?}");
}

/// Through Fleet: a row's own items and a visit, heard off the stream and told as `calls.changed`.
#[tokio::test]
async fn a_job_that_asks_him_is_told_once_and_a_visit_takes_its_call_off_the_list() {
    let rig = rig_placing(None);
    let told: Arc<Mutex<Vec<ipc::CallsChanged>>> = Arc::default();
    let seen = Arc::clone(&told);
    rig.fleet.events().tap(move |event| {
        if let Event::CallsChanged(told) = event {
            seen.lock().unwrap().push(told.clone());
        }
    });
    let _following = crate::standing_calls::keep_the_calls(Arc::clone(&rig.fleet));
    eventually(|| async { rig.fleet.hosts().calls.seeded() }).await;

    let id = rig.start().await;
    rig.send(&id, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.\n\nWant me to land it?");
    let arrivals = |told: &Arc<Mutex<Vec<ipc::CallsChanged>>>| told.lock().unwrap().iter().flat_map(|one| one.arrived.clone()).collect::<Vec<_>>();
    eventually(|| async { !arrivals(&told).is_empty() }).await;
    let first = arrivals(&told);
    assert_eq!((first.len(), first[0].kind, first[0].tone, &first[0].session_id), (1, CallArrivalKind::Needs, CallTone::Waiting, &id), "{first:?}");
    let standing = rig.fleet.get_calls().await.expect("the calls");
    assert_eq!(standing.calls.len(), 1, "{standing:?}");
    assert_eq!(standing.calls[0].owner, format!("session:{}", id.as_str()));
    assert!(!standing.calls[0].opened);

    // In the Job he saw the item there, so it leaves the list; the Job still asks him, so its row's own call
    // takes its place and stays unopened. Nothing is told again.
    rig.fleet.view_session(ViewSession { session_id: id.clone(), on: true }).await.expect("on screen");
    let row_call = |told: &Arc<Mutex<Vec<ipc::CallsChanged>>>| told.lock().unwrap().last().is_some_and(|last| last.calls.len() == 1 && last.calls[0].key.contains(":needs:"));
    eventually(|| async { row_call(&told) }).await;
    assert_eq!(arrivals(&told).len(), 1, "a visit is no news");
    let after = rig.fleet.get_calls().await.expect("the calls");
    assert!(after.calls.len() == 1 && !after.calls[0].opened, "{after:?}");
}

#[tokio::test]
async fn open_call_on_a_jobs_key_is_a_success_that_opens_nothing() {
    let rig = rig_placing(None);
    let id: SessionId = rig.start().await;
    let answered = rig.fleet.open_call(OpenStandingCall { key: format!("session:{}:replied:1", id.as_str()) }).await.expect("a no-op success");
    assert!(answered.calls.is_empty());
}

#[tokio::test]
async fn a_held_checkout_and_a_pull_request_to_merge_by_hand_are_calls_nobody_is_on() {
    let rig = rig_placing(None);
    let record = row(&rig).await;
    let mut keeper = seeded(&record);
    let mut line = red_main("/r", 7, "x");
    line.main = None;
    line.checkout = Some(ipc::HeldCheckout {
        base: "main".into(),
        on: None,
        uncommitted: vec!["a.rs".into()],
        clashing: Vec::new(),
        behind: 880,
        ahead: 0,
        pull_request: None,
    });
    line.pulls = vec![ipc::PullView { held_by: None, tone: ipc::PullTone::Ask, pull: ipc::HubPullRequest {
        number: 4,
        title: "Edit".into(),
        branch: "armada-yml".into(),
        url: "https://forge/4".into(),
        author: None,
        ci: Some(ipc::HubPullCi::Passed),
        queue: None,
        opened_at: None,
        updated_at: None,
        draft: false,
        mergeable: None,
        review: None,
        additions: None,
        deletions: None,
        owner: None,
        merge_by_hand: true,
        mine: true,
    } }];
    keeper.lines = lines(vec![line]);
    let calls = keeper.standing(&nowhere, now());
    let shown: Vec<_> = calls.iter().map(|one| (one.key.as_str(), one.kind, one.title.as_str(), one.text.as_str())).collect();
    assert_eq!(
        shown,
        [
            ("checkout:/r", StandingCallKind::Checkout, "880 commits behind main", "Uncommitted changes"),
            ("pull:4", StandingCallKind::Pull, "armada.yml edit #4", "Needs merging by hand"),
        ]
    );
    keeper.open("checkout:/r", &nowhere, now());
    assert!(keeper.standing(&nowhere, now()).iter().any(|one| one.key == "checkout:/r" && one.opened));
}

fn waiting_to_pair(why: &str) -> ipc::MacClaimWaiting {
    ipc::MacClaimWaiting { mac: "0a0b".into(), name: "Work".into(), why: why.into(), at: 1_762_000_000 }
}

#[test]
fn a_mac_waiting_to_pair_calls_him_with_its_id_and_digits_once_and_stops_when_handed_in_no_more() {
    let mut keeper = Keeper::default();
    keeper.seed(&[], MergeLineViews::default());
    let why = "Work wants to pair. Confirm if it shows 0042.";
    keeper.hold_claims(vec![waiting_to_pair(why)]);
    let told = keeper.changed(&nowhere, now()).expect("a call");
    let call = ipc::StandingCall {
        key: "mac:0a0b".into(),
        owner: "mac:0a0b".into(),
        kind: StandingCallKind::MacPairing,
        session_id: None,
        title: "Confirm Work pairing".into(),
        text: why.into(),
        since: Instant::carried("2025-11-01T12:26:40.000Z"),
        tone: CallTone::Blocked,
        opened: false,
        mac: Some("0a0b".into()),
    };
    assert_eq!(told.calls, [call]);
    let arrival = CallArrival {
        session_id: SessionId::carried("mac:0a0b"),
        title: "Confirm Work pairing".into(),
        text: why.into(),
        tone: CallTone::Blocked,
        kind: CallArrivalKind::MacPairing,
        key: Some("mac:0a0b".into()),
    };
    assert_eq!(told.arrived, [arrival]);

    // The Gateway hands the list in whole after every change: the same Mac again is no news.
    keeper.hold_claims(vec![waiting_to_pair(why)]);
    assert_eq!(keeper.changed(&nowhere, now()), None);
    // It asks him, so `open_call` does not open it.
    keeper.open("mac:0a0b", &nowhere, now());
    assert!(!keeper.standing(&nowhere, now())[0].opened);

    // Confirmed, declined, past its time or the Gateway gone: handed in no more.
    keeper.hold_claims(Vec::new());
    let gone = keeper.changed(&nowhere, now()).expect("it went");
    assert!(gone.calls.is_empty() && gone.arrived.is_empty(), "{gone:?}");
}

#[test]
fn a_mac_that_asked_before_the_calls_were_seeded_is_still_news() {
    let mut keeper = Keeper::default();
    keeper.hold_claims(vec![waiting_to_pair("Press Confirm to pair Work with this Mac")]);
    keeper.seed(&[], MergeLineViews::default());
    let told = keeper.changed(&nowhere, now()).expect("a call");
    assert_eq!(told.arrived.len(), 1, "{told:?}");
}
