//! The phone's door: every route the Phone Gateway calls as the phone, ruled on by name, so the phone sees only
//! Jobs in a project he shares. The rulings are the daemon's share filter; this says which route needs which.
//! `docs/concepts/pocket.md`, *What the phone sees*.
//!
//! **A Job the phone does not see is answered as a Job that does not exist, by the route itself**: it is asked
//! with an id no Job has, and the Job's own id put back in what it answered, so no route needs its own wording.
//! **A route not named here is not there for the phone**, so a new one reaches it only once someone rules on it.

use axum::body::{to_bytes, Body};
use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, Method, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ipc::{DispatchOn, JobDispatched, ManifestSummary, Projects, SessionList, StartSession};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::acting::{CALLER_HEADER, PHONE};
use crate::answers::{answer, refused};
use crate::daemon::{OtherMacs, Sessions};
use crate::served::Served;

/// An id no Job has, asked in place of a Job the phone does not see.
const NO_JOB: &str = "00000000000000000000000000";

/// The most read of a body here: a message's attachments are the largest the phone sends.
const MOST_READ: usize = 64 * 1024 * 1024;

/// What a route needs before the phone has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ruling {
    /// Names no Job of this Mac, or rules on its own through the filter for his other Macs.
    Passes,
    /// The Job is the query's `session_id`.
    JobInQuery,
    /// The Job is the body's `session_id`, or `waiting_session`.
    JobInBody,
    /// A terminal run of a Job, the body's `run_id`.
    RunInBody,
    /// A terminal run of a Job, the path's `:run_id`.
    RunInPath,
    /// The Jobs list, cut to the Jobs it sees.
    Jobs,
    Manifests,
    Projects,
    /// A Job started here.
    Start,
    /// A dispatch, which starts here where it names no other Mac.
    Dispatch,
}

/// The acts the phone forwards, each naming its Job in its body.
const ACTS: &[&str] = &[
    "/sessions/ask/answer",
    "/sessions/waiting/answer",
    "/sessions/plan/approve",
    "/sessions/plan/changes",
    "/sessions/sketch/answer",
    "/sessions/document/answer",
    "/sessions/message",
    "/sessions/begin",
    "/sessions/stop",
    "/sessions/pause",
    "/sessions/resume",
    "/sessions/close",
    "/sessions/tune",
    "/sessions/run/start",
    "/sessions/waiting/dismiss",
    "/sessions/change_waiting",
    "/sessions/start_waiting",
];

fn ruling(method: &Method, path: &str) -> Option<Ruling> {
    // His other Macs' routes, which the Gateway calls for them: each passes the share filter itself, or hands in
    // what another Mac shared.
    if path.starts_with("/other_mac/") || path.starts_with("/other_macs/") {
        return Some(Ruling::Passes);
    }
    let get = *method == Method::GET;
    let post = *method == Method::POST;
    match path {
        // `/events` is cut to what the phone sees by `crate::sockets`, which knows the door it was asked through.
        "/alerts" | "/events" | "/models" | "/plan_usage" | "/queued" | "/sessions/kept_styles" if get => Some(Ruling::Passes),
        // Only acts waiting for another Mac, which holds nothing but what it shared.
        "/queued/take_back" if post => Some(Ruling::Passes),
        "/sessions" if get => Some(Ruling::Jobs),
        "/manifests" if get => Some(Ruling::Manifests),
        "/projects" if get => Some(Ruling::Projects),
        "/sessions/one" | "/sessions/shown" | "/sessions/terminal/list" if get => Some(Ruling::JobInQuery),
        "/sessions/run/write" if post => Some(Ruling::RunInBody),
        "/sessions/run/:run_id/observe" if get => Some(Ruling::RunInPath),
        "/sessions/start" if post => Some(Ruling::Start),
        "/sessions/dispatch" if post => Some(Ruling::Dispatch),
        act if post && ACTS.contains(&act) => Some(Ruling::JobInBody),
        _ => None,
    }
}

/// The field a keystroke names its run in.
#[derive(Deserialize)]
struct NamesARun {
    run_id: String,
}

/// The fields a phone's act names its Job in.
#[derive(Deserialize)]
struct NamesAJob {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    waiting_session: Option<String>,
}

impl NamesAJob {
    fn job(self) -> Option<String> {
        self.session_id.or(self.waiting_session)
    }
}

