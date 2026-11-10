//! The box at the foot of the Operator panel on a real Fleet: the conversation it is handed and
//! keeps, what it refuses, the holds Fleet goes by once he has confirmed them, and the rules and holds
//! he has it forget and lift in words.

use std::sync::{Arc, Mutex};

use ipc::operator_talk::{OperatorHoldUntil, OperatorOrderForm};

use super::operator_acting::{asked, operator_on};
use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_queue::close;
use crate::operator::{Answering, Calling};
use crate::CallFailed;

/// A call that answers with each of its replies in turn, and keeps what it was asked.
#[derive(Default)]
struct Replies {
    replies: Mutex<Vec<String>>,
    asked: Mutex<Vec<String>>,
}

impl Replies {
    fn saying(replies: &[&str]) -> Arc<Replies> {
        Arc::new(Replies { replies: Mutex::new(replies.iter().rev().map(|one| (*one).to_string()).collect()), asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl Calling for Replies {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let reply = self.replies.lock().expect("not poisoned").pop();
        Box::pin(async move { reply.ok_or(CallFailed::SaidNothing) })
    }
}

fn reply(order: &str, said: &str) -> String {
    format!("cites: none\njob: none\nstart: none\norder: {order}\nsaid: {said}")
}

async fn seated(rig: &Rig, replies: &[&str]) -> Arc<Replies> {
    operator_on(rig).await;
    let call = Replies::saying(replies);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    call
}

fn code(refused: api::Refusal) -> String {
    format!("{refused:?}")
}

#[tokio::test]
async fn the_conversation_is_kept_and_handed_to_the_next_call_and_each_answer_is_kept() {
    let rig = rig_placing(None);
    let first = reply("none", "Nothing has been raised today.");
    let second = reply("none", "Because nothing arrived.");
    let call = seated(&rig, &[&first, &second]).await;

    let said = rig.fleet.operator_talked("what is going on?", None).await.expect("answered");
    assert_eq!(said.said, "Nothing has been raised today.");
    rig.fleet.operator_talked("why?", None).await.expect("answered");

    let asked = call.asked();
    assert!(!asked[0].contains("You: Nothing has been raised"));
    assert!(asked[1].contains("Person: what is going on?") && asked[1].contains("You: Nothing has been raised today.") && asked[1].contains("Person: why?"), "{}", asked[1]);
    let talk = rig.fleet.operator_talk_wire().await.expect("read");
    assert_eq!(talk.lines.iter().map(|one| (one.from_person, one.text.as_str())).collect::<Vec<_>>(), vec![
        (true, "what is going on?"),
        (false, "Nothing has been raised today."),
        (true, "why?"),
        (false, "Because nothing arrived."),
    ]);
}

#[tokio::test]
async fn it_cannot_be_talked_to_while_off_nor_past_a_failed_call_and_keeps_nothing_then() {
    let rig = rig_placing(None);
    let call = Replies::saying(&[]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    assert!(code(rig.fleet.operator_talked("hello", None).await.expect_err("off")).contains("fleet.operator_off"));
    assert!(call.asked().is_empty(), "no call");

    operator_on(&rig).await;
    assert!(code(rig.fleet.operator_talked("hello", None).await.expect_err("said nothing")).contains("fleet.operator_said_nothing"));
    assert!(rig.fleet.operator_talk_wire().await.expect("read").lines.is_empty());
}

#[tokio::test]
async fn words_that_fit_no_form_list_the_forms_and_a_hold_naming_a_job_that_is_not_there_is_no_form() {
    let rig = rig_placing(None);
    let unfit = reply("no_form", "No form holds that.");
    let unknown = reply("hold apps/cockpit until 01NOSUCHJOB pushes", "Done.");
    seated(&rig, &[&unfit, &unknown]).await;

    for words in ["be more careful", "hold the cockpit until some Job pushes"] {
        let said = rig.fleet.operator_talked(words, None).await.expect("answered");
        assert!(said.order.is_none(), "{words}");
        assert_eq!(said.forms.len(), 1, "{words}");
    }
    assert!(rig.fleet.operator_orders_wire().await.orders.is_empty(), "nothing stands");
    let holder = asked(&rig, "Build the console").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let elsewhere = OperatorOrderForm::HoldPaths { manifest_id, paths: vec!["  ".into()], job: holder, until: OperatorHoldUntil::Ends };
    let refused = rig.fleet.operator_order_confirmed(elsewhere).await.expect_err("no paths");
    assert!(code(refused).contains("fleet.operator_order_not_a_form"));
}

#[tokio::test]
async fn a_hold_refuses_another_jobs_write_under_its_paths_and_lifts_when_its_job_ends() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
    let holder = asked(&rig, "Build the console").await;
    let other = asked(&rig, "Tidy the cockpit").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let form = OperatorOrderForm::HoldPaths { manifest_id, paths: vec!["apps/cockpit".into()], job: holder.clone(), until: OperatorHoldUntil::Pushes };
    let orders = rig.fleet.operator_order_confirmed(form).await.expect("it stands");
    assert!(orders.orders[0].said.starts_with("Hold apps/cockpit until "), "{}", orders.orders[0].said);

    let held = rig.gate(&other, "Write", &format!(r#"{{"file_path":"{}/apps/cockpit/deck.tsx"}}"#, rig.root)).await;
    assert!(denied(&held), "{held:?}");
    assert!(format!("{held:?}").contains("standing order"), "{held:?}");
    let elsewhere = rig.gate(&other, "Write", &format!(r#"{{"file_path":"{}/apps/cockpits/deck.tsx"}}"#, rig.root)).await;
    assert!(!format!("{elsewhere:?}").contains("standing order"), "a path that only begins the same is not held");
    let own = rig.gate(&holder, "Write", &format!(r#"{{"file_path":"{}/apps/cockpit/deck.tsx"}}"#, rig.root)).await;
    assert!(!format!("{own:?}").contains("standing order"), "the Job it waits on writes");

    close(&rig, &holder).await;
    let lifted = rig.gate(&other, "Write", &format!(r#"{{"file_path":"{}/apps/cockpit/deck.tsx"}}"#, rig.root)).await;
    assert!(!format!("{lifted:?}").contains("standing order"), "{lifted:?}");
    assert!(rig.fleet.operator_orders_wire().await.orders.is_empty(), "its Job ended, so the hold is let go");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert!(log.iter().any(|row| row.outcome == "hold_lifted" && row.because.contains("apps/cockpit")), "{log:?}");
}

#[tokio::test]
async fn the_digest_lists_the_holds_beside_the_rules_and_a_lift_in_the_chat_lets_the_other_job_write() {
    let rig = rig_placing(None);
    let call = seated(&rig, &[]).await;
    let holder = asked(&rig, "Build the console").await;
    let other = asked(&rig, "Tidy the cockpit").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let form = OperatorOrderForm::HoldPaths { manifest_id: manifest_id.clone(), paths: vec!["apps/cockpit".into()], job: holder.clone(), until: OperatorHoldUntil::Ends };
    let hold = rig.fleet.operator_order_confirmed(form).await.expect("it stands").orders[0].id;
    let rule = ipc::operator_page::OperatorRule::PickRule { manifest_id: None, what: "note:file".into(), pick: "just_file".into() };
    let rule = rig.fleet.store().lock().await.add_operator_order(&store::KeptOrder { n: 0, form: rule.kept(), said: "File notes".into(), until: None, at: "2026-11-10T09:00:00.000Z".into() }).expect("kept");
    let path = format!(r#"{{"file_path":"{}/apps/cockpit/deck.tsx"}}"#, rig.root);
    let write = || rig.gate(&other, "Write", &path);

    let digest = rig.fleet.operator_page_wire().await.expect("read").digest;
    assert_eq!(digest.learned.iter().map(|one| one.id).collect::<Vec<_>>(), [rule], "Learned is rules only");
    assert_eq!(digest.holds.len(), 1, "{:?}", digest.holds);
    assert_eq!((digest.holds[0].id, digest.holds[0].manifest_id.as_str()), (hold, manifest_id.as_str()));
    assert!(digest.holds[0].said.starts_with("Hold apps/cockpit until "), "{}", digest.holds[0].said);
    assert!(denied(&write().await), "the hold refuses the other Job");

    // A rule's id is no hold's to lift, and a hold's is no rule's to forget.
    call.replies.lock().expect("not poisoned").push(format!("cites: none\njob: none\nstart: none\norder: none\nforget: {hold}\nlift: {rule}\nsaid: Done."));
    let crossed = rig.fleet.operator_talked("forget the hold and lift the rule", None).await.expect("answered");
    assert_eq!((crossed.forgot, crossed.lifted), (None, None));
    assert!(denied(&write().await), "the hold stands");
    assert_eq!(rig.fleet.operator_page_wire().await.expect("read").digest.learned.len(), 1, "and so does the rule");

    call.replies.lock().expect("not poisoned").push(format!("cites: none\njob: none\nstart: none\norder: none\nlift: {hold}\nsaid: The hold is lifted."));
    let lifted = rig.fleet.operator_talked("lift the hold on the cockpit", None).await.expect("answered");

    assert_eq!(lifted.lifted, Some(hold));
    let after = write().await;
    assert!(!format!("{after:?}").contains("standing order"), "{after:?}");
    let digest = rig.fleet.operator_page_wire().await.expect("read").digest;
    assert!(digest.holds.is_empty() && digest.learned.len() == 1, "{digest:?}");
    let log = rig.fleet.store().lock().await.operator_rulings(10).expect("read");
    assert!(log.iter().any(|row| row.outcome == "hold_lifted" && row.because.contains("apps/cockpit") && row.because.contains("your word")), "{log:?}");
}

#[tokio::test]
async fn a_hold_stands_over_a_restart_and_an_old_form_stays_in_the_store_and_is_read_by_nothing() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
    let holder = asked(&rig, "Build the console").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let form = OperatorOrderForm::HoldPaths { manifest_id, paths: vec!["apps/cockpit".into()], job: holder, until: OperatorHoldUntil::Ends };
    rig.fleet.operator_order_confirmed(form.clone()).await.expect("it stands");
    for old in [r#"{"kind":"role_off","role":"file","manifest_id":"01REPO","today":false}"#, r#"{"kind":"always_raise","what":"report"}"#] {
        let one = store::KeptOrder { n: 0, form: old.into(), said: "old".into(), until: None, at: "2026-11-09T09:00:00Z".into() };
        rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
    }
    rig.fleet.operator_seat().orders_are(crate::operator::Orders::default());
    assert!(rig.fleet.operator_seat().orders().0.is_empty());

    rig.fleet.operator_orders_read().await;

    let seat = rig.fleet.operator_seat();
    assert_eq!(seat.orders().0.iter().map(|order| order.form.clone()).collect::<Vec<_>>(), [form.clone()], "the hold is served and the old forms are read by nothing");
    let wire = rig.fleet.operator_orders_wire().await;
    assert_eq!(wire.orders.len(), 1, "and are not on the board");
    assert_eq!(rig.fleet.store().lock().await.operator_orders().expect("read").len(), 3, "nothing is deleted");
}

#[tokio::test]
async fn a_job_that_ended_today_is_shown_to_the_box_with_what_it_said_and_who_started_it() {
    let rig = rig_placing(None);
    let first = reply("none", "Nothing else.");
    let call = seated(&rig, &[&first]).await;
    let starter = asked(&rig, "Evaluate the architecture").await;
    let ended = asked(&rig, "Compare the editors").await;
    let still_live = asked(&rig, "Tidy the cockpit").await;
    let (starter_title, ended_title) = {
        let store = rig.fleet.store().lock().await;
        (
            store.session(starter.as_str()).expect("read").expect("kept").title.expect("titled"),
            store.session(ended.as_str()).expect("read").expect("kept").title.expect("titled"),
        )
    };
    {
        let mut store = rig.fleet.store().lock().await;
        let at = rig.fleet.now().as_str().to_string();
        let link = store::KeptAttachment {
            holder: store::Holder::session(ended.as_str()),
            kind: "started_by".into(),
            manifest_id: rig.manifest.as_str().to_string(),
            target: starter.as_str().into(),
            state: store::AttachmentState::Spent,
            detail: Default::default(),
            since: at.clone(),
            changed_at: at.clone(),
        };
        store.attach(&link, false).expect("kept");
        store.keep_done(ended.as_str(), &at, Some("Chose the\nsecond editor.")).expect("said");
    }
    close(&rig, &ended).await;

    rig.fleet.operator_talked("what ended today?", None).await.expect("answered");

    let asked = call.asked();
    let section = asked[0].split("\nEnded today:\n").nth(1).expect("a section of its own").split("\n\n").next().expect("a section");
    assert_eq!(
        section,
        format!("  Job {} \"{ended_title}\": Chose the second editor. (started by Job {} \"{starter_title}\")", ended.as_str(), starter.as_str()),
        "{}",
        asked[0]
    );
    assert!(!section.contains(still_live.as_str()), "a Job still live did not end: {section}");
    let board = rig.fleet.operator_board().await;
    assert_eq!(board.ended.len(), 1, "{:?}", board.ended);
}

/// A suggestion no Job holds, as a pull request merging leaves one.
fn suggestion(text: &str) -> store::KeptSuggestion {
    store::KeptSuggestion {
        n: 0,
        arrival: "merged:42".into(),
        arrived: "merged".into(),
        way: "file".into(),
        session_id: String::new(),
        manifest_id: String::new(),
        subject: "Pull request 42".into(),
        words: String::new(),
        group: None,
        other_session: None,
        shared: Vec::new(),
        first_session: None,
        step: None,
        item: None,
        pick: None,
        because: "nothing else covers it".into(),
        what: None,
        text: text.into(),
        raised_at: "2026-11-09T09:00:00.000Z".into(),
    }
}

const MERGED_TEXT: &str = "The Operator would file \"Document the retry cap\" as a new issue now that #42 merged: nothing else covers it";

#[tokio::test]
async fn discussing_a_suggestion_no_job_holds_hands_the_call_its_full_text_and_lists_it_among_those_waiting() {
    let rig = rig_placing(None);
    let first = reply("none", "It would file an issue.");
    let call = seated(&rig, &[&first]).await;
    let n = {
        let mut store = rig.fleet.store().lock().await;
        store.keep_operator_suggestion(&suggestion(MERGED_TEXT)).expect("kept");
        store.operator_suggestions_of("").expect("read")[0].n
    };
    let id = format!("suggestion:{n}");

    rig.fleet.operator_talked("what is this?", Some(&id)).await.expect("answered");

    let asked = call.asked();
    let listed = asked[0].split("\nYour suggestions, waiting on the person to press Do it or Skip:\n").nth(1).expect("a section of its own").split("\n\n").next().expect("a section");
    assert_eq!(listed, format!("  {id}: {MERGED_TEXT}"), "{}", asked[0]);
    assert!(
        asked[0].contains(&format!("The person is talking about this item: your suggestion ({id}), waiting on the person to press Do it or Skip: {MERGED_TEXT}")),
        "{}",
        asked[0]
    );
}

#[tokio::test]
async fn a_settled_suggestion_is_no_longer_on_the_board_and_is_not_listed() {
    let rig = rig_placing(None);
    let first = reply("none", "It is settled.");
    let call = seated(&rig, &[&first]).await;
    let n = {
        let mut store = rig.fleet.store().lock().await;
        store.keep_operator_suggestion(&suggestion(MERGED_TEXT)).expect("kept");
        let n = store.operator_suggestions_of("").expect("read")[0].n;
        assert!(store.settle_operator_suggestion(n, false, "2026-11-09T10:00:00.000Z").expect("settled"));
        n
    };

    rig.fleet.operator_talked("and this?", Some(&format!("suggestion:{n}"))).await.expect("answered");

    let asked = call.asked();
    assert!(asked[0].contains(&format!("The person is talking about item suggestion:{n}, which is no longer on the board.")), "{}", asked[0]);
    assert!(!asked[0].contains(MERGED_TEXT), "{}", asked[0]);
}

#[tokio::test]
async fn an_item_that_names_nothing_is_said_to_the_call_and_not_dropped() {
    let rig = rig_placing(None);
    let first = reply("none", "I cannot find it.");
    let call = seated(&rig, &[&first]).await;

    rig.fleet.operator_talked("what about this one?", Some("suggestion:9999")).await.expect("answered");

    let asked = call.asked();
    assert!(asked[0].contains("\nThe person is talking about item suggestion:9999, which is no longer on the board.\n  Person: what about this one?"), "{}", asked[0]);
}

#[tokio::test]
async fn discussing_a_jobs_needs_you_item_hands_the_call_that_item_and_ties_it_to_the_job_it_is_on() {
    let rig = rig_placing(None);
    let first = reply("none", "It asks whether to land.");
    let call = seated(&rig, &[&first]).await;
    let job = rig.start().await;
    rig.send(&job, "cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.\n\nWant me to land it?");
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { !fleet.operator_board().await.needs_you.is_empty() }
    })
    .await;
    let item = rig.fleet.operator_board().await.needs_you.remove(0);

    rig.fleet.operator_talked("what does it ask?", Some(&item.id)).await.expect("answered");

    let asked = call.asked();
    assert!(
        asked[0].contains(&format!("The person is talking about this item: Job {} \"{}\", waiting on the person ({}): {}", item.session_id, item.title, item.id, item.text)),
        "{}",
        asked[0]
    );
    assert!(item.text.contains("Want me to land it?"), "{}", item.text);
    let live = asked[0].split("\nLive Jobs:\n").nth(1).expect("a section").split("\n\n").next().expect("a section");
    assert!(live.contains(&format!("    Asking the person: Item {}, written in full under Waiting on the person", item.id)), "{live}");
}

#[tokio::test]
async fn a_live_jobs_open_tasks_and_last_words_are_handed_to_the_call_and_bounded() {
    let rig = rig_placing(None);
    let first = reply("none", "It is busy.");
    let call = seated(&rig, &[&first]).await;
    let job = asked(&rig, "Build the console").await;
    {
        let mut store = rig.fleet.store().lock().await;
        let task = |at: usize, status: &str| store::KeptTask {
            id: format!("t{at}"),
            text: format!("Task number {at}"),
            status: status.into(),
            first_seen_at: "2026-11-09T09:00:00.000Z".into(),
            completed_at: None,
            edited: false,
            made_by: None,
        };
        let mut tasks: Vec<store::KeptTask> = (1..=9).map(|at| task(at, "pending")).collect();
        tasks.push(task(10, "completed"));
        store.keep_session_build_tasks(job.as_str(), &tasks).expect("kept");
        for at in 1..=5 {
            let row = ipc::SessionRow::Message {
                id: format!("said-{at}"),
                at: ipc::Instant::carried("2026-11-09T09:00:00.000Z"),
                from: ipc::SessionVoice::Agent,
                text: format!("Said {at}: {}", "word ".repeat(300)),
                files: Vec::new(),
                tags: Vec::new(),
                notice: None,
                queued: false,
            };
            store.keep_session_row(job.as_str(), row.id(), &ipc::encode(&row).expect("a row")).expect("kept");
        }
    }

    rig.fleet.operator_talked("what is it doing?", None).await.expect("answered");

    let asked = call.asked();
    let live = asked[0].split("\nLive Jobs:\n").nth(1).expect("a section").split("\n\n").next().expect("a section");
    assert!(live.contains("    Tasks still open: Task number 1; Task number 2; Task number 3; Task number 4; Task number 5; Task number 6\n"), "{live}");
    assert!(!live.contains("Task number 7") && !live.contains("Task number 10"), "{live}");
    let said: Vec<&str> = live.lines().filter(|line| line.starts_with("    Its agent said: ")).collect();
    assert_eq!(said.len(), 3, "{live}");
    assert!(said[0].contains("Said 3:") && said[2].contains("Said 5:"), "the last three, oldest first: {live}");
    for line in said {
        assert!(line.chars().count() <= "    Its agent said: ".len() + 403, "a line is cut: {}", line.chars().count());
        assert!(line.ends_with("..."), "{line}");
    }
}
