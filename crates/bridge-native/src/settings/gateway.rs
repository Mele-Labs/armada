//! The Phone Gateway's own routes on its loopback port, for Settings › Macs: `GET /admin/setup`,
//! `POST /admin/setup/cloudflare`, `GET /admin/macs`, `POST /admin/macs/pair/{start,claim,confirm}`,
//! `POST /admin/macs/forget`, `GET /admin/devices` and `DELETE /admin/devices/{id}` (`crates/pocket/src/routes.rs`).
//!
//! **Not Fleet's port, and not Fleet's errors.** The Gateway answers a refusal as a plain sentence, and refuses
//! any request that carries `Origin`, so it is asked over hyper's bare client, which sends only the headers
//! written here (`apps/desktop/src/main/phone-gateway.ts`).

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use gpui::{App, AppContext, Task};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{CONTENT_TYPE, HOST};
use hyper::Request;
use hyper_util::rt::TokioIo;
use ipc::{ClaimMacLink, CloudflareState, GatewaySetup, Mac, MacColour, MacLink, MacNamed, MacPairLink, PairedPhone, Served, TailscaleState};
use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// The Gateway's port (`crates/armada/src/pocketing.rs`'s `PORT`).
const PORT: u16 = 8443;

/// Classic's `phone-gateway.ts` waits as long.
const PATIENCE: Duration = Duration::from_secs(5);

/// The tunnel press's body. Empty: the domain comes from the Cloudflare sign-in, not from anything typed (the
/// owner's ruling of 12 Nov, and the Fleet Job "Cloudflare setup with nothing typed"). Until that Job lands, the
/// route still wants `{ domain }` and its refusal is said under the button.
const START_BODY: Option<&str> = None;

/// What is said when nothing answers on the port.
pub const NOT_RUNNING: &str = "The Gateway is not running.";

/// What the tunnel's press did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tunnel {
    /// 204: the tunnel is made.
    Made,
    /// 200: it waits for him to sign in on Cloudflare's page.
    SignIn(String),
}

async fn send(method: &'static str, path: String, body: Option<String>) -> Result<(u16, Vec<u8>), String> {
    let at = SocketAddr::from((Ipv4Addr::LOCALHOST, PORT));
    let stream = timeout(PATIENCE, TcpStream::connect(at)).await.map_err(|_| NOT_RUNNING.to_string())?.map_err(|_| NOT_RUNNING.to_string())?;
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream)).await.map_err(|why| why.to_string())?;
    tokio::spawn(connection);
    let request = Request::builder().method(method).uri(path).header(HOST, format!("127.0.0.1:{PORT}"));
    let request = match body {
        Some(body) => request.header(CONTENT_TYPE, "application/json").body(Full::new(Bytes::from(body))),
        None => request.body(Full::new(Bytes::new())),
    }
    .map_err(|why| why.to_string())?;
    let read = async {
        let response = sender.send_request(request).await.map_err(|why| why.to_string())?;
        let status = response.status().as_u16();
        let body = response.into_body().collect().await.map_err(|why| why.to_string())?;
        Ok((status, body.to_bytes().to_vec()))
    };
    timeout(PATIENCE, read).await.map_err(|_| "The Gateway did not answer in 5 seconds.".to_string())?
}

/// A refusal's own sentence, as the Gateway wrote it.
fn said_of(status: u16, bytes: &[u8]) -> String {
    let said = String::from_utf8_lossy(bytes).trim().to_string();
    if said.is_empty() { format!("The Gateway answered {status}.") } else { said }
}

fn asked<T: Send + 'static>(cx: &App, ask: impl std::future::Future<Output = Result<T, String>> + Send + 'static) -> Task<Result<T, String>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        runtime.block_on(ask)
    })
}

fn read<T: DeserializeOwned + Send + 'static>(path: &'static str, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    asked(cx, async move {
        let (status, bytes) = send("GET", path.into(), None).await?;
        if !(200..300).contains(&status) {
            return Err(said_of(status, &bytes));
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| why.to_string())
    })
}

pub fn setup(cx: &App) -> Task<Result<GatewaySetup, String>> {
    read("/admin/setup", "GatewaySetup", cx)
}

