//! A question a Job waits on, as Fleet reads it from state and hands it to the Operator: the seen
//! mark of the plan group the Job works on, the person's own earlier answer to the same, and
//! whether Autopilot runs for it. An answer settles the item as his press would and says so in
//! the Job's thread; a raise leaves it on his list; each arrives once; Off hands nothing.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use api::{HostedSessions, Sessions};
use ipc::{AnswerWaiting, AutopilotDecision, PlanGroupInput, PlanTaskInput, RecordPlan, SessionId};

use super::autopilot_decisions::{item, keep, permission_asked, rig_calling, started, told, waiting, SURE_OF_THE_FIRST};
use super::rule_acting::notices_in;
use super::session_host::{eventually, rig_placing, Rig};
use super::session_rail::record;
use crate::operator::{Answering, Calling, Group, Present, Way};
use crate::CallFailed;

const ASKED: &str = "Which clock do the tests use?";

async fn switched(rig: &Rig, key: &str, on: bool) {
    let value = match key {
        "features.operator" => ipc::SettingValue::Bool(on),
        _ => ipc::SettingValue::Text(if on { "on" } else { "off" }.to_string()),
    };
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(value))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

async fn positioned(rig: &Rig, key: &str, position: &str) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(ipc::SettingValue::Text(position.to_string())))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

async fn answering(rig: &Rig) {
    switched(rig, "features.operator", true).await;
    switched(rig, "features.operatorAnswer", true).await;
}

/// A call that answers each question with the next of its answers, and keeps what it was asked.
#[derive(Default)]
struct Answers {
    answers: Mutex<VecDeque<String>>,
    asked: Mutex<Vec<String>>,
}

impl Answers {
    fn with(answers: &[&str]) -> Arc<Answers> {
        Arc::new(Answers { answers: Mutex::new(answers.iter().map(|one| one.to_string()).collect()), asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl Calling for Answers {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let answer = self.answers.lock().expect("not poisoned").pop_front();
        Box::pin(async move { answer.ok_or(CallFailed::SaidNothing) })
    }
}

/// A call that counts what it is asked and answers nothing.
#[derive(Default)]
struct Counting(AtomicUsize);

impl Calling for Counting {
    fn ask(&self, _question: String) -> Answering<'_> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Err(CallFailed::SaidNothing) })
    }
}

/// A call whose first ask is held out until released and then says nothing; every later ask answers
/// the first option.
#[derive(Default)]
struct Gated {
    asks: AtomicUsize,
    release: tokio::sync::Notify,
}

impl Calling for Gated {
    fn ask(&self, _question: String) -> Answering<'_> {
        let first = self.asks.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move {
            if !first {
                return Ok("item: 1\nway: 1\nanswer: 1\nbecause: he left it to the recommended option".to_string());
            }
            self.release.notified().await;
            Err(CallFailed::SaidNothing)
        })
    }
}

/// `id`'s plan: one group, "Clock", which the agent marks seen or not.
async fn planned(rig: &Rig, id: &SessionId, seen: bool) {
    let plan = RecordPlan {
        summary: "You get tests that do not wait on the wall clock.".into(),
        groups: vec![PlanGroupInput {
            id: None,
            title: "Clock".into(),
            agent: "rust-engineer".into(),
            paths: vec!["crates/fleet/src/clock.rs".into()],
            issue: None,
            gives: None,
            lands_in: None,
            own_job: false,
            tasks: vec![PlanTaskInput { id: None, title: "Freeze the clock in tests".into() }],
        }],
        sketch: None,
        walk: None,
        session_id: Some(id.clone()),
    };
    rig.fleet.record_plan_seen(None, plan, false, vec![seen]).await.expect("recorded");
}

/// The row read, as a publish reads it, and the questions it noted handed on.
async fn read_and_handed(rig: &Rig, id: &SessionId) {
    let _ = record(rig, id).await;
    rig.fleet.questions_handed().await;
}

async fn said(rig: &Rig, id: &SessionId) -> Vec<String> {
    notices_in(rig, id).await.into_iter().map(|(_, text)| text).collect()
}

