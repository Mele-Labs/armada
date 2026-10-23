//! The shared home's seven routes: the rows, Sync, the three acts on a row, and what syncs.
//! `accounting`'s shape: the home is the Machine's, so no `Resolved` extractor and no path
//! parameter.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::RunId;
use serde::Serialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Refusal, SharedHome};
use crate::served::Served;

fn answered<T: Serialize>(done: Result<T, Refusal>, run_id: &RunId) -> Response {
    match done {
        Ok(read) => answer(StatusCode::OK, &read, run_id),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_shared_home<D: SharedHome>(State(served): State<Served<D>>) -> Response {
    answered(served.daemon().get_shared_home().await, served.run_id())
}

pub(crate) async fn sync_shared_home<D: SharedHome>(State(served): State<Served<D>>) -> Response {
    answered(served.daemon().sync_shared_home().await, served.run_id())
}

pub(crate) async fn add_to_shared_home<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode("a row to add to the shared home", &body) {
        Ok(at) => answered(served.daemon().add_to_shared_home(at).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn bring_from_shared_home<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode("a row to bring from the shared home", &body) {
        Ok(at) => answered(served.daemon().bring_from_shared_home(at).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn choose_shared_home_side<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode("a row and the side to keep", &body) {
        Ok(choose) => answered(served.daemon().choose_shared_home_side(choose).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn get_home_kinds<D: SharedHome>(State(served): State<Served<D>>) -> Response {
    answered(served.daemon().get_home_kinds().await, served.run_id())
}

pub(crate) async fn set_home_kind<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode("a kind and whether it syncs", &body) {
        Ok(switch) => answered(served.daemon().set_home_kind(switch).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}
