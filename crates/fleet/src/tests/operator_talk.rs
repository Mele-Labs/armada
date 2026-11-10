//! The box at the foot of the Operator panel on a real Fleet: the conversation it is handed and
//! keeps, what it refuses, and the standing orders Fleet goes by once he has confirmed them.

use std::sync::{Arc, Mutex};

use api::Sessions;
use ipc::operator_talk::{OperatorHoldUntil, OperatorOrderForm};

use super::operator_acting::{asked, dispatch};
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
    dispatch(rig, false).await;
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

    dispatch(&rig, false).await;
    assert!(code(rig.fleet.operator_talked("hello", None).await.expect_err("said nothing")).contains("fleet.operator_said_nothing"));
    assert!(rig.fleet.operator_talk_wire().await.expect("read").lines.is_empty());
}

#[tokio::test]
async fn words_that_fit_no_form_list_the_forms_and_a_form_naming_what_is_not_there_is_no_form() {
    let rig = rig_placing(None);
    let unfit = reply("no_form", "No form holds that.");
    let unknown = reply("role_off file in 01NOSUCHREPOSITORY", "Done.");
    seated(&rig, &[&unfit, &unknown]).await;

    for words in ["be more careful", "turn File off somewhere else"] {
        let said = rig.fleet.operator_talked(words, None).await.expect("answered");
        assert!(said.order.is_none(), "{words}");
        assert_eq!(said.forms.len(), 3, "{words}");
    }
    assert!(rig.fleet.operator_orders_wire().await.orders.is_empty(), "nothing stands");
    let refused = rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: "anything".into() }).await.expect_err("not a kind");
    assert!(code(refused).contains("fleet.operator_order_not_a_form"));
}

#[tokio::test]
async fn a_kind_always_raised_goes_to_him_with_no_call() {
    let rig = rig_placing(None);
    let call = seated(&rig, &[]).await;
    rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: "report".into() }).await.expect("it stands");
    let job = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let report = ipc::ReportBug { title: "The terminal scrollback is lost on redraw".into(), what: "The pane redraws.".into(), steps: Vec::new(), errors: Vec::new(), logs: Vec::new(), session_id: Some(job) };
    rig.fleet.report_bug(None, report).await.expect("reported");

    rig.fleet.sources_handed().await;
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.store().lock().await.operator_rulings(10).expect("read").iter().any(|row| row.outcome == "raise" && row.because.contains("standing order")) }
    })
    .await;
    assert!(call.asked().is_empty(), "no call");
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
async fn orders_stand_over_a_restart() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
    rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: "note".into() }).await.expect("it stands");
    rig.fleet.operator_seat().orders_are(crate::operator::Orders::default());
    assert!(rig.fleet.operator_seat().orders().0.is_empty());

    rig.fleet.operator_orders_read().await;
    assert!(rig.fleet.operator_seat().orders().always_raises(crate::operator::Arrived::Note));
}

#[tokio::test]
async fn always_raise_note_reads_as_a_standing_order_in_fleets_words_and_stands_once_confirmed() {
    let rig = rig_placing(None);
    let asked_for = reply("always_raise note", "Notes will always come to you.");
    let call = seated(&rig, &[&asked_for]).await;

    let said = rig.fleet.operator_talked("always bring my notes to me", None).await.expect("answered");

    assert!(call.asked()[0].contains("always_raise <note, error, report, issue or red_main>"), "the form is one the call is told of");
    let proposed = said.order.expect("a form");
    assert_eq!(proposed.form, OperatorOrderForm::AlwaysRaise { what: "note".into() });
    assert_eq!(proposed.said, "Always raise notes");
    rig.fleet.operator_order_confirmed(proposed.form).await.expect("it stands");
    assert!(rig.fleet.operator_seat().orders().always_raises(crate::operator::Arrived::Note));
}

#[tokio::test]
async fn always_raise_error_reads_as_a_standing_order_and_stands_once_confirmed() {
    let rig = rig_placing(None);
    let asked_for = reply("always_raise error", "Errors will always come to you.");
    seated(&rig, &[&asked_for]).await;

    let said = rig.fleet.operator_talked("always bring me the errors Bridge shows", None).await.expect("answered");

    let proposed = said.order.expect("a form");
    assert_eq!(proposed.form, OperatorOrderForm::AlwaysRaise { what: "error".into() });
    assert_eq!(proposed.said, "Always raise errors");
    rig.fleet.operator_order_confirmed(proposed.form).await.expect("it stands");
    assert!(rig.fleet.operator_seat().orders().always_raises(crate::operator::Arrived::Error));
}

