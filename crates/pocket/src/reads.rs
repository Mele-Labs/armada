//! The phone's read routes, each one Fleet's answer narrowed to [`PhoneJob`].

use std::collections::BTreeMap;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use hyper::body::Bytes;
use ipc::{AlertList, JobDetail, JobList, JobSummary, ManifestSummary};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::fleet_events::Messages;
use crate::phone::{done, needs_you, running, PhoneJob};
use crate::{fleet_client, live, Gateway};

/// What the owner is told when Fleet cannot be reached. Each says what to do.
const NOT_RUNNING: &str = "Armada is not running on your Mac. Open it there, then try again.";
const NOT_ANSWERING: &str = "Armada is not answering on your Mac. Open it there, then try again.";
const NOT_READABLE: &str =
    "Armada answered in a way this phone cannot read. Update Armada on your Mac.";
const NO_SUCH_JOB: &str = "That Job is not here.";

/// How many finished Jobs the Done list holds.
const DONE_SHOWN: usize = 50;

fn refusal(status: StatusCode, sentence: &'static str) -> Response {
    (status, sentence).into_response()
}

fn port(gateway: &Gateway) -> Result<u16, Response> {
    (gateway.fleet)().map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_RUNNING))
}

/// A read of Fleet, decoded as the type Fleet sends. `Ok(None)` is Fleet
/// saying the thing is not there.
async fn read<T: DeserializeOwned>(
    gateway: &Gateway,
    path: &str,
    what: &'static str,
) -> Result<Option<T>, Response> {
    let port = port(gateway)?;
    let answer = fleet_client::send(port, "GET", path, None)
        .await
        .map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING))?;
    match answer.status {
        200 => ipc::decode(what, &answer.body)
            .map(Some)
            .map_err(|_| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE)),
        404 => Ok(None),
        _ => Err(refusal(StatusCode::BAD_GATEWAY, NOT_ANSWERING)),
    }
}

async fn required<T: DeserializeOwned>(
    gateway: &Gateway,
    path: &str,
    what: &'static str,
) -> Result<T, Response> {
    read(gateway, path, what)
        .await?
        .ok_or_else(|| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE))
}

fn json<T: Serialize>(value: &T) -> Response {
    match ipc::encode(value) {
        Ok(body) => ([(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(_) => refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE),
    }
}

/// Repository labels by Manifest id. A Fleet that will not list them costs the
/// labels, not the page.
async fn repositories(gateway: &Gateway) -> BTreeMap<String, String> {
    read::<Vec<ManifestSummary>>(gateway, "/manifests", "a manifest list")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .map(|manifest| (manifest.id.as_str().to_string(), manifest.repository))
        .collect()
}

fn label<'a>(labels: &'a BTreeMap<String, String>, job: &JobSummary) -> Option<&'a str> {
    labels.get(job.owner_manifest_id.as_str()).map(String::as_str)
}

#[derive(Serialize)]
struct Needs {
    needs: Vec<PhoneJob>,
}

#[derive(Serialize)]
struct Jobs {
    jobs: Vec<PhoneJob>,
}