pub fn macs(cx: &App) -> Task<Result<Vec<Mac>, String>> {
    read("/admin/macs", "Macs", cx)
}

pub fn devices(cx: &App) -> Task<Result<Vec<PairedPhone>, String>> {
    read("/admin/devices", "PairedPhones", cx)
}

pub fn start(cx: &App) -> Task<Result<Tunnel, String>> {
    asked(cx, async move {
        let (status, bytes) = send("POST", "/admin/setup/cloudflare".into(), START_BODY.map(String::from)).await?;
        match status {
            204 => Ok(Tunnel::Made),
            200 => ipc::decode::<ipc::CloudflareSignIn>("CloudflareSignIn", &bytes).map(|sign_in| Tunnel::SignIn(sign_in.login)).map_err(|why| why.to_string()),
            _ => Err(said_of(status, &bytes)),
        }
    })
}

pub fn unpair(id: String, cx: &App) -> Task<Result<(), String>> {
    asked(cx, async move {
        let (status, bytes) = send("DELETE", format!("/admin/devices/{}", crate::link::percent_encoded(&id)), None).await?;
        if (200..300).contains(&status) { Ok(()) } else { Err(said_of(status, &bytes)) }
    })
}

fn posted<T: DeserializeOwned + Send + 'static>(path: &'static str, body: Option<String>, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    asked(cx, async move {
        let (status, bytes) = send("POST", path.into(), body).await?;
        if !(200..300).contains(&status) {
            return Err(said_of(status, &bytes));
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| why.to_string())
    })
}

fn named(mac: String) -> Option<String> {
    serde_json::to_string(&MacNamed { mac }).ok()
}

/// `POST /admin/macs/pair/start`: the link the other Mac pastes, and when it stops working.
pub fn pair_start(cx: &App) -> Task<Result<MacPairLink, String>> {
    posted("/admin/macs/pair/start", None, "MacPairLink", cx)
}

/// `POST /admin/macs/pair/claim`: the other Mac's link, pasted here. Answers that Mac, waiting on its Confirm.
pub fn pair_claim(link: String, cx: &App) -> Task<Result<Mac, String>> {
    posted("/admin/macs/pair/claim", serde_json::to_string(&ClaimMacLink { link }).ok(), "Mac", cx)
}

/// `POST /admin/macs/pair/confirm`: the Mac whose claim waits here, linked.
pub fn pair_confirm(mac: String, cx: &App) -> Task<Result<Mac, String>> {
    posted("/admin/macs/pair/confirm", named(mac), "Mac", cx)
}

/// `POST /admin/macs/forget`: the pairing forgotten here and on the other Mac.
pub fn forget(mac: String, cx: &App) -> Task<Result<(), String>> {
    asked(cx, async move {
        let (status, bytes) = send("POST", "/admin/macs/forget".into(), named(mac)).await?;
        if (200..300).contains(&status) { Ok(()) } else { Err(said_of(status, &bytes)) }
    })
}

/// The walk's and the tests' Gateway, passing through every tunnel state: `cloudflared` not downloaded yet; the
/// first press downloads it and is refused, leaving this Mac signed out; the next asks for the sign-in, whose
/// domain lookup fails once (no tunnel, and why); the press after that signs in and the tunnel runs; and a read
/// after that finds it down, as when the page is opened later. Tailscale Funnel serves phones throughout, as on a
/// Mac set up before the tunnel; the running tunnel names the address from then on. Two phones, and a Mac in each
/// link state the Gateway sets: linked, asleep, a claim waiting here, and one waiting on Confirm there. Pasting
/// [`MOCK_OTHER_LINK`] claims one more; the first Confirm finds the other Mac not answering.
pub struct MockGateway {
    pub running: bool,
    pub setup: GatewaySetup,
    pub macs: Vec<Mac>,
    pub phones: Vec<PairedPhone>,
    /// Confirm presses taken.
    confirms: u8,
    /// Reads left before the sign-in finishes.
    signing: u8,
    /// Whether the running tunnel was read once.
    seen_running: bool,
    /// Presses taken.
    presses: u8,
    /// Whether the sign-in's domain lookup has failed once.
    looked_up: bool,
    /// Whether a running tunnel is found down at the next read, as when the page is opened later.
    drops: bool,
    /// Reads a sign-in takes.
    sign_reads: u8,
}

