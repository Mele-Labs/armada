//! The account's other devices on a store, this device's presence there, and a request to one of them
//! signed exactly as a request to the store is. `docs/concepts/pocket.md`, *Through his store*.

use std::path::Path;

use adapter_traits::{DeviceKind, StoreError, StorePeer};
use serde::{Deserialize, Serialize};

use crate::store::{body_of, decoded, not_answered, succeeded, unix_now, Call, Reply};
use crate::store_device::{device_id, random, to_hex, DeviceKey};
use crate::store_signing;

#[derive(Deserialize)]
struct PeersReply {
    devices: Vec<PeerWire>,
}

#[derive(Deserialize)]
struct PeerWire {
    device: String,
    machine: String,
    kind: String,
    address: String,
    public_key: String,
}

#[derive(Serialize)]
struct PresenceBody<'a> {
    kind: &'static str,
    address: &'a str,
}

pub(crate) fn peers_call() -> Call {
    Call { method: "GET", target: "/v1/peers".to_string(), body: Vec::new() }
}

/// The peers in a `GET /v1/peers` reply, in the store's order. A device of a kind this build does not know is
/// left out rather than failing the list.
pub(crate) fn peers_held(reply: &Reply) -> Result<Vec<StorePeer>, StoreError> {
    if !succeeded(reply) {
        return Err(not_answered(reply));
    }
    let devices = decoded::<PeersReply>("peers", reply)?.devices;
    Ok(devices
        .into_iter()
        .filter_map(|one| {
            let kind = DeviceKind::from_wire(&one.kind)?;
            Some(StorePeer { device: one.device, machine: one.machine, kind, address: one.address, public_key: one.public_key })
        })
        .collect())
}

pub(crate) fn presence_call(kind: DeviceKind, address: &str) -> Result<Call, StoreError> {
    let body = body_of(&PresenceBody { kind: kind.as_wire(), address })?;
    Ok(Call { method: "PUT", target: "/v1/presence".to_string(), body })
}

/// The four `X-Armada-*` headers for a request from this device to another of the account's, signed with the
/// store key kept in `directory` as a request to that store is. Blocking: it reads the key's file.
pub fn signed_for_peer(directory: &Path, method: &str, target: &str, body: &[u8]) -> Result<[(&'static str, String); 4], String> {
    let device = device_id(directory)?.ok_or("This Mac has not joined that store.")?;
    let key = DeviceKey::open(directory)?;
    let nonce = to_hex(&random::<16>()?);
    Ok(store_signing::headers(&key, &device, method, target, unix_now(), &nonce, body))
}
