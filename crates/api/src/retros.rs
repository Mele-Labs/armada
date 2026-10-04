//! `get_job_retro` and `list_lessons`. Since 23.12. `docs/concepts/retro.md`.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::{Queries, Retros};
use crate::reference::Resolved;
use crate::served::Served;

/// How many items `list_lessons` answers where `?most=` is absent.
const MOST: u32 = 200;

/// `?manifest_id=`, `?lands_in=` and `?most=` on `list_lessons`. A `lands_in`
/// that is not one of the three is refused, as any query that will not read is.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    lands_in: Option<ipc::LandsIn>,
    #[serde(default)]
    most: Option<u32>,
}

/// What got in the way while one Job ran. Any Job, ended or not: one still
/// running answers `pending` with its record so far.
pub(crate) async fn get_job_retro<D: Queries + Retros>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_job_retro(job.id()).await {
        Ok(retro) => answer(StatusCode::OK, &retro, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every written retro's items, newest first.
pub(crate) async fn list_lessons<D: Retros>(
    State(served): State<Served<D>>,
    Query(listing): Query<Listing>,
) -> Response {
    let manifest = listing.manifest_id.map(ipc::ManifestId::carried);
    match served
        .daemon()
        .list_lessons(manifest, listing.lands_in, listing.most.unwrap_or(MOST))
        .await
    {
        Ok(lessons) => answer(StatusCode::OK, &lessons, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
