//! Pairing through his store: two Gateways on loopback, each signed in to the fake store as a device of his
//! account, signing with the shipped store signer.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{DeviceKind, Store, StorePeer};
use axum::http::{Method, StatusCode};
use ipc::{MacLink, PairVouchedMac, VouchedMacs};
use testkit::FakeStore;
use tower::ServiceExt;

use crate::mac_keys::{fresh_key, spki_hex};
use crate::mac_tests::{until, Side};
use crate::tunnel::tunnel_name;
use crate::{router, HisStores, JoinedStore, MacKeys};

const STORE: &str = "https://store.example.com";
const OTHER_STORE: &str = "https://team.example.com";

static DIRS: AtomicU32 = AtomicU32::new(0);

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

/// A device signed in to `store` as `id`: its key and id in a folder of their own, and enrolled.
fn device(store: &FakeStore, id: &str, machine: &str, kind: DeviceKind) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pocket-vouched-{}-{}", std::process::id(), DIRS.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let key = adapters::DeviceKey::open(&dir).unwrap();
    adapters::remember_device(&dir, id).unwrap();
    store.enrol_peer(id, machine, kind, &key.public_key());
    dir
}

/// A Mac's stores: each joined store's rows as this Mac's device there sees them, and that device's folder.
struct FakeStores {
    joined: Mutex<Vec<(String, FakeStore, PathBuf)>>,
}

impl FakeStores {
    fn of(joined: Vec<(&str, FakeStore, PathBuf)>) -> Arc<FakeStores> {
        Arc::new(FakeStores { joined: Mutex::new(joined.into_iter().map(|(address, store, dir)| (address.to_string(), store, dir)).collect()) })
    }

    fn join(&self, address: &str, store: FakeStore, dir: PathBuf) {
        self.joined.lock().unwrap().push((address.to_string(), store, dir));
    }

    fn with<T>(&self, address: &str, act: impl FnOnce(&FakeStore, &PathBuf) -> T) -> Result<T, String> {
        let joined = self.joined.lock().unwrap();
        let (_, store, dir) = joined.iter().find(|(at, _, _)| at == address).ok_or("not joined")?;
        Ok(act(store, dir))
    }
}

impl HisStores for FakeStores {
    fn joined(&self) -> Vec<JoinedStore> {
        self.joined.lock().unwrap().iter().map(|(address, _, _)| JoinedStore { address: address.clone(), team: address == OTHER_STORE }).collect()
    }

    fn peers(&self, store: &str) -> Result<Vec<StorePeer>, String> {
        self.with(store, |store, _| store.peers().map_err(|why| why.said()))?
    }

    fn put_presence(&self, store: &str, address: &str) -> Result<(), String> {
        self.with(store, |store, _| store.put_presence(DeviceKind::Mac, address).map_err(|why| why.said()))?
    }

    fn sign(&self, store: &str, method: &str, target: &str, body: &[u8]) -> Result<Vec<(&'static str, String)>, String> {
        let dir = self.with(store, |_, dir| dir.clone())?;
        Ok(adapters::signed_for_peer(&dir, method, target, body)?.to_vec())
    }

    fn client_id(&self) -> Option<String> {
        Some("Iv1.app".to_string())
    }
}

/// A Mac whose Gateway pairs through `stores`, and which has said where it is reached on each.
async fn mac(name: &str, stores: Arc<FakeStores>) -> Side {
    let mut side = Side::new(name, &[]).await;
    side.stop().await;
    side.stores = Some(stores.clone() as Arc<dyn HisStores>);
    side.start();
    for joined in stores.joined() {
        stores.put_presence(&joined.address, &side.address).unwrap();
    }
    side
}

/// Home and work, both his, on one store.
async fn home_and_work(store: &FakeStore) -> (Side, Side, PathBuf) {
    let home_dir = device(store, "7.home", "Home", DeviceKind::Mac);
    let work_dir = device(store, "7.work", "Work", DeviceKind::Mac);
    let home = mac("Home", FakeStores::of(vec![(STORE, store.as_device("7.home"), home_dir)])).await;
    let work = mac("Work", FakeStores::of(vec![(STORE, store.as_device("7.work"), work_dir.clone())])).await;
    (home, work, work_dir)
}

/// What a Mac named `name` sends to pair through his store, with a fresh key.
fn mac_pairing(name: &str) -> Vec<u8> {
    let key = fresh_key().unwrap();
    let hello = crate::MacHello {
        id: crate::pairing::to_hex(name.as_bytes()),
        name: name.to_string(),
        colour: ipc::MacColour::Teal,
        address: Some("http://127.0.0.1:1".to_string()),
        build: None,
        wire: None,
        built_at: None,
        source: None,
        pin: None,
    };
    ipc::encode(&crate::vouched::MacVouched { mac: hello, public_key: spki_hex(&key).unwrap() }).unwrap().into_bytes()
}

