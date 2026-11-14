//! A request between two of his devices, signed with the sender's key for one of his claims stores, and checked
//! by the receiver against what **its own** call to that store lists: the store has proved both are his, so
//! nobody types a code or presses Confirm. `docs/concepts/pocket.md`, *Through his store*.
//!
//! The four `X-Armada-*` headers and the text they sign are a store request's, `adapters::store_signing`, with
//! `X-Armada-Store` naming the store. The receiver takes it only where it joined that store, the time is within a
//! minute, the nonce is unseen, and the store lists the sender among its own account's devices with the key that
//! signed. Anything else is `403 not_vouched`, and nothing is kept.

use adapter_traits::{DeviceKind, StorePeer};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::Signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::macs::MacHello;
use crate::pairing::{from_hex, to_hex};
use crate::Gateway;

pub(crate) const STORE: &str = "x-armada-store";
const DEVICE: &str = "x-armada-device";
const TIME: &str = "x-armada-time";
const NONCE: &str = "x-armada-nonce";
const SIGNATURE: &str = "x-armada-signature";

/// How far a signature's time may be from this Mac's clock, either way.
const SKEW: i64 = 60;

/// `POST /pair/mac/vouched`, and its answer: a Mac's hello and the fresh key it made toward the other, SPKI DER as
/// hex, as the link flow makes one.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct MacVouched {
    pub mac: MacHello,
    pub public_key: String,
}

/// `POST /pair/vouched`: a phone's name and its key, as the code flow's claim carries them.
#[derive(Debug, Deserialize)]
pub(crate) struct PhoneVouched {
    pub name: String,
    pub spki_hex: String,
}

/// The headers of one signed request or answer.
pub(crate) struct Signed {
    pub device: String,
    pub time: i64,
    pub nonce: String,
    pub signature: String,
    pub store: String,
}

impl Signed {
    pub(crate) fn from(headers: &HeaderMap) -> Option<Signed> {
        let get = |name: &str| headers.get(name).and_then(|value| value.to_str().ok()).map(str::to_string);
        Some(Signed {
            device: get(DEVICE)?,
            time: get(TIME)?.parse().ok()?,
            nonce: get(NONCE).filter(|nonce| !nonce.is_empty() && nonce.len() <= 128)?,
            signature: get(SIGNATURE)?,
            store: get(STORE)?,
        })
    }
}

/// The refusal for anything not proven, the same whatever was wrong.
pub(crate) fn not_vouched() -> Response {
    (StatusCode::FORBIDDEN, [("content-type", "application/json")], r#"{"error":"not_vouched"}"#).into_response()
}

/// The text a store request's signature covers. **It must match `adapters::store_signing::signed_text`**, which
/// signs what this checks.
fn signed_text(method: &str, target: &str, time: i64, nonce: &str, body: &[u8]) -> String {
    format!("{method}{target}{time}{nonce}{}", to_hex(&Sha256::digest(body)))
}

/// The device of `kind` that signed `body` as `method` `target`, where the store it names, one this Mac joined,
/// lists it as his with the key that signed. Its nonce is taken, so it is good once. Blocking: it asks the store.
pub(crate) fn vouched(gateway: &Gateway, signed: &Signed, kind: DeviceKind, method: &str, target: &str, body: &[u8]) -> Option<StorePeer> {
    let stores = gateway.macs.stores.as_ref()?;
    if !stores.joined().iter().any(|joined| joined.address == signed.store) {
        return None;
    }
    let now = gateway.pairing.now();
    if (now - signed.time).abs() > SKEW || gateway.macs.nonce_seen(&signed.nonce, now) {
        return None;
    }
    let peer = stores.peers(&signed.store).ok()?.into_iter().find(|peer| peer.device == signed.device && peer.kind == kind)?;
    let key = crate::signing::verifying_key(&B64.decode(&peer.public_key).ok()?)?;
    let signature = Signature::from_slice(&from_hex(&signed.signature)?).ok()?;
    let text = signed_text(method, target, signed.time, &signed.nonce, body);
    key.verify(text.as_bytes(), &signature).ok()?;
    gateway.macs.take_nonce(&signed.nonce, now).then_some(peer)
}

/// The headers that sign `body` as `method` `target` from this Mac on `store`, `X-Armada-Store` among them.
/// Blocking: it reads the store key's file.
pub(crate) fn signing(gateway: &Gateway, store: &str, method: &str, target: &str, body: &[u8]) -> Result<Vec<(&'static str, String)>, String> {
    let stores = gateway.macs.stores.as_ref().ok_or("This Mac has joined no store.")?;
    let mut headers = stores.sign(store, method, target, body)?;
    headers.push((STORE, store.to_string()));
    Ok(headers)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_signed_text_is_the_stores_own() {
        // `adapters`' `the_signed_text_is_method_target_time_nonce_and_the_body_hash_joined_with_nothing` pins the same.
        let text = super::signed_text("GET", "/v1/claims", 1_700_000_000, "00112233445566778899aabbccddeeff", b"");
        assert_eq!(
            text,
            "GET/v1/claims170000000000112233445566778899aabbccddeeffe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
