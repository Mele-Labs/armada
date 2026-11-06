//! The box at the foot of the Operator panel on a real Fleet: the conversation it is handed and
//! keeps, what it refuses, and the standing orders Fleet goes by once he has confirmed them.

use std::sync::{Arc, Mutex};

use ipc::operator_talk::{OperatorHoldUntil, OperatorOrderForm};

use super::operator_acting::{asked, dispatch};
use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_queue::close;
use fleet::operator::{Answering, Calling};
use fleet::CallFailed;

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
async fn the_conversation_is_kept_and_handed_to_the_next_call_and_each_answer_counts_toward_the_ceiling() {
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
    assert_eq!(rig.fleet.store().lock().await.operator_calls_since("2000-01-01T00:00:00.000Z").expect("counted"), 2);
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
    let unknown = reply("role_off answer in 01NOSUCHREPOSITORY", "Done.");
    seated(&rig, &[&unfit, &unknown]).await;

    for words in ["be more careful", "turn Answer off somewhere else"] {
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
    rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: "start".into() }).await.expect("it stands");
    let other = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let start = asked(&rig, "The terminal scrollback is lost on redraw").await;
    assert!(!other.as_str().is_empty());

    rig.fleet.start_handed(start.as_str(), fleet::operator::Source::Dispatch, "The terminal scrollback is lost on redraw").await;
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
    rig.fleet.operator_order_confirmed(OperatorOrderForm::AlwaysRaise { what: "review".into() }).await.expect("it stands");
    rig.fleet.operator_seat().orders_are(fleet::operator::Orders::default());
    assert!(rig.fleet.operator_seat().orders().0.is_empty());

    rig.fleet.operator_orders_read().await;
    assert!(rig.fleet.operator_seat().orders().always_raises(fleet::operator::Arrived::Review));
}

#[tokio::test]
async fn always_raise_note_reads_as_a_standing_order_in_fleets_words_and_stands_once_confirmed() {
    let rig = rig_placing(None);
    let asked_for = reply("always_raise note", "Notes will always come to you.");
    let call = seated(&rig, &[&asked_for]).await;

    let said = rig.fleet.operator_talked("always bring my notes to me", None).await.expect("answered");

    assert!(call.asked()[0].contains("review, later or note>"), "the form is one the call is told of");
    let proposed = said.order.expect("a form");
    assert_eq!(proposed.form, OperatorOrderForm::AlwaysRaise { what: "note".into() });
    assert_eq!(proposed.said, "Always raise notes");
    rig.fleet.operator_order_confirmed(proposed.form).await.expect("it stands");
    assert!(rig.fleet.operator_seat().orders().always_raises(fleet::operator::Arrived::Note));
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