/// The Log's rows on questions; a recorded plan's groups are placed beside them.
async fn log_of(rig: &Rig) -> Vec<store::KeptRuling> {
    let rows = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    rows.into_iter().filter(|row| row.arrival.starts_with("asked:")).collect()
}

#[tokio::test]
async fn a_question_from_a_job_on_a_group_not_seen_is_answered_as_his_press_would_with_a_row_in_its_thread() {
    let rig = rig_placing(None);
    // Answer is the Operator's own: Dispatch stays off.
    answering(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: a frozen clock is the one the agent recommends"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;

    read_and_handed(&rig, &id).await;

    eventually(|| async { !waiting(&rig, &id).await.contains(&"a".to_string()) }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    for part in ["An agent asked a question, from its own agent", "It works on plan group \"Clock\", which changes nothing the person sees", "The answers it may give: 1. \"Frozen (Recommended)\""] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(told(&rig, &id).await.last().unwrap().starts_with(&format!("Re: {ASKED}\nFrozen (Recommended)")), "the agent hears his press");
    eventually(|| async { said(&rig, &id).await.iter().any(|one| one.contains("The Operator answered \"Frozen (Recommended)\": a frozen clock")) }).await;
    let log = log_of(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].arrived.as_str(), log[0].outcome.as_str(), log[0].by.as_str()), ("question", "answer", "operator"));
    assert_eq!((log[0].subject.as_str(), log[0].answer.as_deref()), (ASKED, Some("Frozen (Recommended)")));
}

#[tokio::test]
async fn a_question_from_a_job_on_a_seen_group_is_raised_with_no_call_and_arrives_once_however_often_its_row_is_read() {
    let rig = rig_placing(None);
    answering(&rig).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = rig.start().await;
    planned(&rig, &id, true).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;

    for _ in 0..3 {
        read_and_handed(&rig, &id).await;
    }
    eventually(|| async { !log_of(&rig).await.is_empty() }).await;
    read_and_handed(&rig, &id).await;

    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "Fleet's rules leave it only to him");
    let log = log_of(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].outcome.as_str(), log[0].arrival.clone()), ("raise", format!("asked:{}:a", id.as_str())));
    assert!(log[0].because.contains("\"Clock\", which the person sees"), "{}", log[0].because);
    assert!(waiting(&rig, &id).await.contains(&"a".to_string()), "it stays on his list");
    let plan = rig.fleet.store().lock().await.session_plan(id.as_str()).expect("read").expect("kept");
    assert!(plan.groups[0].seen && plan.seen, "the group's mark is kept, and a group he sees makes a plan he sees");
}