/// A request to `path`, signed by the device in `dir` as it would sign one, naming `store`.
fn signed_request(path: &str, dir: &PathBuf, store: &str, body: &[u8]) -> axum::http::Request<axum::body::Body> {
    let mut request = axum::http::Request::post(path).header("content-type", "application/json");
    for (name, value) in adapters::signed_for_peer(dir, "POST", path, body).unwrap() {
        request = request.header(name, value);
    }
    request.header("x-armada-store", store).body(axum::body::Body::from(body.to_vec())).unwrap()
}

async fn sent(side: &Side, request: axum::http::Request<axum::body::Body>) -> (StatusCode, String) {
    let answer = router(side.gateway.clone()).oneshot(request).await.unwrap();
    let status = answer.status();
    let body = axum::body::to_bytes(answer.into_body(), 1 << 16).await.unwrap();
    (status, String::from_utf8_lossy(&body).to_string())
}

/// Nothing paired, no key kept and no phone added.
async fn nothing_kept(side: &Side) {
    assert_eq!(side.macs().await.len(), 1, "only this Mac");
    assert!(side.gateway.pairing.store.lock().unwrap().paired_devices().unwrap().is_empty());
    assert!(side.keys.read(&crate::pairing::to_hex(b"Stranger")).unwrap().is_none());
}

