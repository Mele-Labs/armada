//! Another of the owner's Macs: what the two Gateways say to each other, the pairing codes and
//! claims this one holds in memory, and how each paired Mac shows on the Devices list.
//! `docs/concepts/pocket.md`, *Another Mac*.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ipc::{Mac, MacColour, MacLink};
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Notify};

use crate::mac_keys::{Held, MacKeys};
use crate::pairing::CODE_TTL;

/// How the link paces itself. Tests shorten it.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// How long `/api/mac/live` stays quiet before it sends a beat.
    pub beat: Duration,
    /// How long the link waits for anything, a beat included, before it calls the stream dead.
    pub silence: Duration,
    pub first_retry: Duration,
    pub longest_retry: Duration,
}

impl Default for Timing {
    fn default() -> Timing {
        Timing {
            beat: Duration::from_secs(15),
            silence: Duration::from_secs(45),
            first_retry: Duration::from_secs(1),
            longest_retry: Duration::from_secs(60),
        }
    }
}

/// Who a Mac is, as its own Gateway says: `GET /api/mac/hello`, and both halves of the pairing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacHello {
    pub id: String,
    pub name: String,
    pub colour: MacColour,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

/// `POST /pair/mac`: the Mac that pasted the link claims it.
#[derive(Debug, Serialize, Deserialize)]
pub struct MacClaim {
    pub code: String,
    pub mac: MacHello,
    /// The claiming Mac's public key, SPKI DER as hex. Its calls here are signed with the private half.
    pub spki_hex: String,
    /// The code the claiming Mac will take back once this one confirms.
    pub back_code: String,
}

/// `POST /pair/mac/back`: the Mac that showed the link confirmed, and hands its own key back.
#[derive(Debug, Serialize, Deserialize)]
pub struct MacBack {
    pub back_code: String,
    pub mac: MacHello,
    pub spki_hex: String,
}

/// A claim held until the owner presses Confirm here.
#[derive(Clone)]
pub(crate) struct HeldClaim {
    pub mac: MacHello,
    pub public_key: Vec<u8>,
    pub back_code: String,
    pub at: i64,
}

struct Code {
    expires: i64,
    claim: Option<HeldClaim>,
}

/// This Gateway's half of its links to other Macs.
#[derive(Clone)]
pub struct Macs {
    pub(crate) keys: Arc<dyn MacKeys>,
    pub(crate) held: Held,
    codes: Arc<Mutex<HashMap<String, Code>>>,
    /// Pairing or forgetting a Mac wakes the link, which starts or stops its task.
    pub(crate) changed: Arc<Notify>,
    closing: Arc<watch::Sender<bool>>,
    pub(crate) timing: Timing,
    pub(crate) http: reqwest::Client,
}

impl Macs {
    pub fn new(keys: Arc<dyn MacKeys>) -> Result<Macs, String> {
        Macs::with_timing(keys, Timing::default())
    }

    pub fn with_timing(keys: Arc<dyn MacKeys>, timing: Timing) -> Result<Macs, String> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|why| format!("Armada could not make its client for other Macs: {why}"))?;
        Ok(Macs {
            keys,
            held: Held::default(),
            codes: Arc::default(),
            changed: Arc::new(Notify::new()),
            closing: Arc::new(watch::channel(false).0),
            timing,
            http,
        })
    }

    /// The Gateway is stopping: every other Mac's open `/api/mac/live` ends now, so it sees the drop.
    pub fn close(&self) {
        self.closing.send_replace(true);
    }

    /// Resolves once [`Macs::close`] is called.
    pub(crate) fn closed(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
        let mut closing = self.closing.subscribe();
        async move {
            let _ = closing.wait_for(|closed| *closed).await;
        }
    }

    pub(crate) fn start(&self, code: String, now: i64) {
        let mut codes = self.codes.lock().unwrap();
        codes.retain(|_, c| c.claim.is_some() || c.expires > now);
        codes.insert(code, Code { expires: now + CODE_TTL, claim: None });
    }

    /// Hold `claim` against `code`, once. `false` where the code is unknown, spent or past its time.
    pub(crate) fn claim(&self, code: &str, claim: HeldClaim) -> bool {
        let mut codes = self.codes.lock().unwrap();
        match codes.get_mut(code) {
            Some(c) if c.expires > claim.at && c.claim.is_none() => {
                // A Mac claiming again replaces what it claimed before.
                codes.retain(|_, c| c.claim.as_ref().is_none_or(|held| held.mac.id != claim.mac.id));
                if let Some(c) = codes.get_mut(code) {
                    c.claim = Some(claim);
                }
                true
            }
            _ => false,
        }
    }

    /// Claims waiting for Confirm here, oldest first. A claim stays until it is confirmed or
    /// forgotten, so a Confirm the other Mac missed can be pressed again.
    pub(crate) fn claims(&self) -> Vec<HeldClaim> {
        let codes = self.codes.lock().unwrap();
        let mut all: Vec<HeldClaim> = codes.values().filter_map(|c| c.claim.clone()).collect();
        all.sort_by(|a, b| (a.at, &a.mac.id).cmp(&(b.at, &b.mac.id)));
        all
    }

    pub(crate) fn claim_of(&self, mac: &str) -> Option<HeldClaim> {
        self.claims().into_iter().find(|claim| claim.mac.id == mac)
    }

    /// Drop the claim of `mac`. `false` where none waited.
    pub(crate) fn drop_claim(&self, mac: &str) -> bool {
        let mut codes = self.codes.lock().unwrap();
        let before = codes.len();
        codes.retain(|_, c| c.claim.as_ref().is_none_or(|held| held.mac.id != mac));
        codes.len() != before
    }
}

