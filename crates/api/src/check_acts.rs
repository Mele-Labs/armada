//! A person's hand on the Checks a Job's gate is running: skip one, skip what
//! is left, run one again. The route is a verb on the Job, as `kill_job` is.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::CheckNamed;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::reference::Resolved;
use crate::served::Served;

/// Run one Check again: `Commands::retry_check`.
pub(crate) async fn retry_check<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let named: CheckNamed = match ipc::decode("a Check to run again", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().retry_check(job.id(), named).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// End one Check, recorded as skipped: `Commands::skip_check`.
pub(crate) async fn skip_check<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let named: CheckNamed = match ipc::decode("a Check to skip", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().skip_check(job.id(), named).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// End every Check that has not finished: `Commands::skip_checks`.
pub(crate) async fn skip_checks<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().skip_checks(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
