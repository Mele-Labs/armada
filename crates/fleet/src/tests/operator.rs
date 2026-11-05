//! The Operator's core: its call is kept started on the Operator model and holds no tools, the
//! day's ceiling stops the calls and says so, a ruling naming what Fleet did not offer is refused,
//! and Fleet keeps the Log in its store, signed by the Operator. The acceptance test
//! `crates/acceptance/tests/operator.rs` drives the batch and the failed and late calls.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{Environment, Model};
use core_model::{Actor, Timestamp};
use testkit::FakeJudge;

use super::session_host::rig_placing;
use crate::operator::{
    Acting, Answering, Arrival, Arrived, Calling, Desk, Done, FleetDesk, InLine, KeptStarted, LiveJob, Logged, Operator, Outcome, PassedOn, Place, Ruled,
    Source, Way, FIRST_WRITE_WAIT,
};
use crate::{CallFailed, JudgeBudget};

const NOW: &str = "2026-11-04T10:00:00.000Z";

fn model(name: &str) -> Option<Model> {
    Model::named(name).ok()
}

#[tokio::test]
async fn the_call_is_kept_started_on_the_operator_model_and_holds_no_tools() {
    let judge = Arc::new(FakeJudge::saying("item: 1\nway: 1\njob: none\nshared: none\nbecause: nothing overlaps"));
    let named = Arc::new(Mutex::new("the-operator-model".to_string()));
    let reading = Arc::clone(&named);
    let call = KeptStarted::new(
        judge.clone(),
        move || model(&reading.lock().expect("not poisoned")),
        Environment::nothing(),
        JudgeBudget::of(Duration::from_secs(10)),
    );

    call.ready().await.expect("started");
    assert_eq!(judge.models().len(), 1, "one process waits before anything arrives");
    call.ready().await.expect("still waiting");
    assert_eq!(judge.models().len(), 1, "and only one");

    let answer = call.ask("Which way?".into()).await.expect("answered");
    assert!(answer.contains("nothing overlaps"), "the question went to the process that waited: {answer}");
    assert_eq!(judge.models().len(), 2, "the next one started while the call was out");
    call.ask("Which way now?".into()).await.expect("answered");
    assert_eq!(judge.models().len(), 3);

    *named.lock().expect("not poisoned") = "another-model".into();
    call.ask("And now?".into()).await.expect("answered");
    assert_eq!(
        judge.models(),
        ["the-operator-model", "the-operator-model", "the-operator-model", "another-model", "another-model"],
        "a process on the old model is not used once the setting moves"
    );
    assert!(judge.directories().iter().all(Option::is_none), "no call is given anything to read");
}

#[tokio::test]
async fn a_call_kept_started_that_fails_says_why() {
    let call = KeptStarted::new(
        Arc::new(FakeJudge::that_fails("the network")),
        || model("the-operator-model"),
        Environment::nothing(),
        JudgeBudget::of(Duration::from_secs(10)),
    );
    let failed = call.ask("Which way?".into()).await.expect_err("it failed");
    assert!(matches!(failed, CallFailed::Refused { .. }), "{failed:?}");
}

/// A call that answers in turn and counts itself.
struct Answers {
    replies: Mutex<Vec<String>>,
    asked: Mutex<Vec<String>>,
}

impl Answers {
    fn of(replies: &[&str]) -> Arc<Answers> {
        Arc::new(Answers { replies: Mutex::new(replies.iter().rev().map(|one| one.to_string()).collect()), asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> usize {
        self.asked.lock().expect("not poisoned").len()
    }
}

impl Calling for Answers {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let reply = self.replies.lock().expect("not poisoned").pop();
        Box::pin(async move { reply.ok_or(CallFailed::SaidNothing) })
    }
}

/// A Desk held in memory, at a fixed instant.
struct Held {
    ceiling: u32,
    live: Vec<LiveJob>,
    waiting: Vec<InLine>,
    log: Mutex<Vec<Logged>>,
    acts: Mutex<Vec<String>>,
}

impl Held {
    fn with(ceiling: u32) -> Arc<Held> {
        // The other Job shares the terminal pane with every start below, so each is put to a call.
        let live = [("01OTHER", "panes/"), ("01THIS", "panes/terminal.tsx")]
            .into_iter()
            .map(|(id, path)| LiveJob {
                session_id: id.into(),
                title: id.into(),
                manifest_id: "01REPO".into(),
                words: String::new(),
                paths: vec![path.into()],
                issues: Vec::new(),
            })
            .collect();
        Arc::new(Held { ceiling, live, waiting: Vec::new(), log: Mutex::new(Vec::new()), acts: Mutex::new(Vec::new()) })
    }

    /// [`Held::with`], with `waiting` waiting to start in the repository, in that order.
    fn with_waiting(ceiling: u32, waiting: &[&str]) -> Arc<Held> {
        let waiting = waiting
            .iter()
            .map(|id| InLine { session_id: (*id).into(), title: format!("Job {id}"), manifest_id: "01REPO".into() })
            .collect();
        let held = Held::with(ceiling);
        Arc::new(Held { waiting, ..Arc::into_inner(held).expect("one holder") })
    }

    fn acts(&self) -> Vec<String> {
        self.acts.lock().expect("not poisoned").clone()
    }

    fn log(&self) -> Vec<Logged> {
        self.log.lock().expect("not poisoned").clone()
    }
}

impl Desk for Held {
    fn now(&self) -> Timestamp {
        Timestamp::from_rfc3339(NOW)
    }

    fn day_began(&self) -> Done<'_, Timestamp> {
        Box::pin(async move { Timestamp::from_rfc3339(format!("{}T00:00:00.000Z", &NOW[..10])) })
    }

    fn daily_calls(&self) -> u32 {
        self.ceiling
    }

    fn live(&self) -> Done<'_, Vec<LiveJob>> {
        Box::pin(async move { self.live.clone() })
    }

