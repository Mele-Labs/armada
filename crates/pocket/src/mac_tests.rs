//! Two Gateways on loopback, each with a fake Fleet and keys in memory, pairing and linking as two
//! of the owner's Macs would.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::{RawQuery, State};
use axum::http::{Method, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::Router;
use ipc::{Cursor, Mac, MacLink, ProtocolId, Resync, StreamMessage};
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use tower::ServiceExt;

use crate::mac_keys::{fresh_key, sign, spki_hex};
use crate::macs::read_link;
use crate::pairing::{from_hex, to_hex};
use crate::{router, Gateway, MacKeys, Macs, MemoryMacKeys, Pairing, Timing};

static DIRS: AtomicU32 = AtomicU32::new(0);

const QUICK: Timing = Timing {
    beat: Duration::from_millis(100),
    silence: Duration::from_millis(1500),
    first_retry: Duration::from_millis(100),
    longest_retry: Duration::from_millis(400),
};

fn row(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","harness":"term","origin":"bridge","manifest_id":"m1","cwd":"/w","title":"Title {id}","state":"live","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{{}},"attachments":[]}}"#
    )
}

const PROJECTS_CHANGED: &str =
    r#"{"message":"event","cursor":2,"event":{"kind":"projects.changed","projects":[],"loose":[],"left_out":[]}}"#;

/// A Fleet that serves `/other_mac/*` and keeps what the Gateway hands it on `/other_macs/*`.
#[derive(Clone)]
struct FakeFleet {
    rows: Arc<Mutex<Vec<String>>>,
    handed: Arc<Mutex<Vec<(String, String)>>>,
    events: broadcast::Sender<String>,
}

impl FakeFleet {
    fn handed(&self, what: &str) -> Vec<String> {
        self.handed.lock().unwrap().iter().filter(|(path, _)| path == what).map(|(_, body)| body.clone()).collect()
    }
}

