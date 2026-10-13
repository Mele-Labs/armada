//! A Commander's card is the Commander's alone to raise, and reaches the daemon
//! with the words and options it was given. `#2166`.

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use super::commander_reach::{calling, commander_holding_one, from, shared, ANYONE, COMMANDER};
use crate::{offerable, CommanderReach, COMMANDER_ALLOWED};

const CARD: &str = r#"{"job_id":"01JOB","body":{"text":"Group 3 is red on desktop_test","reach":"A Check is the owner's to skip","question":"Skip it?","options":["Skip desktop_test","Leave it"]}}"#;

#[test]
fn the_card_is_on_a_commanders_allowlist_and_in_no_other_callers_reach() {
    assert!(COMMANDER_ALLOWED.contains(&"ask_the_owner"));
    assert!(
        !offerable().any(|row| row.operation == "ask_the_owner"),
        "not any agent's, and not Helm's, which talks to him already"
    );
    let reach = CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB"));
    assert!(reach.may("ask_the_owner"));
    assert!(
        !reach.may("answer_commander_card"),
        "his pick is his: a Commander that answered its own card would have asked nobody"
    );
}

#[tokio::test]
async fn a_commander_is_listed_the_card_and_a_persons_agent_is_not() {
    let app = shared(&commander_holding_one());
    let listing = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let commander = from(&app, COMMANDER, &[], listing).await;
    assert!(commander.contains(r#""name":"ask_the_owner""#), "{commander}");
    let anyone = from(&app, ANYONE, &[], listing).await;
    assert!(!anyone.contains(r#""name":"ask_the_owner""#), "{anyone}");
}

#[tokio::test]
async fn a_card_sent_through_the_commanders_door_reaches_the_daemon_numbered() {
    let app = shared(&commander_holding_one());

    let body = from(&app, COMMANDER, &[], &calling("ask_the_owner", CARD)).await;

    assert!(!body.contains("\"isError\":true"), "{body}");
    assert!(body.contains("Skip desktop_test"), "{body}");
    assert!(
        body.contains(r#"\"id\":\"1\""#) && body.contains(r#"\"id\":\"2\""#),
        "an option's id is its place: {body}"
    );
}

#[tokio::test]
async fn a_card_with_one_option_or_a_blank_line_does_not_decode() {
    let app = shared(&commander_holding_one());
    for body in [
        r#"{"job_id":"01JOB","body":{"text":"a","reach":"b","question":"c","options":["only one"]}}"#,
        r#"{"job_id":"01JOB","body":{"text":"a","reach":"b","question":"c","options":["a","a"]}}"#,
        r#"{"job_id":"01JOB","body":{"text":"a","reach":"b","question":"c","options":["a","b","c","d","e"]}}"#,
        r#"{"job_id":"01JOB","body":{"text":"  ","reach":"b","question":"c","options":["a","b"]}}"#,
    ] {
        let answered = from(&app, COMMANDER, &[], &calling("ask_the_owner", body)).await;
        assert!(answered.contains("\"isError\":true"), "{body}: {answered}");
    }
}

/// **The route is not a person's.** The same bytes posted straight to the
/// surface, as Bridge or a script would, carry no Commander and are refused.
#[tokio::test]
async fn a_card_posted_to_the_surface_by_anyone_else_is_refused() {
    let app = shared(&commander_holding_one());
    let request = Request::builder()
        .method("POST")
        .uri("/jobs/01JOB/commander/card")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"text":"a","reach":"b","question":"c","options":["x","y"]}"#,
        ))
        .expect("a request");

    let status = app.oneshot(request).await.expect("an answer").status();

    assert_eq!(status, 409);
}
