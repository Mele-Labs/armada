//! A message or a stop addressed to one Drone of a Job, by its id: #1666,
//! spike 022 slice 5. A Drone that is not live is a 409, never another Drone.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Extension;
use ipc::{DroneId, Redirection};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Redirector};
use crate::door::HelmCalled;
use crate::reference::Resolved;
use crate::served::Served;

/// The Drone a request names, after its Job.
#[derive(Deserialize)]
pub(crate) struct OneDrone {
    drone_id: String,
}

/// Stop one Drone. What comes back is the Job, with that Drone gone.
pub(crate) async fn kill_one_drone<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(OneDrone { drone_id }): Path<OneDrone>,
) -> Response {
    match served
        .shared()
        .kill_one_drone(job.id(), DroneId::carried(drone_id))
        .await
    {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A person's words to one Drone, `redirect_drone`'s body.
pub(crate) async fn redirect_one_drone<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(OneDrone { drone_id }): Path<OneDrone>,
    helm: Option<Extension<HelmCalled>>,
    body: Bytes,
) -> Response {
    let instruction: Redirection = match ipc::decode("a redirect", &body) {
        Ok(instruction) => instruction,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let by = match helm {
        Some(_) => Redirector::Helm,
        None => Redirector::Person,
    };
    match served
        .shared()
        .redirect_one_drone(job.id(), DroneId::carried(drone_id), instruction, by)
        .await
    {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
