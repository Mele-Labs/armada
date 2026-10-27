//! App runs over the router: a repository's app listed, drafted, started, stopped, restarted and
//! saved. **Not under `/servers`**, whose rows are one process: these are the several a run holds.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::AppRuns;
use crate::scoped::InManifest;
use crate::served::Served;

/// `?job_id=` beside `?manifest_id=`: a Job's worktree, where the main checkout is not meant.
#[derive(Deserialize)]
pub(crate) struct OnAJob {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    job_id: Option<String>,
}

pub(crate) async fn list_app_runs<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(asked): Query<OnAJob>,
) -> Response {
    let manifest_id = asked.manifest_id.map(ipc::ManifestId::carried);
    let job = asked.job_id.filter(|id| !id.trim().is_empty());
    match served.daemon().list_app_runs(manifest_id, job).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_app_run_draft<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_app_run_draft(scope.manifest()).await {
        Ok(draft) => answer(StatusCode::OK, &draft, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 202: starting, or already up.
pub(crate) async fn start_app_run<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::NamedAppRun = match ipc::decode_body("a run to start", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_app_run(scope.manifest(), asked).await {
        Ok(run) => answer(StatusCode::ACCEPTED, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_app_run<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::NamedAppRun = match ipc::decode_body("a run to stop", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_app_run(scope.manifest(), asked).await {
        Ok(run) => answer(StatusCode::OK, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn restart_app_run<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::NamedAppRun = match ipc::decode_body("a run to restart", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().restart_app_run(scope.manifest(), asked).await {
        Ok(run) => answer(StatusCode::ACCEPTED, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_app_run<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::SaveAppRun = match ipc::decode_body("a run to save", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().save_app_run(scope.manifest(), asked).await {
        Ok(saved) => answer(StatusCode::ACCEPTED, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn pin_app_run_port<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::PinAppRunPort = match ipc::decode_body("a port to pin", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().pin_app_run_port(scope.manifest(), asked).await {
        Ok(run) => answer(StatusCode::OK, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn unpin_app_run_port<D: AppRuns>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::UnpinAppRunPort = match ipc::decode_body("a port to unpin", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().unpin_app_run_port(scope.manifest(), asked).await {
        Ok(run) => answer(StatusCode::OK, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