    fn calls_since(&self, since: Timestamp) -> Done<'_, u32> {
        Box::pin(async move {
            let log = self.log.lock().expect("not poisoned");
            log.iter().filter(|row| row.at.as_str() >= since.as_str()).filter_map(|row| row.call.clone()).collect::<BTreeSet<_>>().len() as u32
        })
    }

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move { self.log.lock().expect("not poisoned").push(row) })
    }

    fn waiting(&self) -> Done<'_, Vec<InLine>> {
        Box::pin(async move { self.waiting.clone() })
    }
}

impl Acting for Held {
    fn go_ahead(&self, session: &str) {
        self.acts.lock().expect("not poisoned").push(format!("go_ahead {session}"));
    }

    fn hand_over(&self, session: &str, _group: Option<&str>, into: &str, _shared: &[String]) {
        self.acts.lock().expect("not poisoned").push(format!("hand_over {session} {into}"));
    }

    fn merge(&self, ending: &str, into: &str, _shared: &[String]) {
        self.acts.lock().expect("not poisoned").push(format!("merge {ending} {into}"));
    }

    fn keep_in_touch(&self, one: &str, other: &str, _shared: &[String]) {
        self.acts.lock().expect("not poisoned").push(format!("keep_in_touch {one} {other}"));
    }

    fn place(&self, session: &str, place: &Place) {
        self.acts.lock().expect("not poisoned").push(format!("place {session} {}", place.word()));
    }

    fn hold(&self, held: &str, first: &str, _shared: &[String]) {
        self.acts.lock().expect("not poisoned").push(format!("hold {held} until {first}"));
    }
}

fn start(id: &str, ways: &[Way]) -> Arrival {
    Arrival {
        id: format!("start:{id}"),
        arrived: Arrived::Start,
        source: Source::Dispatch,
        session_id: "01THIS".into(),
        manifest_id: "01REPO".into(),
        subject: format!("Job {id}"),
        words: "Fix the terminal pane".into(),
        paths: vec!["panes/terminal.tsx".into()],
        issues: Vec::new(),
        group: None,
        ways: ways.to_vec(),
        answers: Vec::new(),
        rule: None,
    }
}

#[tokio::test]
async fn past_the_days_ceiling_no_call_is_made_and_each_item_passes_on_saying_so() {
    let call = Answers::of(&["item: 1\nway: 1\njob: none\nshared: none\nbecause: nothing overlaps"]);
    let desk = Held::with(1);
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());

    let first = operator.ruled(start("a", Way::STARTING), FIRST_WRITE_WAIT).await;
    let second = operator.ruled(start("b", Way::STARTING), FIRST_WRITE_WAIT).await;

    assert!(matches!(first, Ruled::Ruled(_)), "{first:?}");
    assert_eq!(second, Ruled::PassedOn(PassedOn::Ceiling(1)));
    assert_eq!(call.asked(), 1, "no call past the ceiling");
    let log = desk.log();
    assert_eq!((log[1].outcome, log[1].call.as_deref()), (Outcome::PassedOn, None));
    assert!(log[1].because.contains("ceiling"), "{}", log[1].because);
    assert_eq!(desk.acts(), ["go_ahead 01THIS"], "a new Job goes ahead, and one passed on is not acted on");

    let none = Held::with(0);
    let operator = Operator::new(Answers::of(&[]), none.clone(), none.clone());
    assert_eq!(operator.ruled(start("c", Way::STARTING), FIRST_WRITE_WAIT).await, Ruled::PassedOn(PassedOn::Ceiling(0)));
}