#[tokio::test]
async fn his_own_answer_is_kept_and_a_second_job_asking_the_same_is_offered_it_naming_the_first() {
    let rig = rig_placing(None);
    answering(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: he answered the same"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let first = rig.start().await;
    planned(&rig, &first, true).await;
    keep(&rig, &first, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    let _ = record(&rig, &first).await;
    let press = AnswerWaiting { session_id: first.clone(), item_id: "a".into(), choice: Some(1), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(press).await.expect("answered");
    let log = log_of(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].outcome.as_str(), log[0].by.as_str(), log[0].answer.as_deref()), ("answered", "human", Some("Fake")));

    // The second has recorded no plan, so only his earlier answer may be given.
    let second = rig.start().await;
    keep(&rig, &second, &[item("b", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    read_and_handed(&rig, &second).await;

    eventually(|| async { waiting(&rig, &second).await.is_empty() }).await;
    let asked = call.asked();
    assert!(asked[0].contains(&format!("1. \"Fake\", the person's answer when Job {}", first.as_str())), "{}", asked[0]);
    assert!(told(&rig, &second).await.last().unwrap().starts_with(&format!("Re: {ASKED}\nFake")));
    eventually(|| async { said(&rig, &second).await.iter().any(|one| one.contains("The Operator answered \"Fake\", your answer when")) }).await;
    let row = log_of(&rig).await.into_iter().find(|row| row.session_id == second.as_str()).expect("a row");
    assert_eq!((row.other_session.as_deref(), row.answer.as_deref()), (Some(first.as_str()), Some("Fake")));
}

#[tokio::test]
async fn with_the_operator_off_no_question_is_handed_and_his_press_is_not_kept() {
    let rig = rig_placing(None);
    assert!(!rig.fleet.operator_on(), "the Operator ships off");
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let id = rig.start().await;
    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"]), item("b", "Which runner?", &["Nextest (Recommended)", "Cargo"])]).await;

    read_and_handed(&rig, &id).await;
    read_and_handed(&rig, &id).await;
    let press = AnswerWaiting { session_id: id.clone(), item_id: "a".into(), choice: Some(0), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(press).await.expect("answered");

    assert_eq!(counted.0.load(Ordering::SeqCst), 0, "no call");
    assert!(log_of(&rig).await.is_empty(), "nothing in the Log");
    let left = waiting(&rig, &id).await;
    assert!(!left.contains(&"a".to_string()) && left.contains(&"b".to_string()), "only his press settled anything: {left:?}");

    // The same item with the Operator on is handed, so the silence above is the switch's.
    answering(&rig).await;
    read_and_handed(&rig, &id).await;
    eventually(|| async { counted.0.load(Ordering::SeqCst) == 1 }).await;
}

#[tokio::test]
async fn while_autopilot_runs_he_is_away_its_setup_widens_answer_and_its_pass_leaves_questions_to_the_operator() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    answering(&rig).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "the Operator answers it");
    assert!(judge.asked().is_empty(), "and Autopilot's own call is not made");

    let asked = record(&rig, &id).await.waiting_for.into_iter().next().expect("waiting");
    let question = rig.fleet.question_of(id.as_str(), &asked).await.expect("a question with an Answer path");
    assert_eq!(question.present, Present::Away { lets: vec![AutopilotDecision::Recommended], instruction: "Fix what is open".into() });
    assert_eq!(question.group, Group::NoPlan);
    let arrival = question.arrival().expect("the Operator is on");
    assert_eq!(arrival.ways, vec![Way::Answer, Way::Raise], "a Job with no plan may be answered while he is away, as he checked");
    assert!(arrival.words.contains("The person is away, and left this instruction: Fix what is open"), "{}", arrival.words);

    // Here, the same is his.
    let here = crate::operator::Question { present: Present::Here, ..question };
    assert_eq!(here.arrival().expect("on").ways, vec![Way::Raise]);
}

#[tokio::test]
async fn with_answer_off_a_question_is_ruled_into_the_log_and_left_on_his_list_as_dispatch_off_leaves_a_start() {
    let rig = rig_placing(None);
    switched(&rig, "features.operator", true).await;
    assert!(!rig.fleet.operator_answering("any repository"), "Answer ships Off under the Operator");
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: a frozen clock is the one the agent recommends"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;

    read_and_handed(&rig, &id).await;
    eventually(|| async { !log_of(&rig).await.is_empty() }).await;

    let log = log_of(&rig).await;
    assert_eq!((log[0].outcome.as_str(), log[0].answer.as_deref()), ("answer", Some("Frozen (Recommended)")), "the Log keeps what it would have answered");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(waiting(&rig, &id).await.contains(&"a".to_string()), "nothing is settled");
    assert!(!said(&rig, &id).await.iter().any(|one| one.contains("The Operator answered")), "and the thread says nothing");
}

#[tokio::test]
async fn autopilot_turned_on_offers_what_is_already_waiting_on_him_once_under_its_setup() {
    let rig = rig_placing(None);
    answering(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: he left it to the recommended option"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    planned(&rig, &id, true).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    read_and_handed(&rig, &id).await;
    eventually(|| async { !log_of(&rig).await.is_empty() }).await;
    assert!(call.asked().is_empty(), "raised while he is here, with no call");

    started(&rig, &[AutopilotDecision::Recommended]).await;
    read_and_handed(&rig, &id).await;

    eventually(|| async { !waiting(&rig, &id).await.contains(&"a".to_string()) }).await;
    assert!(told(&rig, &id).await.last().unwrap().starts_with(&format!("Re: {ASKED}\nFrozen (Recommended)")));
    for _ in 0..3 {
        read_and_handed(&rig, &id).await;
    }
    assert_eq!(call.asked().len(), 1, "offered once in the run");
    let outcomes: Vec<String> = log_of(&rig).await.into_iter().map(|row| row.outcome).collect();
    assert_eq!(outcomes, ["answer", "raise"], "newest first");
}

#[tokio::test]
async fn autopilot_turned_on_while_a_call_on_an_item_is_out_offers_it_once_the_call_is_back() {
    let rig = rig_placing(None);
    answering(&rig).await;
    let call = Arc::new(Gated::default());
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    read_and_handed(&rig, &id).await;
    eventually(|| async { call.asks.load(Ordering::SeqCst) == 1 }).await;

    // The run starts while the call is out, and the call comes back after it.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    call.release.notify_one();
    eventually(|| async { !log_of(&rig).await.is_empty() }).await;
    assert_eq!(log_of(&rig).await[0].outcome, "passed_on", "the first call ruled nothing, under the rules from before the run");

    eventually(|| async {
        read_and_handed(&rig, &id).await;
        !waiting(&rig, &id).await.contains(&"a".to_string())
    })
    .await;
    for _ in 0..3 {
        read_and_handed(&rig, &id).await;
    }
    assert_eq!(call.asks.load(Ordering::SeqCst), 2, "offered once in the run, after the call that was out");
}

#[tokio::test]
async fn a_permission_card_for_a_read_is_settled_allow_once_as_his_press_would_and_the_job_goes_on() {
    let rig = rig_placing(None);
    answering(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: it only reads a file in the Job's own slot"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    let held = permission_asked(&rig, &id, "Read", r#"{"file_path":"src/lib.rs"}"#).await;

    read_and_handed(&rig, &id).await;

    tokio::time::timeout(std::time::Duration::from_secs(5), held).await.expect("the card is allowed").expect("joined");
    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    for part in ["It asks to use a tool: Read src/lib.rs", "The answers it may give: 1. \"Allow once\""] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    eventually(|| async { said(&rig, &id).await.iter().any(|one| one.contains("The Operator answered \"Allow once\": it only reads")) }).await;
    let log = log_of(&rig).await;
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!((log[0].arrived.as_str(), log[0].outcome.as_str(), log[0].by.as_str(), log[0].answer.as_deref()), ("card", "answer", "operator", Some("Allow once")));
    for _ in 0..2 {
        read_and_handed(&rig, &id).await;
    }
    assert_eq!(call.asked().len(), 1, "a card arrives once");
}

/// What the Operator made of question `a`, as his list reads it.
async fn read_on_a(rig: &Rig, id: &SessionId) -> Option<ipc::OperatorRead> {
    let read = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read");
    read.session.waiting_for.into_iter().find(|one| one.id == "a").and_then(|one| one.operator_read)
}

#[tokio::test]
async fn with_answer_at_suggest_a_question_shows_the_read_and_the_pick_and_his_press_is_the_one_press() {
    let rig = rig_placing(None);
    switched(&rig, "features.operator", true).await;
    positioned(&rig, "features.operatorAnswer", "suggest").await;
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: a frozen clock is the one the agent recommends"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;

    read_and_handed(&rig, &id).await;
    eventually(|| async { read_on_a(&rig, &id).await.is_some() }).await;

    let read = read_on_a(&rig, &id).await.expect("a read");
    assert_eq!((read.pick.as_str(), read.read.as_str()), ("Frozen (Recommended)", "a frozen clock is the one the agent recommends"));
    assert!(!said(&rig, &id).await.iter().any(|one| one.contains("The Operator answered")), "nothing is settled");
    let said_to = AnswerWaiting { session_id: id.clone(), item_id: "a".into(), choice: Some(0), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).settle_waiting(said_to).await.expect("his press");
    assert!(!waiting(&rig, &id).await.contains(&"a".to_string()));
}

#[tokio::test]
async fn a_repository_stands_apart_from_the_machine_in_either_direction() {
    for (machine, repository, settled) in [("on", "off", false), ("off", "on", true), ("off", "suggest", false)] {
        let rig = rig_placing(None);
        switched(&rig, "features.operator", true).await;
        positioned(&rig, "features.operatorAnswer", machine).await;
        let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: a frozen clock is the one the agent recommends"]);
        assert!(rig.fleet.operator_seated_with(call.clone()));
        let id = rig.start().await;
        let manifest = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted").manifest_id;
        let value = ipc::SettingValue::Map(std::collections::BTreeMap::from([(
            manifest,
            ipc::SettingValue::Map(std::collections::BTreeMap::from([("answer".to_string(), ipc::SettingValue::Text(repository.to_string()))])),
        )]));
        let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operatorRepositories".to_string(), Some(value))]) };
        api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
        planned(&rig, &id, false).await;
        keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;

        read_and_handed(&rig, &id).await;
        eventually(|| async { !log_of(&rig).await.is_empty() }).await;
        match settled {
            true => eventually(|| async { !waiting(&rig, &id).await.contains(&"a".to_string()) }).await,
            false => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                assert!(waiting(&rig, &id).await.contains(&"a".to_string()), "{machine} then {repository}");
            }
        }
        assert_eq!(read_on_a(&rig, &id).await.is_some(), repository == "suggest", "{machine} then {repository}");
    }
}

#[tokio::test]
async fn a_role_off_by_a_standing_order_settles_nothing_until_it_is_removed() {
    use ipc::operator_positions::OperatorRole;
    use ipc::operator_talk::OperatorOrderForm;
    let rig = rig_placing(None);
    answering(&rig).await;
    let call = Answers::with(&["item: 1\nway: 1\nanswer: 1\nbecause: a frozen clock is the one the agent recommends"; 2]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    let manifest_id = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted").manifest_id;
    let form = OperatorOrderForm::RoleOff { role: OperatorRole::Answer, manifest_id, today: true };
    let orders = rig.fleet.operator_order_confirmed(form).await.expect("it stands");
    assert_eq!(orders.orders.len(), 1);
    assert!(orders.orders[0].said.starts_with("Answer off in ") && orders.orders[0].said.ends_with(" today"), "{}", orders.orders[0].said);
    assert!(orders.orders[0].until.is_some(), "today ends");

    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    read_and_handed(&rig, &id).await;
    eventually(|| async { !log_of(&rig).await.is_empty() }).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(waiting(&rig, &id).await.contains(&"a".to_string()), "the order holds");

    rig.fleet.operator_order_removed(orders.orders[0].id).await.expect("one press");
    assert!(rig.fleet.operator_orders_wire().await.orders.is_empty());
    assert_eq!(rig.fleet.operator_position(OperatorRole::Answer, &rig.manifest.as_str().to_string()), ipc::operator_positions::OperatorPosition::On);
}

/// A call held out until released, which then answers the first option.
#[derive(Default)]
struct HeldOut {
    asks: AtomicUsize,
    release: tokio::sync::Notify,
}

impl Calling for HeldOut {
    fn ask(&self, _question: String) -> Answering<'_> {
        self.asks.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.release.notified().await;
            Ok("item: 1\nway: 1\nanswer: 1\nbecause: a frozen clock is the one the agent recommends".to_string())
        })
    }
}

#[tokio::test]
async fn a_call_out_when_a_role_moves_goes_by_the_new_position_as_it_comes_back() {
    let rig = rig_placing(None);
    answering(&rig).await;
    let call = Arc::new(HeldOut::default());
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let id = rig.start().await;
    planned(&rig, &id, false).await;
    keep(&rig, &id, &[item("a", ASKED, &["Frozen (Recommended)", "Fake"])]).await;
    read_and_handed(&rig, &id).await;
    eventually(|| async { call.asks.load(Ordering::SeqCst) == 1 }).await;

    positioned(&rig, "features.operatorAnswer", "suggest").await;
    call.release.notify_one();

    eventually(|| async { read_on_a(&rig, &id).await.is_some() }).await;
    assert!(waiting(&rig, &id).await.contains(&"a".to_string()), "On was out of date: nothing is settled");
}
