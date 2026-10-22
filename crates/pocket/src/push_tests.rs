//! Subscribing, and what reaches a fake push service: a local axum server that
//! counts the pushes it is sent.

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::routing::post;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use p256::SecretKey;

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};
use crate::vapid::Vapid;
use crate::watch::Watch;
use crate::Push;
use store::PushSubscription;

pub(crate) fn test_push() -> Push {
    let mut push = Push::new(Vapid::from_secret(&[5u8; 32]).unwrap(), "mailto:test@example.invalid").unwrap();
    push.plain = true;
    push
}

struct Received {
    headers: HeaderMap,
    body: usize,
}

struct Service {
    url: String,
    got: Arc<Mutex<Vec<Received>>>,
    status: Arc<AtomicU16>,
    /// Statuses to answer first, one per request, before `status`.
    first: Arc<Mutex<Vec<u16>>>,
}

async fn service() -> Service {
    let got = Arc::new(Mutex::new(Vec::new()));
    let status = Arc::new(AtomicU16::new(201));
    let first = Arc::new(Mutex::new(Vec::<u16>::new()));
    let app = Router::new().route(
        "/push/abc",
        post({
            let (got, status, first) = (got.clone(), status.clone(), first.clone());
            move |headers: HeaderMap, body: Bytes| async move {
                got.lock().unwrap().push(Received { headers, body: body.len() });
                let queued = { let mut first = first.lock().unwrap(); (!first.is_empty()).then(|| first.remove(0)) };
                StatusCode::from_u16(queued.unwrap_or_else(|| status.load(Ordering::SeqCst))).unwrap()
            }
        }),
    );
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Service { url: format!("http://127.0.0.1:{port}/push/abc"), got, status, first }
}

