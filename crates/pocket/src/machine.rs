//! `GET /api/machine`: the name the phone labels this Mac's rows with when it
//! is paired with more than one.

use axum::extract::State;
use axum::response::Response;
use serde::Serialize;

use crate::reads::json;
use crate::Gateway;

/// Where neither Tailscale nor the system names this Mac.
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
