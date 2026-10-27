//! The account's four routes: the profiles, signing in, signing out, and
//! choosing which profile a repository is bound to. `settling`'s shape.
//!
//! **No `Resolved` extractor and no path parameter.** The account is the
//! Machine's, and nothing about it is a Job's.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{SetRepositoryProfile, SignOut};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Accounts;
use crate::served::Served;

/// The profiles, and the sign-in in flight.
pub(crate) async fn get_accounts<D: Accounts>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_accounts().await {
        Ok(accounts) => answer(StatusCode::OK, &accounts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Begin signing in. **No body**: which account signs in is whoever types the
/// code at the forge.
pub(crate) async fn start_sign_in<D: Accounts>(State(served): State<Served<D>>) -> Response {
    match served.daemon().start_sign_in().await {
        Ok(accounts) => answer(StatusCode::OK, &accounts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn sign_out<D: Accounts>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let sign_out: SignOut = match ipc::decode_body("a profile to sign out", &body) {
        Ok(sign_out) => sign_out,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().sign_out(sign_out).await {
        Ok(signed_out) => answer(StatusCode::OK, &signed_out, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_repository_profile<D: Accounts>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let set: SetRepositoryProfile = match ipc::decode_body("a repository and its profile", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_repository_profile(set).await {
        Ok(accounts) => answer(StatusCode::OK, &accounts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
