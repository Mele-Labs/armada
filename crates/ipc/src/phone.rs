//! `get_phone` and `set_phone`: whether this Mac serves the phone app, and the Phone Gateway's own
//! `/admin/*` shapes Settings reads: the ways in, the tunnel, and the Devices list.
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

/// `GET /admin/setup` on the Phone Gateway: how far this Mac is along each way in, for Settings to draw one
/// row per step. Read afresh at each ask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewaySetup {
    pub tailscale: TailscaleState,
    pub served: Served,
    /// `pocket-address` names the address, so the owner brought a way in of his own, or the tunnel wrote it.
    pub named: bool,
    /// The address the phone and the other Mac open, where there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    pub cloudflare: CloudflareState,
    /// The version of the `cloudflared` Armada keeps, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloudflared: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TailscaleState {
    Missing,
    SignedOut,
    Running,
}

/// What stands in front of the Gateway through Tailscale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Served {
    /// Nothing: the phone cannot reach it this way.
    None,
    /// `tailscale serve`: reached only while Tailscale runs on the phone.
    Tailnet,
    /// Tailscale Funnel: any phone reaches it, with nothing installed on it.
    Funnel,
}

/// The Mac's Cloudflare tunnel, one step of the setup rows at a time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CloudflareState {
    /// Armada has not downloaded `cloudflared` yet. The setup press downloads it.
    Missing,
    /// Not signed in to Cloudflare on this Mac.
    SignedOut,
    /// Waiting for the owner to sign in and pick the domain, on this page.
    SigningIn { login: String },
    /// Signed in, with no tunnel yet. `why` is the last try's failure, in Cloudflare's or Armada's words.
    NoTunnel {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        why: Option<String>,
    },
    /// The tunnel is connected to Cloudflare and serves the Gateway at `address`.
    Running { address: String },
    /// The tunnel is made but not connected, since `since` (Unix seconds). Fleet keeps trying it.
    Down { address: String, since: i64, why: String },
}

/// `POST /admin/setup/cloudflare`, which takes no body, while it waits for the owner to sign in: the page to open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudflareSignIn {
    pub login: String,
}

/// A Mac on the Devices list: this one, or one paired with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mac {
    /// Minted once by its own Fleet.
    pub id: String,
    pub name: String,
    pub colour: MacColour,
    /// Its way in, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    pub this_mac: bool,
    /// The commit its Fleet was built from, where it says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// How the link to it stands. Absent on this Mac.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<MacLink>,
}

/// Each Mac's one colour, on its chip wherever its Jobs are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacColour {
    Blue,
    Teal,
    Green,
    Orange,
}

impl MacColour {
    pub const ALL: [MacColour; 4] = [MacColour::Blue, MacColour::Teal, MacColour::Green, MacColour::Orange];

    pub fn name(self) -> &'static str {
        match self {
            MacColour::Blue => "blue",
            MacColour::Teal => "teal",
            MacColour::Green => "green",
            MacColour::Orange => "orange",
        }
    }

    pub fn named(name: &str) -> Option<MacColour> {
        MacColour::ALL.into_iter().find(|colour| colour.name() == name)
    }
}

/// How the link to another Mac stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MacLink {
    Linked,
    /// Not answering since `since` (Unix seconds). What is sent to it waits here.
    Asleep { since: i64 },
    /// It runs an older build than this Mac, so the link is frozen until it updates.
    OlderBuild,
    NewerBuild,
    /// It claimed a pairing link and waits for Confirm on this Mac.
    ClaimWaiting,
}

/// A phone on the Devices list. `GET /admin/devices`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedPhone {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<i64>,
}