/// Needs you: the Jobs Bridge's Needs you tab holds, and any Job Fleet's
/// alerts name that the rule does not (a Trigger's repair waiting on a person).
///
/// **`/jobs/reviews` and `/jobs/board` are not read.** Reviews is a subset of
/// the alerts, and the board is the not-yet-started queue, which holds Queued
/// Jobs that are waiting on a slot and not on the owner.
pub async fn needs(State(gateway): State<Gateway>) -> Response {
    let jobs = match required::<JobList>(&gateway, "/jobs", "a job list").await {
        Ok(list) => list.jobs,
        Err(refused) => return refused,
    };
    let alerts = match required::<AlertList>(&gateway, "/alerts", "an alert list").await {
        Ok(alerts) => alerts,
        Err(refused) => return refused,
    };
    let since: BTreeMap<&str, Option<String>> = alerts
        .blocked
        .iter()
        .chain(&alerts.waiting)
        .map(|alert| {
            (
                alert.job_id.as_str(),
                alert.since.as_ref().map(|at| at.as_str().to_string()),
            )
        })
        .collect();
    let labels = repositories(&gateway).await;
    let mut waiting: Vec<&JobSummary> = jobs
        .iter()
        .filter(|job| job.reclaimed_at.is_none())
        .filter(|job| needs_you(job) || since.contains_key(job.id.as_str()))
        .collect();
    waiting.sort_by(|a, b| (a.created_at.as_str(), a.id.as_str()).cmp(&(b.created_at.as_str(), b.id.as_str())));
    let mut needs = Vec::with_capacity(waiting.len());
    for job in waiting {
        let repository = label(&labels, job);
        let detail = read::<JobDetail>(&gateway, &format!("/jobs/{}", job.id.as_str()), "a job")
            .await
            .ok()
            .flatten();
        let mut phone = match detail {
            Some(detail) => PhoneJob::of_detail(&detail, repository),
            None => PhoneJob::of(job, repository),
        };
        phone.waiting_since = since.get(job.id.as_str()).cloned().flatten();
        needs.push(phone);
    }
    json(&Needs { needs })
}

#[derive(Deserialize)]
pub struct Wanted {
    state: Option<String>,
}

/// `?state=running` or `?state=done`. Fleet's `/jobs` takes only a repository,
/// so the state is chosen here.
pub async fn jobs(State(gateway): State<Gateway>, Query(wanted): Query<Wanted>) -> Response {
    let keep: fn(&JobSummary) -> bool = match wanted.state.as_deref() {
        Some("running") => running,
        Some("done") => done,
        _ => {
            return refusal(
                StatusCode::BAD_REQUEST,
                "Ask for state=running or state=done.",
            )
        }
    };
    let list = match required::<JobList>(&gateway, "/jobs", "a job list").await {
        Ok(list) => list.jobs,
        Err(refused) => return refused,
    };
    let labels = repositories(&gateway).await;
    let mut kept: Vec<&JobSummary> = list.iter().filter(|job| keep(job)).collect();
    // Newest first.
    kept.sort_by(|a, b| (b.created_at.as_str(), b.id.as_str()).cmp(&(a.created_at.as_str(), a.id.as_str())));
    if wanted.state.as_deref() == Some("done") {
        kept.truncate(DONE_SHOWN);
    }
    let jobs = kept
        .into_iter()
        .map(|job| PhoneJob::of(job, label(&labels, job)))
        .collect();
    json(&Jobs { jobs })
}

pub async fn job(State(gateway): State<Gateway>, Path(id): Path<String>) -> Response {
    // An id is put into a path to Fleet, so only an id's own characters pass.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return refusal(StatusCode::NOT_FOUND, NO_SUCH_JOB);
    }
    let detail = match read::<JobDetail>(&gateway, &format!("/jobs/{id}"), "a job").await {
        Ok(Some(detail)) => detail,
        Ok(None) => return refusal(StatusCode::NOT_FOUND, NO_SUCH_JOB),
        Err(refused) => return refused,
    };
    let labels = repositories(&gateway).await;
    json(&PhoneJob::of_detail(&detail, label(&labels, &detail.job)))
}

/// Server-Sent Events: each Job change Fleet publishes, narrowed.
pub async fn live(State(gateway): State<Gateway>) -> Response {
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let upgraded = match fleet_client::upgrade(port, "/events").await {
        Ok(upgraded) => upgraded,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let frames = stream::unfold(Messages::new(upgraded), |mut messages| async move {
        loop {
            let text = messages.next().await?;
            if let Some(frame) = live::change(&text).and_then(|change| live::frame(&change)) {
                return Some((Ok::<_, std::convert::Infallible>(Bytes::from(frame)), messages));
            }
        }
    });
    (
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(frames),
    )
        .into_response()
}
