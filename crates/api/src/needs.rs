//! `act_on_need`, `list_needs` and `list_write_clashes`. Since 23.46. `docs/capabilities/needs.md`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Needs;
use crate::served::Served;

/// What `armada need` tells Fleet. 200: the ledger holds it when this answers.
pub(crate) async fn act_on_need<D: Needs>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let call: ipc::NeedCall = match ipc::decode("a need", &body) {
        Ok(call) => call,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().act_on_need(call).await {
        Ok(done) => answer(StatusCode::OK, &done, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?manifest_id=` on `list_needs`.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    manifest_id: Option<String>,
}

pub(crate) async fn list_needs<D: Needs>(
    State(served): State<Served<D>>,
    Query(listing): Query<Listing>,
) -> Response {
    let manifest = listing.manifest_id.map(ipc::ManifestId::carried);
    match served.daemon().list_needs(manifest).await {
        Ok(needs) => answer(StatusCode::OK, &needs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?manifest_id=&holder=` on `list_write_clashes`.
#[derive(Deserialize)]
pub(crate) struct Clashing {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    holder: Option<String>,
}

pub(crate) async fn list_write_clashes<D: Needs>(
    State(served): State<Served<D>>,
    Query(asked): Query<Clashing>,
) -> Response {
    let manifest = asked.manifest_id.map(ipc::ManifestId::carried);
    match served.daemon().list_write_clashes(manifest, asked.holder).await {
        Ok(clashes) => answer(StatusCode::OK, &clashes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
