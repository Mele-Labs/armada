//! Writes. Every one moves something, and each says what it moves it to.
//!
//! **Split from the route table rather than from a line count.** What separates
//! these from `queries` is not size: a command decodes a body, may answer 201,
//! and has refusals that mean the machine would not admit the move. A caller
//! reading one of these needs to know what the Job looks like afterwards, which
//! is what each doc below states and no query has to.
//!
//! The 400 for a body that would not parse is the transport's own and never the
//! daemon's — see `crate::answers::undecodable`. Nothing downstream was asked.

use axum::body::Bytes;
use axum::extract::State;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::scoped::InManifest;
use crate::served::Served;

























/// Pull a held checkout level: `Commands::catch_up_checkout`. The pull's code and
/// words come back, a refused or conflicted one among them.
pub(crate) async fn catch_up_checkout<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::CatchUpCheckout = match ipc::decode_body("a catch-up for a checkout", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().catch_up_checkout(asked).await {
        Ok(pulled) => answer(StatusCode::OK, &pulled, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Send a held checkout's uncommitted files up as a pull request:
/// `Commands::checkout_to_pull_request`. The pull request's number comes back.
pub(crate) async fn checkout_to_pull_request<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::CheckoutToPullRequest = match ipc::decode_body("a checkout to send as a pull request", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().checkout_to_pull_request(asked).await {
        Ok(sent) => answer(StatusCode::OK, &sent, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}




/// Add, remove, close or reopen one slot of the repository's pool. 409
/// naming why a slot cannot go; 422 for a slot the pool does not have.
pub(crate) async fn change_slot_pool<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::ChangeSlotPool = match ipc::decode_body("a change to the slot pool", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .change_slot_pool(asked, scope.manifest())
        .await
    {
        Ok(changed) => answer(StatusCode::OK, &changed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One act on a stranded slot: start the Scout that reads it, stop it, or
/// scrap or stash the work. 409 naming why it cannot; 422 for a slot the pool
/// does not have.
pub(crate) async fn rescue_slot<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let asked: ipc::RescueSlot = match ipc::decode_body("an act on a stranded slot", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().rescue_slot(asked, scope.manifest()).await {
        Ok(done) => answer(StatusCode::OK, &done, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
