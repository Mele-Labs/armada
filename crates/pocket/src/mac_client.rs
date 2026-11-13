//! Calls from this Gateway to another Mac's: unsigned for the two pairing steps, signed for the rest,
//! exactly as `signing::signed` verifies them, with `x-pocket-device` this Mac's id.

use std::time::Duration;

use p256::ecdsa::SigningKey;
use serde::Serialize;

use crate::mac_keys::{from_kept, sign};
use crate::macs::MacHello;
use crate::Gateway;

/// How long one call that is not the live stream may take.
pub(crate) const PATIENCE: Duration = Duration::from_secs(15);

/// A call that went out and came back, with whatever status the other Mac answered.
pub(crate) struct Came {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Came {
    pub(crate) fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The other Gateway's sentence, as it said it.
    pub(crate) fn sentence(&self) -> String {
        String::from_utf8_lossy(&self.body).trim().to_string()
    }
}

pub(crate) fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    ipc::encode(value).map(String::into_bytes).map_err(|why| why.to_string())
}

/// An unsigned `POST` of `body` to `<address><path>`.
pub(crate) async fn post_unsigned(gateway: &Gateway, address: &str, path: &str, body: Vec<u8>) -> Result<Came, String> {
    let sent = gateway
        .macs
        .http
        .post(format!("{address}{path}"))
        .header("content-type", "application/json")
        .body(body)
        .timeout(PATIENCE)
        .send()
        .await
        .map_err(|why| why.to_string())?;
    came(sent).await
}

async fn came(response: reqwest::Response) -> Result<Came, String> {
    let status = response.status().as_u16();
    let body = response.bytes().await.map_err(|why| why.to_string())?.to_vec();
    Ok(Came { status, body })
}

/// This Mac as it tells another: its id, name, colour, address and build. Blocking.
pub(crate) fn hello(gateway: &Gateway) -> Result<MacHello, String> {
    let mac = crate::machine::this_mac(gateway)?;
    Ok(MacHello {
        colour: ipc::MacColour::named(&mac.colour).unwrap_or_default(),
        id: mac.id,
        name: mac.name,
        address: gateway.pairing.address().ok(),
        build: gateway.way_in.build.clone(),
    })
}

/// [`hello`], off the async threads.
pub(crate) async fn hello_now(gateway: &Gateway) -> Result<MacHello, String> {
    let gateway = gateway.clone();
    tokio::task::spawn_blocking(move || hello(&gateway)).await.map_err(|why| why.to_string())?
}

/// This Mac's id and its key toward `mac`, read from the Keychain once a run.
async fn signer(gateway: &Gateway, mac: &str) -> Result<(String, SigningKey), String> {
    let (gateway, mac) = (gateway.clone(), mac.to_string());
    tokio::task::spawn_blocking(move || {
        let me = crate::machine::this_mac(&gateway)?.id;
        if let Some(key) = gateway.macs.held.get(&mac) {
            return Ok((me, key));
        }
        let kept = gateway.macs.keys.read(&mac)?.ok_or_else(|| String::from("This Mac has no key for it. Forget it, then pair again."))?;
        let key = from_kept(&kept).ok_or_else(|| String::from("The key kept for it could not be read. Forget it, then pair again."))?;
        gateway.macs.held.put(&mac, key.clone());
        Ok((me, key))
    })
    .await
    .map_err(|why| why.to_string())?
}

/// A signed request to paired Mac `mac` at `address`. `patience` `None` waits as long as it streams.
pub(crate) async fn signed(
    gateway: &Gateway,
    mac: &str,
    address: &str,
    method: &str,
    target: &str,
    body: Option<Vec<u8>>,
    patience: Option<Duration>,
) -> Result<reqwest::Response, String> {
    let (me, key) = signer(gateway, mac).await?;
    let body = body.unwrap_or_default();
    let headers = sign(&key, &me, method, target, gateway.pairing.now(), &body)?;
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|why| why.to_string())?;
    let mut request = gateway.macs.http.request(method, format!("{address}{target}"));
    for (name, value) in headers {
        request = request.header(name, value);
    }
    if !body.is_empty() {
        request = request.header("content-type", "application/json");
    }
    if let Some(patience) = patience {
        request = request.timeout(patience);
    }
    request.body(body).send().await.map_err(|why| why.to_string())
}

/// [`signed`], read whole.
pub(crate) async fn signed_whole(
    gateway: &Gateway,
    mac: &str,
    address: &str,
    method: &str,
    target: &str,
    body: Option<Vec<u8>>,
) -> Result<Came, String> {
    came(signed(gateway, mac, address, method, target, body, Some(PATIENCE)).await?).await
}
