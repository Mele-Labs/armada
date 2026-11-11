//! What waits on the person, queued in his Your turn by the Operator: once per item, with the read and
//! the pick its review call made, gone when the item leaves its list, and queued with no read where no
//! call could read it. His presses on it, its order and Later, the rules an Always keeps, Then on a
//! go-ahead to file an issue, a red main nobody is on, and the top line he is given on coming back.
//! None of it is a line of the Operator's chat, which stays as it was.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use core_model::Timestamp;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorLater, OperatorPage, OperatorThen, OperatorTurnItem, OperatorTurnKind, OperatorTurnOrder, OperatorUrgency};
use ipc::{OperatorRead, SessionId, SetWaitingFor, WaitingAct, WaitingActKind, WaitingInput, WaitingOption};
use store::{KeptSuggestion, KeptTouch};

use super::autopilot_decisions_repository::{jobs_titled, red_main};
use super::backlog::manifest;
use super::operator_notes::{filing, leave, TEXT};
use super::operator_sources::switched;
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use crate::clock::Clock;
use crate::main_ci::{MainChange, MainChanged};
use crate::operator::{keep_the_raises, Answering, Calling};
use crate::CallFailed;

/// A call that answers with each of its replies in turn, `None` failing, and keeps what it was asked.
pub(super) struct Replies {
    replies: Mutex<Vec<Option<String>>>,
    asked: Mutex<Vec<String>>,
}

impl Replies {
    pub(super) fn saying(replies: &[Option<&str>]) -> Arc<Replies> {
        Arc::new(Replies { replies: Mutex::new(replies.iter().rev().map(|one| one.map(str::to_string)).collect()), asked: Mutex::new(Vec::new()) })
    }

    pub(super) fn asked(&self) -> Vec<String> {
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

/// A call that waits to be let go before it answers.
struct Gated {
    release: tokio::sync::Notify,
    reply: String,
}

impl Calling for Gated {
    fn ask(&self, _question: String) -> Answering<'_> {
        Box::pin(async move {
            self.release.notified().await;
            Ok(self.reply.clone())
        })
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
pub(super) async fn seated(rig: &Rig, replies: &[Option<&str>]) -> Arc<Replies> {
    let call = Replies::saying(replies);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    switched(rig, "features.operator", true).await;
    call
}

pub(super) fn question(id: &str, text: &str, options: &[&str]) -> WaitingInput {
    WaitingInput {
        approves: false,
        id: id.into(),
        text: text.into(),
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.into() }),
        options: options.iter().map(|label| WaitingOption { label: (*label).into(), description: None }).collect(),
        steps: Vec::new(),
    }
}

/// `job`'s list, as its agent sets it whole.
pub(super) async fn waiting(rig: &Rig, job: &SessionId, items: Vec<WaitingInput>) {
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(job.clone()) }).await.expect("set");
}

pub(super) async fn page(rig: &Rig) -> OperatorPage {
    rig.fleet.operator_page_wire().await.expect("read")
}