async fn sessions(State(fleet): State<FakeFleet>) -> String {
    format!(r#"{{"sessions":[{}]}}"#, fleet.rows.lock().unwrap().join(","))
}

async fn thread(RawQuery(query): RawQuery) -> String {
    format!(r#"{{"asked":"{}"}}"#, query.unwrap_or_default())
}

async fn events(State(fleet): State<FakeFleet>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    let mut events = fleet.events.subscribe();
    upgrade.on_upgrade(|mut socket| async move {
        let resync = StreamMessage::Resync(Resync { protocol_id: ProtocolId::current(), cursor: Cursor::at(1) });
        socket.send(Message::Text(ipc::encode(&resync).unwrap())).await.ok();
        while let Ok(text) = events.recv().await {
            if socket.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    })
}

async fn keep(State(fleet): State<FakeFleet>, method: Method, uri: Uri, body: Bytes) -> StatusCode {
    let said = String::from_utf8_lossy(&body).to_string();
    fleet.handed.lock().unwrap().push((format!("{method} {}", uri.path()), said));
    StatusCode::NO_CONTENT
}

async fn fake_fleet(rows: &[&str]) -> (FakeFleet, u16) {
    let fleet = FakeFleet {
        rows: Arc::new(Mutex::new(rows.iter().map(|id| row(id)).collect())),
        handed: Arc::default(),
        events: broadcast::channel(16).0,
    };
    let app = Router::new()
        .route("/other_mac/sessions", get(sessions))
        .route("/other_mac/bug_reports", get(|| async { r#"{"reports":[]}"# }))
        .route("/other_mac/thread", get(thread))
        .route("/other_mac/events", get(events))
        .route("/other_macs/sessions", put(keep))
        .route("/other_macs/bug_reports", put(keep))
        .route("/other_macs/event", post(keep))
        .route("/other_macs/forget", post(keep))
        .with_state(fleet.clone());
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.ok() });
    (fleet, port)
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

/// One Mac: its Gateway served on loopback at `address`, and its link running.
pub(crate) struct Side {
    pub(crate) gateway: Gateway,
    fleet: FakeFleet,
    port: u16,
    pub(crate) address: String,
    pub(crate) keys: Arc<MemoryMacKeys>,
    running: Option<(oneshot::Sender<()>, JoinHandle<()>, JoinHandle<()>)>,
}

fn pairing(name: &str, address: String) -> Pairing {
    let dir = std::env::temp_dir().join(format!("pocket-macs-{}-{}", std::process::id(), DIRS.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let store = store::Store::open_migrated_copy(&dir.join("armada.db")).unwrap();
    let pairing = Pairing::new(store, Arc::new(now), Arc::new(move || Ok(address.clone())));
    let mac = store::ThisMac { id: to_hex(name.as_bytes()), name: name.to_string(), colour: "teal".into(), minted_at: 1 };
    pairing.store.lock().unwrap().this_mac_or(&mac).unwrap();
    pairing
}

impl Side {
    pub(crate) async fn new(name: &str, rows: &[&str]) -> Side {
        let (fleet, fleet_port) = fake_fleet(rows).await;
        let port = crate::bind(0).await.unwrap().local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{port}");
        let keys = Arc::new(MemoryMacKeys::default());
        let gateway = Gateway {
            fleet: Arc::new(move || Ok(fleet_port)),
            assets: None,
            pairing: pairing(name, address.clone()),
            push: crate::push_tests::test_push(),
            way_in: crate::setup::quiet_way_in(),
            macs: Macs::with_timing(keys.clone(), QUICK).unwrap(),
        };
        let mut side = Side { gateway, fleet, port, address, keys, running: None };
        side.start();
        side
    }

    pub(crate) fn id(&self) -> String {
        crate::machine::this_mac(&self.gateway).unwrap().id
    }

    /// Serve, and link, as `pocketing` does: a fresh `Macs` each start, over the same keys.
    pub(crate) fn start(&mut self) {
        self.gateway.macs = Macs::with_timing(self.keys.clone(), QUICK).unwrap();
        let listener = crate::bind_now(self.port).unwrap();
        let (stop, stopped) = oneshot::channel::<()>();
        let app = router(self.gateway.clone());
        let serving = tokio::spawn(async move {
            let stopped = async move {
                stopped.await.ok();
            };
            axum::serve(listener, app).with_graceful_shutdown(stopped).await.ok();
        });
        let linking = tokio::spawn(crate::link(self.gateway.clone()));
        self.running = Some((stop, serving, linking));
    }

    pub(crate) async fn stop(&mut self) {
        let (stop, serving, linking) = self.running.take().unwrap();
        self.gateway.macs.close();
        linking.abort();
        stop.send(()).ok();
        tokio::time::timeout(Duration::from_secs(5), serving).await.expect("the Gateway stopped").ok();
    }

    pub(crate) async fn admin(&self, method: Method, path: &str, body: &str) -> (StatusCode, String) {
        let method = reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap();
        let answer = reqwest::Client::new()
            .request(method, format!("{}{path}", self.address))
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        let status = StatusCode::from_u16(answer.status().as_u16()).unwrap();
        (status, answer.text().await.unwrap())
    }

    pub(crate) async fn macs(&self) -> Vec<Mac> {
        let (status, body) = self.admin(Method::GET, "/admin/macs", "").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        ipc::decode("macs", body.as_bytes()).unwrap()
    }

    pub(crate) async fn mac(&self, id: &str) -> Option<Mac> {
        self.macs().await.into_iter().find(|mac| mac.id == id)
    }

    /// A signed call from this Mac to `other`, as the link makes one.
    async fn call(&self, other: &Side, method: &str, target: &str) -> (u16, String) {
        let came = crate::mac_client::signed_whole(&self.gateway, &other.id(), &other.address, method, target, None)
            .await
            .unwrap();
        (came.status, came.sentence())
    }
}

/// Wait until `check` holds, up to ten seconds.
pub(crate) async fn until<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..200 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never: {what}");
}

async fn link_from(home: &Side) -> String {
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/start", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let link: ipc::MacPairLink = ipc::decode("a link", body.as_bytes()).unwrap();
    assert!(link.link.starts_with(&format!("{}/pair/mac?code=", home.address)), "{}", link.link);
    ipc::encode(&ipc::ClaimMacLink { link: link.link }).unwrap()
}

/// Home shows the link, work pastes it, home presses Confirm.
async fn paired(home: &Side, work: &Side) {
    let claim = link_from(home).await;
    let (status, body) = work.admin(Method::POST, "/admin/macs/pair/claim", &claim).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let waiting: Mac = ipc::decode("a Mac", body.as_bytes()).unwrap();
    assert_eq!((waiting.id.as_str(), waiting.link.clone()), (home.id().as_str(), Some(MacLink::ConfirmWaiting)));
    assert_eq!(waiting.why.as_deref(), Some("Confirm on Home"));
    assert_eq!(work.mac(&home.id()).await, Some(waiting));

    let claimed = home.mac(&work.id()).await.expect("the claim is listed on home");
    assert_eq!(claimed.link, Some(MacLink::ClaimWaiting));
    assert!(claimed.why.is_some_and(|why| why.contains("Confirm")));

    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id() }).unwrap();
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/confirm", &confirm).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn two_macs_pair_by_link_and_confirm_and_each_lists_the_other_as_linked() {
    let (home, work) = (Side::new("Home", &[]).await, Side::new("Work", &[]).await);
    paired(&home, &work).await;
    assert!(home.macs().await[0].this_mac, "this Mac first");
    let work_there = home.mac(&work.id()).await.unwrap();
    assert_eq!((work_there.name.as_str(), work_there.link), ("Work", Some(MacLink::Linked)));
    let home_there = work.mac(&home.id()).await.unwrap();
    assert_eq!((home_there.name.as_str(), home_there.link, home_there.why), ("Home", Some(MacLink::Linked), None));
    // Neither private key crossed: each side holds only its own, under the other's id.
    assert!(home.keys.read(&work.id()).unwrap().is_some() && home.keys.read(&home.id()).unwrap().is_none());
    assert!(work.keys.read(&home.id()).unwrap().is_some() && work.keys.read(&work.id()).unwrap().is_none());
}

#[tokio::test]
async fn a_confirm_the_other_mac_does_not_answer_keeps_the_claim_to_press_again() {
    let (home, mut work) = (Side::new("Home", &[]).await, Side::new("Work", &[]).await);
    let claim = link_from(&home).await;
    assert_eq!(work.admin(Method::POST, "/admin/macs/pair/claim", &claim).await.0, StatusCode::OK);
    work.stop().await;
    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id() }).unwrap();
    let (status, why) = home.admin(Method::POST, "/admin/macs/pair/confirm", &confirm).await;
    assert_eq!((status, why.as_str()), (StatusCode::BAD_GATEWAY, "Work did not answer. Press Confirm again."));
    assert_eq!(home.mac(&work.id()).await.unwrap().link, Some(MacLink::ClaimWaiting));
    work.start();
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/confirm", &confirm).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(work.mac(&home.id()).await.unwrap().link, Some(MacLink::Linked));
}

#[tokio::test]
async fn a_claim_needs_this_macs_own_address_and_a_good_link() {
    let work = Side::new("Work", &[]).await;
    let wrong = ipc::encode(&ipc::ClaimMacLink { link: "http://home.example.com/pair/mac?code=ab".into() }).unwrap();
    assert_eq!(work.admin(Method::POST, "/admin/macs/pair/claim", &wrong).await.0, StatusCode::BAD_REQUEST);
    let mut nowhere = work.gateway.clone();
    nowhere.pairing = Pairing::new(
        store::Store::open_migrated_copy(&std::env::temp_dir().join(format!("pocket-macs-none-{}.db", std::process::id()))).unwrap(),
        Arc::new(now),
        Arc::new(|| Err("Tailscale is not installed on this Mac.".into())),
    );
    let claim = ipc::encode(&ipc::ClaimMacLink { link: "https://home.example.com/pair/mac?code=ab".into() }).unwrap();
    let answer = router(nowhere)
        .oneshot(
            axum::http::Request::post("/admin/macs/pair/claim")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(claim))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE);
    let why = axum::body::to_bytes(answer.into_body(), 1 << 16).await.unwrap();
    assert!(String::from_utf8_lossy(&why).contains("Tailscale is not installed"), "{why:?}");
}

#[tokio::test]
async fn a_signed_call_passes_each_way_and_the_wrong_side_gets_404() {
    let (home, work) = (Side::new("Home", &[]).await, Side::new("Work", &[]).await);
    paired(&home, &work).await;
    let (status, body) = home.call(&work, "GET", "/api/mac/hello").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(&work.id()), "{body}");
    let (status, body) = work.call(&home, "GET", "/api/mac/hello").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(&home.id()), "{body}");

    // A Mac on a phone's route: not there for it.
    assert_eq!(home.call(&work, "GET", "/api/needs").await.0, 404);
    assert_eq!(home.call(&work, "GET", "/api/sessions").await.0, 404);

    // A phone on a Mac's route: the same.
    let code = work.gateway.pairing.start().unwrap().0;
    let phone = p256::ecdsa::SigningKey::from_slice(&[7u8; 32]).unwrap();
    let phone_id = work.gateway.pairing.claim(&code, "Phone", &spki_hex(&phone).unwrap()).ok().unwrap();
    work.gateway.pairing.confirm(&code).ok().unwrap();
    let headers = crate::pairing_tests::signed(&phone_id, "GET", "/api/mac/hello", now(), "");
    let mut asked = reqwest::Client::new().get(format!("{}/api/mac/hello", work.address));
    for (name, value) in headers {
        asked = asked.header(name, value);
    }
    assert_eq!(asked.send().await.unwrap().status().as_u16(), 404);
}

