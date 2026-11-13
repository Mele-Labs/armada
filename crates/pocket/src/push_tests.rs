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
    body: Vec<u8>,
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
                got.lock().unwrap().push(Received { headers, body: body.to_vec() });
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

/// A Job's update carrying these `(id, text, source)` Needs you items.
fn needs(id: &str, items: &[(&str, &str, &str)]) -> String {
    let items: Vec<String> = items
        .iter()
        .map(|(item, text, source)| {
            format!(r#"{{"id":"{item}","text":"{text}","since":"2026-10-08T01:00:00.000Z","source":"{source}"}}"#)
        })
        .collect();
    changed(id, "live", None).replace(r#""attachments":[]"#, &format!(r#""attachments":[],"waiting_for":[{}]"#, items.join(",")))
}

async fn watching(url: &str) -> (Watch, Rig, String) {
    let rig = rig_with(Arc::new(|| Err("down".into())));
    let device = paired(&rig).await;
    assert_eq!(subscribe(&rig, &device, &subscription(url)).await, StatusCode::NO_CONTENT);
    let mut watch = Watch::new(rig.gateway.clone());
    // No hold: most tests are about what is pushed, not when.
    watch.hold = std::time::Duration::ZERO;
    // The first update for a Job only records what is open, so it sends nothing.
    watch.observe(&changed("s2", "live", None)).await;
    (watch, rig, device)
}

/// `watching`, with the hold a new item waits out outside a test.
async fn watching_held(url: &str) -> (Watch, Rig) {
    let (mut watch, rig, _) = watching(url).await;
    watch.hold = crate::watch::SETTLE;
    (watch, rig)
}

/// Waits until `count` pushes reached the fake service. The clock is paused, so this yields instead of
/// sleeping: an idle runtime would jump the clock past the request's timeout and retry it.
async fn reached(service: &Service, count: usize) {
    let by = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while service.got.lock().unwrap().len() < count {
        assert!(std::time::Instant::now() < by, "{count} pushes never reached the service");
        tokio::task::yield_now().await;
    }
}

/// Moves the paused clock on and lets the watch push what is due, as `follow` does on its timer.
async fn after(watch: &mut Watch, by: std::time::Duration) {
    tokio::time::advance(by).await;
    watch.flush_due();
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
async fn a_new_needs_you_item_pushes_once_however_many_events_repeat_it() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    let asked = needs("s2", &[("a", "Which branch?", "agent")]);
    for _ in 0..3 {
        watch.observe(&asked).await;
        watch.settle().await;
    }
    let got = service.got.lock().unwrap();
    assert_eq!(got.len(), 1);
    let sent = &got[0];
    assert_eq!(sent.headers["content-encoding"], "aes128gcm");
    assert_eq!(sent.headers["urgency"], "high");
    assert_eq!(sent.headers["ttl"], "86400");
    assert!(sent.headers["authorization"].to_str().unwrap().starts_with("vapid t="));
    assert!(sent.body.len() > 86 + 16);
}

#[tokio::test]
async fn an_item_raised_again_after_it_left_pushes_again() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    for event in [
        needs("s2", &[("a", "Which branch?", "agent")]),
        changed("s2", "live", None),
        needs("s2", &[("a", "Which branch?", "agent")]),
    ] {
        watch.observe(&event).await;
        watch.settle().await;
    }
    // The push, the clear when its item left, and the push again.
    assert_eq!(service.got.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn a_second_item_pushes_and_the_first_does_not_repeat() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    watch.settle().await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent"), ("b", "Approve the plan", "agent")])).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_restart_does_not_push_items_that_were_already_open() {
    let service = service().await;
    let (_old, rig, _) = watching(&service.url).await;
    let mut watch = Watch::new(rig.gateway.clone());
    watch.hold = std::time::Duration::ZERO;
    let open = needs("s2", &[("a", "Which branch?", "agent")]);
    watch.observe(&open).await;
    watch.observe(&open).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 0);
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent"), ("b", "Run the restart", "agent")])).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_stuck_session_pushes_nothing() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    for (state, standing) in [("live", Some("stuck")), ("live", Some("working")), ("live", Some("done")), ("ended", Some("stuck"))] {
        watch.observe(&changed("s2", state, standing)).await;
        watch.settle().await;
    }
    assert_eq!(service.got.lock().unwrap().len(), 0);
}

