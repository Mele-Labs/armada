//! Pairing through his store: one press, no code and no Confirm, because a store both devices joined lists both
//! as his. `crate::vouched` checks each signed request and answer.
//!
//! | Step | Route | On |
//! |---|---|---|
//! | His Macs his store lists | `GET /admin/macs/found`, `crate::vouched_found` | this |
//! | He presses Pair; this Mac sends its hello and key, signed | `POST /admin/macs/pair/vouched`, then `POST /pair/mac/vouched` | this, then other |
//! | The other keeps this one linked, and answers its own, signed | `POST /pair/mac/vouched` | other, then this |
//! | A phone signed in to his store pairs with this Mac | `GET /pair/store`, then `POST /pair/vouched` | this |
//!
//! Each Mac puts where it is reached to every store it joined, so his other devices find it: [`put_presence`].

use std::collections::HashMap;
use std::time::Duration;

use adapter_traits::DeviceKind;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use ipc::PairVouchedMac;
use p256::ecdsa::SigningKey;
use serde::Serialize;

use crate::his_stores::his_own;
use crate::mac_client::{encoded, hello_now, PATIENCE};
use crate::mac_keys::{fresh_key, kept_form, spki_hex};
use crate::mac_pair_routes::{blocking, fault, said_why};
use crate::macs::{listed, reachable, sane, MacHello};
use crate::pairing::{from_hex, random_hex};
use crate::reads::json;
use crate::vouched::{not_vouched, signing, vouched, MacVouched, PhoneVouched, Signed};
use crate::Gateway;

const MAC_PATH: &str = "/pair/mac/vouched";
const PHONE_PATH: &str = "/pair/vouched";
/// How often the presence task reads this Mac's address again, to put a changed one.
const PRESENCE_EVERY: Duration = Duration::from_secs(60);

/// `GET /pair/store`: where a phone that opened Pocket from this Mac signs in to pair through his store.
#[derive(Serialize)]
struct StoreForPhone {
    store: Option<String>,
    client_id: Option<String>,
}

/// `POST /pair/vouched`'s answer: the id the phone signs as from now on.
#[derive(Serialize)]
struct PhoneAdded {
    device_id: String,
}

/// `GET /pair/store`, unsigned.
pub async fn store_for_phone(State(gateway): State<Gateway>) -> Response {
    let Some(stores) = gateway.macs.stores.clone() else {
        return json(&StoreForPhone { store: None, client_id: None });
    };
    match blocking(move || Ok(StoreForPhone { store: his_own(stores.as_ref()), client_id: stores.client_id() })).await {
        Ok(answer) => json(&answer),
        Err(why) => fault(why),
    }
}

/// The signed request's sender, where `crate::vouched` proves it his. Off the async threads, as it asks the store.
async fn sender(gateway: &Gateway, headers: &HeaderMap, kind: DeviceKind, path: &'static str, body: &Bytes) -> Option<(Signed, adapter_traits::StorePeer)> {
    let signed = Signed::from(headers)?;
    // Each check asks his store, so a stranger's tries are counted as an unsigned pairing's are.
    if !gateway.pairing.attempt(gateway.pairing.now()) {
        return None;
    }
    let (gateway, body) = (gateway.clone(), body.clone());
    tokio::task::spawn_blocking(move || {
        let peer = vouched(&gateway, &signed, kind, "POST", path, &body)?;
        Some((signed, peer))
    })
    .await
    .ok()
    .flatten()
}

/// A paired Mac, linked at once with the key it sent.
fn linked_row(gateway: &Gateway, hello: &MacHello, address: &str, public_key: Vec<u8>) -> store::PairedMac {
    store::PairedMac {
        id: hello.id.clone(),
        name: hello.name.clone(),
        colour: hello.colour.name().to_string(),
        address: address.to_string(),
        public_key: Some(public_key),
        build: hello.build.clone(),
        paired_at: gateway.pairing.now(),
        state: store::MacState::Linked,
        back_code: None,
        last_heard_at: None,
        asleep_since: None,
        why: None,
        wire: hello.wire.clone(),
        built_at: hello.built_at.clone(),
        source: hello.source.clone(),
        pin: hello.pin.clone(),
    }
}

/// Keep `row` linked with this Mac's key toward it, and start its link.
async fn keep_linked(gateway: &Gateway, row: &store::PairedMac, key: SigningKey) -> Result<(), String> {
    blocking({
        let (gateway, row) = (gateway.clone(), row.clone());
        move || {
            gateway.macs.keys.keep(&row.id, kept_form(&key))?;
            gateway.macs.held.put(&row.id, key);
            gateway.pairing.store.lock().unwrap().keep_mac(&row).map_err(|why| why.to_string())
        }
    })
    .await?;
    gateway.macs.changed.notify_one();
    Ok(())
}

