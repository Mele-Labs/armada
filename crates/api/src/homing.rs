//! Kit across his Macs: over the Mac link (a read, a kind or an item switched, a clash settled), and the retired
//! shared home's routes, which Classic Bridge still calls. `accounting`'s shape: no `Resolved` extractor and no
//! path parameter; a read names what it reads on its query string, an act in its body. **Mounted whole by
//! `crate::routes`**, which sits past the line count the gate reports.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use ipc::RunId;
use serde::Serialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Refusal, SharedHome};
use crate::served::Served;

pub(crate) fn mounted<D: SharedHome>() -> Router<Served<D>> {
    Router::new()
        .route("/shared_home", get(get_shared_home::<D>))
        .route("/shared_home/sync", post(sync_shared_home::<D>))
        .route("/shared_home/add", post(add_to_shared_home::<D>))
        .route("/shared_home/bring", post(bring_from_shared_home::<D>))
        .route("/shared_home/choose", post(choose_shared_home_side::<D>))
        .route("/shared_home/kinds", get(get_home_kinds::<D>))
        .route("/shared_home/kinds/set", post(set_home_kind::<D>))
        .route("/shared_home/items/set", post(set_home_item::<D>))
        .route("/shared_home/repository", post(set_home_repository::<D>))
        .route("/kit/link", get(get_kit_link::<D>))
        .route("/kit/link/kinds/set", post(set_kit_link_kind::<D>))
        .route("/kit/link/items/set", post(set_kit_link_item::<D>))
        .route("/kit/link/settle", post(settle_kit_clash::<D>))
}

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

pub(crate) async fn get_kit_link<D: SharedHome>(State(served): State<Served<D>>) -> Response {
    answered(served.daemon().get_kit_link().await, served.run_id())
}

pub(crate) async fn set_kit_link_kind<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("a kind and whether it crosses to his other Macs", &body) {
        Ok(switch) => answered(served.daemon().set_kit_link_kind(switch).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn set_kit_link_item<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("an item and whether it crosses to his other Macs", &body) {
        Ok(switch) => answered(served.daemon().set_kit_link_item(switch).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

pub(crate) async fn settle_kit_clash<D: SharedHome>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body("a clash and the side to keep", &body) {
        Ok(settle) => answered(served.daemon().settle_kit_clash(settle).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}
