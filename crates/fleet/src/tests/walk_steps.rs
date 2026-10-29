//! **`steps` on `show_window`: a scripted walk of any page, and what holds it.** Fleet checks the
//! shape a session wrote and refuses a bad step by its number; a good list rides the window's row
//! and its ledger artifact so Bridge opens the window with it and a reopen finds it again.
//! `docs/concepts/session.md`, *Showing a window*; `docs/practices/protocol.md`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions};
use ipc::{Event, SessionId, SessionRow, SessionVoice, ShowWindow, WalkStep, WindowFailed};

use super::session_host::{rig_placing, Rig};

const PAGE: &str = "http://localhost:5173/depot";

/// A list of steps as a session writes them: JSON, in the mock walks' own shape.
fn steps(json: &str) -> Vec<WalkStep> {
    ipc::decode("steps", json.as_bytes()).expect("steps of a shape the wire reads")
}

fn depot() -> Vec<WalkStep> {
    steps(
        r#"[
          {"say":"The stock table","look":{"role":"table","name":"Stock"}},
          {"say":"Open the receiving tab","press":{"role":"tab","name":"Receiving"}},
          {"say":"Name the item","type":"Bolts\n","into":{"role":"textbox","name":"Item"}},
          {"say":"Add it","press":{"role":"button","name":"Add"}},
          {"say":"It is saved","look":{"role":"status","name":"Saved"}}
        ]"#,
    )
}

fn show_with(id: &SessionId, steps: Vec<WalkStep>) -> ShowWindow {
    ShowWindow { url: PAGE.into(), title: None, session_id: Some(id.clone()), steps, ..Default::default() }
}

/// What a refusal says, so a test can ask that it names the step.
async fn refusal(rig: &Rig, id: &SessionId, steps: Vec<WalkStep>) -> String {
    let refused = rig.fleet.show_window(None, show_with(id, steps)).await.expect_err("refused");
    format!("{refused:?}")
}

async fn windows(rig: &Rig, id: &SessionId) -> usize {
    rig.rows(id).await.into_iter().filter(|row| matches!(row, SessionRow::Window { .. })).count()
}

#[tokio::test]
async fn steps_ride_the_row_bridge_opens_and_the_ledger_a_reopen_reads() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut watching = rig.fleet.events().subscribe();
    let record = rig.fleet.show_window(None, show_with(&id, depot())).await.expect("shown");

    let kept = record
        .attachments
        .iter()
        .find(|one| one.detail.get("form").map(String::as_str) == Some("window"))
        .expect("the window is on the ledger");
    let on_ledger = kept.detail.get("steps").expect("and its steps with it");
    let read: Vec<WalkStep> = ipc::decode("kept steps", on_ledger.as_bytes()).expect("a JSON string Bridge reads back");
    assert_eq!(read, depot(), "the artifact keeps them whole, in order");

    let arrived = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::SessionRow(ipc::SessionRowChanged { row: SessionRow::Window { steps, .. }, .. }) = delivered.event {
                        return steps;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("the window row arrives");
    assert_eq!(arrived, depot(), "Bridge opens the window on the row, with the steps it carries");
}

#[tokio::test]
async fn a_window_shown_without_steps_keeps_neither_a_row_field_nor_a_detail_key() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.show_window(None, show_with(&id, Vec::new())).await.expect("shown");
    let kept = record.attachments.iter().find(|one| one.detail.get("form").map(String::as_str) == Some("window")).unwrap();
    assert!(!kept.detail.contains_key("steps"), "a page that plays its own walk is passed none");
    let row = rig.rows(&id).await.into_iter().find_map(|row| match row {
        SessionRow::Window { steps, .. } => Some(steps),
        _ => None,
    });
    assert_eq!(row, Some(Vec::new()));
}

#[tokio::test]
async fn a_step_names_exactly_one_verb_and_the_refusal_names_the_step() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let none = steps(r#"[{"say":"The table","look":{"role":"table","name":"Stock"}},{"say":"Nothing to do"}]"#);
    let said = refusal(&rig, &id, none).await;
    assert!(said.contains("step 2") && said.contains("exactly one verb"), "{said}");

    let two = steps(
        r#"[{"say":"Both","press":{"role":"button","name":"Add"},"look":{"role":"button","name":"Add"}}]"#,
    );
    let said = refusal(&rig, &id, two).await;
    assert!(said.contains("step 1") && said.contains("exactly one verb"), "{said}");
    assert_eq!(windows(&rig, &id).await, 0, "a refused call writes no row");
}