/// `https://<host>`, or `http://` to a loopback host so two Gateways in a test can pair. One origin,
/// with no path, query or credentials.
pub(crate) fn reachable(address: &str) -> bool {
    let (plain, host) = match address.split_once("://") {
        Some(("https", host)) => (false, host),
        Some(("http", host)) => (true, host),
        _ => return false,
    };
    let fine = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']');
    if host.is_empty() || !host.chars().all(fine) {
        return false;
    }
    if !plain {
        return true;
    }
    let name = match host.rsplit_once(':') {
        Some((name, port)) if !name.ends_with(':') && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

/// The other Mac's address and the code, from `<address>/pair/mac?code=<code>`.
pub(crate) fn read_link(link: &str) -> Option<(String, String)> {
    let (address, code) = link.trim().split_once("/pair/mac?code=")?;
    let plain = !code.is_empty() && code.len() <= 64 && code.bytes().all(|b| b.is_ascii_hexdigit());
    (reachable(address) && plain).then(|| (address.to_string(), code.to_string()))
}

/// A Mac's id, name and address as one that pairs must give them.
pub(crate) fn sane(hello: &MacHello) -> bool {
    let id = !hello.id.is_empty() && hello.id.len() <= 64 && hello.id.bytes().all(|b| b.is_ascii_hexdigit());
    let name = !hello.name.trim().is_empty() && hello.name.len() <= 80;
    id && name && hello.address.as_deref().is_some_and(reachable)
}

/// A paired Mac on the Devices list.
pub(crate) fn listed(row: store::PairedMac) -> Mac {
    let (link, why) = match (row.state, row.asleep_since) {
        (store::MacState::ConfirmWaiting, _) => (MacLink::ConfirmWaiting, Some(format!("Confirm on {}", row.name))),
        (store::MacState::Linked, Some(since)) => (MacLink::Asleep { since }, row.why),
        (store::MacState::Linked, None) => (MacLink::Linked, None),
    };
    Mac {
        id: row.id,
        colour: MacColour::named(&row.colour).unwrap_or_default(),
        name: row.name,
        address: Some(row.address),
        this_mac: false,
        build: row.build,
        link: Some(link),
        last_heard_at: row.last_heard_at,
        why,
    }
}

/// A Mac whose claim waits for Confirm here.
pub(crate) fn claimed(claim: HeldClaim) -> Mac {
    Mac {
        id: claim.mac.id,
        why: Some(format!("Press Confirm to pair {} with this Mac", claim.mac.name)),
        name: claim.mac.name,
        colour: claim.mac.colour,
        address: claim.mac.address,
        this_mac: false,
        build: claim.mac.build,
        link: Some(MacLink::ClaimWaiting),
        last_heard_at: None,
    }
}

pub(crate) fn said(hello: &MacHello, address: &str) -> store::MacSaid {
    store::MacSaid {
        name: hello.name.clone(),
        colour: hello.colour.name().to_string(),
        address: hello.address.clone().unwrap_or_else(|| address.to_string()),
        build: hello.build.clone(),
    }
}

/// Keys in memory and the default pace, for a test that pairs no Mac.
#[cfg(test)]
pub(crate) fn quiet_macs() -> Macs {
    Macs::new(Arc::new(crate::MemoryMacKeys::default())).unwrap()
}
