//! The five mod operations. `docs/concepts/mods.md`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Mods;
use crate::served::Served;

pub(crate) async fn list_mods<D: Mods>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_mods().await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with where the mod is; 422 for a name that is not a slug or a mod that exists.
pub(crate) async fn scaffold_mod<D: Mods>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let scaffold: ipc::ScaffoldMod = match ipc::decode("a mod to make", &body) {
        Ok(scaffold) => scaffold,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().scaffold_mod(scaffold).await {
        Ok(made) => answer(StatusCode::OK, &made, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_mod_enabled<D: Mods>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let set: ipc::SetModEnabled = match ipc::decode("a mod switch", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_mod_enabled(set).await {
        Ok(summary) => answer(StatusCode::OK, &summary, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?name=` on `validate_mod`.
#[derive(Deserialize)]
pub(crate) struct Named {
    name: String,
}

pub(crate) async fn validate_mod<D: Mods>(
    State(served): State<Served<D>>,
    Query(named): Query<Named>,
) -> Response {
    match served.daemon().validate_mod(named.name).await {
        Ok(checked) => answer(StatusCode::OK, &checked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn promote_mod<D: Mods>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let promote: ipc::PromoteMod = match ipc::decode("a mod to promote", &body) {
        Ok(promote) => promote,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().promote_mod(promote).await {
        Ok(promoted) => answer(StatusCode::OK, &promoted, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
