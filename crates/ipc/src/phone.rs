//! `get_phone` and `set_phone`: whether this Mac serves the phone app.
//! `docs/concepts/pocket.md`.

use serde::{Deserialize, Serialize};

/// The Phone Gateway Fleet runs on loopback, as Bridge's Settings, Phone shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneGateway {
    /// The owner opted this Mac in: the `pocket` marker is in the machine directory.
    pub on: bool,
    /// The loopback port it listens on, for `tailscale serve` to name.
    pub port: u16,
    /// It is listening now. `on` and not this is a Gateway that could not start.
    pub listening: bool,
    /// Why it is on and not listening, as a sentence. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// `set_phone`: turn the Gateway on or off, now and at every start after.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetPhoneGateway {
    pub on: bool,
}
