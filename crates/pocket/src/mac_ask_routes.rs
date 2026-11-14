//! Pairing another Mac without a link, found on his Cloudflare account or nearby over Bonjour. Settings
//! lists his other Macs, he presses Pair on one, and both show the same four digits until he confirms on
//! the other.
//!
//! | Step | Route | On |
//! |---|---|---|
//! | His other Macs | `GET /admin/macs/found`, then `GET /pair/mac/hello` on each on the account | this, then each |
//! | This Mac asks, with its key, a back code and a commitment to its half | `POST /admin/macs/pair/ask`, then `POST /pair/mac/ask` | this, then other |
//! | This Mac reveals its half, and both draw the digits | `POST /pair/mac/reveal` | other |
//! | The other confirms or declines | `mac_pair_routes`: `confirm` and `forget`, as on a link | other, then this |
//!
//! A Mac seen nearby is asked at its Wi-Fi door (`door`), and the digits cover both doors' certificates,
//! so a matching Confirm proves nobody sits between the two.
//!
//! **Its address is public, so an ask is held on three counts:** the minute's share of unsigned tries, at most
//! [`crate::macs::ASKS_HELD`] waiting at once, and five minutes each. Nothing is paired until he confirms.

use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::future::join_all;
use ipc::{AskMacPair, FoundMac, FoundMacs, MacColour};

use crate::mac_client::{encoded, hello_now, post_with, Came};
use crate::mac_keys::{fresh_key, spki_hex};
use crate::mac_pair_routes::{claims_told, fault, keep_waiting, said_why};
use crate::machine::short_name;
use crate::macs::{asked, digits, hashed, reachable, sane, Asking, HeldClaim, MacAsk, MacAsked, MacGreeting, MacHello, MacReveal};
use crate::pairing::{from_hex, random_hex, CODE_TTL};
use crate::reads::json;
use crate::tunnel::{tunnel_name, AccountTunnel, TUNNEL_PREFIX};
use crate::Gateway;

/// How long the account's list is kept before Cloudflare is asked again.
const FRESH: Duration = Duration::from_secs(4);
/// How long each other Mac has to say who it is.
const GREETING_WAIT: Duration = Duration::from_secs(3);

/// `GET /pair/mac/hello`, unsigned: this Mac's name, colour and build, and nothing else.
pub async fn hello(State(gateway): State<Gateway>) -> Response {
    match hello_now(&gateway).await {
        Ok(me) => json(&MacGreeting { name: me.name, colour: me.colour, build: me.build }),
        Err(why) => fault(why),
    }
}

/// `GET /admin/macs/found`: the other Macs on the account this Mac signed in with, and those seen nearby.
pub async fn found(State(gateway): State<Gateway>) -> Response {
    json(&found_now(&gateway, false).await)
}

/// The account's other Macs, read again where the last read is older than [`FRESH`] or `again`, with each
/// Mac Bonjour sees. Whether each is paired is read here at each ask.
pub(crate) async fn found_now(gateway: &Gateway, again: bool) -> FoundMacs {
    let kept = {
        let kept = gateway.macs.found.lock().unwrap();
        let fresh = !again && kept.at.is_some_and(|at| at.elapsed() < FRESH);
        fresh.then(|| (kept.macs.clone(), kept.why.clone()))
    };
    let (macs, why) = match kept {
        Some(kept) => kept,
        None => read_account(gateway).await,
    };
    FoundMacs { macs: with_paired(gateway, with_nearby(gateway, macs)), why }
}

/// Mark each Mac on the account that Bonjour sees as nearby, and add each one seen that is not on it.
/// **One row per Mac**: its tunnel name is drawn from its id whether or not it made a tunnel.
fn with_nearby(gateway: &Gateway, mut macs: Vec<FoundMac>) -> Vec<FoundMac> {
    for near in gateway.macs.nearby.seen() {
        let tunnel = tunnel_name(&near.id);
        match macs.iter_mut().find(|mac| mac.tunnel == tunnel) {
            Some(mac) => {
                mac.nearby = true;
                mac.online = true;
            }
            None => macs.push(FoundMac {
                tunnel,
                name: near.name,
                colour: near.colour,
                address: None,
                online: true,
                paired: false,
                build: near.build,
                nearby: true,
            }),
        }
    }
    macs
}

