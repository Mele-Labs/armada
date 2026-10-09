//! Pairing: a code Bridge shows, a phone that presents it with its key, and the
//! owner's confirmation in Bridge. Nothing is a device until the owner confirms.
//!
//! Codes and pending phones live in memory. A Gateway restart drops them, and a
//! half-finished pairing is started again. Devices are in the store.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use store::{Device, Store};

/// Unix seconds now. Injected, so a test moves time.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// The Gateway's `https://<name>` on the tailnet, or the sentence saying why not.
pub type Address = Arc<dyn Fn() -> Result<String, String> + Send + Sync>;

pub const CODE_TTL: i64 = 5 * 60;
/// `/pair` answers this many requests a minute, wrong codes included.
const PAIR_PER_MINUTE: usize = 10;

struct Code {
    expires: i64,
    claimed: Option<Claim>,
}

struct Claim {
    /// Chosen here rather than at Confirm, so the phone knows what to sign as.
    id: String,
    name: String,
    public_key: Vec<u8>,
    at: i64,
}

#[derive(Default)]
struct Memory {
    codes: HashMap<String, Code>,
    pair_attempts: Vec<i64>,
    /// Signature (hex) to the time it was accepted.
    seen: HashMap<String, i64>,
}

pub enum PairRefused {
    TooMany,
    BadCode,
    BadKey,
    NoName,
    NoId(String),
}

pub enum ConfirmRefused {
    BadCode,
    NotClaimed,
    Store(String),
}

#[derive(Clone)]
pub struct Pairing {
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) clock: Clock,
    address: Address,
    memory: Arc<Mutex<Memory>>,
}

impl Pairing {
    pub fn new(store: Store, clock: Clock, address: Address) -> Pairing {
        Pairing {
            store: Arc::new(Mutex::new(store)),
            clock,
            address,
            memory: Arc::default(),
        }
    }

    /// A new single-use code and the address the phone reaches the Gateway at.
    pub fn start(&self) -> Result<(String, String, i64), String> {
        let address = (self.address)()?;
        let now = (self.clock)();
        let code = random_hex()?;
        let mut memory = self.memory.lock().unwrap();
        memory.codes.retain(|_, c| c.expires > now);
        memory.codes.insert(code.clone(), Code { expires: now + CODE_TTL, claimed: None });
        Ok((code, address, now + CODE_TTL))
    }

    /// A phone presents the code, and learns the id it will sign as. Held until
    /// the owner confirms; the id signs nothing before then.
    pub fn claim(&self, code: &str, name: &str, spki_hex: &str) -> Result<String, PairRefused> {
        let now = (self.clock)();
        let mut memory = self.memory.lock().unwrap();
        memory.pair_attempts.retain(|t| *t > now - 60);
        if memory.pair_attempts.len() >= PAIR_PER_MINUTE {
            return Err(PairRefused::TooMany);
        }
        memory.pair_attempts.push(now);
        let public_key = from_hex(spki_hex).ok_or(PairRefused::BadKey)?;
        crate::signing::verifying_key(&public_key).ok_or(PairRefused::BadKey)?;
        let name = name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err(PairRefused::NoName);
        }
        match memory.codes.get_mut(code) {
            Some(c) if c.expires > now && c.claimed.is_none() => {
                let id = random_hex().map_err(PairRefused::NoId)?;
                c.claimed = Some(Claim { id: id.clone(), name: name.to_string(), public_key, at: now });
                Ok(id)
            }
            _ => Err(PairRefused::BadCode),
        }
    }

    /// Codes a phone has claimed that nobody has confirmed: code, name, when claimed.
    pub fn pending(&self) -> Vec<(String, String, i64)> {
        let now = (self.clock)();
        let memory = self.memory.lock().unwrap();
        let mut all: Vec<_> = memory
            .codes
            .iter()
            .filter(|(_, c)| c.expires > now)
            .filter_map(|(code, c)| c.claimed.as_ref().map(|k| (code.clone(), k.name.clone(), k.at)))
            .collect();
        all.sort_by_key(|(_, _, at)| *at);
        all
    }

    /// The owner confirms: the phone becomes a device and the code is burned.
    pub fn confirm(&self, code: &str) -> Result<Device, ConfirmRefused> {
        let now = (self.clock)();
        let mut memory = self.memory.lock().unwrap();
        let live = matches!(memory.codes.get(code), Some(c) if c.expires > now);
        if !live {
            memory.codes.remove(code);
            return Err(ConfirmRefused::BadCode);
        }
        if memory.codes[code].claimed.is_none() {
            return Err(ConfirmRefused::NotClaimed);
        }
        let claim = memory.codes.remove(code).and_then(|c| c.claimed).unwrap();
        let device = Device {
            id: claim.id,
            name: claim.name,
            public_key: claim.public_key,
            created_at: now,
            last_seen_at: None,
        };
        self.store
            .lock()
            .unwrap()
            .add_device(&device)
            .map_err(|why| ConfirmRefused::Store(why.to_string()))?;
        Ok(device)
    }

    /// Records a verified signature. `false` where it was already seen.
    pub(crate) fn first_sight(&self, signature: &str, now: i64) -> bool {
        let mut memory = self.memory.lock().unwrap();
        // A signature is replayable while its time is inside the skew window,
        // which is up to twice the window after it was first accepted.
        memory.seen.retain(|_, at| *at > now - 2 * crate::signing::SKEW);
        memory.seen.insert(signature.to_string(), now).is_none()
    }
}

/// 128 random bits as 32 hex characters, from the operating system.
fn random_hex() -> Result<String, String> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|why| format!("The system's random numbers could not be read: {why}"))?;
    Ok(to_hex(&bytes))
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 || !text.is_ascii() {
        return None;
    }
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok())
        .collect()
}

/// `https://<name>` from the output of `tailscale status --json`. Read by
/// scanning for `Self`'s `DNSName` because bytes are only parsed in `store`
/// and `ipc`.
pub fn address_from_status(output: &str) -> Result<String, String> {
    let signed_out = "Tailscale is not signed in on this Mac. Sign in to it, then start pairing again.";
    if output.contains("\"BackendState\"") && !output.contains("\"BackendState\": \"Running\"")
        && !output.contains("\"BackendState\":\"Running\"")
    {
        return Err(signed_out.to_string());
    }
    let after = &output[output.find("\"Self\"").ok_or(signed_out)?..];
    let rest = &after[after.find("\"DNSName\"").ok_or(signed_out)? + 9..];
    let rest = rest.trim_start().strip_prefix(':').ok_or(signed_out)?.trim_start();
    let rest = rest.strip_prefix('"').ok_or(signed_out)?;
    let name = rest[..rest.find('"').ok_or(signed_out)?].trim_end_matches('.');
    if name.is_empty() {
        return Err(signed_out.to_string());
    }
    Ok(format!("https://{name}"))
}

/// Asks the installed Tailscale. Blocking: callers run it off the async threads.
pub fn tailscale_address() -> Result<String, String> {
    let output = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .map_err(|_| "Tailscale is not installed on this Mac. Install it and sign in, then start pairing again.".to_string())?;
    address_from_status(&String::from_utf8_lossy(&output.stdout))
}
