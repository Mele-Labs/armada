//! The box at the foot of the Operator panel on a real Fleet: what it refuses with the Operator off, and
//! the holds Fleet goes by once he has confirmed them.

use ipc::operator_talk::{OperatorHoldUntil, OperatorOrderForm};

use super::operator_sources::{asked, on as operator_on};
use super::session_host::{denied, rig_placing, Rig};
use super::session_queue::close;

fn code(refused: api::Refusal) -> String {
    format!("{refused:?}")
}

async fn seated(rig: &Rig) {
    operator_on(rig).await;
}

#[tokio::test]
async fn his_words_are_refused_while_the_operator_is_off_and_nothing_is_kept() {
    let rig = rig_placing(None);
    assert!(code(rig.fleet.operator_talked(super::operator_cards::said("hello", None)).await.expect_err("off")).contains("fleet.operator_session_not_running"));
    assert!(rig.fleet.operator_talk_wire().await.expect("read").lines.is_empty());
    assert!(rig.fleet.store().lock().await.operator_held_sends().expect("read").is_empty());
}

#[tokio::test]
async fn a_hold_naming_no_paths_is_no_form() {
    let rig = rig_placing(None);
    seated(&rig).await;
    let holder = asked(&rig, "Build the console").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let elsewhere = OperatorOrderForm::HoldPaths { manifest_id, paths: vec!["  ".into()], job: holder, until: OperatorHoldUntil::Ends };
    let refused = rig.fleet.operator_order_confirmed(elsewhere).await.expect_err("no paths");
    assert!(code(refused).contains("fleet.operator_order_not_a_form"));
}

#[tokio::test]
async fn a_hold_refuses_another_jobs_write_under_its_paths_and_lifts_when_its_job_ends() {
    let rig = rig_placing(None);
    seated(&rig).await;
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
async fn the_digest_lists_the_holds() {
    let rig = rig_placing(None);
    seated(&rig).await;
    let holder = asked(&rig, "Build the console").await;
    let other = asked(&rig, "Tidy the cockpit").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let form = OperatorOrderForm::HoldPaths { manifest_id: manifest_id.clone(), paths: vec!["apps/cockpit".into()], job: holder.clone(), until: OperatorHoldUntil::Ends };
    let hold = rig.fleet.operator_order_confirmed(form).await.expect("it stands").orders[0].id;
    let path = format!(r#"{{"file_path":"{}/apps/cockpit/deck.tsx"}}"#, rig.root);

    let digest = rig.fleet.operator_page_wire().await.expect("read").digest;
    assert_eq!(digest.holds.len(), 1, "{:?}", digest.holds);
    assert_eq!((digest.holds[0].id, digest.holds[0].manifest_id.as_str()), (hold, manifest_id.as_str()));
    assert!(digest.holds[0].said.starts_with("Hold apps/cockpit until "), "{}", digest.holds[0].said);
    assert!(denied(&rig.gate(&other, "Write", &path).await), "the hold refuses the other Job");

    rig.fleet.operator_order_removed(hold).await.expect("forgotten on his press");
    let after = rig.gate(&other, "Write", &path).await;
    assert!(!format!("{after:?}").contains("standing order"), "{after:?}");
    assert!(rig.fleet.operator_page_wire().await.expect("read").digest.holds.is_empty());
}

#[tokio::test]
async fn a_hold_stands_over_a_restart_and_an_old_form_stays_in_the_store_and_is_read_by_nothing() {
    let rig = rig_placing(None);
    seated(&rig).await;
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
async fn an_operator_hold_carries_its_paths() {
    let rig = rig_placing(None);
    seated(&rig).await;
    let holder = asked(&rig, "Build the console").await;
    let manifest_id = rig.fleet.store().lock().await.hosting(holder.as_str()).expect("read").expect("hosted").manifest_id;
    let form = OperatorOrderForm::HoldPaths { manifest_id, paths: vec!["apps/cockpit".into(), "apps/deck".into()], job: holder.clone(), until: OperatorHoldUntil::Pushes };
    rig.fleet.operator_order_confirmed(form).await.expect("it stands");

    let digest = rig.fleet.operator_page_wire().await.expect("read").digest;
    let hold = &digest.holds[0];
    assert_eq!(hold.paths, ["apps/cockpit", "apps/deck"]);
    assert_eq!((hold.job.as_ref(), hold.until), (Some(&holder), Some(OperatorHoldUntil::Pushes)));
}