#[tokio::test]
async fn two_macs_of_one_account_pair_through_their_store_with_one_press_and_each_lists_the_other_linked() {
    let store = FakeStore::new();
    let (home, work, _) = home_and_work(&store).await;
    let (status, body) = home.admin(Method::GET, "/admin/macs/found", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let found: VouchedMacs = ipc::decode("found", body.as_bytes()).unwrap();
    let row = found.macs.iter().find(|row| row.device.as_deref() == Some("7.work")).expect("his store lists Work");
    assert!(row.vouched && row.mac.online && !row.mac.paired, "{row:?}");
    assert_eq!((row.store.as_deref(), row.mac.name.as_str(), row.mac.tunnel.clone()), (Some(STORE), "Work", tunnel_name(&work.id())));

    let press = ipc::encode(&PairVouchedMac { store: STORE.into(), device: "7.work".into() }).unwrap();
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/vouched", &press).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(home.mac(&work.id()).await.unwrap().link, Some(MacLink::Linked));
    assert_eq!(work.mac(&home.id()).await.unwrap().link, Some(MacLink::Linked));
    // No Confirm was asked for on either side.
    assert!(home.claims_handed().is_none_or(|told| told.claims.is_empty()));
    assert!(work.claims_handed().is_none_or(|told| told.claims.is_empty()));
    assert!(home.keys.read(&work.id()).unwrap().is_some() && work.keys.read(&home.id()).unwrap().is_some());
    until("each takes the other's signed calls", || async { home.call(&work, "GET", "/api/mac/hello").await.0 == 200 }).await;
    assert_eq!(work.call(&home, "GET", "/api/mac/hello").await.0, 200);
    let (_, body) = home.admin(Method::GET, "/admin/macs/found", "").await;
    let found: VouchedMacs = ipc::decode("found", body.as_bytes()).unwrap();
    assert!(found.macs.iter().any(|row| row.device.as_deref() == Some("7.work") && row.mac.paired));
}

#[tokio::test]
async fn a_mac_of_another_account_is_refused_and_nothing_is_kept() {
    let store = FakeStore::new();
    let (home, _work, _) = home_and_work(&store).await;
    let stranger = device(&store, "8.stranger", "Stranger", DeviceKind::Mac);
    let body = mac_pairing("Stranger");
    let (status, said) = sent(&home, signed_request("/pair/mac/vouched", &stranger, STORE, &body)).await;
    assert_eq!((status, said.as_str()), (StatusCode::FORBIDDEN, r#"{"error":"not_vouched"}"#));
    nothing_kept(&home).await;
}

#[tokio::test]
async fn a_key_the_store_does_not_hold_for_that_device_is_refused() {
    let store = FakeStore::new();
    let (home, _work, _) = home_and_work(&store).await;
    // Signed as Work's device id, with a key the store never enrolled.
    let forged = std::env::temp_dir().join(format!("pocket-vouched-forged-{}", std::process::id()));
    std::fs::create_dir_all(&forged).unwrap();
    adapters::remember_device(&forged, "7.work").unwrap();
    let body = mac_pairing("Stranger");
    assert_eq!(sent(&home, signed_request("/pair/mac/vouched", &forged, STORE, &body)).await.0, StatusCode::FORBIDDEN);
    nothing_kept(&home).await;
}

#[tokio::test]
async fn a_replayed_nonce_and_a_stale_time_are_refused() {
    let store = FakeStore::new();
    let (home, _work, work_dir) = home_and_work(&store).await;
    let body = mac_pairing("Stranger");
    // Ten minutes on, the same signature is stale.
    let mut later = home.gateway.clone();
    later.pairing.clock = Arc::new(|| now() + 600);
    let answer = router(later).oneshot(signed_request("/pair/mac/vouched", &work_dir, STORE, &body)).await.unwrap();
    assert_eq!(answer.status(), StatusCode::FORBIDDEN);
    nothing_kept(&home).await;

    let first = signed_request("/pair/mac/vouched", &work_dir, STORE, &body);
    let again = {
        let mut again = axum::http::Request::post("/pair/mac/vouched");
        for (name, value) in first.headers() {
            again = again.header(name, value);
        }
        again.body(axum::body::Body::from(body.clone())).unwrap()
    };
    assert_eq!(sent(&home, first).await.0, StatusCode::OK);
    let (status, said) = sent(&home, again).await;
    assert_eq!((status, said.as_str()), (StatusCode::FORBIDDEN, r#"{"error":"not_vouched"}"#), "the nonce was taken");
}

#[tokio::test]
async fn a_mac_that_has_not_joined_the_named_store_refuses() {
    let store = FakeStore::new();
    let (home, _work, _) = home_and_work(&store).await;
    let team = FakeStore::new();
    let work_on_team = device(&team, "7.work", "Work", DeviceKind::Mac);
    let body = mac_pairing("Stranger");
    assert_eq!(sent(&home, signed_request("/pair/mac/vouched", &work_on_team, OTHER_STORE, &body)).await.0, StatusCode::FORBIDDEN);
    nothing_kept(&home).await;
    let press = ipc::encode(&PairVouchedMac { store: OTHER_STORE.into(), device: "7.work".into() }).unwrap();
    let (status, why) = home.admin(Method::POST, "/admin/macs/pair/vouched", &press).await;
    assert_eq!((status, why.as_str()), (StatusCode::BAD_GATEWAY, "This Mac has not joined that store."));
}

fn phone_pairing(name: &str) -> Vec<u8> {
    let spki = spki_hex(&fresh_key().unwrap()).unwrap();
    format!(r#"{{"name":"{name}","spki_hex":"{spki}"}}"#).into_bytes()
}

#[tokio::test]
async fn a_phone_his_store_vouches_for_becomes_a_device_with_no_confirm() {
    let store = FakeStore::new();
    let (home, _work, _) = home_and_work(&store).await;
    let (status, body) = home.admin(Method::GET, "/pair/store", "").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, r#"{"store":"https://store.example.com","client_id":"Iv1.app"}"#));
    let phone = device(&store, "7.phone", "iPhone", DeviceKind::Phone);
    let (status, said) = sent(&home, signed_request("/pair/vouched", &phone, STORE, &phone_pairing("iPhone"))).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let added = home.gateway.pairing.store.lock().unwrap().paired_devices().unwrap();
    assert_eq!(added.len(), 1);
    assert!(said.contains(&added[0].id) && added[0].name == "iPhone", "{said}");
    assert!(home.gateway.pairing.pending().is_empty(), "nothing waits for Confirm");
}

#[tokio::test]
async fn a_phone_of_another_account_is_refused() {
    let store = FakeStore::new();
    let (home, _work, work_dir) = home_and_work(&store).await;
    let phone = device(&store, "8.phone", "iPhone", DeviceKind::Phone);
    assert_eq!(sent(&home, signed_request("/pair/vouched", &phone, STORE, &phone_pairing("iPhone"))).await.0, StatusCode::FORBIDDEN);
    // A Mac of his is not a phone either.
    assert_eq!(sent(&home, signed_request("/pair/vouched", &work_dir, STORE, &phone_pairing("Work"))).await.0, StatusCode::FORBIDDEN);
    nothing_kept(&home).await;
}

#[tokio::test]
async fn presence_is_put_after_a_join() {
    let store = FakeStore::new();
    let dir = device(&store, "7.home", "Home", DeviceKind::Mac);
    let stores = FakeStores::of(Vec::new());
    let home = mac("Home", stores.clone()).await;
    let presence = tokio::spawn(crate::put_presence(home.gateway.clone()));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(store.presence_of("7.home"), Some((DeviceKind::Mac, String::new())), "nothing put before a join");
    stores.join(STORE, store.as_device("7.home"), dir);
    home.gateway.macs.stores_joined();
    let wanted = Some((DeviceKind::Mac, home.address.clone()));
    until("its address is on the store it joined", || async { store.presence_of("7.home") == wanted }).await;
    presence.abort();
}
