//! The two routes the API Client calls, each off the main thread on the port the Fleet connection read.
//! `crates/ipc/operations/get_requests.toml` and `send_request.toml`.

use std::time::Duration;

use gpui::{App, AppContext, SharedString, Task};
use ipc::{RequestHeader, RequestSent, RequestToSend, Requests, SendRequest, SessionId};

use crate::settings::wire::{request, Ask};

/// Fleet stops a send at 30 seconds and answers; the call waits five more for that answer.
const SEND_PATIENCE: Duration = Duration::from_secs(35);

/// `GET /requests?session_id=<job>`: the collections and environments of the Job's repository, and one environment for each
/// server the Job is running.
pub fn read(port: u16, job: &str, cx: &App) -> Task<Result<Requests, String>> {
    let asked = request::<Requests>(port, Ask::Get(format!("/requests?session_id={}", crate::link::percent_encoded(job))), "Requests", cx);
    cx.background_spawn(async move { asked.await.map_err(|refused| refused.said) })
}

/// What a send is made of, as the editor holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToSend {
    pub method: String,
    pub url: String,
    pub environment: Option<String>,
    /// The collection whose headers go under the request's own.
    pub collection: Option<String>,
    pub headers: Vec<RequestHeader>,
    pub body: Option<String>,
}

/// An answer, its body already set out for drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    pub sent: RequestSent,
    pub json: bool,
    pub lines: Vec<SharedString>,
}

impl Answered {
    pub fn of(sent: RequestSent) -> Answered {
        let (text, json) = super::model::body_shown(&sent.body);
        let lines = text.lines().map(|line| SharedString::from(line.to_string())).collect();
        Answered { sent, json, lines }
    }
}

/// `POST /requests/send`, as the owner: the Job's thread gets the row and its agent is told. Nothing answering is an answer with
/// `error`; a refusal is Fleet's own words.
pub fn send(port: u16, job: &str, to_send: ToSend, cx: &App) -> Task<Result<Answered, String>> {
    let body = ipc::encode(&SendRequest {
        session_id: Some(SessionId::carried(job)),
        repository: None,
        request: RequestToSend::Own { method: to_send.method, url: to_send.url, collection: to_send.collection },
        environment: to_send.environment,
        headers: to_send.headers,
        body: to_send.body,
        body_file: None,
        body_inline: true,
        pick: None,
    })
    .map_err(|why| why.why);
    cx.background_spawn(async move {
        let body = body?;
        let route = "/requests/send";
        let size = body.len();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = runtime.block_on(crate::transport::post_waiting(port, route, body, SEND_PATIENCE))?;
        let bytes = crate::refusal::answered("POST", route, Some(size), status, bytes).map_err(|refusal| refusal.to_string())?;
        let sent = ipc::decode::<RequestSent>("RequestSent", &bytes).map_err(|why| why.to_string())?;
        Ok(Answered::of(sent))
    })
}
