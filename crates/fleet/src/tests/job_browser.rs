//! The Job browser through a real Fleet: an agent's act held until Bridge answers it by id, a wait
//! that runs out withdrawn, and an answer naming nothing held refused.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Refusal, Sessions, Subscription};
use ipc::{AnswerBrowserRequest, BrowserAct, BrowserAnswer, BrowserError, BrowserRequest, Event, WalkTarget};

use crate::job_browser::{OWNER_DRIVING_SAID, UNANSWERED_SAID};
use crate::tests::session_running::{rig, Rig};

fn save_button() -> WalkTarget {
    WalkTarget { role: "button".into(), name: Some("Save".into()), exact: None, within: None }
}

fn page(snapshot: &str) -> BrowserAnswer {
    BrowserAnswer::Page {
        url: "http://localhost:5173/settings".into(),
        title: "Settings".into(),
        snapshot: snapshot.into(),
        truncated: false,
    }
}

/// The next `browser.request` on the stream, as Bridge reads it.
async fn requested(watching: &mut Subscription) -> BrowserRequest {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::BrowserRequest(request) = delivered.event {
                        return request;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("a browser.request arrives")
}

/// `act` driven by the rig's Job, on a task of its own, as the agent's held HTTP call is.
fn driving(rig: &Rig, act: BrowserAct) -> tokio::task::JoinHandle<Result<BrowserAnswer, Refusal>> {
    let fleet = Arc::clone(&rig.fleet);
    let session = rig.session.clone();
    tokio::spawn(async move { fleet.drive_browser(None, Some(session), act).await })
}

fn answer(request_id: &str, answer: Option<BrowserAnswer>, error: Option<BrowserError>) -> AnswerBrowserRequest {
    AnswerBrowserRequest { request_id: request_id.into(), answer, error }
}

fn refused(refusal: Refusal) -> (u16, String, String) {
    let status = refusal.status();
    let error = refusal.error().clone();
    (status, error.code, error.message)
}

#[tokio::test]
async fn a_held_call_answers_with_what_bridge_did() {
    let rig = rig().await;
    let mut watching = rig.fleet.events().subscribe();
    let call = driving(&rig, BrowserAct::Click { target: save_button() });

    let request = requested(&mut watching).await;
    assert_eq!(request.session_id, rig.session);
    assert_eq!(request.act, BrowserAct::Click { target: save_button() });
    let answered = rig.fleet.answer_browser_request(answer(&request.request_id, Some(page("button \"Saved\"")), None)).await;
    assert_eq!(answered.expect("taken").session_id, rig.session);
    assert_eq!(call.await.unwrap().expect("answered"), page("button \"Saved\""));

    let again = rig.fleet.answer_browser_request(answer(&request.request_id, Some(page("")), None)).await;
    assert_eq!(refused(again.expect_err("answered once")).0, 409);

    let call = driving(&rig, BrowserAct::Snapshot);
    let request = requested(&mut watching).await;
    let error = Some(BrowserError::OwnerDriving);
    rig.fleet.answer_browser_request(answer(&request.request_id, None, error)).await.expect("taken");
    let (status, code, message) = refused(call.await.unwrap().expect_err("the owner drives"));
    assert_eq!((status, code.as_str(), message.as_str()), (409, "fleet.browser_owner_driving", OWNER_DRIVING_SAID));
}

#[tokio::test]
async fn a_wait_bridge_does_not_answer_withdraws_the_call_and_tells_the_agent() {
    let rig = rig().await;
    rig.fleet.job_browser().held_within(Duration::from_millis(50));
    let mut watching = rig.fleet.events().subscribe();
    let call = driving(&rig, BrowserAct::Snapshot);
    let request = requested(&mut watching).await;

    let (status, code, message) = refused(call.await.unwrap().expect_err("nothing answered"));
    assert_eq!((status, code.as_str(), message.as_str()), (409, "fleet.browser_unanswered", UNANSWERED_SAID));
    let late = rig.fleet.answer_browser_request(answer(&request.request_id, Some(page("")), None)).await;
    let (status, code, _) = refused(late.expect_err("withdrawn"));
    assert_eq!((status, code.as_str()), (409, "fleet.browser_request_withdrawn"));
}

#[tokio::test]
async fn an_answer_naming_no_held_call_is_refused() {
    let rig = rig().await;
    let unknown = rig.fleet.answer_browser_request(answer("browser-01NOBODY", Some(page("")), None)).await;
    let (status, code, _) = refused(unknown.expect_err("never minted"));
    assert_eq!((status, code.as_str()), (404, "fleet.no_such_browser_request"));
}

#[tokio::test]
async fn a_screenshot_is_written_where_fleet_named_and_an_answer_before_it_leaves_the_call_held() {
    let rig = rig().await;
    let mut watching = rig.fleet.events().subscribe();
    let call = driving(&rig, BrowserAct::Screenshot { path: "/tmp/anywhere.png".into() });
    let request = requested(&mut watching).await;
    let BrowserAct::Screenshot { path } = &request.act else { panic!("a screenshot: {:?}", request.act) };
    assert!(path.contains("/job-scratch/") && path.ends_with(&format!("/browser/{}.png", request.request_id)), "{path}");

    let shot = BrowserAnswer::Screenshot { url: "http://localhost:5173/".into(), title: "Home".into(), path: String::new() };
    let early = rig.fleet.answer_browser_request(answer(&request.request_id, Some(shot.clone()), None)).await;
    assert_eq!(refused(early.expect_err("no file yet")).1, "fleet.browser_answer_refused");
    std::fs::write(path, b"\x89PNG").unwrap();
    rig.fleet.answer_browser_request(answer(&request.request_id, Some(shot), None)).await.expect("taken");
    match call.await.unwrap().expect("answered") {
        BrowserAnswer::Screenshot { path: told, .. } => assert_eq!(&told, path),
        other => panic!("a screenshot: {other:?}"),
    }
}

#[tokio::test]
async fn with_no_bridge_open_a_call_is_refused_at_once() {
    let rig = rig().await;
    let call = rig.fleet.drive_browser(None, Some(rig.session.clone()), BrowserAct::Snapshot).await;
    assert_eq!(refused(call.expect_err("nobody listens")).1, "fleet.browser_no_bridge");
}
