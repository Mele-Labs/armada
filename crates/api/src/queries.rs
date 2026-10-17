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

use ipc::{JobLog, WireError};

use crate::answers::{answer, problem, refused};
use crate::daemon::Queries;
use crate::reference::Resolved;
use crate::scoped::{InManifest, InRepository};
use crate::served::Served;




pub(crate) async fn list_jobs<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().list_jobs(scope.manifest()).await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}



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

/// Which repository a branch list is of, absent being the one Fleet was
/// started in.
#[derive(Deserialize)]
pub(crate) struct OfRepository {
    #[serde(default)]
    manifest_id: Option<String>,
}

/// The repository's local branches, the base first. #1605.
pub(crate) async fn list_branches<D: Queries>(
    State(served): State<Served<D>>,
    Query(OfRepository { manifest_id }): Query<OfRepository>,
) -> Response {
    match served
        .daemon()
        .list_branches(manifest_id.map(ipc::ManifestId::carried))
        .await
    {
        Ok(branches) => answer(StatusCode::OK, &branches, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Job in full. **The Board row plus what the list redacts** — the steps
/// and where each got to, the criteria, the branch, the brief.
pub(crate) async fn get_job<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_job(job.id()).await {
        Ok(detail) => answer(StatusCode::OK, &detail, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every move one Job made, oldest first. **The path taken**, which `get_job`
/// answers nothing about — it says where a Job is, and this says how it got
/// there.
///
/// Its own route because a history has no bound and a detail view is fetched to
/// draw a summary. Nothing here folds: the rows are read and rendered, and
/// `crates/store/src/fold.rs` stays the only thing that replays them.
pub(crate) async fn get_job_events<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_job_events(job.id()).await {
        Ok(history) => answer(StatusCode::OK, &history, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}



/// One Job's whole patch. **The expensive read, on the one route that asks for
/// it** — `get_job` is fetched on every open to draw a summary, and the bytes
/// are what a person reading a diff needs and nothing else does.
pub(crate) async fn get_diff<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_diff(job.id()).await {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}







/// One Job's own log, settled: what Fleet did to it, read back once.
///
/// **`observe_job_log`'s backfill without its tail**, and a second route rather
/// than a widened one — [`crate::journal`] argues both, and Bridge keeps the
/// socket it draws the log panel from. It notifies nobody, so it is not a
/// second channel beside `get_events_since`.
///
/// **It goes through the reader and not the daemon**, which is where it parts
/// from every other read on this page: a Job's log is a file rather than a
/// record, and the extractor already resolved the handle that file is named
/// by, so a daemon call would read the Job's row a second time for a fact this
/// handler was handed.
///
/// 404 where the Job is unknown, from the extractor. A Job that wrote no line
/// is an empty window; a log that will not read answers with what it had.
pub(crate) async fn get_job_log<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    let Some(journal) = served.journal() else {
        return problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            &WireError::raised(
                crate::sockets::NO_JOURNAL,
                "this Fleet was built with no reader for a Job's own log",
                served.run_id().clone(),
            )
            .about_job(job.id()),
        );
    };
    // Blocking, and said so — [`crate::journal::pass`]'s reason: the read is a
    // file read, and running it inline would hold a worker for the length of it.
    let (job_id, handle) = (job.id(), job.handle().to_string());
    let Ok(window) = tokio::task::spawn_blocking(move || journal.window(&job_id, &handle)).await
    else {
        return problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            &WireError::raised(
                crate::sockets::NO_JOURNAL,
                "the reader for this Job's own log failed",
                served.run_id().clone(),
            )
            .about_job(job.id()),
        );
    };
    let whole = window.whole();
    let log = JobLog {
        job_id: job.id(),
        path: window.path,
        notes: window.notes,
        from_note: window.from_note,
        total_notes: window.total_notes,
        undecodable: window.undecodable,
        bytes: window.bytes,
        whole,
        unreadable: window.unreadable,
    };
    answer(StatusCode::OK, &log, served.run_id())
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

pub(crate) async fn list_models<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_models().await {
        Ok(models) => answer(StatusCode::OK, &models, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every report filed, newest first, with the counts they are read beside.
///
/// **Not under `/jobs`**, and that is the shape of the claim: a report survives
/// `armada clean` taking its Job away, so it is a record of its own rather than
/// a row beneath one.
pub(crate) async fn list_reports<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_reports().await {
        Ok(reports) => answer(StatusCode::OK, &reports, served.run_id()),
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