/// Mark each Mac paired where a row or a claim here is its tunnel's.
fn with_paired(gateway: &Gateway, macs: Vec<FoundMac>) -> Vec<FoundMac> {
    let rows = gateway.pairing.store.lock().unwrap().paired_macs().unwrap_or_default();
    let claims = gateway.macs.claims(gateway.pairing.now());
    let ids: Vec<String> = rows.into_iter().map(|row| row.id).chain(claims.into_iter().map(|claim| claim.mac.id)).collect();
    macs.into_iter().map(|mac| FoundMac { paired: ids.iter().any(|id| tunnel_name(id) == mac.tunnel), ..mac }).collect()
}

async fn read_account(gateway: &Gateway) -> (Vec<FoundMac>, Option<String>) {
    let tunnel = gateway.way_in.tunnel.clone();
    let read = tokio::task::spawn_blocking(move || (tunnel.tunnels(), tunnel.own_name())).await;
    let (tunnels, own) = read.unwrap_or_else(|_| (Err("The Gateway could not read the Cloudflare account.".to_string()), None));
    let tunnels = match tunnels {
        Ok(tunnels) => tunnels,
        Err(why) => {
            let mut kept = gateway.macs.found.lock().unwrap();
            kept.at = Some(Instant::now());
            kept.why = Some(why.clone());
            return (kept.macs.clone(), Some(why));
        }
    };
    let others: Vec<AccountTunnel> =
        tunnels.into_iter().filter(|t| t.name.starts_with(TUNNEL_PREFIX) && own.as_deref() != Some(t.name.as_str())).collect();
    let greetings = join_all(others.iter().map(|t| greet(gateway, t))).await;
    let rows = gateway.pairing.store.lock().unwrap().paired_macs().unwrap_or_default();
    let before = gateway.macs.found.lock().unwrap().macs.clone();
    let macs: Vec<FoundMac> = others
        .into_iter()
        .zip(greetings)
        .map(|(t, greeting)| {
            let row = rows.iter().find(|row| tunnel_name(&row.id) == t.name);
            let before = before.iter().find(|mac| mac.tunnel == t.name);
            let name = greeting
                .as_ref()
                .map(|g| g.name.clone())
                .or_else(|| row.map(|row| row.name.clone()))
                .or_else(|| before.map(|mac| mac.name.clone()))
                .or_else(|| t.address.as_deref().and_then(short_name))
                .unwrap_or_else(|| t.name.clone());
            let colour = greeting
                .as_ref()
                .map(|g| g.colour)
                .or_else(|| row.and_then(|row| MacColour::named(&row.colour)))
                .or_else(|| before.and_then(|mac| mac.colour));
            let build = greeting
                .and_then(|g| g.build)
                .or_else(|| row.and_then(|row| row.build.clone()))
                .or_else(|| before.and_then(|mac| mac.build.clone()));
            FoundMac { tunnel: t.name, name, colour, address: t.address, online: t.connected, paired: false, build, nearby: false }
        })
        .collect();
    let mut kept = gateway.macs.found.lock().unwrap();
    kept.at = Some(Instant::now());
    kept.macs = macs.clone();
    kept.why = None;
    (macs, None)
}

/// What a connected Mac says of itself, where it answers in time.
async fn greet(gateway: &Gateway, tunnel: &AccountTunnel) -> Option<MacGreeting> {
    let address = tunnel.address.as_deref().filter(|address| tunnel.connected && reachable(address))?;
    let answer = gateway.macs.http.get(format!("{address}/pair/mac/hello")).timeout(GREETING_WAIT).send().await.ok()?;
    if !answer.status().is_success() {
        return None;
    }
    let bytes = answer.bytes().await.ok()?;
    ipc::decode("a Mac's greeting", &bytes).ok()
}