#[tokio::test]
async fn a_ruling_naming_what_fleet_did_not_offer_is_refused_and_the_item_passes_on() {
    let call = Answers::of(&[
        "item: 1\nway: 7\njob: none\nshared: none\nbecause: a way it was not offered",
        "item: 1\nway: 3\njob: 01GONE\nshared: panes/terminal.tsx\nbecause: a Job that is not live",
        "item: 1\nway: 4\njob: 01OTHER\nshared: none\nbecause: nothing named as shared",
        "item: 1\nway: 3\njob: 01THIS\nshared: panes/terminal.tsx\nbecause: its own Job",
        "way: 1\nbecause: no item named",
    ]);
    let desk = Held::with(100);
    let operator = Operator::new(call, desk.clone(), desk.clone());

    for id in ["a", "b", "c", "d"] {
        let ruled = operator.ruled(start(id, Way::STARTING), FIRST_WRITE_WAIT).await;
        assert!(matches!(ruled, Ruled::PassedOn(PassedOn::Refused(_))), "{id}: {ruled:?}");
    }
    let ruled = operator.ruled(start("e", Way::STARTING), FIRST_WRITE_WAIT).await;
    assert!(matches!(ruled, Ruled::PassedOn(PassedOn::Unreadable(_))), "{ruled:?}");
    assert!(desk.log().iter().all(|row| row.outcome == Outcome::PassedOn && row.other.is_none()));
    assert!(desk.acts().is_empty(), "a refused ruling is not acted on: {:?}", desk.acts());
}

#[tokio::test]
async fn an_item_fleets_rules_leave_one_way_is_ruled_without_a_call() {
    let call = Answers::of(&[]);
    let desk = Held::with(100);
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());

    let ruled = operator.ruled(start("a", &[Way::Raise]), FIRST_WRITE_WAIT).await;

    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::Raise), "{ruled:?}");
    assert_eq!(call.asked(), 0);
    assert_eq!((desk.log()[0].outcome, desk.log()[0].call.as_deref()), (Outcome::Ruled(Way::Raise), None), "logged all the same");
}

#[tokio::test]
async fn fleet_keeps_the_log_in_its_store_signed_by_the_operator_and_counts_the_days_calls() {
    let rig = rig_placing(None);
    let on = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operator".to_string(), Some(ipc::SettingValue::Bool(true)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), on).await.expect("the Operator turned on, which ships off");
    let id = rig.start().await;
    let said = Arc::new(Mutex::new(Vec::new()));
    let saying = Arc::clone(&said);
    let desk = FleetDesk::on(&rig.fleet, move |why| saying.lock().expect("not poisoned").push(why.to_string()));

    let live = desk.live().await;
    assert!(live.iter().any(|one| one.session_id == id.as_str()), "a hosted Job not ended is live: {live:?}");
    assert_eq!(desk.daily_calls(), 100, "the setting as it ships");

    let at = desk.now();
    let row = Logged {
        arrival: format!("start:{}", id.as_str()),
        arrived: Arrived::Start,
        source: Source::Job("01PARENT".into()),
        session_id: id.as_str().into(),
        manifest_id: rig.manifest.as_str().into(),
        subject: "Agent Opens Terminal in Tab".into(),
        outcome: Outcome::Ruled(Way::Merge),
        other: Some("01OTHER".into()),
        shared: vec!["panes/terminal.tsx".into()],
        because: "the same group is in the other's plan".into(),
        call: Some("call-1".into()),
        by: Actor::Operator,
        at: at.clone(),
        place: Some(Place::Behind("01WAITING".into())),
        first: Some("01OTHER".into()),
        refused: vec!["a part refused".into()],
        answer: None,
    };
    desk.keep(row.clone()).await;

    let kept = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!(kept, vec![row.kept()]);
    assert_eq!((kept[0].by.as_str(), kept[0].source.as_str(), kept[0].source_session.as_deref()), ("operator", "job", Some("01PARENT")));
    assert_eq!(kept[0].outcome, "merge");
    assert_eq!(desk.calls_since(at).await, 1);
    assert!(said.lock().expect("not poisoned").is_empty(), "nothing went wrong");
}

