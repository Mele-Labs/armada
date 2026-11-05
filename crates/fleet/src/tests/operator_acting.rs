//! What Fleet does on the Operator's rulings, through its own ways of telling and ending a Job:
//! nothing while Dispatch is off; a merge ends the Job that has done less into the other, and one
//! that has pushed is raised instead; a hand-over ends a start or moves one group; a keep in touch
//! tells both, and tells each again when the other moves; a start a call merges stops at its first
//! write, having taken no slot.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::Sessions;
use ipc::{PlanGroupInput, PlanTaskInput, RecordPlan, SessionFact, SessionId, SessionReport, SessionState};

use super::rule_acting::notices_in;
use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_rail::record;
use crate::operator::{Acting, Answering, Calling, FleetActing, Source};
use crate::CallFailed;

async fn switched(rig: &Rig, key: &str, on: bool) {
    let value = match key {
        "features.operator" => ipc::SettingValue::Bool(on),
        _ => ipc::SettingValue::Text(if on { "on" } else { "off" }.to_string()),
    };
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(value))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

/// The Operator turned on, which ships off, and its Dispatch role as `on` says.
pub(super) async fn dispatch(rig: &Rig, on: bool) {
    switched(rig, "features.operator", true).await;
    switched(rig, "features.operatorDispatch", on).await;
    switched(rig, "features.operatorDelegate", on).await;
}

async fn told(rig: &Rig, id: &SessionId) -> Vec<String> {
    notices_in(rig, id).await.into_iter().filter(|(notice, _)| notice.as_deref() == Some("operator")).map(|(_, text)| text).collect()
}

async fn said(rig: &Rig, id: &SessionId) -> Vec<String> {
    notices_in(rig, id).await.into_iter().map(|(_, text)| text).collect()
}

async fn ended(rig: &Rig, id: &SessionId) -> bool {
    record(rig, id).await.state == SessionState::Ended
}

/// A Job asked `words`, its first message.
pub(super) async fn asked(rig: &Rig, words: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, words).await;
    id
}

fn plan(id: &SessionId, groups: &[(&str, &str)]) -> RecordPlan {
    RecordPlan {
        summary: "You get the terminal pane back.".into(),
        groups: groups
            .iter()
            .map(|(title, path)| PlanGroupInput {
                id: None,
                title: (*title).into(),
                agent: "rust-engineer".into(),
                paths: vec![(*path).into()],
                issue: None,
                gives: None,
                lands_in: None,
                own_job: false,
                tasks: vec![PlanTaskInput { id: None, title: format!("Build {title}") }],
            })
            .collect(),
        sketch: None,
        walk: None,
        session_id: Some(id.clone()),
    }
}

async fn pushed(rig: &Rig, id: &SessionId) {
    let harness = record(rig, id).await.harness;
    let fact = SessionFact::Pushed { branch: Some("armada/topic".into()) };
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("kept");
    let fleet = Arc::clone(&rig.fleet);
    let id = id.as_str().to_string();
    eventually(|| {
        let (fleet, id) = (Arc::clone(&fleet), id.clone());
        async move { fleet.store().lock().await.session_work(&id, "").is_ok_and(|work| work.first_push.is_some()) }
    })
    .await;
}

pub(super) fn shared() -> Vec<String> {
    vec!["apps/panes/terminal.tsx".to_string()]
}

#[tokio::test]
async fn with_dispatch_off_a_ruling_is_logged_and_tells_and_ends_no_job() {
    let rig = rig_placing(None);
    dispatch(&rig, false).await;
    let other = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let one = asked(&rig, "The terminal scrollback is lost on redraw").await;
    let reply = format!("item: 1\nway: 3\njob: {}\nshared: terminal scrollback\nbecause: the same fix", other.as_str());
    assert!(rig.fleet.operator_seated_with(Arc::new(Saying(Mutex::new(Some(reply))))));

    rig.fleet.start_handed(one.as_str(), Source::Dispatch, "The terminal scrollback is lost on redraw").await;
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.store().lock().await.operator_rulings(10).expect("read").iter().any(|ruling| ruling.outcome == "merge") }
    })
    .await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(!ended(&rig, &one).await, "Dispatch ships off");
    assert!(told(&rig, &one).await.is_empty() && told(&rig, &other).await.is_empty());
}