#[tokio::test]
async fn the_forms_the_call_is_told_of_leave_out_answer_and_the_kinds_the_operator_is_no_longer_handed() {
    let rig = rig_placing(None);
    let retired = reply("role_off answer in 01NOSUCHREPOSITORY", "Done.");
    let call = seated(&rig, &[&retired]).await;

    let said = rig.fleet.operator_talked("stop answering questions in this repository", None).await.expect("answered");

    let prompt = &call.asked()[0];
    assert!(prompt.contains("role_off <dispatch, delegate or file> in <repository id>"), "{prompt}");
    assert!(!prompt.contains("role_off answer") && !prompt.contains("delegate, answer"), "Answer is not a role an order may name");
    assert!(!prompt.contains("review, later"), "a Job's own events are not kinds an order may name");
    assert!(said.order.is_none(), "role_off answer fits no form");
    assert_eq!(said.forms.len(), 3);
}

#[tokio::test]
async fn an_order_for_the_retired_answer_role_is_refused_even_in_a_repository_that_exists() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
    let holder = asked(&rig, "Build the console").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;

    let refused = rig.fleet.operator_order_confirmed(OperatorOrderForm::RoleOff { role: ipc::operator_positions::OperatorRole::Answer, manifest_id: manifest_id.clone(), today: false }).await.expect_err("retired");
    assert!(code(refused).contains("fleet.operator_order_not_a_form"));
    rig.fleet.operator_order_confirmed(OperatorOrderForm::RoleOff { role: ipc::operator_positions::OperatorRole::File, manifest_id, today: false }).await.expect("a role still acting stands");
    assert!(rig.fleet.operator_orders_wire().await.orders.len() == 1);
}

#[tokio::test]
async fn always_raise_takes_the_five_kinds_the_operator_rules_on_and_refuses_the_rest() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;

    for kept in ["start", "plan", "question", "card", "stopped", "red", "later", "review", "ended", "merged"] {
        let refused = rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: kept.into() }).await.expect_err(kept);
        assert!(code(refused).contains("fleet.operator_order_not_a_form"), "{kept}");
    }
    for taken in ["note", "error", "report", "issue", "red_main"] {
        rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: taken.into() }).await.expect(taken);
    }
    assert_eq!(rig.fleet.operator_orders_wire().await.orders.len(), 5);
}

#[tokio::test]
async fn an_old_order_for_answer_or_for_a_kind_no_longer_handed_stays_in_the_store_and_is_served_by_nothing() {
    let rig = rig_placing(None);
    seated(&rig, &[]).await;
    let holder = asked(&rig, "Build the console").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let old = [
        OperatorOrderForm::RoleOff { role: ipc::operator_positions::OperatorRole::Answer, manifest_id: manifest_id.clone(), today: false },
        OperatorOrderForm::AlwaysRaise { what: "start".into() },
        OperatorOrderForm::AlwaysRaise { what: "review".into() },
        OperatorOrderForm::AlwaysRaise { what: "report".into() },
    ];
    for form in old {
        let one = store::KeptOrder { n: 0, form: form.kept(), said: "old".into(), until: None, at: "2026-11-09T09:00:00Z".into() };
        rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
    }

    rig.fleet.operator_orders_read().await;

    let seat = rig.fleet.operator_seat();
    assert!(seat.orders().always_raises(crate::operator::Arrived::Report), "a kind still taken is served");
    assert_eq!(seat.orders().0.len(), 1, "the rest are read by nothing");
    let wire = rig.fleet.operator_orders_wire().await;
    assert_eq!(wire.orders.len(), 1, "and are not on the board");
    assert_eq!(wire.orders[0].form, OperatorOrderForm::AlwaysRaise { what: "report".into() });
    assert_eq!(rig.fleet.store().lock().await.operator_orders().expect("read").len(), 4, "nothing is deleted");
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
                edited_at: None,
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
