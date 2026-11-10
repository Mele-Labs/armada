//! The Operator raising each Needs you item to the person in its conversation, on a real Fleet: once
//! per item, with the read and the pick its review call made, settled when the item leaves the list,
//! and raised with no read where no call could read it. The briefing of the day, once a day, and the
//! journal of the week every call is handed.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{HostedSessions, Sessions};
use core_model::Timestamp;
use ipc::operator_talk::OperatorTalkLine;
use ipc::{OperatorRead, SessionId, SetWaitingFor, WaitingAct, WaitingActKind, WaitingInput, WaitingOption};

use super::operator_sources::switched;
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use crate::clock::Clock;
use crate::operator::{keep_the_raises, Answering, Calling};
use crate::CallFailed;

/// A call that answers with each of its replies in turn, `None` failing, and keeps what it was asked.
struct Replies {
    replies: Mutex<Vec<Option<String>>>,
    asked: Mutex<Vec<String>>,
}

impl Replies {
    fn saying(replies: &[Option<&str>]) -> Arc<Replies> {
        Arc::new(Replies { replies: Mutex::new(replies.iter().rev().map(|one| one.map(str::to_string)).collect()), asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl Calling for Replies {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let reply = self.replies.lock().expect("not poisoned").pop().flatten();
        Box::pin(async move { reply.ok_or(CallFailed::SaidNothing) })
    }
}

/// A clock a test moves, a millisecond on at each reading.
struct Moved(Arc<AtomicI64>);

impl Clock for Moved {
    fn now(&self) -> Timestamp {
        Timestamp::from_rfc3339(crate::clock::rfc3339_utc(self.0.fetch_add(1, Ordering::SeqCst)))
    }
}

fn millis(at: &str) -> i64 {
    Timestamp::from_rfc3339(at).epoch_millis().expect("an instant")
}

/// The Operator seated on `replies` and switched on.
async fn seated(rig: &Rig, replies: &[Option<&str>]) -> Arc<Replies> {
    let call = Replies::saying(replies);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    switched(rig, "features.operator", true).await;
    call
}

fn question(id: &str, text: &str, options: &[&str]) -> WaitingInput {
    WaitingInput {
        id: id.into(),
        text: text.into(),
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.into() }),
        options: options.iter().map(|label| WaitingOption { label: (*label).into(), description: None }).collect(),
        steps: Vec::new(),
    }
}

/// `job`'s list, as its agent sets it whole.
async fn waiting(rig: &Rig, job: &SessionId, items: Vec<WaitingInput>) {
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(job.clone()) }).await.expect("set");
}

async fn lines(rig: &Rig) -> Vec<OperatorTalkLine> {
    rig.fleet.operator_talk_wire().await.expect("read").lines
}

/// The raises of `job`'s item `id`.
async fn raises_of(rig: &Rig, job: &SessionId, id: &str) -> Vec<OperatorTalkLine> {
    lines(rig).await.into_iter().filter(|one| one.raise.as_ref().is_some_and(|raise| raise.session_id.as_ref() == Some(job) && raise.item.id == id)).collect()
}

