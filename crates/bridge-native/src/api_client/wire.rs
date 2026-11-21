//! The routes the API Client calls, each off the main thread on the port the Fleet connection read.
//! `crates/ipc/operations/get_requests.toml`, `send_request.toml`, the saves, `list_request_runs.toml` and `get_request_run.toml`.

use std::time::Duration;

use gpui::{App, AppContext, SharedString, Task};
use ipc::{
    DeleteRequestEnvironment, RequestCollection, RequestEnvironmentWrite, RequestHeader, RequestRun, RequestRuns, RequestSaved, RequestSent, RequestToSave, RequestToSend, Requests, SaveRequestCollection,
    SaveRequestEnvironment, SaveRequestToCollection, SavedRequestNamed, SendRequest, SessionId,
};

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
    /// The saved request this is, by its collection's id and its own: its method and address are unchanged, so the send is one
    /// of its runs. The headers and body here go over the kept ones.
    pub saved: Option<(String, String)>,
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
        request: match to_send.saved {
            Some((collection, request)) => RequestToSend::Saved(SavedRequestNamed { collection, request }),
            None => RequestToSend::Own { method: to_send.method, url: to_send.url, collection: to_send.collection },
        },
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

fn session(job: &str) -> Option<SessionId> {
    Some(SessionId::carried(job))
}

/// A save's call: the whole body, posted, and Fleet's `Requests` read again from both files, or its own words for refusing.
fn post_requests(port: u16, route: &'static str, body: Result<String, String>, cx: &App) -> Task<Result<Requests, String>> {
    match body {
        Ok(body) => {
            let asked = request::<Requests>(port, Ask::Post(route.into(), body), "Requests", cx);
            cx.background_spawn(async move { asked.await.map_err(|refused| refused.said) })
        }
        Err(why) => cx.background_spawn(async move { Err(why) }),
    }
}

/// `POST /requests/collections`: one collection whole, into the person's own file.
pub fn save_collection(port: u16, job: &str, collection: RequestCollection, cx: &App) -> Task<Result<Requests, String>> {
    let body = ipc::encode(&SaveRequestCollection { repository: None, session_id: session(job), collection }).map_err(|why| why.why);
    post_requests(port, "/requests/collections", body, cx)
}

/// `POST /requests/collections/request`: a request into a collection by its id, replacing one of the same name.
pub fn save_request(port: u16, job: &str, collection: String, kept: RequestToSave, cx: &App) -> Task<Result<RequestSaved, String>> {
    let body = ipc::encode(&SaveRequestToCollection { session_id: session(job), collection, request: kept }).map_err(|why| why.why);
    match body {
        Ok(body) => {
            let asked = request::<RequestSaved>(port, Ask::Post("/requests/collections/request".into(), body), "RequestSaved", cx);
            cx.background_spawn(async move { asked.await.map_err(|refused| refused.said) })
        }
        Err(why) => cx.background_spawn(async move { Err(why) }),
    }
}

/// `POST /requests/environments`: one environment whole. **The one body that carries a secret's value**, once, toward Fleet:
/// the body is made here, handed to the call and dropped with it.
pub fn save_environment(port: u16, job: &str, environment: RequestEnvironmentWrite, cx: &App) -> Task<Result<Requests, String>> {
    let body = ipc::encode(&SaveRequestEnvironment { repository: None, session_id: session(job), environment }).map_err(|why| why.why);
    post_requests(port, "/requests/environments", body, cx)
}

/// `POST /requests/environments/delete`: the person's own environment of that name, its secrets forgotten.
pub fn delete_environment(port: u16, job: &str, name: String, cx: &App) -> Task<Result<Requests, String>> {
    let body = ipc::encode(&DeleteRequestEnvironment { repository: None, session_id: session(job), name }).map_err(|why| why.why);
    post_requests(port, "/requests/environments/delete", body, cx)
}

/// How many runs a page lists.
pub const RUNS_PAGE: usize = 20;

/// `GET /requests/runs`: a page of a saved request's runs, newest first; `before` is the last run of the page before.
pub fn runs(port: u16, job: &str, collection: &str, request_id: &str, before: Option<&str>, cx: &App) -> Task<Result<RequestRuns, String>> {
    let before = before.map(|before| format!("&before={}", crate::link::percent_encoded(before))).unwrap_or_default();
    let route = format!(
        "/requests/runs?session_id={}&collection={}&request={}&limit={RUNS_PAGE}{before}",
        crate::link::percent_encoded(job),
        crate::link::percent_encoded(collection),
        crate::link::percent_encoded(request_id)
    );
    let asked = request::<RequestRuns>(port, Ask::Get(route), "RequestRuns", cx);
    cx.background_spawn(async move { asked.await.map_err(|refused| refused.said) })
}

/// `GET /requests/runs/one`: one run whole, the request as it went and the answer with its headers.
pub fn run(port: u16, job: &str, id: &str, cx: &App) -> Task<Result<RequestRun, String>> {
    let route = format!("/requests/runs/one?session_id={}&id={}", crate::link::percent_encoded(job), crate::link::percent_encoded(id));
    let asked = request::<RequestRun>(port, Ask::Get(route), "RequestRun", cx);
    cx.background_spawn(async move { asked.await.map_err(|refused| refused.said) })
}
