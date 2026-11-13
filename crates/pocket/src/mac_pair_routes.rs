//! Pairing another Mac, and forgetting one. Home shows a link, work pastes it, home presses Confirm.
//!
//! | Step | Route | On |
//! |---|---|---|
//! | Home makes a link | `POST /admin/macs/pair/start` | home |
//! | Work claims it with its key and a back code | `POST /admin/macs/pair/claim`, then `POST /pair/mac` | work, then home |
//! | Home confirms and hands its own key back | `POST /admin/macs/pair/confirm`, then `POST /pair/mac/back` | home, then work |
//! | Either forgets the other | `POST /admin/macs/forget`, then `POST /api/mac/forget` | each |

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use ipc::{ClaimMacLink, MacNamed, MacPairLink};

use crate::mac_client::{encoded, hello_now, post_unsigned, signed_whole};
use crate::mac_keys::{fresh_key, from_kept, kept_form, spki_hex};
use crate::macs::{claimed, listed, read_link, said, sane, HeldClaim, MacBack, MacClaim, MacHello};
use crate::pairing::{from_hex, random_hex};
use crate::reads::json;
use crate::signing::MacId;
use crate::{fleet_client, Gateway};

fn said_why(status: StatusCode, why: impl Into<String>) -> Response {
    (status, why.into()).into_response()
}

fn fault(why: impl ToString) -> Response {
    said_why(StatusCode::INTERNAL_SERVER_ERROR, why.to_string())
}

/// Run blocking work, the store's or the Keychain's, off the async threads.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(work).await.map_err(|why| why.to_string())?
}

/// `POST /admin/macs/pair/start`: the link the other Mac pastes.
pub async fn start(State(gateway): State<Gateway>) -> Response {
    let pairing = gateway.pairing.clone();
    let made = blocking(move || Ok((pairing.address()?, random_hex()?))).await;
    let (address, code) = match made {
        Ok(made) => made,
        Err(why) => return said_why(StatusCode::SERVICE_UNAVAILABLE, why),
    };
    let now = gateway.pairing.now();
    gateway.macs.start(code.clone(), now);
    json(&MacPairLink { link: format!("{address}/pair/mac?code={code}"), expires_at: now + crate::pairing::CODE_TTL })
}

/// `POST /admin/macs/pair/claim { link }`: this Mac claims the other's link, and waits for Confirm there.
pub async fn claim(State(gateway): State<Gateway>, Json(body): Json<ClaimMacLink>) -> Response {
    let Some((address, code)) = read_link(&body.link) else {
        return said_why(
            StatusCode::BAD_REQUEST,
            "This is not a pairing link from another Mac. Copy it again from Settings, Macs on that Mac.",
        );
    };
    let me = match hello_now(&gateway).await {
        Ok(me) => me,
        Err(why) => return fault(why),
    };
    let Some(mine) = me.address.clone() else {
        let why = blocking({
            let pairing = gateway.pairing.clone();
            move || pairing.address()
        })
        .await
        .err()
        .unwrap_or_default();
        return said_why(
            StatusCode::SERVICE_UNAVAILABLE,
            format!("The other Mac needs an address to reach this one, and it has none. {why}"),
        );
    };
    if mine == address {
        return said_why(StatusCode::BAD_REQUEST, "This link is this Mac's own. Paste it on the other Mac.");
    }
    let key = match fresh_key() {
        Ok(key) => key,
        Err(why) => return fault(why),
    };
    let (spki_hex, back_code) = match spki_hex(&key).and_then(|spki| Ok((spki, random_hex()?))) {
        Ok(made) => made,
        Err(why) => return fault(why),
    };
    let claim = MacClaim { code, mac: me, spki_hex, back_code: back_code.clone() };
    let body = match encoded(&claim) {
        Ok(body) => body,
        Err(why) => return fault(why),
    };
    let came = match post_unsigned(&gateway, &address, "/pair/mac", body).await {
        Ok(came) => came,
        Err(why) => {
            return said_why(
                StatusCode::BAD_GATEWAY,
                format!("The Mac at {address} did not answer. Check Armada is open there, then paste the link again. {why}"),
            )
        }
    };
    if !came.ok() {
        return said_why(StatusCode::BAD_GATEWAY, came.sentence());
    }
    let Ok(home) = ipc::decode::<MacHello>("a Mac", &came.body) else {
        return said_why(StatusCode::BAD_GATEWAY, "The other Mac answered in a way this Mac cannot read. Update Armada on both.");
    };
    if !sane(&MacHello { address: Some(address.clone()), ..home.clone() }) {
        return said_why(StatusCode::BAD_GATEWAY, "The other Mac did not say who it is. Update Armada on both.");
    }
    let row = store::PairedMac {
        id: home.id.clone(),
        name: home.name.clone(),
        colour: home.colour.name().to_string(),
        address: address.clone(),
        public_key: None,
        build: home.build.clone(),
        paired_at: gateway.pairing.now(),
        state: store::MacState::ConfirmWaiting,
        back_code: Some(back_code),
        last_heard_at: None,
        asleep_since: None,
        why: None,
    };
    let kept = blocking({
        let (gateway, row) = (gateway.clone(), row.clone());
        move || {
            gateway.macs.keys.keep(&row.id, kept_form(&key))?;
            gateway.macs.held.put(&row.id, key);
            gateway.pairing.store.lock().unwrap().keep_mac(&row).map_err(|why| why.to_string())
        }
    })
    .await;
    match kept {
        Ok(()) => json(&listed(row)),
        Err(why) => fault(why),
    }
}

