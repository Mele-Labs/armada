//! Reads. Every one is request-response, and none of them moves a Job.
//!
//! **Split from the route table rather than from a line count.** `routes` is
//! the inventory and the router — what exists and where it is served — and this
//! is what each read answers with. The seam is a real one: a query never
//! decodes a body, never returns 201, and never has a refusal that means the
//! machine would not admit the move.
//!
//! The expensive reads are here on purpose and each says why in its own doc.
//! `get_diff` spends the patch and `get_check_output` a Check's streams, and
//! both are separate routes so that the reads made on every refresh do not pay
//! for them.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;


use crate::answers::{answer, refused};
use crate::daemon::Queries;
use crate::scoped::{Fresh, InManifest, InRepository};
use crate::served::Served;




/// What Fleet's last re-read of `armada.yml` came to.
///
/// **200 with `null` where Fleet has not re-read it**, rather than a 404. A
/// Fleet still running on the Manifest it booted with is not a missing
/// resource; it is a Fleet with nothing to report, and a client draws nothing
/// either way.
pub(crate) async fn get_manifest_reading<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_manifest_reading(scope.manifest()).await {
        Ok(reading) => answer(StatusCode::OK, &reading, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Whether the repository still has what `armada.yml` names.
///
/// **A read of the repository, where `get_manifest_reading` is a read of the
/// file.** It never refuses and never 404s: every declared line gets a verdict,
/// and a line naming nothing this read can look for says so on its own row.
///
/// **It runs nothing**, which is what lets a surface make this call on opening.
/// The dry-run beside it is a real test suite and is behind its own button.
pub(crate) async fn get_manifest_drift<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_manifest_drift(scope.manifest()).await {
        Ok(drift) => answer(StatusCode::OK, &drift, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What Scan found in the checkout. **Never a 404 for a repository with
/// nothing in it**: that is one workspace, not followed, and a checkout that
/// would not list says why in the answer.
pub(crate) async fn get_repository_scan<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InRepository>,
) -> Response {
    match served
        .daemon()
        .get_repository_scan(scope.repository())
        .await
    {
        Ok(scan) => answer(StatusCode::OK, &scan, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The `?q=` a person has typed after `@`. A query string rather than a path
/// segment — `q` is empty the instant somebody types `@` and before anything
/// follows it, which a path segment cannot carry. Absent decodes the same as
/// empty: a caller that sends no parameter at all gets everything, up to the
/// cap.
#[derive(Deserialize)]
pub(crate) struct Search {
    #[serde(default)]
    q: String,
    /// The checkout searched, absent being the one Fleet was started in.
    #[serde(default)]
    manifest_id: Option<String>,
}

/// Paths under the checkout narrowed against typed text, for Bridge's `@`
/// mention popup. It never refuses — see `Queries::search_files`.
pub(crate) async fn search_files<D: Queries>(
    State(served): State<Served<D>>,
    Query(Search { q, manifest_id }): Query<Search>,
) -> Response {
    match served
        .daemon()
        .search_files(q, manifest_id.map(ipc::ManifestId::carried))
        .await
    {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The workflows a proposal may name. **The set Fleet will accept**, which is
/// why it is served rather than left to a caller to know.
pub(crate) async fn list_workflows<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_workflows().await {
        Ok(workflows) => answer(StatusCode::OK, &workflows, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_manifests<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_manifests().await {
        Ok(manifests) => answer(StatusCode::OK, &manifests, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_models<D: Queries>(State(served): State<Served<D>>, Query(fresh): Query<Fresh>) -> Response {
    match served.daemon().list_models(fresh.asked()).await {
        Ok(models) => answer(StatusCode::OK, &models, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every worktree Fleet is holding disk for, and the test each one did not
/// pass. **Read across every Job at once**, which is the reading a field on
/// `get_job` could not give: what is being decided is which of these to give
/// back, and that is a question about the set.
///
/// A piloted Job's worktree is not in the answer. Fleet drops it — `#367` — so
/// there is nothing here to filter and nothing a client could show by mistake.
pub(crate) async fn list_worktrees<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().list_worktrees(scope.manifest()).await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
