//! The four headers on every request to the central store except enrolment.
//!
//! The signed text is `method + path_and_query + time + nonce + hex(sha256(body))`,
//! joined with nothing between: the text `pocket`'s `signing.rs` verifies, with a
//! nonce added because the store, not a process map, holds the replay set.
//!
//! **Time and nonce are arguments.** Reading the clock in here would make the
//! signature a thing a test could only look at, never reproduce.

use sha2::{Digest, Sha256};

use crate::store_device::{to_hex, DeviceKey};

pub(crate) const DEVICE: &str = "X-Armada-Device";
pub(crate) const TIME: &str = "X-Armada-Time";
pub(crate) const NONCE: &str = "X-Armada-Nonce";
pub(crate) const SIGNATURE: &str = "X-Armada-Signature";

pub(crate) fn signed_text(method: &str, target: &str, time: i64, nonce: &str, body: &[u8]) -> String {
    format!("{method}{target}{time}{nonce}{}", to_hex(&Sha256::digest(body)))
}

/// The headers to send, in the order above.
pub(crate) fn headers(
    key: &DeviceKey,
    device: &str,
    method: &str,
    target: &str,
    time: i64,
    nonce: &str,
    body: &[u8],
) -> [(&'static str, String); 4] {
    [
        (DEVICE, device.to_string()),
        (TIME, time.to_string()),
        (NONCE, nonce.to_string()),
        (SIGNATURE, key.sign(signed_text(method, target, time, nonce, body).as_bytes())),
    ]
}