/// This Mac's address under the domain the sign-in picks (`label_of` the Mac's name).
const MOCK_ADDRESS: &str = "studio.example.com";
/// The address Tailscale Funnel serves this Mac at.
const MOCK_FUNNEL: &str = "https://studio.example.ts.net";
/// A link Start made on another Mac, which the mock's Claim takes.
pub const MOCK_OTHER_LINK: &str = "https://office.example.ts.net/pair/mac?code=7c41e0b2";
/// The code in the link the mock's Start makes.
const MOCK_CODE: &str = "5e0c1f7a";

impl MockGateway {
    pub fn new() -> MockGateway {
        use chrono::TimeZone;
        let now = chrono::Utc::now().timestamp();
        // A fixed morning, so the walk names the asleep Mac's tooltip as it reads.
        let asleep = chrono::Local.with_ymd_and_hms(2026, 11, 12, 9, 12, 0).single().map_or(now - 3 * 3600, |at| at.timestamp());
        MockGateway {
            running: true,
            setup: GatewaySetup { tailscale: TailscaleState::Running, served: Served::Funnel, named: false, address: Some(MOCK_FUNNEL.into()), cloudflare: CloudflareState::Missing, cloudflared: None },
            macs: vec![
                Mac { id: "mac-studio".into(), name: "Studio".into(), colour: MacColour::Teal, address: None, this_mac: true, build: Some("2930ac0".into()), link: None, ..Default::default() },
                Mac { id: "mac-work".into(), name: "Work MacBook".into(), colour: MacColour::Orange, address: Some("work.example.com".into()), this_mac: false, build: Some("2930ac0".into()), link: Some(MacLink::Linked), ..Default::default() },
                Mac { id: "mac-mini".into(), name: "Mac mini".into(), colour: MacColour::Blue, address: Some("mini.example.com".into()), this_mac: false, build: Some("1d199a1".into()), link: Some(MacLink::Asleep { since: asleep }), why: Some("Mac mini went quiet for 90 seconds.".into()), ..Default::default() },
                Mac { id: "mac-air".into(), name: "MacBook Air".into(), colour: MacColour::Orange, address: Some("https://air.example.ts.net".into()), this_mac: false, build: Some("2930ac0".into()), link: Some(MacLink::ConfirmWaiting), why: Some("Confirm on MacBook Air".into()), ..Default::default() },
                Mac { id: "mac-pro".into(), name: "Mac Pro".into(), colour: MacColour::Green, address: Some("https://pro.example.ts.net".into()), this_mac: false, build: Some("2930ac0".into()), link: Some(MacLink::ClaimWaiting), why: Some("Press Confirm to pair Mac Pro with this Mac".into()), ..Default::default() },
            ],
            phones: vec![
                PairedPhone { id: "phone-1".into(), name: "iPhone".into(), created_at: now - 9 * 86400, last_seen_at: Some(now - 4 * 60) },
                PairedPhone { id: "phone-2".into(), name: "iPad".into(), created_at: now - 2 * 86400, last_seen_at: None },
            ],
            confirms: 0,
            signing: 0,
            seen_running: false,
            presses: 0,
            looked_up: false,
            drops: true,
            sign_reads: 1,
        }
    }

    /// A Mac nothing is set up on: the Gateway is off, nothing serves it, and no other Mac is paired. Its setup runs
    /// through in one go and stays up, and the sign-in takes a few reads.
    pub fn fresh() -> MockGateway {
        let mut gateway = MockGateway::without_funnel();
        gateway.running = false;
        gateway.macs.retain(|mac| mac.this_mac);
        gateway.phones.clear();
        gateway.drops = false;
        gateway.sign_reads = 2;
        gateway
    }

