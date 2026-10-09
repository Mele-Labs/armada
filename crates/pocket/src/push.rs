//! Web Push from the Gateway: the key and client it sends with, and the two
//! signed routes the phone uses to subscribe.

use std::path::Path;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use p256::PublicKey;
use serde::{Deserialize, Serialize};
use store::PushSubscription;

use crate::vapid::Vapid;
use crate::{signing::DeviceId, Gateway};

/// The VAPID `sub` when Tailscale is not up. Normally `sub` is the Gateway's
/// own tailnet address, so no personal address leaves the Mac.
pub const PLACEHOLDER_SUBJECT: &str = "mailto:armada@example.invalid";

#[derive(Clone)]
pub struct Push {
    pub(crate) vapid: Arc<Vapid>,
    pub(crate) subject: String,
    pub(crate) client: reqwest::Client,
    /// Whether an `http` endpoint is taken. Only a test sets it: a phone that
    /// could name a plain endpoint could make this Mac post to anything on loopback.
    pub(crate) plain: bool,
}

impl Push {
    pub fn new(vapid: Vapid, subject: &str) -> Result<Push, String> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|why| format!("The push client could not start: {why}"))?;
        Ok(Push { vapid: Arc::new(vapid), subject: subject.to_string(), client, plain: false })
    }

    /// The key in Armada's machine directory, made on first use.
    pub fn open(machine: &Path, subject: &str) -> Result<Push, String> {
        Push::new(Vapid::open(machine)?, subject)
    }
}

#[derive(Serialize)]
struct Key {
    public_key: String,
}

/// `GET /api/push/key`: the `applicationServerKey` the phone subscribes with.
pub async fn key(State(gateway): State<Gateway>) -> Response {
    Json(Key { public_key: gateway.push.vapid.public_key() }).into_response()
}

#[derive(Deserialize)]
pub struct Keys {
    p256dh: String,
    auth: String,
}

/// A browser's `PushSubscription.toJSON()`. Other fields, such as
/// `expirationTime`, are ignored.
#[derive(Deserialize)]
pub struct Subscribe {
    endpoint: String,
    keys: Keys,
}

fn refuse(why: &str) -> Response {
    (StatusCode::BAD_REQUEST, why.to_string()).into_response()
}

/// `POST /api/push/subscribe`: keep this subscription for the signing phone,
/// in place of any it sent before.
pub async fn subscribe(
    State(gateway): State<Gateway>,
    Extension(DeviceId(device)): Extension<DeviceId>,
    Json(sent): Json<Subscribe>,
) -> Response {
    let secure = sent.endpoint.starts_with("https://");
    if !(secure || gateway.push.plain && sent.endpoint.starts_with("http://")) || crate::webpush::origin(&sent.endpoint).is_none() {
        return refuse("The push endpoint is not an https address.");
    }
    let point = B64.decode(sent.keys.p256dh.trim_end_matches('='));
    if !point.is_ok_and(|bytes| PublicKey::from_sec1_bytes(&bytes).is_ok()) {
        return refuse("The push key is not a P-256 key.");
    }
    if !B64.decode(sent.keys.auth.trim_end_matches('=')).is_ok_and(|bytes| bytes.len() == 16) {
        return refuse("The push secret is not 16 bytes.");
    }
    let sub = PushSubscription {
        device_id: device,
        endpoint: sent.endpoint,
        p256dh: sent.keys.p256dh,
        auth: sent.keys.auth,
    };
    let saved = gateway.pairing.store.lock().unwrap().set_push_subscription(&sub);
    match saved {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(why) => (StatusCode::INTERNAL_SERVER_ERROR, why.to_string()).into_response(),
    }
}