#[tokio::test]
async fn the_days_ceiling_counts_from_the_machines_midnight_and_not_utcs() {
    let millis = |at: &str| Timestamp::from_rfc3339(at).epoch_millis().expect("an instant");
    let began = |at: &str, offset: i64| crate::clock::rfc3339_utc(crate::clock::day_began(millis(at), offset));

    assert_eq!(began("2026-11-04T03:00:00.000Z", -5 * 3_600), "2026-11-03T05:00:00.000Z", "10 pm in New York is still the 3rd");
    assert_eq!(began("2026-11-04T06:00:00.000Z", -5 * 3_600), "2026-11-04T05:00:00.000Z");
    assert_eq!(began("2026-11-04T03:00:00.000Z", 9 * 3_600), "2026-11-03T15:00:00.000Z", "noon in Tokyo began at its midnight");
    assert_eq!(began("2026-11-04T03:00:00.000Z", 0), "2026-11-04T00:00:00.000Z");

    // The machine's own zone, whatever it is: a midnight within the last day.
    let now = millis(NOW);
    let local = crate::clock::local_day_began(now).await;
    assert!(local <= now && now - local < 86_400_000, "{}", crate::clock::rfc3339_utc(local));
}

#[tokio::test]
async fn a_start_where_jobs_wait_to_start_is_placed_among_them_and_one_naming_a_job_not_offered_goes_last() {
    let call = Answers::of(&[
        "item: 1\nway: 1\njob: none\nshared: none\nplace: ahead of 01WAITB\nbecause: it is small and the others wait on it",
        "item: 1\nway: 1\njob: none\nshared: none\nplace: ahead of 01GONE\nbecause: a Job not waiting",
    ]);
    let desk = Held::with_waiting(100, &["01WAITA", "01WAITB"]);
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());

    // One way left, and still put to a call: others wait, so it has a place to be given.
    let ruled = operator.ruled(start("a", &[Way::NewJob]), FIRST_WRITE_WAIT).await;
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.place == Some(Place::Ahead("01WAITB".into())) && ruling.refused.is_empty()), "{ruled:?}");
    let asked = call.asked.lock().expect("not poisoned")[0].clone();
    assert!(asked.contains("Waiting to start in its repository") && asked.contains("1. Job 01WAITA") && asked.contains("2. Job 01WAITB"), "{asked}");

    let ruled = operator.ruled(start("b", &[Way::NewJob]), FIRST_WRITE_WAIT).await;
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::NewJob && ruling.place == Some(Place::Last)), "the way stands: {ruled:?}");
    let log = desk.log();
    assert_eq!((log[0].place.clone(), log[1].place.clone()), (Some(Place::Ahead("01WAITB".into())), Some(Place::Last)));
    assert!(log[1].refused.iter().any(|one| one.contains("01GONE") && one.contains("last")), "the Log says what was refused: {:?}", log[1].refused);
    assert_eq!(desk.acts(), ["go_ahead 01THIS", "place 01THIS ahead:01WAITB", "go_ahead 01THIS", "place 01THIS last"]);

    // Nobody waits: no place is asked, and first come, first served stands.
    let none = Held::with(100);
    let operator = Operator::new(Answers::of(&[]), none.clone(), none.clone());
    let ruled = operator.ruled(start("c", &[Way::NewJob]), FIRST_WRITE_WAIT).await;
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.place.is_none()), "{ruled:?}");
}

#[tokio::test]
async fn of_two_kept_in_touch_the_one_named_first_builds_and_a_first_outside_the_pair_holds_nothing() {
    let keep = |first: &str| format!("item: 1\nway: 4\njob: 01OTHER\nshared: panes/terminal.tsx\nfirst: {first}\nbecause: one builds the pane the other calls");
    let call = Answers::of(&[&keep("01OTHER"), &keep("01THIS"), &keep("01GONE"), &keep("none")]);
    let desk = Held::with(100);
    let operator = Operator::new(call, desk.clone(), desk.clone());

    for id in ["a", "b", "c", "d"] {
        let ruled = operator.ruled(start(id, Way::STARTING), FIRST_WRITE_WAIT).await;
        assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::KeepInTouch), "the keep in touch stands: {ruled:?}");
    }

    let acts: Vec<String> = desk.acts().into_iter().filter(|one| one.starts_with("hold")).collect();
    assert_eq!(acts, ["hold 01THIS until 01OTHER", "hold 01OTHER until 01THIS"], "the other of the two is held, and only for one of the pair");
    let log = desk.log();
    assert_eq!(log.iter().map(|row| row.first.as_deref()).collect::<Vec<_>>(), [Some("01OTHER"), Some("01THIS"), None, None]);
    assert!(log[2].refused.iter().any(|one| one.contains("01GONE") && one.contains("nothing is held")), "{:?}", log[2].refused);
    assert!(log[3].refused.is_empty(), "none is no refusal: both build at once");
}
