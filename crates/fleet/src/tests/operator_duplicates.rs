//! **Duplicate work is handed to the Job already on it, after one go-ahead.** A red main a live Job is
//! near offers Hand to beside Start a fix; a Job started on what a live Job holds is raised as a
//! go-ahead, Hand to or Keep both, and a kept rule hands it alone. Where no live Job is near a start
//! there is no call. A Job the person started himself is never held: it starts, and he is asked.

use std::sync::Arc;
use std::time::Duration;

use core_model::Timestamp;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorPage, OperatorRule, OperatorTurnItem};
use ipc::{SessionId, StartAnotherSession};
use store::{KeptTouch, MainFix, MainMerge, TakenBy, TakenHow};
use testkit::{Delivering, FakeVcs};

use super::autopilot_decisions_repository::{jobs_titled, red_main};
use super::operator_sources::{rulings, switched, Answers};
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use crate::main_ci::{MainChange, MainChanged};
use crate::operator::keep_the_raises;

const FILE: &str = "src/terminal/scrollback.rs";
const BRIEF: &str = "Scrollback is lost on redraw in src/terminal/scrollback.rs. Keep it across a resize.";
const LIVE: &str = "Fix the scrollback";

async fn page(rig: &Rig) -> OperatorPage {
    rig.fleet.operator_page_wire().await.expect("read")
}

/// A live Job titled `LIVE` that has written `FILE`.
async fn a_live_job(rig: &Rig) -> SessionId {
    let job = rig.start_titled(Some(LIVE)).await;
    let touch = KeptTouch { by: None, path: FILE.into(), act: "edit".into(), at: "2026-11-10T08:00:00.000Z".into() };
    rig.fleet.store().lock().await.keep_session_touch(job.as_str(), &touch, 50).expect("kept");
    job
}

/// The Operator's ruling that hands item 1 to `job`.
fn hands_to(job: &SessionId) -> String {
    format!("item: 1\nway: 1\njob: {}\nshared: {FILE}\nbecause: the same file", job.as_str())
}

async fn seated(rig: &Rig, answers: &[&str]) -> Arc<Answers> {
    let call = Answers::with(answers);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    switched(rig, "features.operator", true).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    call
}

/// The Operator's go-ahead for `job` in Your turn, once it is there.
async fn turn_of(rig: &Rig, job: &SessionId) -> OperatorTurnItem {
    for _ in 0..500 {
        if let Some(one) = page(rig).await.turn.into_iter().find(|one| one.session_id.as_ref() == Some(job) && one.item.id.starts_with("suggestion:")) {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("nothing is waiting for {}: {:#?}", job.as_str(), page(rig).await.turn);
}

fn press(item: &OperatorTurnItem, answer: OperatorAnswer) -> OperatorAnswerPress {
    OperatorAnswerPress { id: item.id.clone(), answer, then: None }
}

fn labels(item: &OperatorTurnItem) -> Vec<&str> {
    item.item.options.iter().map(|option| option.label.as_str()).collect()
}

async fn ended(rig: &Rig, job: &SessionId) -> bool {
    rig.fleet.store().lock().await.session(job.as_str()).expect("read").expect("there").ended_at.is_some()
}

async fn thread(rig: &Rig, job: &SessionId) -> String {
    rig.fleet.store().lock().await.session_rows(job.as_str()).expect("rows").join("\n")
}

/// A Job `parent` starts on `words`, held on its start card.
async fn started_by(rig: &Rig, parent: &SessionId, words: &str) -> SessionId {
    let start = StartAnotherSession { manifest_id: Some(rig.manifest.clone()), title: "Fix the redraw".into(), message: words.into(), group: None, after: None };
    rig.fleet.another_session_started_by(parent.as_str().to_string(), start).await.expect("started").session_id
}

// ---- A start ----

#[tokio::test]
async fn a_job_started_on_a_live_jobs_files_is_held_for_a_go_ahead_and_yes_hands_its_brief_over_and_tells_the_starter() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let parent = rig.start_titled(Some("The planner")).await;
    let call = seated(&rig, &[&hands_to(&live)]).await;

    let held = started_by(&rig, &parent, BRIEF).await;

    let one = turn_of(&rig, &held).await;
    assert_eq!(labels(&one), [format!("Hand to {LIVE}").as_str(), "Keep both"]);
    assert_eq!(call.asked().iter().filter(|asked| asked.contains("A Job started")).count(), 1, "one call judged it");
    assert!(!ended(&rig, &held).await, "it is held until he answers");
    assert!(!thread(&rig, &live).await.contains("Keep it across a resize"), "nothing is handed before Yes");

    rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes)).await.expect("answered");

    assert!(thread(&rig, &live).await.contains("Keep it across a resize"), "the brief went to the live Job");
    assert!(ended(&rig, &held).await, "the duplicate is closed");
    assert!(thread(&rig, &parent).await.contains("was not started"), "the starter is told where it went");
    let row = rulings(&rig).await.into_iter().find(|row| row.outcome == "existing_job").expect("a Log row");
    assert_eq!((row.by.as_str(), row.other_session.as_deref()), ("operator", Some(live.as_str())));
    assert!(row.because.contains("Fix the redraw") && row.because.contains(LIVE), "it names both Jobs: {}", row.because);
    assert!(page(&rig).await.digest.handled.acts.iter().any(|act| act.outcome.starts_with("Handed over")), "{:#?}", page(&rig).await.digest.handled);
}

