//! `GET /api/machine`: the name the phone labels this Mac's rows with when it
//! is paired with more than one. `GET /admin/macs`: this Mac and the Macs
//! paired with it, for the Devices list beside the phones.

use axum::extract::State;
use axum::response::Response;
use serde::Serialize;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use ipc::{Mac, MacColour};

use crate::reads::json;
use crate::Gateway;

/// Where neither its address nor the system names this Mac.
const UNNAMED: &str = "Mac";

#[derive(Serialize)]
struct Machine {
    name: String,
}

/// `mac-a` from `https://mac-a.tail1234.ts.net`: the name the owner gave the
/// Mac on his tailnet, without the tailnet.
pub(crate) fn short_name(address: &str) -> Option<String> {
    let host = address.strip_prefix("https://")?;
    let first = host.split(['.', ':', '/']).next()?;
    (!first.is_empty()).then(|| first.to_string())
}

fn host_name() -> Option<String> {
    let output = std::process::Command::new("hostname").arg("-s").output().ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !name.is_empty()).then_some(name)
}

pub async fn machine(State(gateway): State<Gateway>) -> Response {
    let pairing = gateway.pairing.clone();
    let name = tokio::task::spawn_blocking(move || {
        pairing
            .address()
            .ok()
            .and_then(|address| short_name(&address))
            .or_else(host_name)
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| UNNAMED.to_string());
    json(&Machine { name })
}

/// This Mac as it was first minted: its id, the host's name then, and one colour drawn at random, so two
/// Macs set up apart are unlikely to share one. Blocking.
pub(crate) fn this_mac(gateway: &Gateway) -> Result<store::ThisMac, String> {
    let random = crate::webpush::random::<16>()?;
    let colour = MacColour::ALL[usize::from(random[0]) % MacColour::ALL.len()];
    let fresh = store::ThisMac {
        id: crate::pairing::to_hex(&random),
        name: host_name().unwrap_or_else(|| UNNAMED.to_string()),
        colour: colour.name().to_string(),
        minted_at: gateway.pairing.now(),
    };
    gateway.pairing.store.lock().unwrap().this_mac_or(&fresh).map_err(|why| why.to_string())
}

fn macs(gateway: &Gateway) -> Result<Vec<Mac>, String> {
    let mac = this_mac(gateway)?;
    let mut all = vec![Mac {
        colour: MacColour::named(&mac.colour).unwrap_or(MacColour::Blue),
        id: mac.id,
        name: mac.name,
        address: gateway.pairing.address().ok(),
        this_mac: true,
        build: gateway.way_in.build.clone(),
        link: None,
        last_heard_at: None,
        why: None,
    }];
    all.extend(crate::mac_pair_routes::others(gateway)?);
    Ok(all)
}

/// `GET /admin/macs`: this Mac first, then each paired Mac, then each claim waiting for Confirm here.
pub async fn this_and_paired_macs(State(gateway): State<Gateway>) -> Response {
    match tokio::task::spawn_blocking(move || macs(&gateway)).await {
        Ok(Ok(macs)) => json(&macs),
        Ok(Err(why)) => (StatusCode::INTERNAL_SERVER_ERROR, why).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "The Gateway could not read this Mac.").into_response(),
    }
}