/// `POST /pair/mac`: another Mac claims a link this one made. Held until Confirm here.
pub async fn claimed_here(State(gateway): State<Gateway>, Json(body): Json<MacClaim>) -> Response {
    let now = gateway.pairing.now();
    if !gateway.pairing.attempt(now) {
        return said_why(StatusCode::TOO_MANY_REQUESTS, "Too many tries. Wait a minute, then try again.");
    }
    let public_key = from_hex(&body.spki_hex).filter(|spki| crate::signing::verifying_key(spki).is_some());
    let (Some(public_key), true) = (public_key, sane(&body.mac)) else {
        return said_why(StatusCode::BAD_REQUEST, "The other Mac's claim could not be read. Update Armada on both.");
    };
    let me = match hello_now(&gateway).await {
        Ok(me) => me,
        Err(why) => return fault(why),
    };
    if body.mac.id == me.id {
        return said_why(StatusCode::BAD_REQUEST, "This link is this Mac's own. Paste it on the other Mac.");
    }
    let claim = HeldClaim { mac: body.mac, public_key, back_code: body.back_code, at: now };
    match gateway.macs.claim(&body.code, claim) {
        true => (StatusCode::ACCEPTED, json(&me)).into_response(),
        false => said_why(
            StatusCode::FORBIDDEN,
            format!("This link is not good any more. Start pairing again on {}.", me.name),
        ),
    }
}

/// This Mac's key toward `mac`: the one made at an earlier Confirm press, so a press the other Mac
/// took but whose answer was lost hands the same key again.
fn key_toward(gateway: &Gateway, mac: &str) -> Result<p256::ecdsa::SigningKey, String> {
    if let Some(key) = gateway.macs.keys.read(mac)?.as_ref().and_then(from_kept) {
        return Ok(key);
    }
    let key = fresh_key()?;
    gateway.macs.keys.keep(mac, kept_form(&key))?;
    Ok(key)
}

/// `POST /admin/macs/pair/confirm { mac }`: the owner confirms a claim here, and this Mac hands its
/// own key back. Pressed again where the other Mac did not answer.
pub async fn confirm(State(gateway): State<Gateway>, Json(body): Json<MacNamed>) -> Response {
    let Some(claim) = gateway.macs.claim_of(&body.mac) else {
        return said_why(StatusCode::NOT_FOUND, "No Mac waits for Confirm here. Paste the link on the other Mac again.");
    };
    let name = claim.mac.name.clone();
    let address = claim.mac.address.clone().unwrap_or_default();
    let made = blocking({
        let (gateway, mac) = (gateway.clone(), body.mac.clone());
        move || {
            let key = key_toward(&gateway, &mac)?;
            Ok((crate::mac_client::hello(&gateway)?, spki_hex(&key)?, key))
        }
    })
    .await;
    let (me, spki_hex, key) = match made {
        Ok(made) => made,
        Err(why) => return fault(why),
    };
    let row = store::PairedMac {
        id: claim.mac.id.clone(),
        name: claim.mac.name.clone(),
        colour: claim.mac.colour.name().to_string(),
        address: address.clone(),
        public_key: Some(claim.public_key.clone()),
        build: claim.mac.build.clone(),
        paired_at: gateway.pairing.now(),
        state: store::MacState::Linked,
        back_code: None,
        last_heard_at: None,
        asleep_since: None,
        why: None,
    };
    // Kept before the other Mac is told, so its first call here, made the moment it links, is known.
    let kept = gateway.pairing.store.lock().unwrap().keep_mac(&row);
    if let Err(why) = kept {
        return fault(why);
    }
    let back = MacBack { back_code: claim.back_code.clone(), mac: me, spki_hex };
    let refused = match encoded(&back) {
        Err(why) => Some(fault(why)),
        Ok(sent) => match post_unsigned(&gateway, &address, "/pair/mac/back", sent).await {
            Ok(came) if came.ok() => None,
            Ok(came) => Some(said_why(StatusCode::BAD_GATEWAY, format!("{name} did not take the pairing. {}", came.sentence()))),
            Err(_) => Some(said_why(StatusCode::BAD_GATEWAY, format!("{name} did not answer. Press Confirm again."))),
        },
    };
    if let Some(refused) = refused {
        // The claim stays for the next press; the row waits for it.
        let _ = gateway.pairing.store.lock().unwrap().remove_mac(&row.id);
        return refused;
    }
    gateway.macs.held.put(&row.id, key);
    gateway.macs.drop_claim(&row.id);
    gateway.macs.changed.notify_one();
    json(&listed(row))
}