/// `POST /admin/macs/pair/ask { tunnel }`: ask the Mac with that tunnel name to pair, at its Wi-Fi door
/// while Bonjour sees it, else at its tunnel. Answers it waiting for Confirm there, with the four digits
/// both show.
pub async fn ask(State(gateway): State<Gateway>, Json(body): Json<AskMacPair>) -> Response {
    let mut found = found_now(&gateway, false).await;
    if !found.macs.iter().any(|mac| mac.tunnel == body.tunnel) {
        found = found_now(&gateway, true).await;
    }
    let Some(other) = found.macs.into_iter().find(|mac| mac.tunnel == body.tunnel) else {
        let why = found.why.unwrap_or_else(|| "That Mac is not on your Cloudflare account or on this network any more.".to_string());
        return said_why(StatusCode::NOT_FOUND, why);
    };
    let linked = gateway
        .pairing
        .store
        .lock()
        .unwrap()
        .paired_macs()
        .unwrap_or_default()
        .into_iter()
        .any(|row| row.state == store::MacState::Linked && tunnel_name(&row.id) == other.tunnel);
    if linked {
        return said_why(StatusCode::CONFLICT, format!("{} is paired with this Mac already.", other.name));
    }
    let near = gateway.macs.nearby.by_tunnel(&other.tunnel);
    let (address, client, seen) = match (near, other.address.clone().filter(|address| reachable(address))) {
        (Some(near), _) => match crate::door::client(None) {
            Ok((client, seen)) => (near.address, client, Some(seen)),
            Err(why) => return fault(why),
        },
        (None, Some(address)) => (address, gateway.macs.http.clone(), None),
        (None, None) => {
            let why = format!("{} is not on this network and has no address on your domain. Pair with a link instead.", other.name);
            return said_why(StatusCode::CONFLICT, why);
        }
    };
    // The other Mac answers back at this Mac's door, or at its address when the ask went by tunnel.
    let me = match hello_now(&gateway).await {
        Ok(me) if seen.is_some() && me.pin.is_some() => me,
        Ok(me) if seen.is_none() && me.address.is_some() => me,
        Ok(_) => return said_why(StatusCode::SERVICE_UNAVAILABLE, "This Mac has no address yet. Press Set up first."),
        Err(why) => return fault(why),
    };
    let key = match fresh_key() {
        Ok(key) => key,
        Err(why) => return fault(why),
    };
    let (spki_hex, back_code, reveal) = match spki_hex(&key).and_then(|spki| Ok((spki, random_hex()?, random_hex()?))) {
        Ok(made) => made,
        Err(why) => return fault(why),
    };
    let my_pin = me.pin.clone();
    let ask = MacAsk { mac: me.clone(), spki_hex: spki_hex.clone(), back_code: back_code.clone(), commit: hashed(&reveal) };
    let sent = match encoded(&ask) {
        Ok(sent) => sent,
        Err(why) => return fault(why),
    };
    let came = match post_with(&client, &address, "/pair/mac/ask", sent).await {
        Ok(came) => came,
        Err(_) => {
            let why = format!("{} did not answer. Check Armada is open there, then press Pair again.", other.name);
            return said_why(StatusCode::BAD_GATEWAY, why);
        }
    };
    // At a door the client always notes the certificate it met; none means the answer did not come over it.
    let met = match seen.as_ref().map(|seen| seen.lock().unwrap().clone()) {
        Some(None) => return said_why(StatusCode::BAD_GATEWAY, format!("{} did not show a certificate. Press Pair again.", other.name)),
        Some(Some(met)) => Some(met),
        None => None,
    };
    let (home, nonce) = match answered(&came, &address, &other, met.as_deref()) {
        Ok(answered) => answered,
        Err(refused) => return refused,
    };
    let sent = match encoded(&MacReveal { mac: me.id.clone(), reveal: reveal.clone() }) {
        Ok(sent) => sent,
        Err(why) => return fault(why),
    };
    match post_with(&client, &address, "/pair/mac/reveal", sent).await {
        Ok(came) if came.ok() => {}
        Ok(came) => return said_why(StatusCode::BAD_GATEWAY, came.sentence()),
        Err(_) => {
            let why = format!("{} did not answer. Check Armada is open there, then press Pair again.", other.name);
            return said_why(StatusCode::BAD_GATEWAY, why);
        }
    }
    let code = digits(&spki_hex, my_pin.as_deref(), home.pin.as_deref(), &reveal, &nonce);
    // Its tunnel, where it has one: the door's address is Bonjour's to say each time.
    let kept_address = home.address.clone().unwrap_or_default();
    let row = match keep_waiting(&gateway, &home, &kept_address, key, back_code).await {
        Ok(row) => row,
        Err(why) => return fault(why),
    };
    let expires = gateway.pairing.now() + CODE_TTL;
    gateway.macs.asking.lock().unwrap().insert(row.id.clone(), Asking { digits: code.clone(), expires });
    json(&asked(row, code, &gateway.way_in))
}