async fn raised(rig: &Rig, job: &SessionId, id: &str) -> OperatorTalkLine {
    let fleet = Arc::clone(&rig.fleet);
    for _ in 0..500 {
        if let Some(one) = raises_of(rig, job, id).await.into_iter().next() {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{id} was not raised: {:#?}", fleet.operator_talk_wire().await);
}

fn review(read: &str, pick: &str) -> String {
    format!("item: 1\nread: {read}\npick: {pick}\n\njournal: Reviewed the plan question.")
}

async fn pause() {
    tokio::time::sleep(Duration::from_millis(200)).await;
}

async fn journal(rig: &Rig) -> String {
    let dir = std::path::Path::new(&rig.fleet.host().operator_journal_dir).to_path_buf();
    let mut all = String::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        all.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
    }
    all
}

#[tokio::test]
async fn a_new_item_on_a_live_job_is_raised_once_with_the_read_and_pick_its_review_made() {
    let rig = rig_placing(None);
    let reply = review("It asks to approve the plan, which covers the pane only.", "Yes");
    let call = seated(&rig, &[Some(&reply)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;

    let line = raised(&rig, &job, "plan").await;
    let raise = line.raise.clone().expect("a raise");
    assert_eq!(raise.read, Some(OperatorRead { read: "It asks to approve the plan, which covers the pane only.".into(), pick: "Yes".into() }));
    assert_eq!(raise.job.as_deref(), Some("Build the pane"));
    assert_eq!(raise.item.options.len(), 2, "the item as it was raised, with its acts");
    assert!(!line.from_person && !line.briefing && raise.settled_at.is_none());
    assert_eq!(line.text, "Build the pane needs you: Approve the plan? It asks to approve the plan, which covers the pane only. I'd pick: Yes.");
    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("Item 1: Job") && asked[0].contains("Approve the plan?") && asked[0].contains("\"Yes\", \"No\""), "{}", asked[0]);

    // The same list again raises nothing.
    rig.fleet.published_hosted(job.as_str()).await.expect("published");
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    pause().await;
    assert_eq!(raises_of(&rig, &job, "plan").await.len(), 1);
    assert_eq!(call.asked().len(), 1, "no second review");
}

#[tokio::test]
async fn an_item_leaving_the_list_marks_its_raise_settled_and_the_journal_keeps_both() {
    let rig = rig_placing(None);
    let reply = review("It asks to approve the plan.", "Yes");
    let said = "cites: none\njob: none\nstart: none\norder: none\nsaid: Nothing else waits.";
    seated(&rig, &[Some(&reply), Some(said)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    raised(&rig, &job, "plan").await;

    waiting(&rig, &job, Vec::new()).await;

    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.operator_talk_wire().await.expect("read").lines.iter().any(|one| one.raise.as_ref().is_some_and(|raise| raise.settled_at.is_some())) }
    })
    .await;
    rig.fleet.operator_talked("anything else?", None).await.expect("answered");
    let written = journal(&rig).await;
    assert!(written.contains("Raised: Build the pane: Approve the plan?. Read: It asks to approve the plan.. Pick: Yes"), "{written}");
    assert!(written.contains("Settled: Build the pane: Approve the plan?"), "{written}");
    assert!(written.contains("He wrote: anything else? I answered: Nothing else waits."), "{written}");
    assert!(written.contains("Reviewed the plan question."), "the call's own journal line: {written}");
    assert!(written.starts_with("# The Operator's journal, ") && written.contains("\n## "), "{written}");
}

#[tokio::test]
async fn items_open_at_start_and_when_the_switch_comes_on_are_raised_and_none_while_off() {
    let rig = rig_placing(None);
    let call = seated(&rig, &[Some(&review("First.", "Yes")), Some(&review("Second.", "No"))]).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("first", "Approve the plan?", &["Yes", "No"])]).await;

    keep_the_raises(Arc::clone(&rig.fleet)).await;
    assert_eq!(raised(&rig, &job, "first").await.raise.expect("a raise").read.expect("read").read, "First.");

    switched(&rig, "features.operator", false).await;
    waiting(&rig, &job, vec![question("first", "Approve the plan?", &["Yes", "No"]), question("second", "Merge it?", &["Yes", "No"])]).await;
    pause().await;
    assert!(raises_of(&rig, &job, "second").await.is_empty(), "nothing is raised while off");
    assert_eq!(call.asked().len(), 1, "and no call made");

    switched(&rig, "features.operator", true).await;
    let second = raised(&rig, &job, "second").await.raise.expect("a raise");
    assert_eq!(second.read, Some(OperatorRead { read: "Second.".into(), pick: "No".into() }));
    assert_eq!(raises_of(&rig, &job, "first").await.len(), 1, "the first is not raised again");
}

#[tokio::test]
async fn a_failed_or_unreadable_review_still_raises_the_item_with_no_read() {
    let rig = rig_placing(None);
    seated(&rig, &[None, Some("I would rather not say.")]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("plan", "Approve the plan", &["Yes", "No"])]).await;
    let failed = raised(&rig, &job, "plan").await;
    assert_eq!(failed.raise.expect("a raise").read, None);
    assert_eq!(failed.text, "Build the pane needs you: Approve the plan.");

    waiting(&rig, &job, vec![question("plan", "Approve the plan", &["Yes", "No"]), question("merge", "Merge it?", &["Yes", "No"])]).await;
    let unreadable = raised(&rig, &job, "merge").await;
    assert_eq!(unreadable.raise.expect("a raise").read, None);
}