/// The other Mac's hello and key, read where its hello is sound and its key one this Mac can verify with.
fn their_half(body: &[u8], address: &str) -> Option<(MacHello, Vec<u8>)> {
    let said: MacVouched = ipc::decode("a Mac's pairing", body).ok()?;
    let public_key = from_hex(&said.public_key).filter(|spki| crate::signing::verifying_key(spki).is_some())?;
    let hello = MacHello { address: said.mac.address.clone().or_else(|| (!address.is_empty()).then(|| address.to_string())), ..said.mac };
    sane(&hello).then_some((hello, public_key))
}

/// `POST /pair/mac/vouched`, signed with the other Mac's store key: it is his, so it is linked now, and this Mac
/// answers its own hello and key, signed the same way.
pub async fn mac_vouched_here(State(gateway): State<Gateway>, headers: HeaderMap, body: Bytes) -> Response {
    let Some((signed, peer)) = sender(&gateway, &headers, DeviceKind::Mac, MAC_PATH, &body).await else {
        return not_vouched();
    };
    let Some((hello, public_key)) = their_half(&body, &peer.address) else {
        return said_why(StatusCode::BAD_REQUEST, "The other Mac's pairing could not be read. Update Armada on both.");
    };
    let me = match hello_now(&gateway).await {
        Ok(me) => me,
        Err(why) => return fault(why),
    };
    if hello.id == me.id {
        return said_why(StatusCode::BAD_REQUEST, "A Mac cannot pair with itself.");
    }
    let made = fresh_key().and_then(|key| Ok((spki_hex(&key)?, key)));
    let (spki, key) = match made {
        Ok(made) => made,
        Err(why) => return fault(why),
    };
    let answer = match encoded(&MacVouched { mac: me, public_key: spki }) {
        Ok(answer) => answer,
        Err(why) => return fault(why),
    };
    let signed_answer = blocking({
        let (gateway, store, answer) = (gateway.clone(), signed.store.clone(), answer.clone());
        move || signing(&gateway, &store, "POST", MAC_PATH, &answer)
    })
    .await;
    let signed_answer = match signed_answer {
        Ok(headers) => headers,
        Err(why) => return fault(why),
    };
    let row = linked_row(&gateway, &hello, hello.address.as_deref().unwrap_or_default(), public_key);
    if let Err(why) = keep_linked(&gateway, &row, key).await {
        return fault(why);
    }
    let mut response = (StatusCode::OK, [("content-type", "application/json")], answer).into_response();
    for (name, value) in signed_answer {
        if let (Ok(name), Ok(value)) = (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(&value)) {
            response.headers_mut().insert(name, value);
        }
    }
    response
}