/// The layer: a call the Gateway makes as the phone goes on as its route's ruling says, and every other passes.
pub(crate) async fn to_the_phone<D: OtherMacs + Sessions>(State(served): State<Served<D>>, request: Request, next: Next) -> Response {
    let phone = request.headers().get(CALLER_HEADER).is_some_and(|said| said.as_bytes() == PHONE.as_bytes());
    if !phone {
        return next.run(request).await;
    }
    let path = request.extensions().get::<MatchedPath>().map(|matched| matched.as_str().to_string());
    let Some(ruling) = path.and_then(|path| ruling(request.method(), &path)) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match ruling {
        Ruling::Passes => next.run(request).await,
        Ruling::JobInQuery => {
            let job = query_job(request.uri());
            match job {
                Some(job) if !served.daemon().job_for_phone(&job).await => {
                    let (mut parts, body) = request.into_parts();
                    parts.uri = without_job(&parts.uri);
                    as_no_job(next.run(Request::from_parts(parts, body)).await, &job).await
                }
                _ => next.run(request).await,
            }
        }
        Ruling::JobInBody => {
            let (parts, body) = request.into_parts();
            let Ok(bytes) = to_bytes(body, MOST_READ).await else {
                return StatusCode::PAYLOAD_TOO_LARGE.into_response();
            };
            let job = ipc::decode::<NamesAJob>("an act's Job", &bytes).ok().and_then(NamesAJob::job);
            match job {
                Some(job) if !served.daemon().job_for_phone(&job).await => {
                    let asked = String::from_utf8_lossy(&bytes).replace(&format!("\"{job}\""), &format!("\"{NO_JOB}\""));
                    // A Job named some other way than the text replaced is not asked about at all.
                    if ipc::decode::<NamesAJob>("an act's Job", asked.as_bytes()).ok().and_then(NamesAJob::job).as_deref() != Some(NO_JOB) {
                        return StatusCode::NOT_FOUND.into_response();
                    }
                    as_no_job(next.run(Request::from_parts(parts, Body::from(asked))).await, &job).await
                }
                _ => next.run(Request::from_parts(parts, Body::from(bytes))).await,
            }
        }
        Ruling::RunInPath => {
            // `/sessions/run/<run>/observe`: the run is the third segment.
            let run = request.uri().path().split('/').nth(3).unwrap_or_default().to_string();
            if run.is_empty() || served.daemon().run_for_phone(&run).await {
                return next.run(request).await;
            }
            // The route has read the run from the path already, so a new path would not reach it: the run is
            // answered here, as the route answers one Fleet does not hold.
            let answered = match served.daemon().observe_session_run(NO_JOB.to_string()).await {
                Err(refusal) => refused(refusal),
                Ok(_) => StatusCode::NOT_FOUND.into_response(),
            };
            as_no_job(answered, &run).await
        }
        Ruling::RunInBody => {
            let (parts, body) = request.into_parts();
            let Ok(bytes) = to_bytes(body, MOST_READ).await else {
                return StatusCode::PAYLOAD_TOO_LARGE.into_response();
            };
            let run = ipc::decode::<NamesARun>("a keystroke's run", &bytes).ok().map(|named| named.run_id);
            match run {
                Some(run) if !served.daemon().run_for_phone(&run).await => {
                    let asked = String::from_utf8_lossy(&bytes).replace(&format!("\"{run}\""), &format!("\"{NO_JOB}\""));
                    if ipc::decode::<NamesARun>("a keystroke's run", asked.as_bytes()).ok().map(|named| named.run_id).as_deref() != Some(NO_JOB) {
                        return StatusCode::NOT_FOUND.into_response();
                    }
                    as_no_job(next.run(Request::from_parts(parts, Body::from(asked))).await, &run).await
                }
                _ => next.run(Request::from_parts(parts, Body::from(bytes))).await,
            }
        }
        Ruling::Jobs => {
            let answered = next.run(request).await;
            let Some(list) = read::<SessionList>(answered).await else {
                return StatusCode::BAD_GATEWAY.into_response();
            };
            let list = match list {
                Ok(list) => list,
                Err(as_said) => return as_said,
            };
            let mut sessions = Vec::with_capacity(list.sessions.len());
            for record in list.sessions {
                if served.daemon().job_for_phone(record.id.as_str()).await {
                    sessions.push(record);
                }
            }
            answer(StatusCode::OK, &SessionList { sessions }, served.run_id())
        }
        Ruling::Manifests => match read::<Vec<ManifestSummary>>(next.run(request).await).await {
            Some(Ok(list)) => answer(StatusCode::OK, &served.daemon().manifests_for_phone(list).await, served.run_id()),
            Some(Err(as_said)) => as_said,
            None => StatusCode::BAD_GATEWAY.into_response(),
        },
        Ruling::Projects => match read::<Projects>(next.run(request).await).await {
            Some(Ok(projects)) => answer(StatusCode::OK, &served.daemon().projects_for_phone(projects).await, served.run_id()),
            Some(Err(as_said)) => as_said,
            None => StatusCode::BAD_GATEWAY.into_response(),
        },
        Ruling::Start | Ruling::Dispatch => {
            let (parts, body) = request.into_parts();
            let Ok(bytes) = to_bytes(body, MOST_READ).await else {
                return StatusCode::PAYLOAD_TOO_LARGE.into_response();
            };
            // A body that does not read goes on, and its route refuses it as it refuses anyone's.
            let start = match ruling {
                Ruling::Start => ipc::decode_body::<StartSession>("a Job to start", &bytes).ok(),
                _ => ipc::decode_body::<DispatchOn>("a dispatch", &bytes).ok().filter(|on| on.mac.is_none()).map(|on| on.job.start),
            };
            if let Some(start) = start {
                if let Err(refusal) = served.daemon().start_from_phone(&start).await {
                    return refused(refusal);
                }
            }
            let answered = next.run(Request::from_parts(parts, Body::from(bytes))).await;
            if ruling == Ruling::Start || answered.status() != StatusCode::OK {
                return answered;
            }
            match read::<JobDispatched>(answered).await {
                Some(Ok(mut done)) => {
                    // The Operator may hand the words to a live Job the phone does not see: it is told nothing of it.
                    if let Some(job) = done.session.as_ref().map(|record| record.id.as_str().to_string()) {
                        if !served.daemon().job_for_phone(&job).await {
                            done.session = None;
                            done.operator = None;
                        }
                    }
                    answer(StatusCode::OK, &done, served.run_id())
                }
                Some(Err(as_said)) => as_said,
                None => StatusCode::BAD_GATEWAY.into_response(),
            }
        }
    }
}