/// `POST /pair/mac/back`: the Mac this one claimed confirmed. Taken only with the code minted for it.
pub async fn back(State(gateway): State<Gateway>, Json(body): Json<MacBack>) -> Response {
    let now = gateway.pairing.now();
    if !gateway.pairing.attempt(now) {
        return said_why(StatusCode::TOO_MANY_REQUESTS, "Too many tries. Wait a minute, then try again.");
    }
    let refused = || said_why(StatusCode::FORBIDDEN, "This pairing is not waiting here any more. Start pairing again.");
    let public_key = from_hex(&body.spki_hex).filter(|spki| crate::signing::verifying_key(spki).is_some());
    let Some(public_key) = public_key else { return refused() };
    let found = gateway.pairing.store.lock().unwrap().paired_mac(&body.mac.id);
    let Ok(Some(row)) = found else { return refused() };
    let waiting = row.state == store::MacState::ConfirmWaiting && row.back_code.as_deref() == Some(body.back_code.as_str());
    // The same Confirm pressed again after its answer was lost: it is linked already.
    let again = row.state == store::MacState::Linked && row.public_key.as_deref() == Some(public_key.as_slice());
    if !waiting && !again {
        return refused();
    }
    let kept = gateway.pairing.store.lock().unwrap().link_mac(&row.id, &public_key, &said(&body.mac, &row.address));
    if let Err(why) = kept {
        return fault(why);
    }
    gateway.macs.changed.notify_one();
    StatusCode::NO_CONTENT.into_response()
}

/// Forget `mac` on this side: its row, this Mac's key toward it, and what this Fleet holds from it.
pub(crate) async fn forget_here(gateway: &Gateway, mac: &str) -> Result<bool, String> {
    let gone = blocking({
        let (gateway, mac) = (gateway.clone(), mac.to_string());
        move || {
            let gone = gateway.pairing.store.lock().unwrap().remove_mac(&mac).map_err(|why| why.to_string())?;
            gateway.macs.held.drop_key(&mac);
            gateway.macs.keys.forget(&mac)?;
            Ok(gone)
        }
    })
    .await?;
    gateway.macs.changed.notify_one();
    if let Ok(port) = (gateway.fleet)() {
        if let Ok(sent) = encoded(&MacNamed { mac: mac.to_string() }) {
            // Fleet drops the rows when it next hears; one that is down holds none worth keeping.
            let _ = fleet_client::send(port, "POST", "/other_macs/forget", Some(&sent)).await;
        }
    }
    Ok(gone)
}

/// `POST /admin/macs/forget { mac }`: tell the other Mac, as far as it answers, then forget it here.
pub async fn forget(State(gateway): State<Gateway>, Json(body): Json<MacNamed>) -> Response {
    if gateway.macs.drop_claim(&body.mac) {
        return StatusCode::NO_CONTENT.into_response();
    }
    let found = gateway.pairing.store.lock().unwrap().paired_mac(&body.mac);
    let row = match found {
        Ok(Some(row)) => row,
        Ok(None) => return said_why(StatusCode::NOT_FOUND, "No Mac is paired with that id."),
        Err(why) => return fault(why),
    };
    if row.state == store::MacState::Linked {
        // Best effort: an asleep Mac finds out at its next call, which answers 401.
        let _ = signed_whole(&gateway, &row.id, &row.address, "POST", "/api/mac/forget", None).await;
    }
    match forget_here(&gateway, &row.id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(why) => fault(why),
    }
}

/// `POST /api/mac/forget`, signed: the other Mac forgot this one, so this one forgets it.
pub async fn forgotten(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>) -> Response {
    match forget_here(&gateway, &mac).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(why) => fault(why),
    }
}

/// `GET /admin/macs`, past this Mac: each paired Mac, then each claim waiting here.
pub(crate) fn others(gateway: &Gateway) -> Result<Vec<ipc::Mac>, String> {
    let rows = gateway.pairing.store.lock().unwrap().paired_macs().map_err(|why| why.to_string())?;
    let mut all: Vec<ipc::Mac> = rows.into_iter().map(listed).collect();
    all.extend(gateway.macs.claims().into_iter().map(claimed));
    Ok(all)
}