    /// A Mac Tailscale Funnel does not serve, set up in one go: the first press asks for the sign-in, and the sign-in
    /// finds the domain.
    pub fn without_funnel() -> MockGateway {
        let mut gateway = MockGateway::new();
        gateway.setup.served = Served::None;
        gateway.setup.address = None;
        gateway.presses = 1;
        gateway.looked_up = true;
        gateway
    }

    fn up(&self) -> Result<(), String> {
        if self.running { Ok(()) } else { Err(NOT_RUNNING.into()) }
    }

    pub fn read_setup(&mut self) -> Result<GatewaySetup, String> {
        self.up()?;
        let since = chrono::Utc::now().timestamp() - 40 * 60;
        let next = match &self.setup.cloudflare {
            CloudflareState::SigningIn { .. } if self.signing > 0 => {
                self.signing -= 1;
                None
            }
            CloudflareState::SigningIn { .. } if !self.looked_up => {
                self.looked_up = true;
                Some(CloudflareState::NoTunnel { why: Some("The sign-in picked no domain on your Cloudflare account. Press Set up to sign in again.".into()) })
            }
            CloudflareState::SigningIn { .. } => {
                self.setup.address = Some(MOCK_ADDRESS.into());
                self.setup.named = true;
                Some(CloudflareState::Running { address: MOCK_ADDRESS.into() })
            }
            CloudflareState::Running { address } if self.seen_running && self.drops => Some(CloudflareState::Down { address: address.clone(), since, why: "Cloudflare closed the connection: the tunnel's credentials were refused.".into() }),
            CloudflareState::Running { .. } => {
                self.seen_running = true;
                None
            }
            _ => None,
        };
        if let Some(next) = next {
            self.setup.cloudflare = next;
            if let CloudflareState::Running { .. } = self.setup.cloudflare {
                self.seen_running = true;
            }
        }
        Ok(self.setup.clone())
    }

    pub fn start(&mut self) -> Result<Tunnel, String> {
        self.up()?;
        if let CloudflareState::SigningIn { .. } = self.setup.cloudflare {
            return Err("The tunnel is already being set up.".into());
        }
        self.presses += 1;
        if self.presses == 1 {
            self.setup.cloudflared = Some("2025.11.0".into());
            self.setup.cloudflare = CloudflareState::SignedOut;
            return Err("Cloudflare did not answer the sign-in request.".into());
        }
        let login = "https://dash.cloudflare.com/argotunnel?aud=&callback=https%3A%2F%2Flogin.cloudflareaccess.org%2Fmock".to_string();
        self.setup.cloudflared.get_or_insert_with(|| "2025.11.0".into());
        self.setup.cloudflare = CloudflareState::SigningIn { login: login.clone() };
        self.signing = self.sign_reads;
        Ok(Tunnel::SignIn(login))
    }

    pub fn unpair(&mut self, id: &str) -> Result<(), String> {
        self.up()?;
        let before = self.phones.len();
        self.phones.retain(|one| one.id != id);
        if self.phones.len() == before { Err("No phone is paired with that id.".into()) } else { Ok(()) }
    }

    /// This Mac's address with its scheme, where it has one.
    fn address(&self) -> Option<String> {
        self.setup.address.as_ref().map(|address| if address.starts_with("https://") { address.clone() } else { format!("https://{address}") })
    }

    pub fn read_macs(&self) -> Result<Vec<Mac>, String> {
        self.up()?;
        Ok(self.macs.clone())
    }

    /// A paired Mac goes in after the others paired, ahead of the claims waiting here, as the Gateway lists them.
    fn put_paired(&mut self, mac: Mac) {
        let at = self.macs.iter().position(|one| one.link == Some(MacLink::ClaimWaiting)).unwrap_or(self.macs.len());
        self.macs.insert(at, mac);
    }

    pub fn pair_start(&mut self) -> Result<MacPairLink, String> {
        self.up()?;
        let address = self.address().ok_or("Tailscale is not signed in on this Mac. Sign in to it, then start pairing again.")?;
        Ok(MacPairLink { link: format!("{address}/pair/mac?code={MOCK_CODE}"), expires_at: chrono::Utc::now().timestamp() + 5 * 60 })
    }

