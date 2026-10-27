//! The run sheet's routes: Journey 9, *Running one* and *Running one inside a
//! Job*.
//!
//! **One subject on one page**, the reads and the writes together, because a
//! reader of one needs the other; `crate::routes` still holds the table.

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::Query;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{NamedRun, StartCheckoutRun};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Queries, Rehearsing};
use crate::scoped::InCheckout;
use crate::served::Served;

/// The segment after `runs` on `get_run_output`.
#[derive(Deserialize)]
pub(crate) struct Ran {
    run_id: String,
}








// ---------------------------------------------------------------------------
// The checkout's half — Journey 9, *Running one*
// ---------------------------------------------------------------------------
//
// **No `Resolved` extractor on any of these.** There is no Job to resolve:
// these routes are under `/manifest`, beside `get_manifest_reading`, for the
// reason that one is — a Fleet serves one repository, and this reads it before
// any Job exists to read one from instead.

pub(crate) async fn get_checkout_run_sheet<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    match served
        .daemon()
        .get_checkout_run_sheet(manifest_id, repository)
        .await
    {
        Ok(sheet) => answer(StatusCode::OK, &sheet, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_checkout_runs<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    match served
        .daemon()
        .list_checkout_runs(manifest_id, repository)
        .await
    {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_checkout_run_output<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    Path(Ran { run_id }): Path<Ran>,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    match served
        .daemon()
        .get_checkout_run_output(run_id, manifest_id, repository)
        .await
    {
        Ok(output) => answer(StatusCode::OK, &output, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What one checkout run changed, against its own snapshot. A read: nothing in
/// the checkout is written, staged or committed.
pub(crate) async fn get_checkout_run_diff<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    Path(Ran { run_id }): Path<Ran>,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    match served
        .daemon()
        .get_checkout_run_diff(run_id, manifest_id, repository)
        .await
    {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One checkout run's output, as it prints — `observe_run`'s socket, one
/// owner over.
pub(crate) async fn observe_checkout_run<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    Path(Ran { run_id }): Path<Ran>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    match served
        .daemon()
        .observe_checkout_run(run_id, manifest_id, repository)
        .await
    {
        Ok(observed) => {
            upgrade.on_upgrade(move |socket| crate::watching_run::relay_checkout(socket, observed))
        }
        Err(refusal) => refused(refusal),
    }
}

/// 202: the run is underway and has not finished.
pub(crate) async fn start_checkout_run<D: Rehearsing>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    body: Bytes,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    let asked: StartCheckoutRun = match ipc::decode_body("a run to start in the checkout", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .shared()
        .start_checkout_run(asked, manifest_id, repository)
        .await
    {
        Ok(underway) => answer(StatusCode::ACCEPTED, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 202: the Verify is underway, its first step out. **The body is optional**:
/// empty runs the root's Manifest, and `workspace` names one below it.
pub(crate) async fn start_checkout_verify<D: Rehearsing>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    body: Bytes,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    // Empty is the root's file, which every press before workspaces sent.
    let asked = if body.is_empty() {
        ipc::StartCheckoutVerify::default()
    } else {
        match ipc::decode_body("a Verify to start", &body) {
            Ok(asked) => asked,
            Err(why) => return undecodable(&why.to_string(), served.run_id()),
        }
    };
    match served
        .shared()
        .start_checkout_verify(asked, manifest_id, repository)
        .await
    {
        Ok(verify) => answer(StatusCode::ACCEPTED, &verify, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_checkout_run<D: Rehearsing>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    body: Bytes,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    let named: NamedRun = match ipc::decode_body("a run to stop", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .stop_checkout_run(named, manifest_id, repository)
        .await
    {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn undo_checkout_run<D: Rehearsing>(
    State(served): State<Served<D>>,
    Query(scope): Query<InCheckout>,
    body: Bytes,
) -> Response {
    let (manifest_id, repository) = scope.scope();
    let named: NamedRun = match ipc::decode_body("a run to undo", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .undo_checkout_run(named, manifest_id, repository)
        .await
    {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
