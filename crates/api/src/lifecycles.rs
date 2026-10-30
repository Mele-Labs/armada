//! Workspaces, lifecycles and rules over the router: what applies to a repository, what a Job
//! holds, and saving or deleting one piece in Kit.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Lifecycles;
use crate::scoped::InManifest;
use crate::served::Served;

/// The Workspaces that hold one repository.
pub(crate) async fn list_repository_workspaces<D: Lifecycles>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().list_repository_workspaces(scope.manifest()).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The lifecycles that apply to one repository.
pub(crate) async fn list_lifecycles<D: Lifecycles>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().list_lifecycles(scope.manifest()).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The lifecycle rules that apply to one repository.
pub(crate) async fn list_lifecycle_rules<D: Lifecycles>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().list_lifecycle_rules(scope.manifest()).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=`, read only where the connection places no Job.
#[derive(Deserialize)]
pub(crate) struct ForJobLifecycles {
    #[serde(default)]
    session_id: Option<String>,
}

/// What a Job holds and where it stands. **The Job is placed by the connection**, as a watch's is.
pub(crate) async fn get_job_lifecycles<D: Lifecycles>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForJobLifecycles>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    match served.daemon().get_job_lifecycles(crate::acting::asking(), named).await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Write one piece in Kit's folder. **200 and not 201**, for `save_workflow`'s reason: the file is
/// on disk, and read by the next read, when this answers.
pub(crate) async fn save_lifecycle_piece<D: Lifecycles>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::SaveLifecyclePiece = match ipc::decode_body("a piece to save", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_lifecycle_piece(asked).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Delete the Kit file that declares one piece.
pub(crate) async fn delete_lifecycle_piece<D: Lifecycles>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::DeleteLifecyclePiece = match ipc::decode_body("a piece to delete", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().delete_lifecycle_piece(asked).await {
        Ok(deleted) => answer(StatusCode::OK, &deleted, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