#[tokio::test]
async fn no_leaves_a_started_job_as_it_was_asked() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let parent = rig.start_titled(Some("The planner")).await;
    seated(&rig, &[&hands_to(&live)]).await;
    let held = started_by(&rig, &parent, BRIEF).await;
    let one = turn_of(&rig, &held).await;

    rig.fleet.operator_answered(press(&one, OperatorAnswer::No)).await.expect("answered");

    assert!(!ended(&rig, &held).await, "it stands, held on its start card as it was asked");
    assert!(!thread(&rig, &live).await.contains("Keep it across a resize"));
    assert!(page(&rig).await.turn.iter().all(|one| one.session_id.as_ref() != Some(&held) || !one.item.id.starts_with("suggestion:")));
}

#[tokio::test]
async fn a_kept_rule_hands_a_started_job_over_alone_and_the_log_names_it() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let parent = rig.start_titled(Some("The planner")).await;
    let rule = OperatorRule::PickRule { manifest_id: None, what: "start:existing_job".into(), pick: "do_it".into() };
    let kept = store::KeptOrder { n: 0, form: rule.kept(), said: "Hand a Job that repeats a live Job's work to that Job".into(), until: None, at: "2026-11-10T09:00:00.000Z".into() };
    rig.fleet.store().lock().await.add_operator_order(&kept).expect("kept");
    seated(&rig, &[&hands_to(&live)]).await;

    let held = started_by(&rig, &parent, BRIEF).await;

    eventually(|| async { rulings(&rig).await.iter().any(|row| row.outcome == "existing_job") }).await;
    assert!(ended(&rig, &held).await);
    assert!(thread(&rig, &live).await.contains("Keep it across a resize"));
    assert!(page(&rig).await.turn.iter().all(|one| !one.item.id.starts_with("suggestion:")), "nobody was asked");
    let row = rulings(&rig).await.into_iter().find(|row| row.outcome == "existing_job").expect("a Log row");
    assert_eq!(row.by, "operator");
    assert!(row.because.starts_with("By a rule, Hand a Job that repeats"), "{}", row.because);
}