/// The item `id` of `job` in Your turn, once it is there.
pub(super) async fn queued(rig: &Rig, job: &SessionId, id: &str) -> OperatorTurnItem {
    for _ in 0..500 {
        if let Some(one) = page(rig).await.turn.into_iter().find(|one| one.session_id.as_ref() == Some(job) && one.item.id == id) {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{id} was not queued: {:#?}", page(rig).await.turn);
}

/// The item whose id is `id`, whichever Job holds it, once it is there.
async fn queued_as(rig: &Rig, id: &str) -> OperatorTurnItem {
    for _ in 0..500 {
        if let Some(one) = page(rig).await.turn.into_iter().find(|one| one.item.id == id) {
            return one;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{id} was not queued: {:#?}", page(rig).await.turn);
}

pub(super) fn press(item: &OperatorTurnItem, answer: OperatorAnswer, then: Option<OperatorThen>) -> OperatorAnswerPress {
    OperatorAnswerPress { id: item.id.clone(), answer, then }
}

fn review(read: &str, pick: &str) -> String {
    format!("item: 1\nread: {read}\npick: {pick}\n\njournal: Reviewed the plan question.")
}

pub(super) async fn pause() {
    tokio::time::sleep(Duration::from_millis(200)).await;
}

pub(super) async fn journal(rig: &Rig) -> String {
    let dir = std::path::Path::new(&rig.fleet.host().operator_journal_dir).to_path_buf();
    let mut all = String::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        all.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
    }
    all
}

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

#[tokio::test]
async fn a_new_item_is_queued_once_with_its_read_and_pick_and_the_chat_stays_as_it_was() {
    let rig = rig_placing(None);
    let reply = review("It asks to approve the plan, which covers the pane only.", "Yes");
    let call = seated(&rig, &[Some(&reply)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;

    let one = queued(&rig, &job, "plan").await;
    assert_eq!(one.read, Some(OperatorRead { read: "It asks to approve the plan, which covers the pane only.".into(), pick: "Yes".into() }));
    assert_eq!((one.kind, one.job.as_deref(), one.item.options.len()), (OperatorTurnKind::Decision, Some("Build the pane"), 2));
    assert!(!one.keeps && one.then.is_empty() && one.later_until.is_none());
    assert!(rig.fleet.operator_talk_wire().await.expect("read").lines.is_empty(), "a queued item is never a line of the chat");
    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("Item 1: Job") && asked[0].contains("Approve the plan?") && asked[0].contains("\"Yes\", \"No\""), "{}", asked[0]);

    // The same list again queues nothing.
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    pause().await;
    assert_eq!(page(&rig).await.turn.len(), 1);
    assert_eq!(call.asked().len(), 1, "no second review");
}

#[tokio::test]
async fn an_item_leaving_its_list_leaves_the_queue_and_the_journal_keeps_both() {
    let rig = rig_placing(None);
    let reply = review("It asks to approve the plan.", "Yes");
    seated(&rig, &[Some(&reply)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    queued(&rig, &job, "plan").await;

    waiting(&rig, &job, Vec::new()).await;

    eventually(|| async { page(&rig).await.turn.is_empty() }).await;
    let written = journal(&rig).await;
    assert!(written.contains("Queued: Build the pane: Approve the plan?. Read: It asks to approve the plan.. Pick: Yes"), "{written}");
    assert!(written.contains("Settled: Build the pane: Approve the plan?"), "{written}");
    assert!(written.contains("Reviewed the plan question."), "the call's own journal line: {written}");
    assert!(written.starts_with("# The Operator's journal, ") && written.contains("\n## "), "{written}");
}

#[tokio::test]
async fn items_open_at_start_and_when_the_switch_comes_on_are_queued_and_none_while_off() {
    let rig = rig_placing(None);
    let call = seated(&rig, &[Some(&review("First.", "Yes")), Some(&review("Second.", "No"))]).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("first", "Approve the plan?", &["Yes", "No"])]).await;

    keep_the_raises(Arc::clone(&rig.fleet)).await;
    assert_eq!(queued(&rig, &job, "first").await.read.expect("read").read, "First.");

    switched(&rig, "features.operator", false).await;
    waiting(&rig, &job, vec![question("first", "Approve the plan?", &["Yes", "No"]), question("second", "Merge it?", &["Yes", "No"])]).await;
    pause().await;
    assert!(page(&rig).await.turn.is_empty(), "off, Your turn is empty");
    assert_eq!(call.asked().len(), 1, "and no call made");

    switched(&rig, "features.operator", true).await;
    let second = queued(&rig, &job, "second").await;
    assert_eq!(second.read, Some(OperatorRead { read: "Second.".into(), pick: "No".into() }));
    assert_eq!(page(&rig).await.turn.len(), 2, "the first is not queued again");
}

#[tokio::test]
async fn a_failed_or_unreadable_review_still_queues_the_item_with_no_read() {
    let rig = rig_placing(None);
    seated(&rig, &[None, Some("I would rather not say.")]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("plan", "Approve the plan", &["Yes", "No"])]).await;
    assert_eq!(queued(&rig, &job, "plan").await.read, None);

    waiting(&rig, &job, vec![question("plan", "Approve the plan", &["Yes", "No"]), question("merge", "Merge it?", &["Yes", "No"])]).await;
    assert_eq!(queued(&rig, &job, "merge").await.read, None);
}

#[tokio::test]
async fn a_pick_naming_an_option_not_offered_is_dropped_and_the_read_stays() {
    let rig = rig_placing(None);
    seated(&rig, &[Some(&review("It is safe to merge.", "Merge and deploy"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("merge", "Merge it?", &["Yes", "No"])]).await;

    assert_eq!(queued(&rig, &job, "merge").await.read, Some(OperatorRead { read: "It is safe to merge.".into(), pick: String::new() }));
}

#[tokio::test]
async fn while_a_review_is_out_the_page_says_what_the_operator_is_on() {
    let rig = rig_placing(None);
    let gated = Arc::new(Gated { release: tokio::sync::Notify::new(), reply: review("It covers the pane.", "Yes") });
    assert!(rig.fleet.operator_seated_with(gated.clone()));
    switched(&rig, "features.operator", true).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    assert!(page(&rig).await.on_it.is_none());

    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;

    eventually(|| async { page(&rig).await.on_it.is_some() }).await;
    let on_it = page(&rig).await.on_it.expect("on it");
    assert_eq!(on_it.reading, "Build the pane: Approve the plan?");
    assert!(page(&rig).await.turn.is_empty(), "nothing is queued until it has read it");
    gated.release.notify_one();
    queued(&rig, &job, "plan").await;
    eventually(|| async { page(&rig).await.on_it.is_none() }).await;
}

#[tokio::test]
async fn he_orders_the_queue_and_puts_one_later_for_the_rest_of_the_local_day() {
    let clock = Arc::new(AtomicI64::new(millis("2026-11-10T12:00:00.000Z")));
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| fittings.clock = Arc::new(Moved(held)));
    let both = "item: 1\nread: A.\npick: Yes\n\nitem: 2\nread: B.\npick: Yes\n\njournal: Reviewed two.";
    seated(&rig, &[Some(both)]).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("a", "First?", &["Yes", "No"]), question("b", "Second?", &["Yes", "No"]), question("c", "Third?", &["Yes", "No"])]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    queued(&rig, &job, "c").await;
    let ids = |page: &OperatorPage| page.turn.iter().map(|one| one.item.id.clone()).collect::<Vec<_>>();
    let turn = page(&rig).await.turn;
    assert_eq!(turn.len(), 3);
    let (a, b, c) = (turn.iter().find(|one| one.item.id == "a").expect("a").id.clone(), turn.iter().find(|one| one.item.id == "b").expect("b").id.clone(), turn.iter().find(|one| one.item.id == "c").expect("c").id.clone());

    let dragged = rig.fleet.operator_turn_ordered(OperatorTurnOrder { ids: vec![c.clone(), "turn:9999".into(), a.clone()] }).await.expect("ordered");
    assert_eq!(ids(&dragged), ["c", "a", "b"], "the ones he named first, the rest as they were");

    let later = rig.fleet.operator_turn_later(OperatorLater { id: c.clone() }).await.expect("later");
    assert_eq!(ids(&later), ["a", "b", "c"], "Later puts it last");
    assert!(later.turn[2].later_until.is_some());

    clock.fetch_add(48 * 3_600_000, Ordering::SeqCst);
    let next_day = page(&rig).await;
    assert_eq!(ids(&next_day), ["c", "a", "b"], "the day is over, so it is back where he left it");
    assert!(next_day.turn.iter().all(|one| one.later_until.is_none()));

    let unknown = rig.fleet.operator_turn_later(OperatorLater { id: "turn:9999".into() }).await.expect_err("no such item");
    assert_eq!(code(unknown), (404, "fleet.operator_turn_unknown".to_string()));
    let _ = b;
}

#[tokio::test]
async fn yes_answers_the_option_the_operator_picked_and_the_item_is_settled() {
    let rig = rig_placing(None);
    seated(&rig, &[Some(&review("It covers the pane only.", "Yes"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["No", "Yes"])]).await;
    let one = queued(&rig, &job, "plan").await;

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, None)).await.expect("answered");

    assert!(answered.open.is_none() && answered.page.turn.is_empty(), "{answered:#?}");
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    assert!(record.waiting_for.iter().all(|item| item.id != "plan"), "the option picked, the second, was answered on the Job's list");
    assert!(journal(&rig).await.contains("Answered yes: Build the pane: Approve the plan?"));
    let again = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, None)).await.expect_err("settled");
    assert_eq!(code(again), (404, "fleet.operator_turn_unknown".to_string()));
}

#[tokio::test]
async fn no_drops_the_item_and_it_is_not_queued_again_while_it_stays_on_its_list() {
    let rig = rig_placing(None);
    let call = seated(&rig, &[Some(&review("Not yet.", "No"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    let one = queued(&rig, &job, "plan").await;

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::No, None)).await.expect("answered");
    assert!(answered.page.turn.is_empty());
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"]), question("other", "Merge it?", &["Yes", "No"])]).await;
    queued(&rig, &job, "other").await;

    assert!(page(&rig).await.turn.iter().all(|one| one.item.id != "plan"), "the item he dropped stays dropped while it is still there");
    assert_eq!(call.asked().len(), 2, "only the other was reviewed");
}

#[tokio::test]
async fn yes_on_an_item_fleet_cannot_perform_hands_its_act_to_bridge_and_it_stands_until_its_list_drops_it() {
    let rig = rig_placing(None);
    seated(&rig, &[Some(&review("It is ready to read.", "Open the pull request"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let walk = WaitingAct { kind: WaitingActKind::ApprovePr, target: "3421".into() };
    waiting(&rig, &job, vec![WaitingInput { approves: false, id: "pr".into(), text: "Approve the pull request".into(), act: Some(walk.clone()), options: Vec::new(), steps: Vec::new() }]).await;
    let one = queued(&rig, &job, "pr").await;

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, None)).await.expect("answered");

    assert_eq!(answered.open, Some(walk));
    assert_eq!(answered.page.turn.len(), 1, "it stands");
    waiting(&rig, &job, Vec::new()).await;
    eventually(|| async { page(&rig).await.turn.is_empty() }).await;
}

#[tokio::test]
async fn always_is_refused_on_a_decision() {
    let rig = rig_placing(None);
    seated(&rig, &[Some(&review("It covers the pane.", "Yes"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    let one = queued(&rig, &job, "plan").await;

    for answer in [OperatorAnswer::Always, OperatorAnswer::AlwaysHere] {
        let refused = rig.fleet.operator_answered(press(&one, answer, None)).await.expect_err("not offered");
        assert_eq!(code(refused), (422, "fleet.operator_answer_not_offered".to_string()));
    }
    assert_eq!(page(&rig).await.turn.len(), 1, "nothing was done");
    assert!(page(&rig).await.digest.learned.is_empty());
}

/// A suggestion of the Operator's to file the note `n`, as the Operator keeps it, with its note left.
async fn file_go_ahead(rig: &Rig, n: &str, text: &str) {
    leave(rig, n, "open", None, text);
    let kept = KeptSuggestion {
        n: 0,
        arrival: format!("note:{}:{n}", manifest(rig).as_str()),
        arrived: "note".into(),
        way: "file".into(),
        session_id: String::new(),
        manifest_id: manifest(rig).as_str().to_string(),
        subject: text.into(),
        words: format!("{text}\n\nLeft on the usage screen."),
        group: None,
        other_session: None,
        shared: Vec::new(),
        first_session: None,
        step: None,
        item: None,
        pick: None,
        because: "no open issue covers it".into(),
        what: None,
        text: format!("The Operator would file your note \"{text}\" as a new issue: no open issue covers it"),
        raised_at: "2026-11-10T09:00:00.000Z".into(),
    };
    rig.fleet.store().lock().await.keep_operator_suggestion(&kept).expect("kept");
}

const FILE_PICK: &str = "item: 1\nread: No open issue covers it.\npick: Do it\nthen: just_file\n\njournal: Reviewed a note.";

#[tokio::test]
async fn always_here_on_a_go_ahead_does_it_keeps_a_rule_and_the_next_one_is_done_with_no_press_while_on() {
    let rig = filing().await;
    let call = Replies::saying(&[Some(FILE_PICK)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    file_go_ahead(&rig, "n-1", TEXT).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let one = queued_as(&rig, &format!("suggestion:{}", rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")[0].n)).await;
    assert_eq!((one.kind, one.keeps), (OperatorTurnKind::GoAhead, true));

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::AlwaysHere, Some(OperatorThen::JustFile))).await.expect("answered");

    let issue = rig.fleet.open_issues(&manifest(&rig)).await.expect("read").into_iter().find(|issue| issue.title.as_written() == TEXT).expect("filed");
    assert_eq!(answered.filed.as_deref(), Some(issue.reference.as_str()));
    assert_eq!(answered.page.digest.learned.len(), 1);
    let learned = &answered.page.digest.learned[0];
    assert!(learned.said.starts_with("File ") && learned.said.ends_with(" as an issue"), "{}", learned.said);
    assert_eq!((learned.manifest_id.as_deref(), learned.repository.is_some()), (Some(manifest(&rig).as_str()), true));
    // A rule is no standing order: the orders a person sees do not list it.
    assert!(rig.fleet.operator_orders_wire().await.orders.is_empty());

    // The next note of the kind is filed as it arrives, and never queued.
    file_go_ahead(&rig, "n-2", "The sidebar should remember its width").await;
    rig.fleet.events().publish(ipc::Event::OperatorBoardChanged(ipc::operator_board::OperatorBoardChanged {}));
    eventually(|| async { rig.fleet.open_issues(&manifest(&rig)).await.expect("read").iter().any(|issue| issue.title.as_written() == "The sidebar should remember its width") }).await;
    eventually(|| async { !page(&rig).await.digest.handled.acts.is_empty() }).await;
    let after = page(&rig).await;
    assert!(after.turn.is_empty(), "{:#?}", after.turn);
    assert!(after.digest.handled.acts.iter().any(|act| act.outcome.starts_with("By a rule: ") && act.title.contains("sidebar")), "{:#?}", after.digest.handled);
    assert_eq!(call.asked().len(), 1, "no review for what a rule did");
    assert!(journal(&rig).await.contains("By a rule, File "));

    // While the Operator is off no rule applies, and nothing is queued either.
    switched(&rig, "features.operator", false).await;
    file_go_ahead(&rig, "n-3", "The tabs should scroll").await;
    rig.fleet.events().publish(ipc::Event::OperatorBoardChanged(ipc::operator_board::OperatorBoardChanged {}));
    pause().await;
    assert!(!rig.fleet.open_issues(&manifest(&rig)).await.expect("read").iter().any(|issue| issue.title.as_written() == "The tabs should scroll"));
    assert!(page(&rig).await.turn.is_empty());
}

#[tokio::test]
async fn a_go_ahead_to_file_offers_hand_only_where_a_live_job_works_the_same_files_and_each_then_is_done() {
    let rig = filing().await;
    let call = Replies::saying(&[Some("item: 1\nread: A live Job is on those files.\npick: Do it\nthen: file_and_hand\n\njournal: Reviewed a note.")]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let job = rig.start_titled(Some("Fix the scrollback")).await;
    let text = "Scrollback is lost on redraw in src/terminal/scrollback.rs";
    file_go_ahead(&rig, "n-1", text).await;
    let touch = KeptTouch { by: None, path: "src/terminal/scrollback.rs".into(), act: "edit".into(), at: "2026-11-10T08:00:00.000Z".into() };
    rig.fleet.store().lock().await.keep_session_touch(job.as_str(), &touch, 50).expect("kept");
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let id = format!("suggestion:{}", rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")[0].n);
    let one = queued_as(&rig, &id).await;

    let thens: Vec<_> = one.then.iter().map(|offer| (offer.then, offer.pick, offer.hand_to.as_ref().map(|to| to.session_id.clone()))).collect();
    assert_eq!(thens, [(OperatorThen::JustFile, false, None), (OperatorThen::FileAndStart, false, None), (OperatorThen::FileAndHand, true, Some(job.clone()))], "the Operator's read marks hand as its pick");
    assert!(call.asked()[0].contains("Its Thens, what follows filing it: just_file, file_and_start, file_and_hand (to Job"), "{}", call.asked()[0]);

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, Some(OperatorThen::FileAndStart))).await.expect("answered");

    assert!(answered.filed.is_some(), "{answered:#?}");
    assert!(answered.started.is_some() && answered.handed_to.is_none());
    assert_eq!(jobs_titled(&rig, text).await, 1, "a Job was started on it");
}

#[tokio::test]
async fn file_and_hand_hands_the_filed_issue_to_the_job_and_is_refused_where_no_job_works_the_same_files() {
    let rig = filing().await;
    assert!(rig.fleet.operator_seated_with(Replies::saying(&[Some(FILE_PICK), Some(FILE_PICK)])));
    let job = rig.start_titled(Some("Fix the scrollback")).await;
    file_go_ahead(&rig, "n-1", "Scrollback is lost on redraw in src/terminal/scrollback.rs").await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let id = format!("suggestion:{}", rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")[0].n);
    let one = queued_as(&rig, &id).await;
    assert_eq!(one.then.iter().map(|offer| offer.then).collect::<Vec<_>>(), [OperatorThen::JustFile, OperatorThen::FileAndStart], "no Job has written those files");

    let refused = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, Some(OperatorThen::FileAndHand))).await.expect_err("not offered");
    assert_eq!(code(refused), (422, "fleet.operator_answer_not_offered".to_string()));
    assert!(rig.fleet.open_issues(&manifest(&rig)).await.expect("read").is_empty(), "nothing was filed");

    let touch = KeptTouch { by: None, path: "src/terminal/scrollback.rs".into(), act: "edit".into(), at: "2026-11-10T08:00:00.000Z".into() };
    rig.fleet.store().lock().await.keep_session_touch(job.as_str(), &touch, 50).expect("kept");
    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, Some(OperatorThen::FileAndHand))).await.expect("answered");
    assert!(answered.filed.is_some());
    assert_eq!(answered.handed_to, Some(job));
}

#[tokio::test]
async fn a_red_main_nobody_is_on_joins_the_queue_and_yes_starts_a_fix_at_once_with_no_plan_step() {
    let rig = rig_placing(None);
    seated(&rig, &[Some("item: 1\nread: A test fails on main.\npick: Start a fix\n\njournal: Reviewed a red main.")]).await;
    let commit = "b".repeat(40);
    red_main(&rig, &commit).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    let went_red = [MainChanged { change: MainChange::WentRed, now: main }];

    rig.fleet.red_offered(&went_red).await;
    rig.fleet.red_offered(&went_red).await;

    let one = queued_as(&rig, &format!("red:{}:{commit}", manifest(&rig).as_str())).await;
    assert_eq!((one.kind, one.keeps, one.session_id.is_none()), (OperatorTurnKind::GoAhead, true, true));
    assert_eq!(one.item.options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Start a fix"]);
    assert!(one.item.text.starts_with("Main is red at bbbbbbb"), "{}", one.item.text);
    assert_eq!(one.read.as_ref().map(|read| read.pick.as_str()), Some("Start a fix"), "the pick is its one option");
    assert_eq!(page(&rig).await.turn.len(), 1, "once for each red commit");

    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, None)).await.expect("answered");

    let job = answered.started.expect("a Job started");
    assert_eq!(jobs_titled(&rig, "Fix main").await, 1);
    let rows = rig.fleet.store().lock().await.session_rows(job.as_str()).expect("rows").join("\n");
    assert!(rows.contains("There is no plan step") && rows.contains("pull request ready"), "{rows}");
    assert!(answered.page.turn.is_empty(), "a Job is on it, so it is no longer waiting");
    let again = rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, None)).await.expect_err("settled");
    assert_eq!(again.status(), 404);
}

#[tokio::test]
async fn a_red_main_reads_urgent() {
    let rig = rig_placing(None);
    seated(&rig, &[Some("item: 1\nread: A test fails on main.\npick: Start a fix\n\njournal: Reviewed a red main.")]).await;
    let commit = "c".repeat(40);
    red_main(&rig, &commit).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();

    rig.fleet.red_offered(&[MainChanged { change: MainChange::WentRed, now: main }]).await;

    let one = queued_as(&rig, &format!("red:{}:{commit}", manifest(&rig).as_str())).await;
    assert_eq!(one.urgency, OperatorUrgency::Urgent);
}

#[tokio::test]
async fn a_decision_reads_decision() {
    let rig = rig_placing(None);
    seated(&rig, &[Some(&review("It asks to approve the plan.", "Yes"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;

    let one = queued(&rig, &job, "plan").await;
    assert_eq!((one.kind, one.urgency), (OperatorTurnKind::Decision, OperatorUrgency::Decision));
}

#[tokio::test]
async fn a_go_ahead_reads_go_ahead() {
    let rig = filing().await;
    assert!(rig.fleet.operator_seated_with(Replies::saying(&[Some(FILE_PICK)])));
    file_go_ahead(&rig, "n-1", TEXT).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;

    let one = queued_as(&rig, &format!("suggestion:{}", rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")[0].n)).await;

    assert_eq!((one.kind, one.urgency), (OperatorTurnKind::GoAhead, OperatorUrgency::GoAhead));
}

#[tokio::test]
async fn a_message_about_a_queued_item_is_handed_it_with_the_operators_own_read() {
    let rig = rig_placing(None);
    let said = "cites: none\njob: none\nstart: none\norder: none\nsaid: The pane waits on you.";
    let call = seated(&rig, &[Some(&review("It asks to approve the plan.", "Yes")), Some(said)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("plan", "Approve the plan?", &["Yes", "No"])]).await;
    let one = queued(&rig, &job, "plan").await;

    rig.fleet.operator_talked("why yes?", Some(&one.id)).await.expect("answered");

    let asked = call.asked();
    let talk = asked.last().expect("asked");
    assert!(talk.contains(&format!("Job \"Build the pane\"'s item in the person's Your turn ({}): Approve the plan? Your read: It asks to approve the plan. Your pick: Yes", one.id)), "{talk}");
    assert!(talk.contains("journal-<year>-W<week>.md"), "the question names the journal");
    assert!(journal(&rig).await.contains("He wrote: why yes? I answered: The pane waits on you."));
}

#[tokio::test]
async fn coming_back_after_time_away_sets_when_he_last_looked_and_rewrites_the_top_line() {
    let clock = Arc::new(AtomicI64::new(millis("2026-11-10T12:00:00.000Z")));
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| fittings.clock = Arc::new(Moved(held)));
    let top = "journal: Told him about the pane.\ntop: Since you left, the pane's plan is waiting on you.";
    let call = seated(&rig, &[Some(top), None, Some("journal: Told him again.\ntop: The pane is still waiting.")]).await;

    assert!(rig.fleet.operator_in_front_now().await, "the first time ever");
    eventually(|| async { page(&rig).await.top.is_some() }).await;
    let first = page(&rig).await;
    assert_eq!(first.top.as_deref(), Some("Since you left, the pane's plan is waiting on you."));
    let looked = first.looked_at.expect("looked").as_str().to_string();
    assert!(looked.as_str() < "2026-11-10T00:00:00.000Z" || looked.starts_with("2026-11-09"), "the first counts from the start of the day before: {looked}");
    assert!(call.asked()[0].contains("Nothing moved and nothing waits on him."), "{}", call.asked()[0]);

    clock.fetch_add(60_000, Ordering::SeqCst);
    assert!(!rig.fleet.operator_in_front_now().await, "a minute later is not a return");
    pause().await;
    assert_eq!(call.asked().len(), 1);
    assert_eq!(page(&rig).await.looked_at.expect("looked").as_str(), looked);

    clock.fetch_add(3_600_000, Ordering::SeqCst);
    assert!(rig.fleet.operator_in_front_now().await, "an hour away is");
    pause().await;
    let second = page(&rig).await;
    assert_eq!(second.top.as_deref(), Some("Since you left, the pane's plan is waiting on you."), "its call failed, so the line is as it was");
    assert!(second.looked_at.expect("looked").as_str() > looked.as_str(), "and he looked when he came back");

    clock.fetch_add(3_600_000, Ordering::SeqCst);
    assert!(rig.fleet.operator_in_front_now().await);
    eventually(|| async { page(&rig).await.top.as_deref() == Some("The pane is still waiting.") }).await;
    assert!(journal(&rig).await.contains("Told him: The pane is still waiting."));
}

#[tokio::test]
async fn off_a_return_moves_nothing() {
    let rig = rig_placing(None);
    let call = Replies::saying(&[Some("top: hello")]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    assert!(!rig.fleet.operator_in_front_now().await);
    pause().await;
    assert!(call.asked().is_empty());
    // The head's mains and slots are Fleet's own, whether the Operator is on or off.
    let page = OperatorPage { mains: Vec::new(), slots: None, ..page(&rig).await };
    assert_eq!(page, OperatorPage::default());
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
