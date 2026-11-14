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
    /// The protocol ID its build speaks. A build from before Macs compared them sends none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
    /// `main` or `preview`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The pin of its Wi-Fi door's certificate, where the door is open. `crate::door`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
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

/// `POST /pair/mac/ask`: another of his Macs asks this one to pair, with no link. What a claim
/// carries, since the owner's Confirm here answers it the same way, and the hash of the asking Mac's half
/// of the code, which it reveals only once this Mac has drawn its own.
#[derive(Debug, Serialize, Deserialize)]
pub struct MacAsk {
    pub mac: MacHello,
    pub spki_hex: String,
    pub back_code: String,
    /// [`hashed`] of the half [`MacReveal`] brings.
    pub commit: String,
}

/// `POST /pair/mac/reveal`: the asking Mac's half of the code, now this Mac has answered with its own.
#[derive(Debug, Serialize, Deserialize)]
pub struct MacReveal {
    /// The asking Mac.
    pub mac: String,
    pub reveal: String,
}

/// The answer to [`MacAsk`]: who this Mac is, and the random half of the code both Macs show.
#[derive(Debug, Serialize, Deserialize)]
pub struct MacAsked {
    pub mac: MacHello,
    pub nonce: String,
}

/// `POST /pair/mac/declined`: the Mac that was asked declined, so the asking one drops its wait.
#[derive(Debug, Serialize, Deserialize)]
pub struct MacDeclined {
    /// The Mac that declined.
    pub mac: String,
    pub back_code: String,
}

/// `GET /pair/mac/hello`, unsigned: what any caller may know of this Mac, for the list of Macs on his
/// account. Nothing that lets it act here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacGreeting {
    pub name: String,
    pub colour: MacColour,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

/// The four digits both Macs show for an ask: from the asking Mac's key, each Mac's door certificate as
/// the asking Mac met them, and a random half from each. **The asking Mac commits to its half before it
/// sees the other's**, so nobody between them can try halves until the digits match.
pub(crate) fn digits(spki_hex: &str, asking_pin: Option<&str>, asked_pin: Option<&str>, reveal: &str, nonce: &str) -> String {
    use sha2::{Digest, Sha256};
    let (asking_pin, asked_pin) = (asking_pin.unwrap_or_default(), asked_pin.unwrap_or_default());
    let hash = Sha256::digest(format!("{spki_hex}:{asking_pin}:{asked_pin}:{reveal}:{nonce}").as_bytes());
    format!("{:04}", u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) % 10_000)
}

/// What an ask commits to: the SHA-256 of the asking Mac's half, as hex.
pub(crate) fn hashed(reveal: &str) -> String {
    use sha2::{Digest, Sha256};
    crate::pairing::to_hex(&Sha256::digest(reveal.as_bytes()))
}

/// How many asks from other Macs wait here at once. His address is public, so a stranger can fill no more.
pub(crate) const ASKS_HELD: usize = 3;

/// A claim held until the owner presses Confirm here.
#[derive(Clone)]
pub(crate) struct HeldClaim {
    pub mac: MacHello,
    pub public_key: Vec<u8>,
    pub back_code: String,
    pub at: i64,
    /// The four digits, where the claim is an ask from Settings rather than a pasted link, once the
    /// asking Mac revealed its half.
    pub digits: Option<String>,
    /// What an ask committed to. `None` on a pasted link.
    pub commit: Option<String>,
    /// The asking Mac's key as it sent it, for the digits. `None` on a pasted link.
    pub spki_hex: Option<String>,
}

impl HeldClaim {
    /// An ask from Settings, rather than a pasted link.
    pub(crate) fn asked(&self) -> bool {
        self.commit.is_some()
    }

    /// Listed, and Confirm takes it: a link's at once, an ask's once its digits are drawn.
    fn shown(&self) -> bool {
        !self.asked() || self.digits.is_some()
    }
}

struct Code {
    expires: i64,
    claim: Option<HeldClaim>,
}

impl Code {
    /// An ask is gone once its time is past, claimed or not. A claim on a link waits for Confirm.
    fn live(&self, now: i64) -> bool {
        match &self.claim {
            Some(claim) if !claim.asked() => true,
            _ => self.expires > now,
        }
    }
}

/// An ask this Mac sent, waiting for Confirm on the other.
#[derive(Clone)]
pub(crate) struct Asking {
    pub digits: String,
    pub expires: i64,
}

/// The account's other Macs as last read, so Settings' polling does not ask Cloudflare at each look.
#[derive(Default)]
pub(crate) struct FoundKept {
    pub at: Option<std::time::Instant>,
    pub macs: Vec<ipc::FoundMac>,
    pub why: Option<String>,
}

