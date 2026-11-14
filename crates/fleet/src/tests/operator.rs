//! The Operator's core: its call is kept started on the Operator model and holds no tools, the
//! week's plan used to the pause stops the calls and says so, a ruling naming what Fleet did not offer is refused,
//! and Fleet keeps the Log in its store, signed by the Operator. The acceptance test
//! `crates/acceptance/tests/operator.rs` drives the batch and the failed and late calls.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{Environment, Model};
use core_model::{Actor, Timestamp};
use testkit::FakeJudge;

use super::session_host::rig_placing;
use crate::operator::{
    carried, Acted, Acting, Answering, Arrival, Arrived, Calling, Desk, Done, FleetDesk, KeptStarted, LiveJob, Logged, Operator, Outcome, PassedOn, Paused, Place, Ruled, Source, Way,
    NEEDS_YOU_WAIT,
};
use crate::{CallFailed, JudgeBudget};

/// The ways a bug report is offered where an open issue names none: 1 new job, 2 existing job, 3 raise.
const OFFERED: &[Way] = &[Way::NewJob, Way::ExistingJob, Way::Raise];

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
    paused: Option<Paused>,
    live: Vec<LiveJob>,
    log: Mutex<Vec<Logged>>,
    acts: Mutex<Vec<String>>,
}

impl Held {
    fn with() -> Arc<Held> {
        // The other Job shares the terminal pane with every report below, so each is put to a call.
        let live = [("01OTHER", "panes/"), ("01THIS", "panes/terminal.tsx")]
            .into_iter()
            .map(|(id, path)| LiveJob {
                session_id: id.into(),
                title: id.into(),
                manifest_id: "01REPO".into(),
                words: String::new(),
                paths: vec![path.into()],
                issues: Vec::new(),
                running: Vec::new(),
                started_by: Vec::new(),
                shared: false,
            })
            .collect();
        Arc::new(Held { paused: None, live, log: Mutex::new(Vec::new()), acts: Mutex::new(Vec::new()) })
    }

    /// [`Held::with`], paused for the week's plan.
    fn paused(paused: Paused) -> Arc<Held> {
        Arc::new(Held { paused: Some(paused), ..Arc::into_inner(Held::with()).expect("one holder") })
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

    fn plan_pause(&self) -> Done<'_, Option<Paused>> {
        Box::pin(async move { self.paused })
    }

    fn live(&self) -> Done<'_, Vec<LiveJob>> {
        Box::pin(async move { self.live.clone() })
    }

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move { self.log.lock().expect("not poisoned").push(row) })
    }
}

impl Acting for Held {
    fn sourced(&self, arrival: &Arrival, ruling: &crate::operator::Ruling) -> Acted<'_> {
        self.acts.lock().expect("not poisoned").push(format!("{} {}", ruling.way.word(), arrival.session_id));
        carried()
    }
}

/// A bug report from Job 01THIS, about the terminal pane.
fn start(id: &str, ways: &[Way]) -> Arrival {
    Arrival {
        id: format!("report:{id}"),
        arrived: Arrived::Report,
        source: Source::Agent,
        session_id: "01THIS".into(),
        manifest_id: "01REPO".into(),
        subject: format!("Report {id}"),
        words: "Fix the terminal pane".into(),
        paths: vec!["panes/terminal.tsx".into()],
        issues: Vec::new(),
        ways: ways.to_vec(),
        open: Vec::new(),
    }
}

#[tokio::test]
async fn while_paused_a_review_makes_no_call_and_every_item_comes_back_with_no_read() {
    let call = Answers::of(&["item: 1\nread: It asks to approve the plan.\npick: Yes"]);
    let desk = Held::paused(Paused { week: 83, at: 80 });
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());
    let item = ipc::WaitingItem {
        approves: false,
        id: "plan".into(),
        text: "Approve the plan?".into(),
        since: ipc::Instant::carried(NOW),
        source: ipc::WaitingSource::Agent,
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
    };
    let items = [crate::operator::ToRaise { session: "01THIS".into(), manifest_id: String::new(), job: Some("Build the pane".into()), item, go_ahead: false, thens: Vec::new(), vet: None }];

    let reviewed = operator.reviewed(&items).await;

    assert_eq!(reviewed.reads, vec![None]);
    assert!(reviewed.failed.is_none(), "a pause is no failure");
    assert_eq!(call.asked(), 0, "no call while paused");
}