/// The query's `session_id`, as the route reads it.
fn query_job(uri: &Uri) -> Option<String> {
    axum::extract::Query::<NamesAJob>::try_from_uri(uri).ok().and_then(|query| query.0.session_id)
}

/// `uri` asking for [`NO_JOB`] wherever its query names a Job.
fn without_job(uri: &Uri) -> Uri {
    let query = uri.query().unwrap_or_default();
    let asked: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some(("session_id", _)) => format!("session_id={NO_JOB}"),
            _ => pair.to_string(),
        })
        .collect();
    format!("{}?{}", uri.path(), asked.join("&")).parse().unwrap_or_else(|_| Uri::from_static("/"))
}

/// What a route answered for [`NO_JOB`], with `job` in its place.
async fn as_no_job(answered: Response, job: &str) -> Response {
    let (mut parts, body) = answered.into_parts();
    let Ok(bytes) = to_bytes(body, MOST_READ).await else {
        return StatusCode::BAD_GATEWAY.into_response();
    };
    let said = String::from_utf8_lossy(&bytes).replace(NO_JOB, job);
    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(said))
}

/// A route's answer read as `T` where it is 200, or handed back as it was said. `None` for a 200 that will not read.
async fn read<T: DeserializeOwned>(answered: Response) -> Option<Result<T, Response>> {
    if answered.status() != StatusCode::OK {
        return Some(Err(answered));
    }
    let bytes = to_bytes(answered.into_body(), MOST_READ).await.ok()?;
    ipc::decode::<T>("an answer for the phone", &bytes).ok().map(Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_route_the_gateway_forwards_is_ruled() {
        for act in ACTS {
            assert_eq!(ruling(&Method::POST, act), Some(Ruling::JobInBody), "{act}");
        }
        assert_eq!(ruling(&Method::GET, "/sessions/one"), Some(Ruling::JobInQuery));
        assert_eq!(ruling(&Method::GET, "/sessions/terminal/list"), Some(Ruling::JobInQuery));
        assert_eq!(ruling(&Method::POST, "/sessions/run/write"), Some(Ruling::RunInBody));
        assert_eq!(ruling(&Method::GET, "/sessions/run/:run_id/observe"), Some(Ruling::RunInPath));
        assert_eq!(ruling(&Method::GET, "/settings"), None, "a route the phone has no use for is not there");
        assert_eq!(ruling(&Method::POST, "/sessions"), None);
    }

    #[test]
    fn a_hidden_job_is_asked_as_no_job_with_the_rest_of_the_query_kept() {
        let uri: Uri = "/sessions/shown?session_id=01SECRET&path=a.png".parse().unwrap();
        assert_eq!(query_job(&uri).as_deref(), Some("01SECRET"));
        assert_eq!(without_job(&uri).to_string(), format!("/sessions/shown?session_id={NO_JOB}&path=a.png"));
    }
}