    pub fn pair_claim(&mut self, link: &str) -> Result<Mac, String> {
        self.up()?;
        let link = link.trim();
        let Some(host) = link.strip_prefix("https://").and_then(|rest| rest.split_once("/pair/mac?code=")).map(|(host, _)| host) else {
            return Err("This is not a pairing link from another Mac. Copy it again from Settings, Macs on that Mac.".into());
        };
        if self.address().is_some_and(|mine| mine == format!("https://{host}")) {
            return Err("This link is this Mac's own. Paste it on the other Mac.".into());
        }
        if link != MOCK_OTHER_LINK {
            return Err(format!("The Mac at https://{host} did not answer. Check Armada is open there, then paste the link again. Connection refused."));
        }
        if let Some(held) = self.macs.iter().find(|one| one.id == "mac-office") {
            return Ok(held.clone());
        }
        let mac = Mac {
            id: "mac-office".into(),
            name: "Office iMac".into(),
            colour: MacColour::Blue,
            address: Some(format!("https://{host}")),
            this_mac: false,
            build: Some("2930ac0".into()),
            link: Some(MacLink::ConfirmWaiting),
            why: Some("Confirm on Office iMac".into()),
            ..Default::default()
        };
        self.put_paired(mac.clone());
        Ok(mac)
    }

    /// The first press finds the other Mac not answering, and the claim stays for the next.
    pub fn pair_confirm(&mut self, id: &str) -> Result<Mac, String> {
        self.up()?;
        let Some(at) = self.macs.iter().position(|one| one.id == id && one.link == Some(MacLink::ClaimWaiting)) else {
            return Err("No Mac waits for Confirm here. Paste the link on the other Mac again.".into());
        };
        self.confirms += 1;
        if self.confirms == 1 {
            return Err(format!("{} did not answer. Press Confirm again.", self.macs[at].name));
        }
        let mut mac = self.macs.remove(at);
        mac.link = Some(MacLink::Linked);
        mac.why = None;
        mac.last_heard_at = Some(chrono::Utc::now().timestamp());
        self.put_paired(mac.clone());
        Ok(mac)
    }