/// Who answered the ask and its half of the code, where it is the Mac with the tunnel name that was asked.
/// At a door, `met` is the certificate the door showed, and the Mac must name the same.
fn answered(came: &Came, address: &str, other: &FoundMac, met: Option<&str>) -> Result<(MacHello, String), Response> {
    if came.status == 429 {
        return Err(said_why(StatusCode::TOO_MANY_REQUESTS, came.sentence()));
    }
    if !came.ok() {
        return Err(said_why(StatusCode::BAD_GATEWAY, came.sentence()));
    }
    let unreadable = || said_why(StatusCode::BAD_GATEWAY, "The other Mac answered in a way this Mac cannot read. Update Armada on both.");
    let asked: MacAsked = ipc::decode("a Mac's answer", &came.body).map_err(|_| unreadable())?;
    let home = match met {
        Some(_) => asked.mac,
        None => MacHello { address: Some(address.to_string()), ..asked.mac },
    };
    let plain = !asked.nonce.is_empty() && asked.nonce.len() <= 64 && asked.nonce.bytes().all(|b| b.is_ascii_hexdigit());
    if !sane(&home) || !plain {
        return Err(unreadable());
    }
    if met.is_some() && home.pin.as_deref() != met {
        return Err(said_why(StatusCode::BAD_GATEWAY, format!("The Mac at {address} did not show its own certificate. Press Pair again.")));
    }
    if tunnel_name(&home.id) != other.tunnel {
        let why = format!("The Mac at {address} is not the one on {}'s tunnel. Pair with a link instead.", other.name);
        return Err(said_why(StatusCode::BAD_GATEWAY, why));
    }
    Ok((home, asked.nonce))
}

/// `POST /pair/mac/ask`, unsigned: another Mac asks this one to pair. Held, unlisted, until it reveals its
/// half; then for his Confirm here.
pub async fn asked_here(State(gateway): State<Gateway>, Json(body): Json<MacAsk>) -> Response {
    let now = gateway.pairing.now();
    if !gateway.pairing.attempt(now) {
        return said_why(StatusCode::TOO_MANY_REQUESTS, "Too many tries. Wait a minute, then try again.");
    }
    let public_key = from_hex(&body.spki_hex).filter(|spki| crate::signing::verifying_key(spki).is_some());
    let committed = body.commit.len() == 64 && body.commit.bytes().all(|b| b.is_ascii_hexdigit());
    let (Some(public_key), true, true) = (public_key, sane(&body.mac), committed) else {
        return said_why(StatusCode::BAD_REQUEST, "The other Mac's ask could not be read. Update Armada on both.");
    };
    let me = match hello_now(&gateway).await {
        Ok(me) => me,
        Err(why) => return fault(why),
    };
    if body.mac.id == me.id {
        return said_why(StatusCode::BAD_REQUEST, "A Mac cannot pair with itself.");
    }
    let nonce = match random_hex() {
        Ok(nonce) => nonce,
        Err(why) => return fault(why),
    };
    let claim = HeldClaim {
        mac: body.mac,
        public_key,
        back_code: body.back_code,
        at: now,
        digits: None,
        commit: Some(body.commit),
        spki_hex: Some(body.spki_hex),
    };
    if !gateway.macs.hold_ask(nonce.clone(), claim) {
        return said_why(
            StatusCode::TOO_MANY_REQUESTS,
            format!("Other Macs are already waiting to pair with {}. Wait a few minutes, then press Pair again.", me.name),
        );
    }
    tokio::spawn(told_when_past(gateway.clone()));
    (StatusCode::ACCEPTED, json(&MacAsked { mac: me, nonce })).into_response()
}

/// `POST /pair/mac/reveal`, unsigned: the asking Mac's half, now this Mac has answered with its own. The
/// digits are drawn and the ask is listed for his Confirm. **This Mac's door is the one the asking Mac
/// should have met**: a Mac between them that showed another makes the two sides draw different digits.
pub async fn revealed_here(State(gateway): State<Gateway>, Json(body): Json<MacReveal>) -> Response {
    let now = gateway.pairing.now();
    if !gateway.pairing.attempt(now) {
        return said_why(StatusCode::TOO_MANY_REQUESTS, "Too many tries. Wait a minute, then try again.");
    }
    let mine = gateway.macs.nearby.own_pin();
    let drawn = gateway.macs.reveal(&body.mac, &body.reveal, now, |claim, nonce| {
        let spki_hex = claim.spki_hex.clone().unwrap_or_default();
        digits(&spki_hex, claim.mac.pin.as_deref(), mine.as_deref(), &body.reveal, nonce)
    });
    if !drawn {
        return said_why(StatusCode::FORBIDDEN, "This ask is not waiting here any more. Press Pair again.");
    }
    claims_told(&gateway).await;
    StatusCode::NO_CONTENT.into_response()
}

/// An ask's time passed: Fleet is handed the claims again, without it. Nothing where the Gateway stopped first.
async fn told_when_past(gateway: Gateway) {
    let past = tokio::time::sleep(Duration::from_secs(CODE_TTL.unsigned_abs() + 1));
    tokio::select! {
        _ = past => claims_told(&gateway).await,
        _ = gateway.macs.closed() => {}
    }
}