/// This Gateway's half of its links to other Macs.
#[derive(Clone)]
pub struct Macs {
    pub(crate) keys: Arc<dyn MacKeys>,
    pub(crate) held: Held,
    codes: Arc<Mutex<HashMap<String, Code>>>,
    /// The asks this Mac sent, by the asked Mac's id.
    pub(crate) asking: Arc<Mutex<HashMap<String, Asking>>>,
    pub(crate) found: Arc<Mutex<FoundKept>>,
    /// Pairing or forgetting a Mac wakes the link, which starts or stops its task.
    pub(crate) changed: Arc<Notify>,
    /// Held while the claims are read and handed to Fleet, so the last list Fleet takes is the last one read.
    pub(crate) telling: Arc<tokio::sync::Mutex<()>>,
    closing: Arc<watch::Sender<bool>>,
    pub(crate) timing: Timing,
    pub(crate) http: reqwest::Client,
    /// This Mac's Wi-Fi door and the Macs Bonjour sees. `crate::nearby`.
    pub(crate) nearby: crate::nearby::Nearby,
    /// A client to each paired Mac's door, by its pin.
    pinned: Arc<Mutex<HashMap<String, reqwest::Client>>>,
    /// The loopback ports opened for other Macs' pages. `crate::mac_pages`.
    pub(crate) pages: crate::mac_pages::Opened,
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
            asking: Arc::default(),
            found: Arc::default(),
            changed: Arc::new(Notify::new()),
            telling: Arc::default(),
            closing: Arc::new(watch::channel(false).0),
            timing,
            http,
            nearby: Default::default(),
            pinned: Arc::default(),
            pages: Default::default(),
        })
    }

    /// The client that reaches a door only where its certificate is `pin`.
    pub(crate) fn pinned(&self, pin: &str) -> Result<reqwest::Client, String> {
        let mut pinned = self.pinned.lock().unwrap();
        if let Some(client) = pinned.get(pin) {
            return Ok(client.clone());
        }
        let (client, _) = crate::door::client(Some(pin))?;
        pinned.insert(pin.to_string(), client.clone());
        Ok(client)
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
        codes.retain(|_, c| c.live(now));
        codes.insert(code, Code { expires: now + CODE_TTL, claim: None });
    }

    /// Hold an ask from another Mac against `nonce` until it is confirmed or its time passes. A Mac asking
    /// again replaces its own ask. `false` where [`ASKS_HELD`] others already wait.
    pub(crate) fn hold_ask(&self, nonce: String, claim: HeldClaim) -> bool {
        let mut codes = self.codes.lock().unwrap();
        let now = claim.at;
        codes.retain(|_, c| c.live(now) && c.claim.as_ref().is_none_or(|held| held.mac.id != claim.mac.id));
        let waiting = codes.values().filter(|c| c.claim.as_ref().is_some_and(HeldClaim::asked)).count();
        if waiting >= ASKS_HELD {
            return false;
        }
        codes.insert(nonce, Code { expires: now + CODE_TTL, claim: Some(claim) });
        true
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
    pub(crate) fn claims(&self, now: i64) -> Vec<HeldClaim> {
        let codes = self.codes.lock().unwrap();
        let mut all: Vec<HeldClaim> =
            codes.values().filter(|c| c.live(now)).filter_map(|c| c.claim.clone()).filter(HeldClaim::shown).collect();
        all.sort_by(|a, b| (a.at, &a.mac.id).cmp(&(b.at, &b.mac.id)));
        all
    }

    /// Draw the digits of `mac`'s ask, held against its nonce, where `reveal` is what it committed to.
    /// `false` where no such ask waits for its half, or the half is not the one committed to.
    pub(crate) fn reveal(&self, mac: &str, reveal: &str, now: i64, draw: impl FnOnce(&HeldClaim, &str) -> String) -> bool {
        let mut codes = self.codes.lock().unwrap();
        let waiting = codes.iter_mut().find(|(_, c)| {
            c.live(now) && c.claim.as_ref().is_some_and(|held| held.mac.id == mac && held.asked() && held.digits.is_none())
        });
        let Some((nonce, code)) = waiting else { return false };
        let Some(claim) = code.claim.as_mut() else { return false };
        if claim.commit.as_deref() != Some(hashed(reveal).as_str()) {
            return false;
        }
        claim.digits = Some(draw(claim, nonce));
        true
    }

    /// Asks held, revealed or not.
    #[cfg(test)]
    pub(crate) fn asks_held(&self, now: i64) -> usize {
        let codes = self.codes.lock().unwrap();
        codes.values().filter(|c| c.live(now) && c.claim.as_ref().is_some_and(HeldClaim::asked)).count()
    }

    pub(crate) fn claim_of(&self, mac: &str, now: i64) -> Option<HeldClaim> {
        self.claims(now).into_iter().find(|claim| claim.mac.id == mac)
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

/// A Mac's id, name and a way to reach it as one that pairs must give them: an address, or the pin of
/// a door, which Bonjour says where to find. An address it gives must be one this Mac would call.
pub(crate) fn sane(hello: &MacHello) -> bool {
    let id = !hello.id.is_empty() && hello.id.len() <= 64 && hello.id.bytes().all(|b| b.is_ascii_hexdigit());
    let name = !hello.name.trim().is_empty() && hello.name.len() <= 80;
    let pin = hello.pin.as_deref().map(crate::door::a_pin);
    let way = match hello.address.as_deref() {
        Some(address) => reachable(address) && pin != Some(false),
        None => pin == Some(true),
    };
    id && name && way
}

/// Whether linked Mac `row` runs a build whose wire this one's cannot read, and which of the two is older.
/// **A Mac whose hello named no wire runs a build from before Macs compared them**, so it is the older.
/// Builds that speak the same wire stay linked, whatever their commits.
pub(crate) fn frozen(me: &crate::WayIn, row: &store::PairedMac) -> Option<MacLink> {
    if row.state != store::MacState::Linked || row.wire.as_deref() == Some(me.wire.as_str()) {
        return None;
    }
    let older = match (row.wire.as_ref(), row.built_at.as_deref(), me.built_at.as_deref()) {
        (None, _, _) | (_, None, _) => true,
        (_, Some(_), None) => false,
        (_, Some(theirs), Some(mine)) => theirs <= mine,
    };
    Some(if older { MacLink::OlderBuild } else { MacLink::NewerBuild })
}

/// The sentence a frozen link carries as `why`.
pub(crate) fn frozen_why(link: &MacLink, name: &str) -> String {
    match link {
        MacLink::NewerBuild => format!("{name} runs a newer build. Nothing crosses until this Mac updates."),
        _ => format!("{name} runs an older build. Nothing crosses until it updates."),
    }
}

/// A paired Mac on the Devices list. **A frozen link says so over asleep**: it is why nothing crosses.
pub(crate) fn listed(row: store::PairedMac, me: &crate::WayIn) -> Mac {
    let (link, why) = match (row.state, row.asleep_since, frozen(me, &row)) {
        (store::MacState::ConfirmWaiting, _, _) => (MacLink::ConfirmWaiting, Some(format!("Confirm on {}", row.name))),
        (store::MacState::Linked, _, Some(link)) => {
            let why = frozen_why(&link, &row.name);
            (link, Some(why))
        }
        (store::MacState::Linked, Some(since), None) => (MacLink::Asleep { since }, row.why),
        (store::MacState::Linked, None, None) => (MacLink::Linked, None),
    };
    Mac {
        id: row.id,
        colour: MacColour::named(&row.colour).unwrap_or_default(),
        name: row.name,
        // A Mac paired nearby with no tunnel has no address: Bonjour says where its door is.
        address: (!row.address.is_empty()).then_some(row.address),
        this_mac: false,
        build: row.build,
        link: Some(link),
        last_heard_at: row.last_heard_at,
        why,
        code: None,
    }
}

/// A Mac this one asked to pair, waiting for Confirm on it, with the digits both show.
pub(crate) fn asked(row: store::PairedMac, digits: String, me: &crate::WayIn) -> Mac {
    let why = format!("Confirm on {} if it shows {digits}.", row.name);
    Mac { why: Some(why), code: Some(digits), ..listed(row, me) }
}

/// A Mac whose claim waits for Confirm here.
pub(crate) fn claimed(claim: HeldClaim) -> Mac {
    let why = match &claim.digits {
        Some(digits) => format!("{} wants to pair. Confirm if it shows {digits}.", claim.mac.name),
        None => format!("Press Confirm to pair {} with this Mac", claim.mac.name),
    };
    Mac {
        id: claim.mac.id,
        why: Some(why),
        code: claim.digits,
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
        wire: hello.wire.clone(),
        built_at: hello.built_at.clone(),
        source: hello.source.clone(),
    }
}

/// Keys in memory and the default pace, for a test that pairs no Mac.
#[cfg(test)]
pub(crate) fn quiet_macs() -> Macs {
    Macs::new(Arc::new(crate::MemoryMacKeys::default())).unwrap()
}
