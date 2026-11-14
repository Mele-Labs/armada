//! Another of his Macs: pairing it from Settings, and what its Phone Gateway hands this Fleet once paired.
//! `docs/concepts/pocket.md`, *Another Mac*.
//!
//! What leaves the Mac that holds a Job has passed that Fleet's share filter first. What arrives is kept
//! by this Fleet with the sending Mac's id, and served beside its own.

use serde::{Deserialize, Serialize};

use crate::{BugReport, Event, Project, SessionRecord};

/// `POST /admin/macs/pair/start` on the Gateway: the one-time link the other Mac's Settings takes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacPairLink {
    pub link: String,
    /// Unix seconds. After this the link is refused, and pairing is started again.
    pub expires_at: i64,
}

/// `POST /admin/macs/pair/claim`: a link pasted from the other Mac. Answers the [`crate::Mac`] it
/// names, waiting on a Confirm there.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimMacLink {
    pub link: String,
}

/// `POST /admin/macs/pair/confirm` and `POST /admin/macs/forget`: the Mac pressed on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacNamed {
    pub mac: String,
}

/// `GET /admin/macs/found` on the Gateway: his other Macs on the Cloudflare account this Mac signed in with,
/// and those Bonjour sees on the same network.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundMacs {
    pub macs: Vec<FoundMac>,
    /// Why the account could not be read, as a sentence. The list is then what was read last, or empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// One of his other Macs, by the tunnel Armada names for it on his account, whether or not it made one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundMac {
    /// The tunnel's name on the account, `armada-<its id's first 12>`. What Pair names.
    pub tunnel: String,
    /// What it calls itself where it answered, else the name it was paired under, else its address's first label.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<crate::MacColour>,
    /// Its address under his domain, where the account routes one to it. Without one it is asked only while `nearby`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Cloudflare says its tunnel is connected, or Bonjour sees it nearby. Not online is a Mac asleep or with Armada closed.
    pub online: bool,
    /// It is paired with this Mac already, or a pairing with it waits on a Confirm.
    pub paired: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// Bonjour sees it on the same network now, so Pair reaches it there, with no tunnel. A Mac found
    /// only nearby has no `address`.
    #[serde(default)]
    pub nearby: bool,
    /// It runs a build older than this Mac's, on another wire, so Pair is off until it updates. False where
    /// it speaks this Mac's wire, or where nothing was heard from it to tell.
    #[serde(default)]
    pub older_build: bool,
}

/// `POST /admin/macs/pair/ask` on the Gateway: ask the Mac with this tunnel to pair, with no link.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AskMacPair {
    pub tunnel: String,
}

/// `PUT /other_macs/sessions` on Fleet: every Job row Mac `mac` shares, replacing all this Fleet held from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacSessions {
    pub mac: String,
    pub sessions: Vec<SessionRecord>,
}

/// `PUT /other_macs/bug_reports` on Fleet: every report Mac `mac` shares, replacing all this Fleet held from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacBugReports {
    pub mac: String,
    pub reports: Vec<BugReport>,
}

/// `PUT /other_macs/projects` on Fleet: every project Mac `mac` shares, with its tasks, replacing all this Fleet held from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacProjects {
    pub mac: String,
    pub projects: Vec<Project>,
}

/// `open_other_mac_page`: a page a Job on another Mac showed or serves, a walk or a dev server, to open on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenOtherMacPage {
    pub session_id: crate::SessionId,
    /// The page's address as the Job named it, on the loopback of the Mac that holds it.
    pub url: String,
}

/// What `open_other_mac_page` answers: where the page opens on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacPageOpened {
    pub url: String,
}

/// `POST /other_macs/event` on Fleet: one event that crossed from Mac `mac`, for this Fleet to keep and publish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OtherMacEvent {
    pub mac: String,
    pub event: Event,
}

/// `PUT /other_macs/claims` on Fleet: every other Mac waiting for his Confirm here, as this Mac's Phone Gateway
/// holds them, replacing all Fleet held. Fleet calls him with one `mac_pairing` call for each.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacClaimsWaiting {
    pub claims: Vec<MacClaimWaiting>,
}

/// One Mac waiting for his Confirm here: it pasted this Mac's link, or asked from its Settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacClaimWaiting {
    /// The asking Mac's id: what Confirm and Decline name.
    pub mac: String,
    pub name: String,
    /// What `GET /admin/macs` says of it, with the four digits where it asked.
    pub why: String,
    /// Unix seconds: when the claim arrived.
    pub at: i64,
}