#[tokio::test]
async fn a_merge_ends_the_job_that_has_done_less_and_sends_its_brief_plan_and_branch_to_the_other() {
    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let ending = asked(&rig, "Keep the terminal's scrollback when it redraws").await;
    rig.fleet.record_plan(None, plan(&ending, &[("Scrollback kept", "apps/panes/terminal.tsx")]), false).await.expect("recorded");
    let into = asked(&rig, "Fix the terminal pane").await;

    FleetActing::on(&rig.fleet).merge(ending.as_str(), into.as_str(), &shared());

    eventually(|| async { ended(&rig, &ending).await }).await;
    let message = told(&rig, &into).await.join("\n");
    for part in ["merged", "Keep the terminal's scrollback when it redraws", "Scrollback kept (apps/panes/terminal.tsx)", "Build Scrollback kept", "apps/panes/terminal.tsx"] {
        assert!(message.contains(part), "{part}:\n{message}");
    }
    assert!(said(&rig, &ending).await.iter().any(|one| one.contains("merged this Job into")), "the ended Job says where it went");
    assert!(!ended(&rig, &into).await);
}

#[tokio::test]
async fn a_job_that_has_pushed_is_raised_rather_than_merged_and_one_that_has_not_ends_into_it() {
    let rig = rig_placing(None);
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    dispatch(&rig, true).await;
    let (first, second) = (asked(&rig, "Keep the scrollback").await, asked(&rig, "Keep it too").await);
    pushed(&rig, &first).await;

    // The Operator named `first` to end, but it pushed and `second` has not: `second` ends.
    FleetActing::on(&rig.fleet).merge(first.as_str(), second.as_str(), &shared());
    eventually(|| async { ended(&rig, &second).await }).await;
    assert!(!ended(&rig, &first).await);

    // Both have pushed now: the one named is raised, and nothing ends.
    let third = asked(&rig, "And again").await;
    pushed(&rig, &third).await;
    FleetActing::on(&rig.fleet).merge(third.as_str(), first.as_str(), &shared());
    eventually(|| async { said(&rig, &third).await.iter().any(|one| one.contains("has pushed, so it was not ended")) }).await;
    let calls = rig.fleet.store().lock().await.calls_of(third.as_str()).expect("read");
    assert!(calls.iter().any(|call| call.notice == "operator"), "raised on his Needs you list: {calls:?}");
    assert!(!ended(&rig, &third).await && !ended(&rig, &first).await);
}

#[tokio::test]
async fn a_hand_over_ends_a_start_but_moves_one_group_of_a_plan_and_leaves_its_job_working() {
    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let (start, into) = (asked(&rig, "Open terminals in a tab").await, asked(&rig, "Terminal bugs").await);
    let acting = FleetActing::on(&rig.fleet);

    acting.hand_over(start.as_str(), None, into.as_str(), &shared());
    eventually(|| async { ended(&rig, &start).await }).await;
    assert!(told(&rig, &into).await.iter().any(|one| one.contains("Open terminals in a tab")));

    let planner = asked(&rig, "Tabs for reviews").await;
    rig.fleet.record_plan(None, plan(&planner, &[("Tab host", "apps/panes/host.tsx"), ("Terminal in a tab", "apps/panes/terminal.tsx")]), false).await.expect("recorded");
    acting.hand_over(planner.as_str(), Some("g2"), into.as_str(), &shared());
    eventually(|| async { told(&rig, &planner).await.iter().any(|one| one.contains("Leave it to that Job")) }).await;
    assert!(told(&rig, &into).await.iter().any(|one| one.contains("Terminal in a tab") && !one.contains("Tab host")));
    assert!(!ended(&rig, &planner).await, "a group goes, the Job stays");
}

