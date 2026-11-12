//! What a press on Your turn does, and what is queued there at all: Later until a time Bridge names,
//! told when it comes back; a pick that is one of the item's options, or Later, and never a sentence;
//! Yes on a held Job's start starting it; a reply's ask left out beside Fleet's own item for the same
//! thing; and suggestions that ask the same thing raised once.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use api::{Next, Refusal, Sessions};
use core_model::Timestamp;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorLater, OperatorPage, OperatorTurnItem, OperatorTurnOrder, RuleScope};
use ipc::{Event, Instant, WaitingAct, WaitingActKind, WaitingInput, WaitingItem, WaitingSource};
use store::KeptSuggestion;

use super::agent_starts::{begins, caller, eventually_read};
use super::backlog::manifest;
use super::operator_raises::{journal, page, press, question, queued, seated, waiting, Replies};
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use crate::clock::Clock;
use crate::operator::{keep_the_raises, without_echoes};

/// A clock a test sets, a millisecond on at each reading.
struct Set(Arc<AtomicI64>);

impl Clock for Set {
    fn now(&self) -> Timestamp {
        Timestamp::from_rfc3339(crate::clock::rfc3339_utc(self.0.fetch_add(1, Ordering::SeqCst)))
    }
}

fn millis(at: &str) -> i64 {
    Timestamp::from_rfc3339(at).epoch_millis().expect("an instant")
}

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

fn ids(page: &OperatorPage) -> Vec<String> {
    page.turn.iter().map(|one| one.item.id.clone()).collect()
}

/// How many times the page was announced changed, until the stream goes quiet.
async fn changes(watching: &mut api::Subscription) -> usize {
    let mut found = 0;
    while let Ok(next) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => found += usize::from(matches!(delivered.event, Event::OperatorPageChanged(_))),
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    found
}

#[tokio::test]
async fn a_later_with_a_time_sits_last_until_then_and_the_page_is_told_both_times() {
    let clock = Arc::new(AtomicI64::new(millis("2026-11-10T12:00:00.000Z")));
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| fittings.clock = Arc::new(Set(held)));
    seated(&rig, &[Some("item: 1\nread: A.\npick: Yes\n\nitem: 2\nread: B.\npick: Yes\n\njournal: Reviewed two.")]).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("a", "First?", &["Yes", "No"]), question("b", "Second?", &["Yes", "No"])]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let a = queued(&rig, &job, "a").await;
    queued(&rig, &job, "b").await;
    rig.fleet.laters_run_out().await;
    let mut watching = rig.fleet.events().subscribe();

    // An hour on, as Bridge sends it from his zone: kept as the same instant.
    let later = rig.fleet.operator_turn_later(OperatorLater { id: a.id.clone(), until: Some(Instant::carried("2026-11-10T08:00:00-05:00")) }).await.expect("later");
    assert_eq!(ids(&later), ["b", "a"], "Later puts it last");
    assert_eq!(later.turn[1].later_until, Some(Instant::carried("2026-11-10T13:00:00.000Z")));
    assert_eq!(changes(&mut watching).await, 1, "told when it went Later");

    rig.fleet.laters_run_out().await;
    assert_eq!(changes(&mut watching).await, 0, "the hour is not over");

    clock.store(millis("2026-11-10T13:00:01.000Z"), Ordering::SeqCst);
    rig.fleet.laters_run_out().await;
    assert_eq!(changes(&mut watching).await, 1, "told when it came back");
    let back = page(&rig).await;
    assert_eq!(ids(&back), ["a", "b"], "back where he left it");
    assert!(back.turn.iter().all(|one| one.later_until.is_none()));
    rig.fleet.laters_run_out().await;
    assert_eq!(changes(&mut watching).await, 0, "told once");

    let past = rig.fleet.operator_turn_later(OperatorLater { id: a.id.clone(), until: Some(Instant::carried("2026-11-10T12:30:00.000Z")) }).await.expect_err("in the past");
    assert_eq!(past.error().message, "That time has passed. Pick a later one.", "his words, with no stamp");
    assert_eq!(code(past), (422, "fleet.operator_later_past".to_string()));
    assert!(page(&rig).await.turn.iter().all(|one| one.later_until.is_none()), "nothing moved");
}

fn off_the_page(id: &str, text: &str, act: Option<WaitingAct>) -> WaitingInput {
    WaitingInput { approves: false, id: id.into(), text: text.into(), act, options: Vec::new(), steps: Vec::new() }
}

