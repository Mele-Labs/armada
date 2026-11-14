//! A Mac waiting to pair calls him in Needs you on the Mac it waits on, through that Mac's real Gateway and Fleet:
//! one call and one arrival while it waits, and gone once he confirms or declines it there.

use std::sync::{Arc, Mutex};

use axum::http::{Method, StatusCode};
use ipc::{CallArrivalKind, CallTone, CallsChanged, Event, StandingCall, StandingCallKind, StandingCalls};

use super::{until, OneMac, Ticking};

/// What `calls.changed` told on `mac` from now on.
fn told(mac: &OneMac) -> Arc<Mutex<Vec<CallsChanged>>> {
    let told: Arc<Mutex<Vec<CallsChanged>>> = Arc::default();
    let seen = Arc::clone(&told);
    mac.rig.fleet.events().tap(move |event| {
        if let Event::CallsChanged(told) = event {
            seen.lock().expect("not poisoned").push(told.clone());
        }
    });
    told
}

async fn calls(mac: &OneMac) -> Vec<StandingCall> {
    let calls: StandingCalls = ipc::decode("the calls", mac.read("/calls").await.as_bytes()).expect("the calls");
    calls.calls
}

/// Home shows the link and Work pastes it, so Work's claim waits on Home's Confirm.
async fn claimed(home: &OneMac, work: &OneMac) {
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/start", String::new()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let link: ipc::MacPairLink = ipc::decode("a link", body.as_bytes()).expect("a link");
    let claim = ipc::encode(&ipc::ClaimMacLink { link: link.link }).expect("encodes");
    let (status, body) = work.admin(Method::POST, "/admin/macs/pair/claim", claim).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Work's claim on Home stands as one call, told once, with Work's id and what Settings says of it.
async fn calls_once(home: &OneMac, work: &OneMac, told: &Arc<Mutex<Vec<CallsChanged>>>) {
    let key = format!("mac:{}", work.id);
    until("Work's claim calls on Home", || async { calls(home).await.iter().any(|one| one.key == key) }).await;
    let call = calls(home).await.into_iter().find(|one| one.key == key).expect("the call");
    let why = home.mac(&work.id).await.and_then(|mac| mac.why).expect("Settings says why");
    assert_eq!(
        (call.kind, call.owner.as_str(), call.title.as_str(), call.text.as_str(), call.tone, call.opened, call.mac.as_deref(), call.session_id),
        (StandingCallKind::MacPairing, key.as_str(), "Confirm Work pairing", why.as_str(), CallTone::Blocked, false, Some(work.id.as_str()), None)
    );
    let arrived: Vec<_> = told.lock().expect("not poisoned").iter().flat_map(|one| one.arrived.clone()).filter(|one| one.session_id.as_str() == key).collect();
    assert_eq!(arrived.len(), 1, "told once: {arrived:?}");
    assert_eq!((arrived[0].kind, arrived[0].tone, arrived[0].title.as_str()), (CallArrivalKind::MacPairing, CallTone::Blocked, "Confirm Work pairing"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_mac_waiting_to_pair_calls_him_until_he_confirms_it() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let told = told(&home);
    claimed(&home, &work).await;
    calls_once(&home, &work, &told).await;

    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id.clone() }).expect("encodes");
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/confirm", confirm).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(calls(&home).await.iter().all(|one| one.kind != StandingCallKind::MacPairing), "Confirm takes the call off");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_mac_waiting_to_pair_calls_him_until_he_declines_it() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let told = told(&home);
    claimed(&home, &work).await;
    calls_once(&home, &work, &told).await;

    let decline = ipc::encode(&ipc::MacNamed { mac: work.id.clone() }).expect("encodes");
    let (status, body) = home.admin(Method::POST, "/admin/macs/forget", decline).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert!(calls(&home).await.iter().all(|one| one.kind != StandingCallKind::MacPairing), "Decline takes the call off");
}
