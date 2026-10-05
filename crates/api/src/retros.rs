//! `get_job_retro`, `list_lessons`, and the two acts on a Lesson. Since 23.12;
//! the acts and `?state=` since 23.26. `docs/concepts/retro.md`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Extension;
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::{Queries, Redirector, Retros};
use crate::door::HelmCalled;
use crate::reference::Resolved;
use crate::served::Served;

/// How many items `list_lessons` answers where `?most=` is absent.
const MOST: u32 = 200;

/// `?manifest_id=`, `?lands_in=`, `?state=` and `?most=` on `list_lessons`. A
/// `lands_in` or `state` that is not one of its values is refused, as any
/// query that will not read is. **`state` absent is `open`**: what a person has
/// not answered yet, and a Lesson once answered is listed where it was sent.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    lands_in: Option<ipc::LandsIn>,
    #[serde(default)]
    state: Option<ipc::LessonState>,
    #[serde(default)]
    most: Option<u32>,
}

/// The item an act names, by `Lesson::id`.
#[derive(Deserialize)]
pub(crate) struct NamedLesson {
    lesson_id: String,
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

/// Every written retro's items in one state, newest first.
pub(crate) async fn list_lessons<D: Retros>(
    State(served): State<Served<D>>,
    Query(listing): Query<Listing>,
) -> Response {
    let manifest = listing.manifest_id.map(ipc::ManifestId::carried);
    match served
        .daemon()
        .list_lessons(
            manifest,
            listing.lands_in,
            listing.state,
            listing.most.unwrap_or(MOST),
        )
        .await
    {
        Ok(lessons) => answer(StatusCode::OK, &lessons, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Agree with an item. **Answers with the item as it stands**, the Job
/// proposed for it on it, and answers the same for an item already answered.
pub(crate) async fn agree_lesson<D: Retros>(
    State(served): State<Served<D>>,
    Path(NamedLesson { lesson_id }): Path<NamedLesson>,
    helm: Option<Extension<HelmCalled>>,
) -> Response {
    let by = match helm {
        Some(_) => Redirector::Helm,
        None => Redirector::Person,
    };
    match served.shared().agree_lesson(lesson_id, by).await {
        Ok(lesson) => answer(StatusCode::OK, &lesson, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Disagree with an item. The row stays, `discarded`.
pub(crate) async fn disagree_lesson<D: Retros>(
    State(served): State<Served<D>>,
    Path(NamedLesson { lesson_id }): Path<NamedLesson>,
) -> Response {
    match served.daemon().disagree_lesson(lesson_id).await {
        Ok(lesson) => answer(StatusCode::OK, &lesson, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