/// `POST /admin/macs/pair/vouched { store, device }`: pair now with the Mac `device` his store lists. Both sides
/// are linked when it answers.
pub async fn pair_mac(State(gateway): State<Gateway>, Json(body): Json<PairVouchedMac>) -> Response {
    let Some(stores) = gateway.macs.stores.clone() else {
        return said_why(StatusCode::CONFLICT, "This Mac has joined no store. Pair with a link instead.");
    };
    let listed_now = blocking({
        let store = body.store.clone();
        move || match stores.joined().iter().any(|joined| joined.address == store) {
            true => stores.peers(&store),
            false => Err("This Mac has not joined that store.".to_string()),
        }
    })
    .await;
    let peers = match listed_now {
        Ok(peers) => peers,
        Err(why) => return said_why(StatusCode::BAD_GATEWAY, why),
    };
    let Some(peer) = peers.into_iter().find(|peer| peer.device == body.device && peer.kind == DeviceKind::Mac) else {
        return said_why(StatusCode::NOT_FOUND, "Your store does not list that Mac any more.");
    };
    if !reachable(&peer.address) {
        let why = format!("{} has not said where it is reached. Open Armada there, then press Pair again.", peer.machine);
        return said_why(StatusCode::CONFLICT, why);
    }
    let me = match hello_now(&gateway).await {
        Ok(me) => me,
        Err(why) => return fault(why),
    };
    let made = fresh_key().and_then(|key| Ok((spki_hex(&key)?, key)));
    let (spki, key) = match made {
        Ok(made) => made,
        Err(why) => return fault(why),
    };
    let sent = match encoded(&MacVouched { mac: me.clone(), public_key: spki }) {
        Ok(sent) => sent,
        Err(why) => return fault(why),
    };
    let headers = blocking({
        let (gateway, store, sent) = (gateway.clone(), body.store.clone(), sent.clone());
        move || signing(&gateway, &store, "POST", MAC_PATH, &sent)
    })
    .await;
    let headers = match headers {
        Ok(headers) => headers,
        Err(why) => return fault(why),
    };
    let mut request = gateway.macs.http.post(format!("{}{MAC_PATH}", peer.address)).header("content-type", "application/json").timeout(PATIENCE);
    for (name, value) in headers {
        request = request.header(name, value);
    }
    let name = peer.machine.clone();
    let answer = match request.body(sent).send().await {
        Ok(answer) => answer,
        Err(_) => return said_why(StatusCode::BAD_GATEWAY, format!("{name} did not answer. Check Armada is open there, then press Pair again.")),
    };
    let status = answer.status().as_u16();
    let answered = Signed::from(answer.headers());
    let Ok(said) = answer.bytes().await else {
        return said_why(StatusCode::BAD_GATEWAY, format!("{name} did not answer. Press Pair again."));
    };
    if status == 403 {
        return said_why(StatusCode::FORBIDDEN, format!("{name} could not find this Mac in your store. Pair with a link instead."));
    }
    if status != 200 {
        return said_why(StatusCode::BAD_GATEWAY, String::from_utf8_lossy(&said).trim().to_string());
    }
    // The answer is checked as the request was: signed by the device asked, on the store asked.
    let proven = match answered.filter(|signed| signed.device == peer.device && signed.store == body.store) {
        Some(signed) => {
            let (gateway, said) = (gateway.clone(), said.clone());
            tokio::task::spawn_blocking(move || vouched(&gateway, &signed, DeviceKind::Mac, "POST", MAC_PATH, &said).is_some()).await.unwrap_or(false)
        }
        None => false,
    };
    if !proven {
        return said_why(StatusCode::BAD_GATEWAY, format!("The Mac at {} did not prove it is {name}. Pair with a link instead.", peer.address));
    }
    let Some((hello, public_key)) = their_half(&said, &peer.address).filter(|(hello, _)| hello.id != me.id) else {
        return said_why(StatusCode::BAD_GATEWAY, "The other Mac answered in a way this Mac cannot read. Update Armada on both.");
    };
    let row = linked_row(&gateway, &hello, hello.address.as_deref().unwrap_or(&peer.address), public_key);
    match keep_linked(&gateway, &row, key).await {
        Ok(()) => json(&listed(row, &gateway.way_in)),
        Err(why) => fault(why),
    }
}

/// `POST /pair/vouched`, signed with a phone's store key: it is his, so it is a device now.
pub async fn phone_vouched_here(State(gateway): State<Gateway>, headers: HeaderMap, body: Bytes) -> Response {
    if sender(&gateway, &headers, DeviceKind::Phone, PHONE_PATH, &body).await.is_none() {
        return not_vouched();
    }
    let Ok(said) = ipc::decode::<PhoneVouched>("a phone's pairing", &body) else {
        return said_why(StatusCode::BAD_REQUEST, "The phone's pairing could not be read.");
    };
    let public_key = from_hex(&said.spki_hex).filter(|spki| crate::signing::verifying_key(spki).is_some());
    let name = said.name.trim();
    let (Some(public_key), false, true) = (public_key, name.is_empty(), name.len() <= 80) else {
        return said_why(StatusCode::BAD_REQUEST, "The phone's name or key could not be read.");
    };
    let id = match random_hex() {
        Ok(id) => id,
        Err(why) => return fault(why),
    };
    let device = store::Device { id: id.clone(), name: name.to_string(), public_key, created_at: gateway.pairing.now(), last_seen_at: None };
    match gateway.pairing.store.lock().unwrap().add_device(&device) {
        Ok(()) => json(&PhoneAdded { device_id: id }),
        Err(why) => fault(why),
    }
}

/// Put this Mac's presence to every store it joined: at once, whenever [`crate::Macs::stores_joined`] says it
/// joined one, and when its address changes, read each [`PRESENCE_EVERY`]. A store that did not take it is tried
/// again at the next. Runs until the Gateway stops.
pub async fn put_presence(gateway: Gateway) {
    let Some(stores) = gateway.macs.stores.clone() else { return };
    let mut put: HashMap<String, String> = HashMap::new();
    loop {
        let pairing = gateway.pairing.clone();
        let stores = stores.clone();
        let kept = put.clone();
        put = blocking(move || {
            let mut put = kept;
            // An address that cannot be read now leaves what each store holds: it may be Tailscale starting.
            let Ok(address) = pairing.address() else { return Ok(put) };
            for joined in stores.joined() {
                if put.get(&joined.address) != Some(&address) && stores.put_presence(&joined.address, &address).is_ok() {
                    put.insert(joined.address, address.clone());
                }
            }
            Ok(put)
        })
        .await
        .unwrap_or_default();
        tokio::select! {
            _ = tokio::time::sleep(PRESENCE_EVERY) => {}
            _ = gateway.macs.presence.notified() => {}
            _ = gateway.macs.closed() => return,
        }
    }
}