#[tokio::test]
async fn a_start_no_live_job_is_near_makes_no_call() {
    let rig = rig_placing(None);
    let _elsewhere = rig.start_titled(Some("Tidy the settings page")).await;
    let parent = rig.start_titled(Some("The planner")).await;
    let call = seated(&rig, &[]).await;

    let held = started_by(&rig, &parent, "Rename the colour tokens in packages/tokens/colour.ts.").await;

    tokio::time::sleep(Duration::from_millis(300)).await;
    let judged = call.asked().into_iter().filter(|asked| asked.contains("A Job started")).count();
    assert_eq!(judged, 0, "no Job shares anything, so no call judges it");
    assert!(page(&rig).await.turn.iter().all(|one| one.session_id.as_ref() != Some(&held) || !one.item.id.starts_with("suggestion:")));
    assert!(rulings(&rig).await.is_empty(), "and no row is kept");
}

#[tokio::test]
async fn a_job_the_person_started_himself_starts_and_is_raised_and_yes_hands_it_over_and_closes_it() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let call = seated(&rig, &[&hands_to(&live)]).await;

    let started = rig.start_requesting(BRIEF, |_| {}).await;

    let mine = SessionId::carried(started.id.as_str());
    let one = turn_of(&rig, &mine).await;
    assert_eq!(labels(&one), [format!("Hand to {LIVE}").as_str(), "Keep both"]);
    assert!(!ended(&rig, &mine).await, "his own start is raised, never held or closed unasked");
    assert_eq!(call.asked().iter().filter(|asked| asked.contains("A Job started")).count(), 1);

    rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes)).await.expect("answered");

    assert!(thread(&rig, &live).await.contains("Keep it across a resize"));
    assert!(ended(&rig, &mine).await);
}

// ---- A red main ----

/// Fittings whose forge says every pull request changed `FILE`.
fn a_forge_that_says_the_merge_changed_the_file(fittings: &mut crate::daemon::Fittings<testkit::FakeHarness, FakeVcs, testkit::FakeWorkProduct>) {
    fittings.vcs = FakeVcs::new().delivering(Delivering {
        pull_request_diff: Some(adapter_traits::PullRequestDiff { files: vec![FILE.into()], patch: String::new() }),
        ..Delivering::default()
    });
}

async fn went_red(rig: &Rig, commit: &str, merge: Option<u64>) -> store::MainCi {
    red_main(rig, commit).await;
    let mut main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    main.merge = merge.map(|number| MainMerge { number, url: None, branch: None, session: None });
    rig.fleet.store().lock().await.record_main_ci(&main).expect("kept");
    rig.fleet.red_offered(&[MainChanged { change: MainChange::WentRed, now: main.clone() }]).await;
    main
}

async fn red_item(rig: &Rig, commit: &str) -> OperatorTurnItem {
    let id = format!("red:{}:{commit}", super::backlog::manifest(rig).as_str());
    for _ in 0..500 {
        if let Some(one) = page(rig).await.turn.into_iter().find(|one| one.item.id == id) {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{id} was not queued: {:#?}", page(rig).await.turn);
}

const PICKS_HAND: &str = "item: 1\nread: The Job that took the last red is on it.\npick: Hand to Fix the scrollback\n\njournal: Reviewed a red main.";

#[tokio::test]
async fn a_red_main_offers_hand_to_the_job_that_took_the_last_red_and_yes_gives_it_the_red() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let at = Timestamp::from_rfc3339("2026-01-01T00:00:00.000Z");
    let earlier = MainFix { repository: rig.root.clone(), red_at: at.clone(), by: TakenBy::Session(live.as_str().to_string()), forked_from: None, how: TakenHow::Took, check: "ci".into(), test: None, merge: None, taken_at: at.clone(), ended_at: Some(at), fixed_in: None };
    rig.fleet.store().lock().await.keep_main_fix(&earlier).expect("kept");
    seated(&rig, &[PICKS_HAND]).await;

    let main = went_red(&rig, &"d".repeat(40), None).await;

    let one = red_item(&rig, &"d".repeat(40)).await;
    assert_eq!(labels(&one), ["Start a fix", format!("Hand to {LIVE}").as_str()]);
    assert_eq!(one.read.as_ref().map(|read| read.pick.as_str()), Some("Hand to Fix the scrollback"));

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes)).await.expect("answered");

    assert_eq!(answered.handed_to.as_ref(), Some(&live));
    assert!(answered.started.is_none() && jobs_titled(&rig, "Fix main").await == 0, "no Job was started");
    assert!(rig.fleet.taking(&main).await.is_some(), "the red is claimed for the live Job");
    assert!(thread(&rig, &live).await.contains("Main went red at"));
    let row = rulings(&rig).await.into_iter().find(|row| row.outcome == "existing_job").expect("a Log row");
    assert_eq!((row.by.as_str(), row.other_session.as_deref()), ("operator", Some(live.as_str())));
}