#[tokio::test]
async fn kept_in_touch_each_is_told_the_others_groups_and_told_again_when_it_moves_on_their_ground() {
    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let (tabs, terminal) = (asked(&rig, "Tabs for reviews").await, asked(&rig, "Terminal bugs").await);
    rig.fleet.record_plan(None, plan(&tabs, &[("Tab host", "apps/panes/host.tsx"), ("Review list", "apps/reviews/")]), false).await.expect("recorded");
    let shared = vec!["apps/panes/host.tsx".to_string()];
    // That plan was recorded before the two were in touch, so it is nobody's word.
    rig.fleet.in_touch_told().await;

    FleetActing::on(&rig.fleet).keep_in_touch(terminal.as_str(), tabs.as_str(), &shared);
    eventually(|| async { !told(&rig, &terminal).await.is_empty() && !told(&rig, &tabs).await.is_empty() }).await;
    let to_terminal = told(&rig, &terminal).await.join("\n");
    for part in ["Tab host (apps/panes/host.tsx)", "It owns: Tab host; Review list", "Fleet tells you when it changes its plan or pushes"] {
        assert!(to_terminal.contains(part), "{part}:\n{to_terminal}");
    }
    assert!(told(&rig, &tabs).await.join("\n").contains("It has no plan yet. It was asked: Terminal bugs"));

    // A plan off their ground says nothing; one on it is told.
    rig.fleet.record_plan(None, plan(&tabs, &[("Review list", "apps/reviews/")]), false).await.expect("recorded");
    rig.fleet.in_touch_told().await;
    assert_eq!(told(&rig, &terminal).await.len(), 1);
    rig.fleet.record_plan(None, plan(&tabs, &[("Tab host, again", "apps/panes/host.tsx")]), false).await.expect("recorded");
    rig.fleet.in_touch_told().await;
    eventually(|| async { told(&rig, &terminal).await.iter().any(|one| one.contains("changed its plan on ground you share")) }).await;
}

#[tokio::test]
async fn a_held_start_nothing_shares_anything_with_starts_as_his_press_would() {
    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let held = rig.start_held().await;

    FleetActing::on(&rig.fleet).go_ahead(held.as_str());

    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let (fleet, id) = (Arc::clone(&fleet), held.as_str().to_string());
        async move { fleet.store().lock().await.hosting(&id).ok().flatten().is_some_and(|one| !one.start_held) }
    })
    .await;
}

/// A call that answers what it is given, once.
struct Saying(Mutex<Option<String>>);

impl Calling for Saying {
    fn ask(&self, _question: String) -> Answering<'_> {
        let said = self.0.lock().expect("not poisoned").take();
        Box::pin(async move { said.ok_or(CallFailed::SaidNothing) })
    }
}

