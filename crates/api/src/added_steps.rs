//! Steps added to one Job: the two acts that change them on a Job underway.
//! `docs/concepts/trigger.md`. The read is `JobDetail.additions`, and placing
//! one at the approval press is `approve_dispatch`'s `additions`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Authoring, Queries};
use crate::reference::Resolved;
use crate::served::Served;

/// Add a step to this Job. **200**: the addition is recorded when this answers.
pub(crate) async fn add_job_step<D: Authoring + Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let add: ipc::AddStep = match ipc::decode("a step to add", &body) {
        Ok(add) => add,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().add_job_step(job.id(), add).await {
        Ok(added) => answer(StatusCode::OK, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Take a step off this Job before it fires.
pub(crate) async fn remove_job_step<D: Authoring + Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let remove: ipc::RemoveAddedStep = match ipc::decode("a step to remove", &body) {
        Ok(remove) => remove,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().remove_job_step(job.id(), remove).await {
        Ok(removed) => answer(StatusCode::OK, &removed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Change an added step's switches before it fires.
pub(crate) async fn edit_job_step<D: Authoring + Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let edit: ipc::EditAddedStep = match ipc::decode("a step to edit", &body) {
        Ok(edit) => edit,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().edit_job_step(job.id(), edit).await {
        Ok(edited) => answer(StatusCode::OK, &edited, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What a repair's branch changes against the Job's.
pub(crate) async fn get_repair_diff<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    Query(of): Query<ipc::RepairOf>,
) -> Response {
    match served.daemon().get_repair_diff(job.id(), of).await {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
