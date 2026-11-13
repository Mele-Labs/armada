//! Request signing for every `/api/*` route.
//!
//! The phone sends `X-Pocket-Device`, `X-Pocket-Time` (Unix seconds) and
//! `X-Pocket-Signature` (hex of WebCrypto's raw r||s). The signature is ECDSA
//! P-256 with SHA-256 over `method + path-and-query + time + hex(sha256(body))`,
//! joined with nothing between.

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use p256::pkcs8::DecodePublicKey;
use sha2::{Digest, Sha256};

use crate::pairing::{from_hex, to_hex, Pairing};

/// The paired device whose signature a request passed, for the handler to read.
#[derive(Clone)]
pub struct DeviceId(pub String);

/// The paired Mac whose signature a `/api/mac/*` request passed.
#[derive(Clone)]
pub struct MacId(pub String);

/// Only another Mac calls these, and it calls nothing else. A phone's routes are not share-filtered,
/// so this line is what keeps the other Mac off private work.
const MAC_ROUTES: &str = "/api/mac/";

/// Whose key `device` is: a phone's, or a linked Mac's.
fn key_of(pairing: &Pairing, device: &str) -> Option<(Vec<u8>, bool)> {
    let store = pairing.store.lock().unwrap();
    if let Ok(Some(phone)) = store.paired_device(device) {
        return Some((phone.public_key, false));
    }
    let mac = store.paired_mac(device).ok().flatten()?;
    let linked = mac.state == store::MacState::Linked;
    mac.public_key.filter(|_| linked).map(|key| (key, true))
}

pub const SKEW: i64 = 60;
const BODY_LIMIT: usize = 1024 * 1024;

pub(crate) fn verifying_key(spki: &[u8]) -> Option<VerifyingKey> {
    VerifyingKey::from_public_key_der(spki).ok()
}

/// The phone app reads 401 as "this Mac does not know me" and offers to pair
/// again, so only an unknown device answers it. Every other failure is the
/// request's, said in a sentence the app shows as it is.
fn refuse(status: StatusCode, why: &'static str) -> Response {
    (status, why).into_response()
}

pub async fn signed(State(pairing): State<Pairing>, request: Request, next: Next) -> Response {
    if !request.uri().path().starts_with("/api/") {
        return next.run(request).await;
    }
    let sent = {
        let get = |name: &str| {
            request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        (get("x-pocket-device"), get("x-pocket-time"), get("x-pocket-signature"))
    };
    let (Some(device), Some(time), Some(signature)) = sent else {
        return refuse(
            StatusCode::BAD_REQUEST,
            "This request is not signed. Open Armada on the phone it was paired with.",
        );
    };
    let to_a_mac = request.uri().path().starts_with(MAC_ROUTES);
    let Some((public_key, from_a_mac)) = key_of(&pairing, &device) else {
        let why = match to_a_mac {
            true => "This Mac is not paired with that one. Pair them again from Settings, Macs.",
            false => "This phone is not paired. Pair it again from Bridge, Settings, Phone.",
        };
        return refuse(StatusCode::UNAUTHORIZED, why);
    };
    let now = (pairing.clock)();
    let Ok(sent) = time.parse::<i64>() else {
        return refuse(StatusCode::BAD_REQUEST, "The time on this request is not a number.");
    };
    if (now - sent).abs() > SKEW {
        return refuse(
            StatusCode::BAD_REQUEST,
            "The phone's clock is more than a minute off this Mac's. Set it to automatic.",
        );
    }
    let (mut parts, body) = request.into_parts();
    let Ok(body) = to_bytes(body, BODY_LIMIT).await else {
        return (StatusCode::PAYLOAD_TOO_LARGE, "This request is too large.").into_response();
    };
    let target = parts.uri.path_and_query().map_or("", |p| p.as_str());
    let message = format!(
        "{}{}{}{}",
        parts.method,
        target,
        time,
        to_hex(&Sha256::digest(&body))
    );
    let valid = verifying_key(&public_key)
        .zip(from_hex(&signature).and_then(|raw| Signature::from_slice(&raw).ok()))
        .is_some_and(|(key, sig)| key.verify(message.as_bytes(), &sig).is_ok());
    if !valid {
        return refuse(StatusCode::BAD_REQUEST, "The signature on this request does not match the phone's key.");
    }
    if !pairing.first_sight(&signature, now) {
        return refuse(StatusCode::CONFLICT, "This request was already sent.");
    }
    // The wrong side's route is not there for it, the same as an unlisted one.
    if from_a_mac != to_a_mac {
        return StatusCode::NOT_FOUND.into_response();
    }
    if from_a_mac {
        parts.extensions.insert(MacId(device));
    } else {
        let _ = pairing.store.lock().unwrap().touch_device(&device, now);
        parts.extensions.insert(DeviceId(device));
    }
    next.run(Request::from_parts(parts, Body::from(body))).await
}