#[tokio::test]
async fn a_red_main_offers_hand_to_a_job_whose_paths_meet_the_files_of_the_merge_before_it() {
    let rig = rig_fitted(a_forge_that_says_the_merge_changed_the_file);
    let _live = a_live_job(&rig).await;
    seated(&rig, &[PICKS_HAND]).await;

    went_red(&rig, &"e".repeat(40), Some(7)).await;

    assert_eq!(labels(&red_item(&rig, &"e".repeat(40)).await), ["Start a fix", format!("Hand to {LIVE}").as_str()]);
}

#[tokio::test]
async fn a_red_main_offers_only_start_a_fix_where_no_live_job_is_near_it() {
    let rig = rig_fitted(a_forge_that_says_the_merge_changed_the_file);
    let _elsewhere = rig.start_titled(Some("Tidy the settings page")).await;
    seated(&rig, &[PICKS_HAND]).await;

    went_red(&rig, &"f".repeat(40), Some(7)).await;

    assert_eq!(labels(&red_item(&rig, &"f".repeat(40)).await), ["Start a fix"]);
}

#[tokio::test]
async fn always_here_on_a_hand_over_keeps_the_pick_as_the_rule_and_the_next_red_is_handed_alone() {
    let rig = rig_placing(None);
    let live = a_live_job(&rig).await;
    let at = Timestamp::from_rfc3339("2026-01-01T00:00:00.000Z");
    let earlier = MainFix { repository: rig.root.clone(), red_at: at.clone(), by: TakenBy::Session(live.as_str().to_string()), forked_from: None, how: TakenHow::Took, check: "ci".into(), test: None, merge: None, taken_at: at.clone(), ended_at: Some(at), fixed_in: None };
    rig.fleet.store().lock().await.keep_main_fix(&earlier).expect("kept");
    seated(&rig, &[PICKS_HAND]).await;
    went_red(&rig, &"a".repeat(40), None).await;
    let one = red_item(&rig, &"a".repeat(40)).await;

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::AlwaysHere)).await.expect("answered");

    assert_eq!(answered.page.digest.learned.iter().map(|rule| rule.said.as_str()).collect::<Vec<_>>(), ["Hand a red main to the live Job near it"]);
    // Main goes green, then red again at another commit: the rule hands it with no press.
    rig.fleet.store().lock().await.end_main_fixes(&rig.root, &rig.fleet.now(), None).expect("ended");
    let next = "b".repeat(40);
    red_main(&rig, &next).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    rig.fleet.red_offered(&[MainChanged { change: MainChange::BackToRed, now: main.clone() }]).await;

    eventually(|| async { rig.fleet.taking(&main).await.is_some() }).await;
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0);
    assert!(page(&rig).await.turn.is_empty(), "he was not asked");
    let by_rule = rulings(&rig).await.into_iter().any(|row| row.outcome == "existing_job" && row.by == "operator" && row.because.starts_with("By a rule, Hand a red main to the live Job near it"));
    assert!(by_rule, "{:#?}", rulings(&rig).await);
}

// ---- Autopilot ----

const ARMADA: &str = "version: 1\nid: armada\n";
const SURE: &str = "issue: R-AAAAAA\nworkflow: bug\nsure: yes\nbecause: plainly covered";