/// `needs`, with the Job snoozed.
fn snoozed(id: &str, items: &[(&str, &str, &str)]) -> String {
    needs(id, items).replace(r#""attachments":[]"#, r#""attachments":[],"snoozed_until":"2026-10-08T05:00:00.000Z""#)
}

#[tokio::test]
async fn a_snoozed_job_pushes_once_when_it_wakes_saying_its_newest_item() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    // Raised before the snooze: pushed then, and not again on waking.
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    watch.settle().await;
    for event in [
        snoozed("s2", &[("a", "Which branch?", "agent")]),
        snoozed("s2", &[("a", "Which branch?", "agent"), ("b", "Approve the plan", "agent")]),
        snoozed("s2", &[("a", "Which branch?", "agent"), ("b", "Approve the plan", "agent"), ("c", "Run the restart", "agent")]),
    ] {
        watch.observe(&event).await;
        watch.settle().await;
    }
    assert_eq!(service.got.lock().unwrap().len(), 1, "nothing is pushed while snoozed");
    let woken = needs("s2", &[("a", "Which branch?", "agent"), ("b", "Approve the plan", "agent"), ("c", "Run the restart", "agent")]);
    for _ in 0..2 {
        watch.observe(&woken).await;
        watch.settle().await;
    }
    let got = service.got.lock().unwrap();
    assert_eq!(got.len(), 2, "one push on waking");
    let said = opened(&got[1].body);
    assert!(said.contains(r#""title":"Title s2""#), "{said}");
    assert!(said.contains(r#""reason":"Run the restart""#), "{said}");
}

#[tokio::test]
async fn a_snoozed_job_whose_items_all_left_pushes_nothing_when_it_wakes() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.observe(&snoozed("s2", &[("a", "Which branch?", "agent")])).await;
    watch.observe(&snoozed("s2", &[])).await;
    watch.observe(&changed("s2", "live", None)).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 0);
    // Awake, a new item pushes as before.
    watch.observe(&needs("s2", &[("b", "Approve the plan", "agent")])).await;
    watch.settle().await;
    assert_eq!(service.got.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_walk_item_says_a_walk_is_ready_and_no_url() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.observe(&needs("s2", &[("walk:armada://walk/pocket", "armada://walk/pocket", "walk")])).await;
    watch.settle().await;
    let got = service.got.lock().unwrap();
    assert_eq!(got.len(), 1);
    let said = opened(&got[0].body);
    assert!(said.contains(r#""reason":"A walk is ready""#), "{said}");
    assert!(!said.contains("armada://"), "{said}");
}

#[tokio::test]
async fn a_410_deletes_the_subscription() {
    let service = service().await;
    service.status.store(410, Ordering::SeqCst);
    let (mut watch, rig, _) = watching(&service.url).await;
    assert_eq!(rig.gateway.pairing.store.lock().unwrap().push_subscriptions().unwrap().len(), 1);
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
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
    let stalled = needs("s2", &[("a", "Which branch?", "agent")]);
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
    let stalled = needs("s2", &[("a", "Which branch?", "agent")]);
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
        vapid_private: None,
    };
    rig.gateway.pairing.store.lock().unwrap().set_push_subscription(&other).unwrap();
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
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
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    assert_eq!(working.got.lock().unwrap().len(), 1);
}

const SECOND: std::time::Duration = std::time::Duration::from_secs(1);

#[tokio::test]
async fn an_item_cleared_inside_the_window_pushes_nothing() {
    tokio::time::pause();
    let service = service().await;
    let (mut watch, _rig) = watching_held(&service.url).await;
    watch.observe(&needs("s2", &[("a", "Allow cargo build?", "permission")])).await;
    after(&mut watch, crate::watch::SETTLE - SECOND).await;
    watch.observe(&changed("s2", "live", None)).await;
    after(&mut watch, 2 * SECOND).await;
    after(&mut watch, crate::watch::SETTLE).await;
    assert_eq!(watch.pushes_started(), 0);
    assert!(service.got.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_item_still_open_after_the_window_pushes_once() {
    tokio::time::pause();
    let service = service().await;
    let (mut watch, _rig) = watching_held(&service.url).await;
    let asked = needs("s2", &[("a", "Which branch?", "agent")]);
    watch.observe(&asked).await;
    after(&mut watch, crate::watch::SETTLE - SECOND).await;
    assert_eq!(watch.pushes_started(), 0, "held until the window ends");
    after(&mut watch, SECOND).await;
    watch.observe(&asked).await;
    after(&mut watch, crate::watch::SETTLE).await;
    assert_eq!(watch.pushes_started(), 1);
    reached(&service, 1).await;
    assert!(opened(&service.got.lock().unwrap()[0].body).contains(r#""reason":"Which branch?""#));
}

#[tokio::test]
async fn two_items_in_the_window_push_once_saying_the_newest() {
    tokio::time::pause();
    let service = service().await;
    let (mut watch, _rig) = watching_held(&service.url).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    after(&mut watch, 20 * SECOND).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent"), ("b", "Approve the plan", "agent")])).await;
    after(&mut watch, 41 * SECOND).await;
    assert_eq!(watch.pushes_started(), 1);
    reached(&service, 1).await;
    assert!(opened(&service.got.lock().unwrap()[0].body).contains(r#""reason":"Approve the plan""#));
}

#[tokio::test]
async fn when_the_newest_left_inside_the_window_the_push_says_the_one_still_open() {
    tokio::time::pause();
    let service = service().await;
    let (mut watch, _rig) = watching_held(&service.url).await;
    watch.observe(&needs("s2", &[("c", "Run the restart", "agent"), ("d", "Review the walk", "agent")])).await;
    watch.observe(&needs("s2", &[("c", "Run the restart", "agent")])).await;
    after(&mut watch, crate::watch::SETTLE).await;
    assert_eq!(watch.pushes_started(), 1);
    reached(&service, 1).await;
    assert!(opened(&service.got.lock().unwrap()[0].body).contains(r#""reason":"Run the restart""#));
}

#[tokio::test]
async fn a_job_whose_pushed_items_all_left_sends_one_clear() {
    tokio::time::pause();
    let service = service().await;
    let (mut watch, _rig) = watching_held(&service.url).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    after(&mut watch, crate::watch::SETTLE).await;
    reached(&service, 1).await;
    let gone = changed("s2", "live", None);
    watch.observe(&gone).await;
    watch.observe(&gone).await;
    after(&mut watch, crate::watch::SETTLE).await;
    assert_eq!(watch.pushes_started(), 2, "the push and one clear");
    reached(&service, 2).await;
    let said = opened(&service.got.lock().unwrap()[1].body);
    assert!(said.contains(r#""clear":true"#) && said.contains(r#""session_id":"s2""#), "{said}");
    assert!(!opened(&service.got.lock().unwrap()[0].body).contains("clear"));
}

#[tokio::test]
async fn an_item_that_left_before_its_hold_ended_sends_no_clear_and_a_snooze_keeps_it_for_the_wake() {
    tokio::time::pause();
    let service = service().await;
    let (mut watch, _rig) = watching_held(&service.url).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    watch.observe(&changed("s2", "live", None)).await;
    assert_eq!(watch.pushes_started(), 0, "nothing was pushed, so nothing is taken back");
    watch.observe(&needs("s2", &[("b", "Approve the plan", "agent")])).await;
    watch.observe(&snoozed("s2", &[("b", "Approve the plan", "agent")])).await;
    after(&mut watch, crate::watch::SETTLE).await;
    assert_eq!(watch.pushes_started(), 0, "a snoozed Job pushes nothing");
    watch.observe(&needs("s2", &[("b", "Approve the plan", "agent")])).await;
    assert_eq!(watch.pushes_started(), 1, "one push when it wakes");
}

/// RFC 8291 undone with the phone's key from [`subscription`]: the payload as sent.
fn opened(body: &[u8]) -> String {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes128Gcm, KeyInit, Nonce};
    use hkdf::Hkdf;
    use sha2::Sha256;
    let phone = SecretKey::from_slice(&[3u8; 32]).unwrap();
    let auth = B64.decode("BTBZMqHH6r4Tts7J_aSIgg").unwrap();
    let (salt, rest) = body.split_at(16);
    let id_len = rest[4] as usize;
    let (sender, sealed) = rest[5..].split_at(id_len);
    let sender_key = p256::PublicKey::from_sec1_bytes(sender).unwrap();
    let shared = p256::ecdh::diffie_hellman(phone.to_nonzero_scalar(), sender_key.as_affine());
    let mut info = b"WebPush: info\0".to_vec();
    info.extend_from_slice(&phone.public_key().to_sec1_bytes());
    info.extend_from_slice(sender);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(&auth), shared.raw_secret_bytes()).expand(&info, &mut ikm).unwrap();
    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let (mut cek, mut nonce) = ([0u8; 16], [0u8; 12]);
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek).unwrap();
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce).unwrap();
    let mut plain = Aes128Gcm::new(&cek.into()).decrypt(Nonce::from_slice(&nonce), sealed).unwrap();
    assert_eq!(plain.pop(), Some(0x02));
    String::from_utf8(plain).unwrap()
}

/// The `k=` of a VAPID `Authorization`: the public key the push was signed with.
fn signed_with(headers: &HeaderMap) -> String {
    headers["authorization"].to_str().unwrap().split(", k=").nth(1).unwrap().to_string()
}

#[tokio::test]
async fn a_second_mac_pushes_under_the_key_the_first_gave_the_phone() {
    let service = service().await;
    let mac_a = rig_with(Arc::new(|| Err("down".into())));
    let on_a = paired(&mac_a).await;
    let time = mac_a.now.load(Ordering::SeqCst);
    let headers = signed(&on_a, "GET", "/api/push/vapid_private", time, "");
    let (status, given) = call(&mac_a.app, Method::GET, "/api/push/vapid_private", &headers, "").await;
    assert_eq!(status, StatusCode::OK, "{given}");
    let key = given.split("\"vapid_private\":\"").nth(1).unwrap().split('"').next().unwrap().to_string();

    let mut gateway = rig_with(Arc::new(|| Err("down".into()))).gateway;
    gateway.push = Push::new(Vapid::from_secret(&[6u8; 32]).unwrap(), "mailto:b@example.invalid").unwrap();
    gateway.push.plain = true;
    let mac_b = Rig { app: crate::router(gateway.clone()), now: Arc::new(std::sync::atomic::AtomicI64::new(1_000_000)), gateway };
    let on_b = paired(&mac_b).await;
    let bad = subscription(&service.url).replacen('{', r#"{"vapid_private":"AA","#, 1);
    assert_eq!(subscribe(&mac_b, &on_b, &bad).await, StatusCode::BAD_REQUEST);
    let carried = subscription(&service.url).replacen('{', &format!(r#"{{"vapid_private":"{key}","#), 1);
    assert_eq!(subscribe(&mac_b, &on_b, &carried).await, StatusCode::NO_CONTENT);

    let mut watch = Watch::new(mac_b.gateway.clone());
    watch.hold = std::time::Duration::ZERO;
    watch.observe(&changed("s2", "live", None)).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    watch.settle().await;
    let got = service.got.lock().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(signed_with(&got[0].headers), test_push().vapid.public_key(), "signed with Mac A's key");
    assert_eq!(
        opened(&got[0].body),
        r#"{"session_id":"s2","title":"Title s2","reason":"Which branch?","origin":"https://mac.tail.ts.net"}"#
    );
}

#[tokio::test]
async fn a_mac_pushes_under_its_own_key_where_the_subscription_carries_none() {
    let service = service().await;
    let (mut watch, _rig, _) = watching(&service.url).await;
    watch.observe(&needs("s2", &[("a", "Which branch?", "agent")])).await;
    watch.settle().await;
    let got = service.got.lock().unwrap();
    assert_eq!(signed_with(&got[0].headers), test_push().vapid.public_key());
    assert!(opened(&got[0].body).ends_with(r#","origin":"https://mac.tail.ts.net"}"#));
}
