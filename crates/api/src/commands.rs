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
use ipc::FileReport;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::reference::Resolved;
use crate::scoped::InManifest;
use crate::served::Served;

























/// The unit of work, not the process. A separate operation from `kill_drone`
/// because two of the edges into `killed` leave a status no
/// Drone has ever existed under, and neither one can be spelled as killing a
/// Drone.
pub(crate) async fn kill_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().kill_job(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}



/// Pull a held checkout level: `Commands::catch_up_checkout`. The pull's code and
/// words come back, a refused or conflicted one among them.
pub(crate) async fn catch_up_checkout<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::CatchUpCheckout = match ipc::decode("a catch-up for a checkout", &body) {
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
    let asked: ipc::CheckoutToPullRequest = match ipc::decode("a checkout to send as a pull request", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().checkout_to_pull_request(asked).await {
        Ok(sent) => answer(StatusCode::OK, &sent, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}




/// Delete the Job's whole record. **Real deletion, not a status** — the row
/// and everything beneath it are gone, and there is no undo.
///
/// 409 where the Job is not yet terminal: `kill_job` is the act that ends one
/// still in flight, and this only clears a Board of work that has already
/// finished. It does not touch the worktree or the branch — `armada clean`
/// keeps that concern.
pub(crate) async fn forget_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().forget_job(job.id()).await {
        Ok(forgotten) => answer(StatusCode::OK, &forgotten, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Give the Job's worktree and branch back. **The record survives** — this is
/// disk, not the row, and `forget_job` is the act that clears the Board.
///
/// 409 where the Job is not yet terminal, for `forget_job`'s reason. 200 where
/// either half was left standing: a branch holding commits the base cannot
/// reach is kept on purpose, and the answer names it rather than failing.
pub(crate) async fn reclaim_worktree<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().reclaim_worktree(job.id()).await {
        Ok(reclaimed) => answer(StatusCode::OK, &reclaimed, served.run_id()),
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
    let asked: ipc::ChangeSlotPool = match ipc::decode("a change to the slot pool", &body) {
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
    let asked: ipc::RescueSlot = match ipc::decode("an act on a stranded slot", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().rescue_slot(asked, scope.manifest()).await {
        Ok(done) => answer(StatusCode::OK, &done, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Delete the Job's branch, guarded by the tip in the body. 409 where the Job
/// is not terminal, its checkout still stands, the branch is gone, or the tip
/// moved.
pub(crate) async fn delete_branch<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let asked: ipc::DeleteBranch = match ipc::decode("the tip to delete", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().delete_branch(job.id(), asked).await {
        Ok(deleted) => answer(StatusCode::OK, &deleted, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}




















/// A person says this Job failed in error. **201, because a report now
/// exists** — and nothing else does: no Job is proposed, no Drone is spawned,
/// and the Job it names is exactly where it was.
///
/// 422 on a blank sentence. The record was already served by three other
/// routes before anybody pressed anything, so a filing with the bundle and no
/// sentence has added nothing at all.
pub(crate) async fn file_report<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let filing: FileReport = match ipc::decode("a report", &body) {
        Ok(filing) => filing,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().file_report(job.id(), filing).await {
        Ok(report) => answer(StatusCode::CREATED, &report, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
