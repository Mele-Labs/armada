//! The pairing and device routes. `/pair` is the phone's, the rest are Bridge's
//! on loopback behind `admin::loopback_only`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::pairing::{ConfirmRefused, PairRefused};
use crate::Gateway;

#[derive(Serialize)]
pub struct Started {
    code: String,
    address: String,
    expires_at: i64,
}

#[derive(Deserialize)]
pub struct Claim {
    code: String,
    name: String,
    /// The SPKI DER from WebCrypto `exportKey("spki")`, as hex.
    spki_hex: String,
}

#[derive(Deserialize)]
pub struct Confirm {
    code: String,
}

#[derive(Serialize)]
pub struct DeviceView {
    id: String,
    name: String,
    created_at: i64,
    last_seen_at: Option<i64>,
}

fn view(d: store::Device) -> DeviceView {
    DeviceView { id: d.id, name: d.name, created_at: d.created_at, last_seen_at: d.last_seen_at }
}

fn fault(why: impl ToString) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, why.to_string()).into_response()
}

pub async fn start(State(gateway): State<Gateway>) -> Response {
    let pairing = gateway.pairing.clone();
    match tokio::task::spawn_blocking(move || pairing.start()).await {
        Ok(Ok((code, address, expires_at))) => Json(Started { code, address, expires_at }).into_response(),
        Ok(Err(why)) => (StatusCode::SERVICE_UNAVAILABLE, why).into_response(),
        Err(why) => fault(why),
    }
}

pub async fn claim(State(gateway): State<Gateway>, Json(claim): Json<Claim>) -> Response {
    match gateway.pairing.claim(&claim.code, &claim.name, &claim.spki_hex) {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(PairRefused::TooMany) => {
            (StatusCode::TOO_MANY_REQUESTS, "Too many tries. Wait a minute, then try again.").into_response()
        }
        Err(PairRefused::BadCode) => {
            (StatusCode::FORBIDDEN, "This code is not good any more. Start pairing again in Bridge.").into_response()
        }
        Err(PairRefused::BadKey) => (StatusCode::BAD_REQUEST, "The phone's key could not be read.").into_response(),
        Err(PairRefused::NoName) => (StatusCode::BAD_REQUEST, "The phone needs a name.").into_response(),
    }
}

pub async fn confirm(State(gateway): State<Gateway>, Json(body): Json<Confirm>) -> Response {
    match gateway.pairing.confirm(&body.code) {
        Ok(device) => Json(view(device)).into_response(),
        Err(ConfirmRefused::BadCode) => {
            (StatusCode::NOT_FOUND, "This code is not good any more. Start pairing again.").into_response()
        }
        Err(ConfirmRefused::NotClaimed) => {
            (StatusCode::CONFLICT, "No phone has entered this code yet.").into_response()
        }
        Err(ConfirmRefused::Store(why)) => fault(why),
    }
}

pub async fn devices(State(gateway): State<Gateway>) -> Response {
    match gateway.pairing.store.lock().unwrap().paired_devices() {
        Ok(all) => Json(all.into_iter().map(view).collect::<Vec<_>>()).into_response(),
        Err(why) => fault(why),
    }
}

pub async fn unpair(State(gateway): State<Gateway>, Path(id): Path<String>) -> Response {
    match gateway.pairing.store.lock().unwrap().remove_device(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "No phone is paired with that id.").into_response(),
        Err(why) => fault(why),
    }
}