    pub fn forget(&mut self, id: &str) -> Result<(), String> {
        self.up()?;
        let before = self.macs.len();
        self.macs.retain(|one| one.this_mac || one.id != id);
        if self.macs.len() == before { Err("No Mac is paired with that id.".into()) } else { Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(gateway: &mut MockGateway) -> &'static str {
        match gateway.read_setup().expect("read").cloudflare {
            CloudflareState::Missing => "missing",
            CloudflareState::SignedOut => "signed_out",
            CloudflareState::SigningIn { .. } => "signing_in",
            CloudflareState::NoTunnel { .. } => "no_tunnel",
            CloudflareState::Running { .. } => "running",
            CloudflareState::Down { .. } => "down",
        }
    }

    #[test]
    fn the_mock_passes_through_every_tunnel_state_with_one_press_each() {
        let mut gateway = MockGateway::new();
        assert_eq!(state(&mut gateway), "missing");
        assert!(gateway.start().is_err(), "the first press is refused");
        assert_eq!(state(&mut gateway), "signed_out");
        assert!(matches!(gateway.start(), Ok(Tunnel::SignIn(_))));
        assert!(gateway.start().is_err(), "a second press while it signs in is busy");
        assert_eq!(state(&mut gateway), "signing_in");
        assert_eq!(state(&mut gateway), "no_tunnel");
        assert!(matches!(gateway.start(), Ok(Tunnel::SignIn(_))));
        assert_eq!(state(&mut gateway), "signing_in");
        assert_eq!(state(&mut gateway), "running");
        assert_eq!(gateway.setup.address.as_deref(), Some(MOCK_ADDRESS), "the running tunnel names the address");
        assert_eq!(state(&mut gateway), "down");
    }

    #[test]
    fn a_mac_without_funnel_runs_the_tunnel_after_one_press() {
        let mut gateway = MockGateway::without_funnel();
        assert_eq!(state(&mut gateway), "missing");
        assert!(matches!(gateway.start(), Ok(Tunnel::SignIn(_))));
        assert_eq!(gateway.setup.cloudflared.as_deref(), Some("2025.11.0"), "the press downloads cloudflared");
        assert_eq!(state(&mut gateway), "signing_in");
        assert_eq!(state(&mut gateway), "running");
        assert_eq!(gateway.setup.served, Served::None);
        assert_eq!(state(&mut gateway), "down");
    }

    fn ids(gateway: &MockGateway) -> Vec<String> {
        gateway.read_macs().expect("read").into_iter().map(|one| one.id).collect()
    }

    #[test]
    fn the_macs_come_this_mac_first_then_paired_then_claims_waiting() {
        let gateway = MockGateway::new();
        assert_eq!(ids(&gateway), ["mac-studio", "mac-work", "mac-mini", "mac-air", "mac-pro"]);
        assert!(gateway.macs[0].this_mac && gateway.macs[0].link.is_none(), "this Mac has no link");
    }

    #[test]
    fn start_claim_and_confirm_pair_a_mac() {
        let mut gateway = MockGateway::new();
        let made = gateway.pair_start().expect("start");
        assert_eq!(made.link, format!("{MOCK_FUNNEL}/pair/mac?code={MOCK_CODE}"));
        assert!(made.expires_at > chrono::Utc::now().timestamp(), "the link is good for a while");
        let claimed = gateway.pair_claim(MOCK_OTHER_LINK).expect("claim");
        assert_eq!((claimed.link, claimed.why.as_deref()), (Some(MacLink::ConfirmWaiting), Some("Confirm on Office iMac")));
        assert_eq!(ids(&gateway), ["mac-studio", "mac-work", "mac-mini", "mac-air", "mac-office", "mac-pro"], "a claim made here is paired, ahead of the claims waiting");
        assert_eq!(gateway.pair_confirm("mac-pro"), Err("Mac Pro did not answer. Press Confirm again.".into()));
        assert_eq!(gateway.macs.iter().find(|one| one.id == "mac-pro").and_then(|one| one.link.clone()), Some(MacLink::ClaimWaiting), "the claim stays");
        let linked = gateway.pair_confirm("mac-pro").expect("confirm");
        assert_eq!((linked.link, linked.why), (Some(MacLink::Linked), None));
    }

    #[test]
    fn each_pairing_route_refuses_in_the_gateways_words() {
        let mut gateway = MockGateway::new();
        assert_eq!(gateway.pair_claim("hello"), Err("This is not a pairing link from another Mac. Copy it again from Settings, Macs on that Mac.".into()));
        let mine = gateway.pair_start().expect("start").link;
        assert_eq!(gateway.pair_claim(&mine), Err("This link is this Mac's own. Paste it on the other Mac.".into()));
        assert!(gateway.pair_claim("https://elsewhere.example/pair/mac?code=1").is_err_and(|said| said.starts_with("The Mac at https://elsewhere.example did not answer.")));
        assert_eq!(gateway.pair_confirm("mac-work"), Err("No Mac waits for Confirm here. Paste the link on the other Mac again.".into()));
        assert_eq!(gateway.forget("mac-gone"), Err("No Mac is paired with that id.".into()));
        let mut bare = MockGateway::without_funnel();
        assert_eq!(bare.pair_start(), Err("Tailscale is not signed in on this Mac. Sign in to it, then start pairing again.".into()));
    }

    #[test]
    fn forget_takes_the_mac_off_and_never_this_one() {
        let mut gateway = MockGateway::new();
        assert!(gateway.forget("mac-work").is_ok());
        assert!(gateway.macs.iter().all(|one| one.id != "mac-work"));
        assert!(gateway.forget("mac-studio").is_err(), "this Mac is not paired with itself");
    }

    #[test]
    fn unpair_takes_the_phone_off_and_refuses_one_not_paired() {
        let mut gateway = MockGateway::new();
        assert!(gateway.unpair("phone-2").is_ok());
        assert!(gateway.phones.iter().all(|one| one.id != "phone-2"));
        assert_eq!(gateway.unpair("phone-2"), Err("No phone is paired with that id.".into()));
    }
}