#[tokio::test]
async fn a_request_this_mac_signs_passes_another_gateways_check() {
    let other = crate::pairing_tests::rig();
    let key = fresh_key().unwrap();
    let mac = store::PairedMac {
        id: "abc1".into(),
        name: "Work".into(),
        colour: "green".into(),
        address: "https://work.example.com".into(),
        public_key: from_hex(&spki_hex(&key).unwrap()),
        build: None,
        paired_at: 1,
        state: store::MacState::Linked,
        back_code: None,
        last_heard_at: None,
        asleep_since: None,
        why: None,
        wire: None,
        built_at: None,
        source: None,
    };
    other.gateway.pairing.store.lock().unwrap().keep_mac(&mac).unwrap();
    let time = other.gateway.pairing.now();
    let signed = |key: &p256::ecdsa::SigningKey| sign(key, "abc1", "GET", "/api/mac/hello", time, b"").unwrap();
    let ask = |headers: [(&'static str, String); 3]| {
        let mut request = axum::http::Request::get("/api/mac/hello");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let app = other.app.clone();
        async move { app.oneshot(request.body(axum::body::Body::empty()).unwrap()).await.unwrap().status() }
    };
    let good = signed(&key);
    assert_eq!(ask(good.clone()).await, StatusCode::OK);
    assert_eq!(ask(good).await, StatusCode::CONFLICT, "a replay is refused");
    // The same request signed twice reads as two, so the link may ask the same thing in one second.
    assert_eq!(ask(signed(&key)).await, StatusCode::OK);
    assert_eq!(ask(signed(&fresh_key().unwrap())).await, StatusCode::BAD_REQUEST, "a forged signature");
    let unknown = sign(&key, "ffff", "GET", "/api/mac/hello", time, b"").unwrap();
    assert_eq!(ask(unknown).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_mac_on_a_build_that_speaks_another_wire_is_answered_503_on_all_but_hello_and_update() {
    let other = crate::pairing_tests::rig();
    let key = fresh_key().unwrap();
    // A build from before Macs compared wires says none, and is the older.
    let mac = store::PairedMac {
        id: "abc1".into(),
        name: "Work".into(),
        colour: "green".into(),
        address: "https://work.example.com".into(),
        public_key: from_hex(&spki_hex(&key).unwrap()),
        build: Some("1111111".into()),
        paired_at: 1,
        state: store::MacState::Linked,
        back_code: None,
        last_heard_at: None,
        asleep_since: None,
        why: None,
        wire: None,
        built_at: None,
        source: None,
    };
    other.gateway.pairing.store.lock().unwrap().keep_mac(&mac).unwrap();
    let ask = |method: &'static str, path: &'static str| {
        let time = other.gateway.pairing.now();
        let headers = sign(&key, "abc1", method, path, time, b"").unwrap();
        let mut request = axum::http::Request::builder().method(method).uri(path);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let app = other.app.clone();
        async move {
            let answer = app.oneshot(request.body(axum::body::Body::empty()).unwrap()).await.unwrap();
            let status = answer.status();
            let body = axum::body::to_bytes(answer.into_body(), 1 << 16).await.unwrap();
            (status, String::from_utf8_lossy(&body).to_string())
        }
    };
    let frozen = "Work runs an older build. Nothing crosses until it updates.";
    for (method, path) in [("GET", "/api/mac/sessions"), ("GET", "/api/mac/live"), ("POST", "/api/mac/act"), ("POST", "/api/mac/project_act")] {
        assert_eq!(ask(method, path).await, (StatusCode::SERVICE_UNAVAILABLE, frozen.to_string()), "{method} {path}");
    }
    assert_eq!(ask("GET", "/api/mac/hello").await.0, StatusCode::OK);
    assert_ne!(ask("POST", "/api/mac/update").await.1, frozen, "the update crosses a frozen link");
    let listed = crate::macs::listed(other.gateway.pairing.store.lock().unwrap().paired_mac("abc1").unwrap().unwrap(), &other.gateway.way_in);
    assert_eq!((listed.link, listed.why.as_deref()), (Some(MacLink::OlderBuild), Some(frozen)));

    // On this build's wire it is linked, whatever its commit.
    let matched = store::PairedMac { wire: Some(other.gateway.way_in.wire.clone()), ..mac };
    other.gateway.pairing.store.lock().unwrap().keep_mac(&matched).unwrap();
    assert_ne!(ask("GET", "/api/mac/sessions").await.1, frozen);
    let listed = crate::macs::listed(matched, &other.gateway.way_in);
    assert_eq!(listed.link, Some(MacLink::Linked));
}

#[tokio::test]
async fn the_link_hands_the_other_macs_rows_to_this_fleet_and_reads_them_again_when_projects_change() {
    let (home, work) = (Side::new("Home", &[]).await, Side::new("Work", &["w1"]).await);
    paired(&home, &work).await;
    let work_id = work.id();
    until("work's rows and reports reach home's Fleet", || async {
        let rows = home.fleet.handed("PUT /other_macs/sessions").iter().any(|body| body.contains("\"w1\"") && body.contains(&work_id));
        rows && home.fleet.handed("PUT /other_macs/bug_reports").iter().any(|body| body.contains(&work_id))
    })
    .await;
    let heard = home.mac(&work_id).await.unwrap();
    assert!(heard.last_heard_at.is_some() && heard.link == Some(MacLink::Linked), "{heard:?}");

    work.fleet.rows.lock().unwrap().push(row("w2"));
    work.fleet.events.send(PROJECTS_CHANGED.to_string()).unwrap();
    until("the event and the list read again reach home's Fleet", || async {
        let event = home.fleet.handed("POST /other_macs/event").iter().any(|body| body.contains("projects.changed"));
        let again = home.fleet.handed("PUT /other_macs/sessions").iter().any(|body| body.contains("\"w2\""));
        event && again
    })
    .await;
}

#[tokio::test]
async fn a_mac_that_stops_shows_asleep_with_why_and_links_again_when_it_starts() {
    let (home, mut work) = (Side::new("Home", &[]).await, Side::new("Work", &["w1"]).await);
    paired(&home, &work).await;
    let work_id = work.id();
    until("linked", || async { !home.fleet.handed("PUT /other_macs/sessions").is_empty() }).await;
    let before = now();
    work.stop().await;
    until("work shows asleep on home", || async {
        matches!(home.mac(&work_id).await.unwrap().link, Some(MacLink::Asleep { .. }))
    })
    .await;
    let asleep = home.mac(&work_id).await.unwrap();
    let Some(MacLink::Asleep { since }) = asleep.link else { unreachable!() };
    assert!(since >= before - 1 && since <= now(), "{since}");
    assert!(asleep.why.as_deref().is_some_and(|why| why.starts_with("Work ")), "{asleep:?}");
    work.start();
    until("work is linked again", || async { home.mac(&work_id).await.unwrap().link == Some(MacLink::Linked) }).await;
    assert_eq!(home.mac(&work_id).await.unwrap().why, None);
}

#[tokio::test]
async fn a_thread_is_read_from_the_other_mac_and_nothing_else_is() {
    let (home, work) = (Side::new("Home", &[]).await, Side::new("Work", &[]).await);
    paired(&home, &work).await;
    let (status, body) =
        crate::read_other_mac(&home.gateway, &work.id(), "/other_mac/thread?session_id=s1&before=5").await.unwrap();
    assert_eq!((status, String::from_utf8(body).unwrap()), (200, r#"{"asked":"session_id=s1&before=5"}"#.to_string()));
    assert!(crate::read_other_mac(&home.gateway, &work.id(), "/other_mac/sessions").await.is_err());
    assert!(crate::read_other_mac(&home.gateway, "nobody", "/other_mac/thread?session_id=s1").await.is_err());
}

#[tokio::test]
async fn forgetting_a_mac_forgets_it_on_both_sides() {
    let (home, work) = (Side::new("Home", &[]).await, Side::new("Work", &[]).await);
    paired(&home, &work).await;
    let (home_id, work_id) = (home.id(), work.id());
    let forget = ipc::encode(&ipc::MacNamed { mac: work_id.clone() }).unwrap();
    let (status, body) = home.admin(Method::POST, "/admin/macs/forget", &forget).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert!(home.mac(&work_id).await.is_none());
    assert!(work.mac(&home_id).await.is_none(), "work was told");
    assert!(home.keys.read(&work_id).unwrap().is_none() && work.keys.read(&home_id).unwrap().is_none());
    assert!(home.fleet.handed("POST /other_macs/forget").iter().any(|body| body.contains(&work_id)));
    assert!(work.fleet.handed("POST /other_macs/forget").iter().any(|body| body.contains(&home_id)));
}

#[test]
fn a_pairing_link_is_https_or_loopback() {
    assert_eq!(
        read_link("https://home.example.com/pair/mac?code=ab12"),
        Some(("https://home.example.com".into(), "ab12".into()))
    );
    assert_eq!(read_link("http://127.0.0.1:8443/pair/mac?code=ab12").map(|(a, _)| a), Some("http://127.0.0.1:8443".into()));
    for wrong in [
        "http://home.example.com/pair/mac?code=ab12",
        "https://home.example.com/pair?code=ab12",
        "https://home.example.com/pair/mac?code=",
        "https://home.example.com/pair/mac?code=zz",
        "https://user@home.example.com/pair/mac?code=ab12",
    ] {
        assert_eq!(read_link(wrong), None, "{wrong}");
    }
}