#[tokio::test]
async fn a_verb_is_paired_with_its_own_companion_key_and_no_other() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let button = r#"{"role":"button","name":"Add"}"#;
    let field = r#"{"role":"textbox","name":"Item"}"#;
    let cases = [
        // `type` with no `into`, `key` with no `on`, `select` with no `in`.
        (format!(r#"[{{"say":"s","type":"x"}}]"#), "to have a target"),
        (format!(r#"[{{"say":"s","key":"Tab"}}]"#), "to have a target"),
        (format!(r#"[{{"say":"s","select":"Bolts"}}]"#), "to have a target"),
        // `drag` with no `by`.
        (format!(r#"[{{"say":"s","drag":{button}}}]"#), "to pair each verb"),
        // A companion that belongs to another verb.
        (format!(r#"[{{"say":"s","press":{button},"into":{field}}}]"#), "to pair each verb"),
        (format!(r#"[{{"say":"s","type":"x","into":{field},"on":{field}}}]"#), "to pair each verb"),
        (format!(r#"[{{"say":"s","look":{button},"by":{{"x":1,"y":1}}}}]"#), "to pair each verb"),
    ];
    for (json, wanted) in cases {
        let said = refusal(&rig, &id, steps(&json)).await;
        assert!(said.contains("step 1") && said.contains(wanted), "{json}: {said}");
    }
    // Each pairing, written right, is shown.
    let right = steps(&format!(
        r#"[{{"say":"a","type":"x","into":{field}}},{{"say":"b","key":"Meta+n","on":{field}}},
            {{"say":"c","select":"Bolts","in":{field}}},{{"say":"d","drag":{button},"by":{{"x":20,"y":-5}}}},
            {{"say":"e","paste":{field}}},{{"say":"f","hover":{button}}},{{"say":"g","later":{button}}}]"#
    ));
    rig.fleet.show_window(None, show_with(&id, right)).await.expect("every verb with its own key is shown");
}

#[tokio::test]
async fn a_target_needs_a_role_and_a_text_target_needs_the_words() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    for (json, wanted) in [
        (r#"[{"say":"s","press":{"role":""}}]"#, "a `role` of one word"),
        (r#"[{"say":"s","press":{"role":"push button"}}]"#, "a `role` of one word"),
        (r#"[{"say":"s","look":{"role":"text"}}]"#, "a `name` on a `text` target"),
        (r#"[{"say":"s","look":{"role":"button","name":"  "}}]"#, "words in it"),
        (
            r#"[{"say":"s","look":{"role":"button","name":"a","within":{"role":"form","within":{"role":"form","within":{"role":"form","within":{"role":"form","within":{"role":"form","within":{"role":"form"}}}}}}}}]"#,
            "no more than four deep",
        ),
    ] {
        let said = refusal(&rig, &id, steps(json)).await;
        assert!(said.contains("step 1") && said.contains(wanted), "{json}: {said}");
    }
}

#[tokio::test]
async fn the_caps_hold_and_the_first_step_over_is_the_one_named() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let look = |say: &str| format!(r#"{{"say":{say:?},"look":{{"role":"table","name":"Stock"}}}}"#);

    let many = format!("[{}]", vec![look("s"); 201].join(","));
    let said = refusal(&rig, &id, steps(&many)).await;
    assert!(said.contains("no more than 200 steps"), "{said}");

    let long = "x".repeat(241);
    let said = refusal(&rig, &id, steps(&format!("[{},{}]", look("fine"), look(&long)))).await;
    assert!(said.contains("step 2") && said.contains("240 characters"), "{said}");

    let blank = refusal(&rig, &id, steps(&format!("[{}]", look("   ")))).await;
    assert!(blank.contains("step 1") && blank.contains("what it shows"), "{blank}");

    let typed = "y".repeat(1001);
    let said = refusal(&rig, &id, steps(&format!(r#"[{{"say":"s","type":{typed:?},"into":{{"role":"textbox","name":"Item"}}}}]"#))).await;
    assert!(said.contains("1000 characters"), "{said}");

    let said = refusal(&rig, &id, steps(r#"[{"say":"s","drag":{"role":"button","name":"Add"},"by":{"x":5001,"y":0}}]"#)).await;
    assert!(said.contains("5000 pixels"), "{said}");

    let said = refusal(&rig, &id, steps(r#"[{"say":"s","key":"","on":{"role":"textbox","name":"Item"}}]"#)).await;
    assert!(said.contains("`key` such as"), "{said}");

    // Written out, 200 steps of the longest allowed caption and name would weigh more than 32 KiB.
    let heavy = format!(
        r#"{{"say":{:?},"look":{{"role":"table","name":{:?}}}}}"#,
        "s".repeat(240),
        "n".repeat(200)
    );
    let weighs = format!("[{}]", vec![heavy; 100].join(","));
    let said = refusal(&rig, &id, steps(&weighs)).await;
    assert!(said.contains("32 KiB"), "{said}");
    assert_eq!(windows(&rig, &id).await, 0, "nothing was written by any of these");
}

#[tokio::test]
async fn a_key_the_wire_does_not_know_is_not_a_step() {
    let stray: Result<Vec<WalkStep>, _> =
        ipc::decode("steps", br#"[{"say":"s","press":{"role":"button"},"click":true}]"#);
    assert!(stray.is_err(), "a misspelt verb is not read as no verb");
    let target: Result<Vec<WalkStep>, _> =
        ipc::decode("steps", br#"[{"say":"s","press":{"role":"button","label":"Add"}}]"#);
    assert!(target.is_err(), "nor a misspelt target key");
}

#[tokio::test]
async fn a_caption_is_trimmed_before_it_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let spaced = steps(r#"[{"say":"  The stock table \n","look":{"role":"table","name":"Stock"}}]"#);
    let record = rig.fleet.show_window(None, show_with(&id, spaced)).await.expect("shown");
    let kept = record.attachments.iter().find(|one| one.detail.get("form").map(String::as_str) == Some("window")).unwrap();
    assert!(kept.detail["steps"].contains(r#""say":"The stock table""#), "{}", kept.detail["steps"]);
}

#[tokio::test]
async fn a_step_bridge_cannot_play_is_told_to_the_session_as_a_step_and_not_as_a_page_that_failed() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_window(None, show_with(&id, depot())).await.expect("shown");

    let held = WindowFailed {
        session_id: id.clone(),
        url: PAGE.into(),
        why: "step 3, a textbox named Item, is not on the page".into(),
    };
    let told = Arc::clone(&rig.fleet).window_failed(held).await.expect("told");
    assert!(told.told);
    let said = rig
        .rows(&id)
        .await
        .into_iter()
        .find_map(|row| match row {
            SessionRow::Message { from: SessionVoice::Session { title, .. }, text, .. } if title == "Fleet" => Some(text),
            _ => None,
        })
        .expect("Fleet said it");
    assert!(said.contains("held at step 3") && said.contains(PAGE), "{said}");
    assert!(said.contains("Fix the steps") && said.contains("show_window"), "{said}");
    assert!(!said.contains("could not show"), "it is not a page that failed to load: {said}");
}

#[tokio::test]
async fn steps_set_aside_for_the_pages_own_walk_are_told_to_the_session_as_that_and_not_as_a_failure() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_window(None, show_with(&id, depot())).await.expect("shown");

    let set_aside =
        WindowFailed { session_id: id.clone(), url: PAGE.into(), why: "steps set aside: this page plays its own walk".into() };
    let told = Arc::clone(&rig.fleet).window_failed(set_aside).await.expect("told");
    assert!(told.told);
    let said = rig
        .rows(&id)
        .await
        .into_iter()
        .find_map(|row| match row {
            SessionRow::Message { from: SessionVoice::Session { title, .. }, text, .. } if title == "Fleet" => Some(text),
            _ => None,
        })
        .expect("Fleet said it");
    assert!(said.contains("set aside") && said.contains(PAGE), "{said}");
    assert!(said.contains("Show it again without steps"), "{said}");
    assert!(!said.contains("could not show"), "it is not a page that failed to load: {said}");
    assert!(!said.contains("held at"), "it is not a step that cannot be played: {said}");

    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session;
    let window = record.attachments.iter().find(|one| one.detail.get("form").map(String::as_str) == Some("window")).expect("window");
    assert_ne!(window.state, ipc::AttachmentState::Spent, "the page loaded, so the walk is still to be walked");
}