fn subscription(endpoint: &str) -> String {
    let phone = SecretKey::from_slice(&[3u8; 32]).unwrap();
    let point = B64.encode(phone.public_key().to_sec1_bytes());
    format!(r#"{{"endpoint":"{endpoint}","expirationTime":null,"keys":{{"p256dh":"{point}","auth":"BTBZMqHH6r4Tts7J_aSIgg"}}}}"#)
}

async fn subscribe(rig: &Rig, device: &str, body: &str) -> StatusCode {
    let time = rig.now.load(Ordering::SeqCst);
    let mut headers = signed(device, "POST", "/api/push/subscribe", time, body);
    headers.push(("content-type", "application/json".to_string()));
    call(&rig.app, Method::POST, "/api/push/subscribe", &headers, body).await.0
}

/// A Session as Fleet publishes it after any fact about it.
fn changed(id: &str, state: &str, standing: Option<&str>) -> String {
    let standing = standing.map(|one| format!(r#","standing":"{one}""#)).unwrap_or_default();
    format!(
        r#"{{"message":"event","cursor":2,"event":{{"kind":"session.changed","id":"{id}","harness":"term","origin":"bridge","cwd":"/r","title":"Title {id}","state":"{state}","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{{}},"attachments":[]{standing}}}}}"#
    )
}

async fn watching(url: &str) -> (Watch, Rig, String) {
    let rig = rig_with(Arc::new(|| Err("down".into())));
    let device = paired(&rig).await;
    assert_eq!(subscribe(&rig, &device, &subscription(url)).await, StatusCode::NO_CONTENT);
    (Watch::new(rig.gateway.clone()), rig, device)
}

#[tokio::test]
async fn subscribe_without_a_signature_is_400() {
    let rig = rig_with(Arc::new(|| Err("down".into())));
    let (status, _) = call(&rig.app, Method::POST, "/api/push/subscribe", &[], &subscription("https://x.test/p")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(&rig.app, Method::GET, "/api/push/key", &[], "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_phone_keeps_one_subscription_and_can_read_the_key() {
    let rig = rig_with(Arc::new(|| Err("down".into())));
    let device = paired(&rig).await;
    assert_eq!(subscribe(&rig, &device, &subscription("ftp://push.test/x")).await, StatusCode::BAD_REQUEST);
    let bad_key = subscription("https://push.test/x").replace("BTBZMqHH6r4Tts7J_aSIgg", "AA");
    assert_eq!(subscribe(&rig, &device, &bad_key).await, StatusCode::BAD_REQUEST);
    // Outside a test, a plain http endpoint is refused too.
    let mut strict = rig.gateway.clone();
    strict.push.plain = false;
    let strict = Rig { app: crate::router(strict), now: rig.now.clone(), gateway: rig.gateway.clone() };
    assert_eq!(subscribe(&strict, &device, &subscription("http://127.0.0.1:1/x")).await, StatusCode::BAD_REQUEST);
    for endpoint in ["https://push.test/one", "https://push.test/two"] {
        assert_eq!(subscribe(&rig, &device, &subscription(endpoint)).await, StatusCode::NO_CONTENT);
    }
    let kept = rig.gateway.pairing.store.lock().unwrap().push_subscriptions().unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].endpoint, "https://push.test/two");
    assert_eq!(kept[0].device_id, device);
    let time = rig.now.load(Ordering::SeqCst);
    let headers = signed(&device, "GET", "/api/push/key", time, "");
    let (status, body) = call(&rig.app, Method::GET, "/api/push/key", &headers, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, format!(r#"{{"public_key":"{}"}}"#, test_push().vapid.public_key()));
}

#[tokio::test]
async fn a_stuck_session_pushes_once_however_many_events_repeat_it() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    let stuck = changed("s2", "live", Some("stuck"));
    for _ in 0..3 {
        watch.observe(&stuck).await;
        watch.settle().await;
    }
    let got = service.got.lock().unwrap();
    assert_eq!(got.len(), 1);
    let sent = &got[0];
    assert_eq!(sent.headers["content-encoding"], "aes128gcm");
    assert_eq!(sent.headers["urgency"], "high");
    assert_eq!(sent.headers["ttl"], "86400");
    assert!(sent.headers["authorization"].to_str().unwrap().starts_with("vapid t="));
    assert!(sent.body > 86 + 16);
}

#[tokio::test]
async fn a_session_that_moves_on_and_goes_quiet_again_pushes_again() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.observe(&changed("s2", "live", Some("stuck"))).await;
    watch.settle().await;
    watch.observe(&changed("s2", "live", Some("working"))).await;
    watch.settle().await;
    watch.observe(&changed("s2", "live", Some("stuck"))).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn nothing_but_a_live_stuck_session_pushes() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    for (state, standing) in [
        ("live", Some("working")),
        ("live", Some("replied")),
        ("live", Some("done")),
        ("live", Some("idle")),
        ("live", None),
        ("ended", Some("stuck")),
    ] {
        watch.observe(&changed("s2", state, standing)).await;
        watch.settle().await;
    }
    assert_eq!(service.got.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn a_410_deletes_the_subscription() {
    let service = service().await;
    service.status.store(410, Ordering::SeqCst);
    let (mut watch, rig, _) = watching(&service.url).await;
    assert_eq!(rig.gateway.pairing.store.lock().unwrap().push_subscriptions().unwrap().len(), 1);
    watch.observe(&changed("s2", "live", Some("stuck"))).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 1);
    assert!(rig.gateway.pairing.store.lock().unwrap().push_subscriptions().unwrap().is_empty());
}

#[tokio::test]
async fn a_failed_push_is_retried_and_delivered_once() {
    let service = service().await;
    *service.first.lock().unwrap() = vec![500];
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.retries = vec![std::time::Duration::from_millis(10); 3];
    let stalled = changed("s2", "live", Some("stuck"));
    watch.observe(&stalled).await;
    watch.settle().await;
    watch.observe(&stalled).await;
    watch.settle().await;
    // One refused, one delivered, and nothing after.
    assert_eq!(service.got.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_push_that_keeps_failing_stops_after_three_retries() {
    let service = service().await;
    service.status.store(500, Ordering::SeqCst);
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.retries = vec![std::time::Duration::from_millis(10); 3];
    let stalled = changed("s2", "live", Some("stuck"));
    watch.observe(&stalled).await;
    watch.settle().await;
    watch.observe(&stalled).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 4);
}

/// Two phones, one Session. The first phone's endpoint keeps failing and its retries
/// are an hour apart, so it is still retrying when the second is delivered. No
/// clock is advanced: the second delivery has to arrive without waiting on the first.
#[tokio::test]
async fn a_phone_that_keeps_failing_does_not_hold_up_another() {
    let (failing, working) = (service().await, service().await);
    failing.status.store(500, Ordering::SeqCst);
    let (mut watch, rig, _) = watching(&failing.url).await;
    watch.retries = vec![std::time::Duration::from_secs(3600); 3];
    let other = subscription(&working.url);
    let other = PushSubscription {
        device_id: "other-phone".into(),
        endpoint: working.url.clone(),
        p256dh: other.split("\"p256dh\":\"").nth(1).unwrap().split('"').next().unwrap().to_string(),
        auth: "BTBZMqHH6r4Tts7J_aSIgg".into(),
    };
    rig.gateway.pairing.store.lock().unwrap().set_push_subscription(&other).unwrap();
    watch.observe(&changed("s2", "live", Some("stuck"))).await;
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while working.got.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the working phone was never pushed");
    assert_eq!(working.got.lock().unwrap().len(), 1);
    assert!(!failing.got.lock().unwrap().is_empty());
    // The same stop again, while the first is still retrying, sends nothing more.
    watch.observe(&changed("s2", "live", Some("stuck"))).await;
    assert_eq!(working.got.lock().unwrap().len(), 1);
}
