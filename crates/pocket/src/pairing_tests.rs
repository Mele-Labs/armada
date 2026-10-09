//! Pairing and request signing, against a clock the test moves.

use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::EncodePublicKey;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use crate::pairing::{address_from_status, to_hex};
use crate::{router, Gateway, Pairing};

static DIRS: AtomicU32 = AtomicU32::new(0);

pub(crate) struct Rig {
    pub app: Router,
    pub now: Arc<AtomicI64>,
    pub gateway: Gateway,
}

pub(crate) fn pairing(now: Arc<AtomicI64>, address: Result<String, String>) -> Pairing {
    let dir = std::env::temp_dir().join(format!(
        "pocket-test-{}-{}",
        std::process::id(),
        DIRS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let store = store::Store::open(&dir.join("armada.db")).unwrap();
    Pairing::new(
        store,
        Arc::new(move || now.load(Ordering::Relaxed)),
        Arc::new(move || address.clone()),
    )
}

pub(crate) fn rig_pairing() -> Pairing {
    pairing(Arc::new(AtomicI64::new(0)), Err("no tailscale in tests".into()))
}

pub(crate) fn rig() -> Rig {
    rig_with(Arc::new(|| Err("Fleet is not running".to_string())))
}

pub(crate) fn rig_with(fleet: crate::Fleet) -> Rig {
    let now = Arc::new(AtomicI64::new(1_000_000));
    let gateway = Gateway {
        fleet,
        assets: None,
        pairing: pairing(now.clone(), Ok("https://mac.tail.ts.net".into())),
        push: crate::push_tests::test_push(),
    };
    Rig { app: router(gateway.clone()), now, gateway }
}

fn key() -> SigningKey {
    SigningKey::from_slice(&[7u8; 32]).unwrap()
}

pub(crate) async fn call(app: &Router, method: Method, path: &str, headers: &[(&str, String)], body: &str) -> (StatusCode, String) {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let answer = app.clone().oneshot(request.body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = answer.status();
    (status, String::from_utf8(to_bytes(answer.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap())
}

/// A body field, found without parsing: the tests' own bodies are flat.
fn field(body: &str, name: &str) -> String {
    let rest = &body[body.find(&format!("\"{name}\":\"")).unwrap() + name.len() + 4..];
    rest[..rest.find('"').unwrap()].to_string()
}

async fn start(rig: &Rig) -> String {
    let (status, body) = call(&rig.app, Method::POST, "/admin/pair/start", &[], "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    field(&body, "code")
}

async fn claim(rig: &Rig, code: &str) -> StatusCode {
    let spki = to_hex(key().verifying_key().to_public_key_der().unwrap().as_bytes());
    let body = format!("{{\"code\":\"{code}\",\"name\":\"Phone\",\"spki_hex\":\"{spki}\"}}");
    call(&rig.app, Method::POST, "/pair", &[("content-type", "application/json".into())], &body).await.0
}

async fn confirm(rig: &Rig, code: &str) -> (StatusCode, String) {
    let body = format!("{{\"code\":\"{code}\"}}");
    call(&rig.app, Method::POST, "/admin/pair/confirm", &[("content-type", "application/json".into())], &body).await
}

pub(crate) async fn paired(rig: &Rig) -> String {
    let code = start(rig).await;
    assert_eq!(claim(rig, &code).await, StatusCode::ACCEPTED);
    let (status, body) = confirm(rig, &code).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    field(&body, "id")
}

pub(crate) fn signed(device: &str, method: &str, path: &str, time: i64, body: &str) -> Vec<(&'static str, String)> {
    let message = format!("{method}{path}{time}{}", to_hex(&Sha256::digest(body.as_bytes())));
    let signature: Signature = key().sign(message.as_bytes());
    vec![
        ("x-pocket-device", device.to_string()),
        ("x-pocket-time", time.to_string()),
        ("x-pocket-signature", to_hex(&signature.to_bytes())),
    ]
}

/// Past the signature, `/api/needs` asks Fleet, which the rig has down: 503.
async fn get_needs(rig: &Rig, headers: &[(&str, String)]) -> StatusCode {
    call(&rig.app, Method::GET, "/api/needs", headers, "").await.0
}

#[tokio::test]
async fn start_says_where_the_gateway_is_and_gives_a_128_bit_code() {
    let rig = rig();
    let (status, body) = call(&rig.app, Method::POST, "/admin/pair/start", &[], "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&body, "code").len(), 32);
    assert_eq!(field(&body, "address"), "https://mac.tail.ts.net");
}

#[tokio::test]
async fn start_without_tailscale_says_what_to_do() {
    let now = Arc::new(AtomicI64::new(0));
    let gateway = Gateway {
        fleet: Arc::new(|| Err(String::new())),
        assets: None,
        pairing: pairing(now, Err("Tailscale is not installed on this Mac.".into())),
        push: crate::push_tests::test_push(),
    };
    let (status, body) = call(&router(gateway), Method::POST, "/admin/pair/start", &[], "").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("Tailscale"));
}

#[test]
fn the_address_is_self_dnsname_without_its_dot() {
    let out = r#"{"BackendState": "Running", "Self": {"ID":"x","DNSName": "mac.tail-1.ts.net.","Online":true},"Peer":{"a":{"DNSName":"other.ts.net."}}}"#;
    assert_eq!(address_from_status(out).unwrap(), "https://mac.tail-1.ts.net");
    assert!(address_from_status(r#"{"BackendState":"NeedsLogin"}"#).is_err());
    assert!(address_from_status("").is_err());
}

#[tokio::test]
async fn a_code_is_burned_by_confirming() {
    let rig = rig();
    let code = start(&rig).await;
    assert_eq!(claim(&rig, &code).await, StatusCode::ACCEPTED);
    assert_eq!(confirm(&rig, &code).await.0, StatusCode::OK);
    assert_eq!(confirm(&rig, &code).await.0, StatusCode::NOT_FOUND);
    assert_eq!(claim(&rig, &code).await, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_code_is_burned_by_its_ttl() {
    let rig = rig();
    let code = start(&rig).await;
    rig.now.fetch_add(301, Ordering::Relaxed);
    assert_eq!(claim(&rig, &code).await, StatusCode::FORBIDDEN);
    let code = start(&rig).await;
    assert_eq!(claim(&rig, &code).await, StatusCode::ACCEPTED);
    rig.now.fetch_add(301, Ordering::Relaxed);
    assert_eq!(confirm(&rig, &code).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_code_cannot_be_confirmed_before_a_phone_claims_it_or_claimed_twice() {
    let rig = rig();
    let code = start(&rig).await;
    assert_eq!(confirm(&rig, &code).await.0, StatusCode::CONFLICT);
    assert_eq!(claim(&rig, &code).await, StatusCode::ACCEPTED);
    assert_eq!(claim(&rig, &code).await, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_claimed_code_is_pending_until_confirmed_or_expired() {
    let rig = rig();
    let pending = || async {
        call(&rig.app, Method::GET, "/admin/pair/pending", &[], "").await.1
    };
    assert_eq!(pending().await, "[]");
    let unclaimed = start(&rig).await;
    assert_eq!(pending().await, "[]");
    let code = start(&rig).await;
    claim(&rig, &code).await;
    let listed = pending().await;
    assert!(listed.contains(&code) && listed.contains("\"name\":\"Phone\"") && listed.contains("claimed_at"), "{listed}");
    assert!(!listed.contains(&unclaimed));
    confirm(&rig, &code).await;
    assert_eq!(pending().await, "[]");
    let code = start(&rig).await;
    claim(&rig, &code).await;
    rig.now.fetch_add(301, Ordering::Relaxed);
    assert_eq!(pending().await, "[]");
}

#[tokio::test]
async fn pairing_is_rate_limited() {
    let rig = rig();
    for _ in 0..10 {
        assert_eq!(claim(&rig, "wrong").await, StatusCode::FORBIDDEN);
    }
    assert_eq!(claim(&rig, "wrong").await, StatusCode::TOO_MANY_REQUESTS);
    rig.now.fetch_add(61, Ordering::Relaxed);
    assert_eq!(claim(&rig, "wrong").await, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_signed_request_from_a_paired_phone_passes_and_marks_it_seen() {
    let rig = rig();
    let id = paired(&rig).await;
    let now = rig.now.load(Ordering::Relaxed);
    assert_eq!(get_needs(&rig, &signed(&id, "GET", "/api/needs", now, "")).await, StatusCode::SERVICE_UNAVAILABLE);
    let (_, list) = call(&rig.app, Method::GET, "/admin/devices", &[], "").await;
    assert!(list.contains(&format!("\"last_seen_at\":{now}")), "{list}");
}

#[tokio::test]
async fn an_unsigned_request_is_refused() {
    let rig = rig();
    assert_eq!(get_needs(&rig, &[]).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_replayed_signature_is_refused() {
    let rig = rig();
    let id = paired(&rig).await;
    let now = rig.now.load(Ordering::Relaxed);
    let headers = signed(&id, "GET", "/api/needs", now, "");
    assert_eq!(get_needs(&rig, &headers).await, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(get_needs(&rig, &headers).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_skewed_request_is_refused() {
    let rig = rig();
    let id = paired(&rig).await;
    let now = rig.now.load(Ordering::Relaxed);
    for time in [now - 61, now + 61] {
        assert_eq!(get_needs(&rig, &signed(&id, "GET", "/api/needs", time, "")).await, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(get_needs(&rig, &signed(&id, "GET", "/api/needs", now - 60, "")).await, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn an_unpaired_device_is_refused() {
    let rig = rig();
    let now = rig.now.load(Ordering::Relaxed);
    assert_eq!(get_needs(&rig, &signed("nobody", "GET", "/api/needs", now, "")).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_bad_signature_or_a_changed_body_is_refused() {
    let rig = rig();
    let id = paired(&rig).await;
    let now = rig.now.load(Ordering::Relaxed);
    let mut forged = signed(&id, "GET", "/api/needs", now, "");
    forged[2].1 = "00".repeat(64);
    assert_eq!(get_needs(&rig, &forged).await, StatusCode::UNAUTHORIZED);
    let other_path = signed(&id, "GET", "/api/jobs", now + 1, "");
    assert_eq!(get_needs(&rig, &other_path).await, StatusCode::UNAUTHORIZED);
    let headers = signed(&id, "POST", "/api/jobs", now + 2, "{\"a\":1}");
    let (status, _) = call(&rig.app, Method::POST, "/api/jobs", &headers, "{\"a\":2}").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&rig.app, Method::POST, "/api/jobs", &headers, "{\"a\":1}").await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn after_an_unpair_the_next_request_is_refused() {
    let rig = rig();
    let id = paired(&rig).await;
    let now = rig.now.load(Ordering::Relaxed);
    assert_eq!(get_needs(&rig, &signed(&id, "GET", "/api/needs", now, "")).await, StatusCode::SERVICE_UNAVAILABLE);
    let (status, _) = call(&rig.app, Method::DELETE, &format!("/admin/devices/{id}"), &[], "").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(get_needs(&rig, &signed(&id, "GET", "/api/needs", now + 1, "")).await, StatusCode::UNAUTHORIZED);
}