#[tokio::test]
async fn while_paused_no_call_is_made_and_each_item_passes_on_saying_so() {
    let call = Answers::of(&["item: 1\nway: 1\njob: none\nshared: none\nbecause: nothing overlaps"]);
    let desk = Held::with();
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());
    let first = operator.ruled(start("a", OFFERED), NEEDS_YOU_WAIT).await;
    assert!(matches!(first, Ruled::Ruled(_)), "{first:?}");

    let paused = Paused { week: 83, at: 80 };
    let desk = Held::paused(paused);
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());
    let second = operator.ruled(start("b", OFFERED), NEEDS_YOU_WAIT).await;

    assert_eq!(second, Ruled::PassedOn(PassedOn::Paused(paused)));
    assert_eq!(call.asked(), 1, "no call while paused");
    let log = desk.log();
    assert_eq!((log[0].outcome, log[0].call.as_deref()), (Outcome::PassedOn, None));
    assert!(log[0].because.contains("83% used") && log[0].because.contains("pause at 80%"), "{}", log[0].because);
    assert!(desk.acts().is_empty(), "one passed on is not acted on");
}

#[tokio::test]
async fn a_ruling_naming_what_fleet_did_not_offer_is_refused_and_the_item_passes_on() {
    let call = Answers::of(&[
        "item: 1\nway: 7\njob: none\nshared: none\nbecause: a way it was not offered",
        "item: 1\nway: 2\njob: 01GONE\nshared: panes/terminal.tsx\nbecause: a Job that is not live",
        "item: 1\nway: 2\njob: 01OTHER\nshared: none\nbecause: nothing named as shared",
        "item: 1\nway: 2\njob: 01THIS\nshared: panes/terminal.tsx\nbecause: its own Job",
        "way: 1\nbecause: no item named",
    ]);
    let desk = Held::with();
    let operator = Operator::new(call, desk.clone(), desk.clone());

    for id in ["a", "b", "c", "d"] {
        let ruled = operator.ruled(start(id, OFFERED), NEEDS_YOU_WAIT).await;
        assert!(matches!(ruled, Ruled::PassedOn(PassedOn::Refused(_))), "{id}: {ruled:?}");
    }
    let ruled = operator.ruled(start("e", OFFERED), NEEDS_YOU_WAIT).await;
    assert!(matches!(ruled, Ruled::PassedOn(PassedOn::Unreadable(_))), "{ruled:?}");
    assert!(desk.log().iter().all(|row| row.outcome == Outcome::PassedOn && row.other.is_none()));
    assert!(desk.acts().is_empty(), "a refused ruling is not acted on: {:?}", desk.acts());
}

#[tokio::test]
async fn an_item_fleets_rules_leave_one_way_is_ruled_without_a_call() {
    let call = Answers::of(&[]);
    let desk = Held::with();
    let operator = Operator::new(call.clone(), desk.clone(), desk.clone());

    let ruled = operator.ruled(start("a", &[Way::Raise]), NEEDS_YOU_WAIT).await;

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

    let at = desk.now();
    let row = Logged {
        arrival: format!("report:{}", id.as_str()),
        arrived: Arrived::Report,
        source: Source::Job("01PARENT".into()),
        session_id: id.as_str().into(),
        manifest_id: rig.manifest.as_str().into(),
        subject: "Agent Opens Terminal in Tab".into(),
        outcome: Outcome::Ruled(Way::ExistingJob),
        other: Some("01OTHER".into()),
        shared: vec!["panes/terminal.tsx".into()],
        because: "the same group is in the other's plan".into(),
        call: Some("call-1".into()),
        by: Actor::Operator,
        handed: at.clone(),
        at: at.clone(),
        place: Some(Place::Behind("01WAITING".into())),
        refused: vec!["a part refused".into()],
        issue: None,
    };
    desk.keep(row.clone()).await;

    let kept = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!(kept, vec![row.kept()]);
    assert_eq!((kept[0].by.as_str(), kept[0].source.as_str(), kept[0].source_session.as_deref()), ("operator", "job", Some("01PARENT")));
    assert_eq!(kept[0].outcome, "existing_job");
    assert!(said.lock().expect("not poisoned").is_empty(), "nothing went wrong");
}

#[tokio::test]
async fn a_day_begins_at_the_machines_midnight_and_not_utcs() {
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