/// A rig whose picker is sure of the one reported bug, whose title names `FILE`, with a live Job on that file.
async fn a_rig_whose_picker_is_sure_of_a_reported_bug_on_the_live_jobs_file() -> (Rig, SessionId) {
    let judge = Arc::new(testkit::FakeJudge::saying(SURE));
    let rig = rig_fitted(move |fittings| {
        fittings.judge = judge;
        fittings.starting().workflows = [super::daemon::workflow_named("bug")].into_iter().map(|one| (one.id().clone(), one)).collect();
        fittings.starting().manifest = config::Manifest::parse(std::path::Path::new("armada.yml"), ARMADA).expect("loads");
    });
    let report = store::KeptBugReport {
        id: "01R00000000000000000AAAAAA".into(),
        session_id: "reporter".into(),
        job_title: String::new(),
        title: "Scrollback is lost on redraw in src/terminal/scrollback.rs".into(),
        what: "It broke.".into(),
        steps: Vec::new(),
        errors: Vec::new(),
        logs: Vec::new(),
        at: "2026-11-03T08:00:00.000Z".into(),
        kept: None,
        fixed: None,
    };
    rig.fleet.store().lock().await.keep_bug_report(&report).expect("kept");
    let settings = ipc::AutopilotSettings {
        instruction: "Fix what is open".into(),
        at_once: 3,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        weekly_cap_percent: None,
        on_autopilot: ipc::AutopilotWork { backlog: false, running: false, reports: true },
        decisions: Default::default(),
        project: None,
    };
    api::Autopilots::start_autopilot(rig.fleet.as_ref(), rig.manifest.clone(), settings).await.expect("started");
    let live = a_live_job(&rig).await;
    (rig, live)
}

async fn the_pick(rig: &Rig) -> ipc::AutopilotPick {
    api::Autopilots::get_autopilot(rig.fleet.as_ref(), rig.manifest.clone()).await.expect("read").picks.remove(0)
}

async fn the_go_ahead(rig: &Rig) -> store::KeptSuggestion {
    for _ in 0..500 {
        if let Some(one) = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").into_iter().next() {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no go-ahead was kept");
}

#[tokio::test]
async fn an_autopilot_pick_a_live_job_is_near_is_held_and_no_starts_it_as_asked() {
    let (rig, live) = a_rig_whose_picker_is_sure_of_a_reported_bug_on_the_live_jobs_file().await;
    seated(&rig, &[&hands_to(&live)]).await;

    assert_eq!(rig.fleet.autopilot_pass().await.expect("passed"), 1);

    let held = the_go_ahead(&rig).await;
    let pick = the_pick(&rig).await;
    assert!(pick.session_id.is_none(), "no Job is started while it is asked: {pick:#?}");
    assert_eq!(rig.fleet.autopilot_pass().await.expect("passed"), 0, "and Autopilot does not start it meanwhile");
    assert!(the_pick(&rig).await.session_id.is_none());

    let skipped = ipc::operator_board::AnswerOperatorSuggestion { id: format!("suggestion:{}", held.n), choice: crate::operator::SKIP };
    rig.fleet.suggestion_answered(skipped).await.expect("answered");

    eventually(|| async { the_pick(&rig).await.session_id.is_some() }).await;
    assert!(!thread(&rig, &live).await.contains("It broke"), "nothing went to the live Job");
}

#[tokio::test]
async fn yes_hands_an_autopilot_pick_to_the_live_job_and_no_job_starts_on_it() {
    let (rig, live) = a_rig_whose_picker_is_sure_of_a_reported_bug_on_the_live_jobs_file().await;
    seated(&rig, &[&hands_to(&live)]).await;
    rig.fleet.autopilot_pass().await.expect("passed");
    let held = the_go_ahead(&rig).await;

    let done = ipc::operator_board::AnswerOperatorSuggestion { id: format!("suggestion:{}", held.n), choice: 0 };
    rig.fleet.suggestion_answered(done).await.expect("answered");

    assert!(thread(&rig, &live).await.contains("Scrollback is lost on redraw"), "the issue went to the live Job");
    assert!(the_pick(&rig).await.session_id.is_none());
    assert_eq!(the_pick(&rig).await.state, ipc::AutopilotPickState::Dismissed);
}