#[tokio::test]
async fn a_start_a_call_merges_stops_at_its_first_write_and_takes_no_slot() {
    let rig = rig_placing(None);
    dispatch(&rig, true).await;
    let into = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let start = asked(&rig, "The terminal scrollback is lost on redraw").await;
    let reply = format!("item: 1\nway: 3\njob: {}\nshared: terminal scrollback\nbecause: the same fix", into.as_str());
    assert!(rig.fleet.operator_seated_with(Arc::new(Saying(Mutex::new(Some(reply))))));

    rig.fleet.start_handed(start.as_str(), Source::Dispatch, "The terminal scrollback is lost on redraw").await;
    let answer = rig.gate(&start, "Write", &format!(r#"{{"file_path":"{}/src/pane.rs"}}"#, rig.root)).await;

    assert!(denied(&answer), "{answer:?}");
    assert!(format!("{answer:?}").contains("already doing this work"), "{answer:?}");
    eventually(|| async { ended(&rig, &start).await }).await;
    let hosting = rig.fleet.store().lock().await.hosting(start.as_str()).expect("read").expect("kept");
    assert!(hosting.lease_slot.is_none(), "no slot was leased for it");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert_eq!((log.len(), log[0].outcome.as_str()), (1, "merge"));
}

/// A call that counts what it is asked and answers nothing.
#[derive(Default)]
struct Counting(std::sync::atomic::AtomicUsize);

impl Calling for Counting {
    fn ask(&self, _question: String) -> Answering<'_> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move { Err(CallFailed::SaidNothing) })
    }
}

#[tokio::test]
async fn as_it_ships_the_operator_is_off_and_a_dispatch_and_a_plan_go_on_with_no_ruling_and_no_call() {
    let rig = rig_placing(None);
    assert!(!rig.fleet.operator_on(), "the Operator ships off");
    switched(&rig, "features.operatorDispatch", true).await;
    let counted = Arc::new(Counting::default());
    assert!(rig.fleet.operator_seated_with(counted.clone()));
    let other = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    rig.fleet.record_plan(None, plan(&other, &[("Scrollback", "apps/panes/terminal.tsx")]), false).await.expect("recorded");

    let dispatched = rig.start_requesting("The terminal scrollback is lost on redraw", |_| {}).await.id;
    let planner = asked(&rig, "Tabs for the panes").await;
    rig.fleet.record_plan(None, plan(&planner, &[("Terminal tab", "apps/panes/terminal.tsx")]), false).await.expect("recorded");
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(counted.0.load(std::sync::atomic::Ordering::SeqCst), 0, "no call");
    assert!(rig.fleet.store().lock().await.operator_rulings(10).expect("read").is_empty(), "no ruling");
    assert!(rig.fleet.operator_seat().start_ruling(dispatched.as_str()).is_none(), "nothing for a first write to wait on");

    // The same plan with the Operator on is ruled, so the silence above is the switch's.
    switched(&rig, "features.operator", true).await;
    rig.fleet.record_plan(None, plan(&planner, &[("Terminal tab, again", "apps/panes/terminal.tsx")]), false).await.expect("recorded");
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { !fleet.store().lock().await.operator_rulings(10).expect("read").is_empty() }
    })
    .await;
    assert_eq!(counted.0.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_call_is_kept_started_only_while_the_operator_is_on() {
    let rig = rig_placing(None);
    let judge = Arc::new(testkit::FakeJudge::saying("item: 1\nway: 1\njob: none\nshared: none\nbecause: nothing"));
    let call = Arc::new(crate::operator::KeptStarted::new(
        judge.clone(),
        || adapter_traits::Model::named("the-operator-model").ok(),
        adapter_traits::Environment::nothing(),
        crate::JudgeBudget::of(Duration::from_secs(10)),
    ));

    rig.fleet.operator_kept(Arc::clone(&call)).await;
    assert!(judge.models().is_empty() && !call.is_waiting().await, "off: no process is started");

    switched(&rig, "features.operator", true).await;
    rig.fleet.operator_kept_in_step().await;
    assert_eq!(judge.models().len(), 1);
    assert!(call.is_waiting().await, "on: one waits");

    switched(&rig, "features.operator", false).await;
    rig.fleet.operator_kept_in_step().await;
    assert!(!call.is_waiting().await, "turned off live: the one waiting is stopped");
}

#[tokio::test]
async fn every_role_ships_off_and_a_switch_saved_with_two_positions_carries_across() {
    use ipc::operator_positions::{OperatorPosition as Position, OperatorRole as Role};
    let rig = rig_placing(None);
    switched(&rig, "features.operator", true).await;
    for role in [Role::Dispatch, Role::Delegate, Role::Answer, Role::File] {
        assert_eq!(rig.fleet.operator_position(role, "a repository"), Position::Off, "{role:?}");
    }

    // Saved before a role had three positions; Dispatch's switch held Delegate too.
    let saved = [("features.operatorDispatch", true), ("features.operatorAnswer", false), ("features.operatorFile", true)];
    let save = ipc::SaveSettings {
        changes: saved.into_iter().map(|(key, on)| (key.to_string(), Some(ipc::SettingValue::Bool(on)))).collect(),
    };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("a saved switch is still read");

    for (role, position) in [(Role::Dispatch, Position::On), (Role::Delegate, Position::On), (Role::Answer, Position::Off), (Role::File, Position::On)] {
        assert_eq!(rig.fleet.operator_position(role, "a repository"), position, "{role:?}");
    }
    switched(&rig, "features.operatorDelegate", false).await;
    assert_eq!(rig.fleet.operator_position(Role::Delegate, "a repository"), Position::Off, "its own value wins once he sets one");
}