#[tokio::test]
async fn a_pick_naming_an_option_not_offered_is_dropped_and_the_read_stays() {
    let rig = rig_placing(None);
    seated(&rig, &[Some(&review("It is safe to merge.", "Merge and deploy"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("merge", "Merge it?", &["Yes", "No"])]).await;

    let line = raised(&rig, &job, "merge").await;
    assert_eq!(line.raise.expect("a raise").read, Some(OperatorRead { read: "It is safe to merge.".into(), pick: String::new() }));
    assert_eq!(line.text, "Build the pane needs you: Merge it? It is safe to merge.");
}

#[tokio::test]
async fn a_message_is_handed_his_conversation_and_the_raises_still_open_but_not_the_raise_lines() {
    let rig = rig_placing(None);
    let said = "cites: none\njob: none\nstart: none\norder: none\nsaid: The pane waits on you.";
    let call = seated(&rig, &[Some(&review("It asks to approve the plan.", "Yes")), Some(said)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    let line = raised(&rig, &job, "plan").await;

    rig.fleet.operator_talked("what waits on me?", None).await.expect("answered");

    let asked = call.asked();
    let talk = asked.last().expect("asked");
    let so_far = &talk[talk.find("The conversation so far:").expect("a conversation")..];
    assert!(!so_far.contains(&line.text), "the raise is not a line of the conversation: {so_far}");
    assert!(so_far.contains("Person: what waits on me?"));
    let raised_at = talk.find("What you raised to the person").expect("the raises still open");
    assert!(talk[raised_at..].contains("\"Build the pane\": Approve the plan? Your read: It asks to approve the plan. Your pick: Yes"), "{talk}");
    assert!(talk.contains("journal-<year>-W<week>.md"), "the question names the journal");
}

#[tokio::test]
async fn only_the_first_time_in_front_each_local_day_briefs_and_a_failed_briefing_tries_again() {
    let clock = Arc::new(AtomicI64::new(millis("2026-11-10T12:00:00.000Z")));
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| fittings.clock = Arc::new(Moved(held)));
    let briefing = "journal: Briefed him on the pane.\nbriefing: The pane Job waits on your plan approval.\nNothing else moved.";
    let call = seated(&rig, &[None, Some(briefing), Some(briefing)]).await;
    let briefings = || {
        let fleet = Arc::clone(&rig.fleet);
        async move { fleet.operator_talk_wire().await.expect("read").lines.into_iter().filter(|one| one.briefing).collect::<Vec<_>>() }
    };

    assert!(Arc::clone(&rig.fleet).operator_in_front().await.expect("answered").briefing, "the day's first");
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { Arc::clone(&fleet).operator_in_front().await.expect("answered").briefing }
    })
    .await;
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.operator_talk_wire().await.expect("read").lines.iter().any(|one| one.briefing) }
    })
    .await;
    assert_eq!(call.asked().len(), 2, "the failed briefing kept nothing, and the next time in front tried again");
    let kept = briefings().await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].text, "The pane Job waits on your plan approval.\nNothing else moved.");
    assert!(call.asked()[1].contains("You have not briefed him before."), "{}", call.asked()[1]);
    assert!(journal(&rig).await.contains("Briefed him: The pane Job waits on your plan approval."));

    assert!(!Arc::clone(&rig.fleet).operator_in_front().await.expect("answered").briefing, "the same day again");
    pause().await;
    assert_eq!(call.asked().len(), 2);

    clock.fetch_add(24 * 3_600_000, Ordering::SeqCst);
    assert!(Arc::clone(&rig.fleet).operator_in_front().await.expect("answered").briefing, "the next day's first");
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.operator_talk_wire().await.expect("read").lines.iter().filter(|one| one.briefing).count() == 2 }
    })
    .await;
    assert!(call.asked()[2].contains("You last briefed him at 2026-11-10T12:"), "{}", call.asked()[2]);
}

#[tokio::test]
async fn off_never_briefs() {
    let rig = rig_placing(None);
    let call = Replies::saying(&[Some("briefing: hello")]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    assert!(!Arc::clone(&rig.fleet).operator_in_front().await.expect("answered").briefing);
    pause().await;
    assert!(call.asked().is_empty());
}

#[test]
fn an_iso_week_belongs_to_the_year_its_thursday_falls_in() {
    let week = |at: &str| {
        let reading = crate::clock::reading_at(millis(at), 0);
        (reading.week_year, reading.week, reading.weekday)
    };
    assert_eq!(week("2026-11-10T12:00:00.000Z"), (2026, 46, 2));
    assert_eq!(week("2021-01-01T12:00:00.000Z"), (2020, 53, 5));
    assert_eq!(week("2024-12-30T12:00:00.000Z"), (2025, 1, 1));
    assert_eq!(week("2026-01-04T23:59:00.000Z"), (2026, 1, 7));
}
