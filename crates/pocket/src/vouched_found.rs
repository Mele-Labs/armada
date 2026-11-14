//! His Macs as his stores list them, beside those on his Cloudflare account and those Bonjour sees: **one row
//! per Mac**, marked `vouched` where a store this Mac joined lists it, so Pair pairs it at once.

use std::time::{Duration, Instant};

use adapter_traits::DeviceKind;
use futures_util::future::join_all;
use ipc::{FoundMac, VouchedMac};

use crate::macs::{reachable, MacGreeting};
use crate::Gateway;

/// How long the stores' lists are kept before they are asked again, as the account's is.
const FRESH: Duration = Duration::from_secs(4);
/// How long each Mac a store lists has to say who it is.
const GREETING_WAIT: Duration = Duration::from_secs(3);

/// The Macs his stores list with an address, each greeted there, and why a store could not be read. Read again
/// where the last read is older than [`FRESH`] or `again`.
pub(crate) async fn store_macs(gateway: &Gateway, again: bool) -> (Vec<VouchedMac>, Option<String>) {
    let Some(stores) = gateway.macs.stores.clone() else { return (Vec::new(), None) };
    {
        let kept = gateway.macs.found.lock().unwrap();
        if !again && kept.store_at.is_some_and(|at| at.elapsed() < FRESH) {
            return (kept.store_macs.clone(), kept.store_why.clone());
        }
    }
    let read = tokio::task::spawn_blocking(move || {
        let mut listed = Vec::new();
        let mut why = None;
        for joined in stores.joined() {
            match stores.peers(&joined.address) {
                Ok(peers) => listed.extend(peers.into_iter().map(|peer| (joined.address.clone(), peer))),
                Err(said) => why = Some(format!("Your store at {} could not be read: {said}", joined.address)),
            }
        }
        (listed, why)
    })
    .await;
    let (listed, why) = read.unwrap_or_else(|_| (Vec::new(), Some("The Gateway could not read your store.".to_string())));
    let mut macs: Vec<(String, adapter_traits::StorePeer)> = Vec::new();
    for (store, peer) in listed {
        // A Mac in two of his stores is one Mac, reached at one address.
        if peer.kind == DeviceKind::Mac && reachable(&peer.address) && !macs.iter().any(|(_, one)| one.address == peer.address) {
            macs.push((store, peer));
        }
    }
    let greetings = join_all(macs.iter().map(|(_, peer)| greet_at(gateway, &peer.address))).await;
    let rows: Vec<VouchedMac> = macs
        .into_iter()
        .zip(greetings)
        .map(|((store, peer), greeting)| {
            let online = greeting.is_some();
            let mac = FoundMac {
                tunnel: greeting.as_ref().and_then(|g| g.tunnel.clone()).unwrap_or_default(),
                name: greeting.as_ref().map_or_else(|| peer.machine.clone(), |g| g.name.clone()),
                colour: greeting.as_ref().map(|g| g.colour),
                address: Some(peer.address),
                online,
                build: greeting.and_then(|g| g.build),
                ..FoundMac::default()
            };
            VouchedMac { mac, vouched: true, store: Some(store), device: Some(peer.device) }
        })
        .collect();
    let mut kept = gateway.macs.found.lock().unwrap();
    kept.store_at = Some(Instant::now());
    kept.store_macs = rows.clone();
    kept.store_why = why.clone();
    (rows, why)
}

/// What the Mac at `address` says of itself, where it answers in time.
async fn greet_at(gateway: &Gateway, address: &str) -> Option<MacGreeting> {
    let answer = gateway.macs.http.get(format!("{address}/pair/mac/hello")).timeout(GREETING_WAIT).send().await.ok()?;
    if !answer.status().is_success() {
        return None;
    }
    let bytes = answer.bytes().await.ok()?;
    ipc::decode("a Mac's greeting", &bytes).ok()
}

/// The account's and nearby Macs, `found`, with the Macs his stores list: a store's Mac that is one of them, by
/// its tunnel or its address, marks that row; any other is a row of its own.
pub(crate) fn with_vouched(found: Vec<FoundMac>, theirs: Vec<VouchedMac>) -> Vec<VouchedMac> {
    let mut rows: Vec<VouchedMac> = found.into_iter().map(|mac| VouchedMac { mac, ..VouchedMac::default() }).collect();
    for one in theirs {
        let same = rows.iter_mut().find(|row| {
            (!one.mac.tunnel.is_empty() && row.mac.tunnel == one.mac.tunnel) || (row.mac.address.is_some() && row.mac.address == one.mac.address)
        });
        match same {
            Some(row) => {
                row.vouched = true;
                row.store = one.store;
                row.device = one.device;
                row.mac.online |= one.mac.online;
                if row.mac.address.is_none() {
                    row.mac.address = one.mac.address;
                }
            }
            None => rows.push(one),
        }
    }
    rows
}
