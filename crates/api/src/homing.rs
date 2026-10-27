//! The shared home's routes: the rows, Sync, the three acts on a row, and what syncs.
//! `accounting`'s shape: a home is a profile's, so no `Resolved` extractor and no path
//! parameter; a read names the profile on its query string, an act in its body.

use axum::body::Bytes;
use axum::extract::{Query, State};
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

/// `?login=`. A query without one is refused by the extractor as any malformed query is.
pub(crate) async fn get_shared_home<D: SharedHome>(
    State(served): State<Served<D>>,
    Query(profile): Query<ipc::HomeProfile>,
) -> Response {
    answered(served.daemon().get_shared_home(profile).await, served.run_id())
}

pub(crate) async fn sync_shared_home<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("the profile whose home to sync", &body) {
        Ok(profile) => answered(served.daemon().sync_shared_home(profile).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn add_to_shared_home<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("a row to add to the shared home", &body) {
        Ok(at) => answered(served.daemon().add_to_shared_home(at).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn bring_from_shared_home<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("a row to bring from the shared home", &body) {
        Ok(at) => answered(served.daemon().bring_from_shared_home(at).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn choose_shared_home_side<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("a row and the side to keep", &body) {
        Ok(choose) => answered(served.daemon().choose_shared_home_side(choose).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn get_home_kinds<D: SharedHome>(
    State(served): State<Served<D>>,
    Query(profile): Query<ipc::HomeProfile>,
) -> Response {
    answered(served.daemon().get_home_kinds(profile).await, served.run_id())
}

pub(crate) async fn set_home_kind<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("a kind and whether it syncs", &body) {
        Ok(switch) => answered(served.daemon().set_home_kind(switch).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn set_home_item<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("an item and whether it syncs", &body) {
        Ok(switch) => answered(served.daemon().set_home_item(switch).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn set_home_repository<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("an owner and a name for the shared home", &body) {
        Ok(choice) => answered(served.daemon().set_home_repository(choice).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}