#[tokio::test]
async fn a_pick_is_one_of_the_items_options_or_later_and_never_a_sentence() {
    let rig = rig_placing(None);
    let reads = "item: 1\nread: The card opens on L.\npick: Answer call-8 with the card's default choice (the first one, opened on L). I'd expect a short ladder.\n\n\
item: 2\nread: Nothing is needed until you have restarted and counted the rows.\npick: Leave it until you restart and count the rows\n\n\
item: 3\nread: He restarts the Mac first.\npick: Later\n\njournal: Reviewed three.";
    let call = seated(&rig, &[Some(reads)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let message = WaitingAct { kind: WaitingActKind::Message, target: job.as_str().to_string() };
    waiting(
        &rig,
        &job,
        vec![
            off_the_page("card", "Answer the card", Some(message)),
            off_the_page("rows", "Count the armada rows", None),
            off_the_page("restart", "Restart the Mac, then count the rows", None),
        ],
    )
    .await;

    assert_eq!(queued(&rig, &job, "card").await.read.expect("read").pick, "", "a sentence is no pick");
    assert_eq!(queued(&rig, &job, "rows").await.read.expect("read").pick, "", "nor is an act the item cannot do");
    let restart = queued(&rig, &job, "restart").await;
    assert_eq!(restart.read.as_ref().expect("read").pick, "Later");
    assert!(call.asked()[0].contains("never a sentence"), "{}", call.asked()[0]);

    let answered = rig.fleet.operator_answered(press(&restart, OperatorAnswer::Yes, None)).await.expect("Yes does Later");
    assert!(answered.open.is_none());
    let put = answered.page.turn.iter().find(|one| one.item.id == "restart").expect("it stands");
    assert!(put.later_until.is_some(), "{answered:#?}");
    assert_eq!(answered.page.turn.last().map(|one| one.item.id.as_str()), Some("restart"), "last");
}

#[tokio::test]
async fn yes_on_a_held_jobs_start_starts_that_job() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.an_agent().await;
    eventually_read(&rig, &agent).await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", "Read crates/reader.")).await.expect("held").session_id;
    eventually_read(&rig, &held).await;
    assert!(rig.fleet.operator_seated_with(Replies::saying(&[Some("item: 1\nread: The Job is ready.\npick: Press Start\n\njournal: Reviewed a start.")])));
    let on = ipc::SaveSettings { changes: std::collections::BTreeMap::from([("features.operator".to_string(), Some(ipc::SettingValue::Bool(true)))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), on).await.expect("saved");
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let start = format!("start:{}", held.as_str());
    let mut one = None;
    for _ in 0..500 {
        one = rig.fleet.operator_page_wire().await.expect("read").turn.into_iter().find(|turn| turn.item.id == start);
        if one.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let one = one.expect("the start is queued");

    let answered = rig.fleet.operator_answered(OperatorAnswerPress { id: one.id.clone(), answer: OperatorAnswer::Yes, then: None, option: None, project: None }).await.expect("answered");

    assert!(answered.open.is_none(), "it starts the Job, not open it: {answered:#?}");
    assert_eq!(answered.started.as_ref(), Some(&held));
    let hosting = rig.fleet.store().lock().await.hosting(held.as_str()).expect("read").expect("hosted");
    assert!(!hosting.start_held, "the Job is no longer held");
}

fn item(id: &str, text: &str, source: WaitingSource, act: WaitingAct) -> WaitingItem {
    WaitingItem {
        approves: false,
        id: id.into(),
        text: text.into(),
        since: Instant::carried("2026-11-12T14:38:28.421Z"),
        source,
        act: Some(act),
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

#[test]
fn a_reply_repeating_fleets_own_item_is_not_raised_beside_it() {
    let start = item("start:child", "Press Start on \"Operator Later\"", WaitingSource::HeldChild, WaitingAct { kind: WaitingActKind::Message, target: "child".into() });
    let replied = item("replied:2026-11-12T14:38:42.146Z:0", "Press Start whenever you want it.", WaitingSource::Replied, WaitingAct { kind: WaitingActKind::Answer, target: String::new() });

    let met: Vec<String> = without_echoes(vec![start.clone(), replied.clone()]).into_iter().map(|one| one.id).collect();
    assert_eq!(met, [start.id.clone()], "one item, not two");
    assert_eq!(without_echoes(vec![replied]).len(), 1, "alone, a reply's ask is still raised");
}

fn filing(n: i64, manifest: &str, pull: &str, subject: &str) -> KeptSuggestion {
    let what = "Show Jobs waiting for a slot in the Operator's Up next";
    KeptSuggestion {
        n,
        arrival: format!("merged:job:{pull}"),
        arrived: "merged".into(),
        way: "file".into(),
        session_id: String::new(),
        manifest_id: manifest.into(),
        subject: subject.into(),
        words: String::new(),
        group: None,
        other_session: None,
        shared: Vec::new(),
        first_session: None,
        step: None,
        item: None,
        pick: None,
        because: format!("#{pull} left this Later item open"),
        what: Some(what.into()),
        text: format!("The Operator would file \"{what}\" as a new issue now that #{pull} merged"),
        raised_at: format!("2026-11-09T18:0{n}:00.000Z"),
    }
}

#[tokio::test]
async fn suggestions_that_file_the_same_issue_are_one_turn_and_one_answer_settles_them() {
    let rig = rig_placing(None);
    let repository = manifest(&rig);
    {
        let mut store = rig.fleet.store().lock().await;
        for (n, pull, subject) in [(1, "3385", "Bring back Overrule"), (2, "3386", "Show the Job the Operator offers"), (3, "3387", "Keep the card")] {
            store.keep_operator_suggestion(&filing(n, repository.as_str(), pull, subject)).expect("kept");
        }
    }
    seated(&rig, &[Some("item: 1\nread: It is filed already.\npick: Skip\n\njournal: Reviewed a suggestion.")]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    eventually(|| async { !page(&rig).await.turn.is_empty() }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let turn = page(&rig).await.turn;
    assert_eq!(turn.len(), 1, "one turn for the one request: {turn:#?}");
    assert_eq!(turn[0].item.id, "suggestion:1", "the earliest");
    assert_eq!(turn[0].also_raised_by, ["#3386 merging", "#3387 merging"], "the others ride on it");

    rig.fleet.operator_answered(press(&turn[0], OperatorAnswer::No, None)).await.expect("skipped");
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty(), "none of the three stands");
    assert!(page(&rig).await.turn.is_empty());
}

#[tokio::test]
async fn copies_of_an_answered_suggestion_are_dropped_and_never_queued() {
    let rig = rig_placing(None);
    let repository = manifest(&rig);
    {
        let mut store = rig.fleet.store().lock().await;
        for (n, pull, subject) in [(1, "3385", "Bring back Overrule"), (2, "3386", "Show the Job the Operator offers"), (3, "3387", "Keep the card")] {
            store.keep_operator_suggestion(&filing(n, repository.as_str(), pull, subject)).expect("kept");
        }
        assert!(store.settle_operator_suggestion(1, true, "2026-11-12T14:42:01.840Z").expect("settled"));
    }
    seated(&rig, &[Some("item: 1\nread: Again.\npick: Skip\n\njournal: Reviewed a suggestion.")]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty() }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(page(&rig).await.turn.is_empty(), "never queued");
    let answered = rig.fleet.store().lock().await.operator_turn_settled_since("2026-01-01T00:00:00.000Z").expect("read");
    assert!(answered.is_empty(), "and not under Answered: {answered:#?}");
}

#[tokio::test]
async fn a_jobs_asks_stand_together_at_the_first_ones_place_and_move_together() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
    let pane = rig.start_titled(Some("Build the pane")).await;
    let clock = rig.start_titled(Some("Fix the clock")).await;
    waiting(&rig, &pane, vec![question("a", "First?", &["Yes", "No"])]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    queued(&rig, &pane, "a").await;
    waiting(&rig, &clock, vec![question("c", "Which zone?", &["UTC", "Local"])]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    queued(&rig, &clock, "c").await;
    waiting(&rig, &pane, vec![question("a", "First?", &["Yes", "No"]), question("b", "Second?", &["Yes", "No"])]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    queued(&rig, &pane, "b").await;

    let now = page(&rig).await;
    assert_eq!(ids(&now), ["a", "b", "c"], "the pane's two at the first one's place, in the order raised");
    let groups: Vec<Option<String>> = now.turn.iter().map(|one| one.group.clone()).collect();
    assert_eq!(groups, [Some(pane.as_str().to_string()), Some(pane.as_str().to_string()), None], "a single ask is a plain item");

    let turn = |page: &OperatorPage, item: &str| page.turn.iter().find(|one| one.item.id == item).map(|one| one.id.clone()).expect("queued");
    let moved = rig.fleet.operator_turn_ordered(OperatorTurnOrder { ids: vec![turn(&now, "c")] }).await.expect("ordered");
    assert_eq!(ids(&moved), ["c", "a", "b"], "the group moves whole");
    let back = rig.fleet.operator_turn_ordered(OperatorTurnOrder { ids: vec![turn(&moved, "b"), turn(&moved, "c")] }).await.expect("ordered");
    assert_eq!(ids(&back), ["a", "b", "c"], "naming one of the group moves the group");
}

/// A press, boxed: each holds the whole answer path, and a test that makes several overflows its stack.
async fn answer(rig: &Rig, press: OperatorAnswerPress) -> Result<ipc::operator_page::OperatorAnswered, Refusal> {
    Box::pin(rig.fleet.operator_answered(press)).await
}

/// A routine question queued as a go-ahead, on a clock the test sets.
async fn a_go_ahead_on_a_set_clock() -> (Rig, Arc<AtomicI64>, OperatorTurnItem) {
    let clock = Arc::new(AtomicI64::new(millis("2026-11-10T12:00:00.000Z")));
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| fittings.clock = Arc::new(Set(held)));
    seated(&rig, &[Some("item: 1\nread: The suite is the cheap safe step.\npick: Run the full suite\nsure: yes\nabout: none\n\njournal: Reviewed a question.")]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", "Run the full suite, or push now?", &["Run the full suite", "Push now"])]).await;
    let one = queued(&rig, &job, "full").await;
    assert!(one.keeps, "offered with Always: {one:#?}");
    (rig, clock, one)
}

#[tokio::test]
async fn always_just_after_a_yes_keeps_the_rule_and_does_not_do_it_again() {
    let (rig, _clock, one) = a_go_ahead_on_a_set_clock().await;
    let yes = answer(&rig, press(&one, OperatorAnswer::Yes, None)).await.expect("yes");
    assert!(yes.page.turn.is_empty() && yes.page.digest.learned.is_empty(), "{yes:#?}");
    let before = journal(&rig).await;

    let always = answer(&rig, press(&one, OperatorAnswer::AlwaysHere, None)).await.expect("Always here after the yes");

    let learned = &always.page.digest.learned;
    assert_eq!(learned.len(), 1, "{learned:#?}");
    assert!(matches!(learned[0].scope, RuleScope::Repository { .. }), "{learned:#?}");
    assert!(always.open.is_none() && always.started.is_none() && always.filed.is_none());
    let after = journal(&rig).await;
    assert_eq!(after.matches("Answered").count(), before.matches("Answered").count(), "the act is not done again: {after}");
    assert!(after.contains("Kept always here after a yes"), "{after}");
}

#[tokio::test]
async fn on_a_settled_item_only_an_always_within_a_minute_of_his_yes_is_taken() {
    let (rig, clock, one) = a_go_ahead_on_a_set_clock().await;
    answer(&rig, press(&one, OperatorAnswer::Yes, None)).await.expect("yes");
    let again = answer(&rig, press(&one, OperatorAnswer::Yes, None)).await.expect_err("a yes is not taken twice");
    assert_eq!(code(again).0, 404);
    let no = answer(&rig, press(&one, OperatorAnswer::No, None)).await.expect_err("nor a no after it");
    assert_eq!(code(no).0, 404);
    clock.fetch_add(61_000, Ordering::SeqCst);
    let late = answer(&rig, press(&one, OperatorAnswer::Always, None)).await.expect_err("over a minute on");
    assert_eq!(code(late).0, 404);
    assert!(page(&rig).await.digest.learned.is_empty(), "no rule kept");
}

#[tokio::test]
async fn an_always_after_a_no_is_refused() {
    let (rig, _clock, one) = a_go_ahead_on_a_set_clock().await;
    answer(&rig, press(&one, OperatorAnswer::No, None)).await.expect("no");
    let after_no = answer(&rig, press(&one, OperatorAnswer::Always, None)).await.expect_err("an Always follows a yes only");
    assert_eq!(code(after_no).0, 404);
    assert!(page(&rig).await.digest.learned.is_empty(), "no rule kept");
}
