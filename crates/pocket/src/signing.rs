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

pub const SKEW: i64 = 60;
const BODY_LIMIT: usize = 1024 * 1024;

pub(crate) fn verifying_key(spki: &[u8]) -> Option<VerifyingKey> {
    VerifyingKey::from_public_key_der(spki).ok()
}

fn refuse(why: &'static str) -> Response {
    (StatusCode::UNAUTHORIZED, why).into_response()
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
        return refuse("This request is not signed. Open Armada on the phone it was paired with.");
    };
    let now = (pairing.clock)();
    let Ok(sent) = time.parse::<i64>() else {
        return refuse("The time on this request is not a number.");
    };
    if (now - sent).abs() > SKEW {
        return refuse("The phone's clock is more than a minute off this Mac's. Set it to automatic.");
    }
    let found = pairing.store.lock().unwrap().paired_device(&device);
    let Ok(Some(found)) = found else {
        return refuse("This phone is not paired. Pair it again from Bridge, Settings, Phone.");
    };
    let (parts, body) = request.into_parts();
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
    let valid = verifying_key(&found.public_key)
        .zip(from_hex(&signature).and_then(|raw| Signature::from_slice(&raw).ok()))
        .is_some_and(|(key, sig)| key.verify(message.as_bytes(), &sig).is_ok());
    if !valid {
        return refuse("The signature on this request does not match the phone's key.");
    }
    if !pairing.first_sight(&signature, now) {
        return refuse("This request was already sent.");
    }
    let _ = pairing.store.lock().unwrap().touch_device(&device, now);
    next.run(Request::from_parts(parts, Body::from(body))).await
}
