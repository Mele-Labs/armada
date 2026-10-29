//! The Job browser: the agent's acts through the door, each held until Bridge answers it, and
//! Bridge's answer. `docs/practices/protocol/the-job-browser.md`.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use serde::de::DeserializeOwned;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Sessions;
use crate::served::Served;

/// Decode `body` as `T` and hand its Job and act to the daemon. **The Job is placed by the
/// connection** where the call came through the agent's door, as `type_in_terminal`'s is.
async fn driven<D: Sessions, T: DeserializeOwned>(
    served: &Served<D>,
    expected: &'static str,
    body: &Bytes,
    act: impl FnOnce(T) -> (Option<ipc::SessionId>, ipc::BrowserAct),
) -> Response {
    let asked: T = match ipc::decode_body(expected, body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let (session_id, act) = act(asked);
    match served.daemon().drive_browser(crate::acting::asking(), session_id, act).await {
        Ok(done) => answer(StatusCode::OK, &done, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn browser_open<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "a page to open", &body, |open: ipc::BrowserOpen| {
        (open.session_id, ipc::BrowserAct::Open { url: open.url })
    })
    .await
}

pub(crate) async fn browser_click<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "a target to press", &body, |click: ipc::BrowserClick| {
        (click.session_id, ipc::BrowserAct::Click { target: click.target })
    })
    .await
}

pub(crate) async fn browser_type<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "words to type into a target", &body, |typed: ipc::BrowserType| {
        (typed.session_id, ipc::BrowserAct::Type { target: typed.target, text: typed.text, submit: typed.submit })
    })
    .await
}

pub(crate) async fn browser_snapshot<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "a snapshot to take", &body, |look: ipc::BrowserLook| (look.session_id, ipc::BrowserAct::Snapshot))
        .await
}

pub(crate) async fn browser_screenshot<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "a screenshot to take", &body, |look: ipc::BrowserLook| {
        (look.session_id, ipc::BrowserAct::Screenshot { path: String::new() })
    })
    .await
}

pub(crate) async fn browser_wait_for<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "words or a target to wait for", &body, |wait: ipc::BrowserWaitFor| {
        let act = ipc::BrowserAct::WaitFor { text: wait.text, target: wait.target, timeout_ms: wait.timeout_ms.unwrap_or(0) };
        (wait.session_id, act)
    })
    .await
}

pub(crate) async fn browser_evaluate<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    driven(&served, "an expression to run", &body, |evaluate: ipc::BrowserEvaluate| {
        (evaluate.session_id, ipc::BrowserAct::Evaluate { expression: evaluate.expression })
    })
    .await
}

/// 200 with the call it answered. 404 for an id Fleet never minted, 409 for one withdrawn or
/// answered already, 422 for an answer that does not fit the act.
pub(crate) async fn answer_browser_request<D: Sessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let said: ipc::AnswerBrowserRequest = match ipc::decode_body("an answer to a browser request", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_browser_request(said).await {
        Ok(answered) => answer(StatusCode::OK, &answered, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
