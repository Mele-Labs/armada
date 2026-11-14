//! The Phone Gateway's own routes on its loopback port, for Settings › Macs: `GET /admin/setup`,
//! `POST /admin/setup/cloudflare`, `GET /admin/macs`, `POST /admin/macs/pair/{start,claim,confirm}`,
//! `POST /admin/macs/forget`, `GET /admin/macs/found`, `POST /admin/macs/pair/ask`,
//! `POST /admin/pair/{start,confirm,decline}`, `GET /admin/pair/pending`, `GET /admin/devices`
//! and `DELETE /admin/devices/{id}` (`crates/pocket/src/routes.rs`).
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
use ipc::{AskMacPair, BuildSource, ClaimMacLink, CloudflareState, FleetBuildChanging, FoundMac, FoundMacs, GatewaySetup, Mac, MacColour, MacLink, MacNamed, MacPairLink, PairedPhone, Served, TailscaleState, UpdateMac};
use serde::de::DeserializeOwned;
use serde::Deserialize;
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

/// `GET /admin/macs/found`: his other Macs on the Cloudflare account, and those Bonjour sees on this network.
pub fn found(cx: &App) -> Task<Result<FoundMacs, String>> {
    read("/admin/macs/found", "FoundMacs", cx)
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

/// `POST /admin/macs/pair/ask`: ask the Mac with this tunnel to pair, with no link. Answers that Mac, waiting on its
/// Confirm, with the four digits both show in its `why`.
pub fn pair_ask(tunnel: String, cx: &App) -> Task<Result<Mac, String>> {
    posted("/admin/macs/pair/ask", serde_json::to_string(&AskMacPair { tunnel }).ok(), "Mac", cx)
}

/// `POST /admin/macs/forget`: the pairing forgotten here and on the other Mac.
pub fn forget(mac: String, cx: &App) -> Task<Result<(), String>> {
    asked(cx, async move {
        let (status, bytes) = send("POST", "/admin/macs/forget".into(), named(mac)).await?;
        if (200..300).contains(&status) { Ok(()) } else { Err(said_of(status, &bytes)) }
    })
}

/// `POST /admin/pair/start`: the code a phone pairs with, where it is reached and when it stops working. `ipc` has no
/// type for it: the Gateway writes it from a local struct (`crates/pocket/src/pair_routes.rs`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PhoneCode {
    pub code: String,
    pub address: String,
    /// Unix seconds.
    pub expires_at: i64,
}

impl PhoneCode {
    /// What the phone's camera opens; Pocket reads the code from it.
    pub fn link(&self) -> String {
        format!("{}/pair?code={}", self.address, self.code)
    }
}

/// One entry of `GET /admin/pair/pending`: a phone that entered a code, waiting on Confirm here.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PhoneClaim {
    pub code: String,
    pub name: String,
    pub claimed_at: i64,
}

/// What a refused decline says when the code is gone, as the Gateway writes it.
pub const CODE_GONE: &str = "This code is not good any more.";

fn coded(code: String) -> Option<String> {
    serde_json::to_string(&serde_json::json!({ "code": code })).ok()
}

/// `POST /admin/pair/start`.
pub fn phone_start(cx: &App) -> Task<Result<PhoneCode, String>> {
    posted("/admin/pair/start", None, "PhoneCode", cx)
}

/// `GET /admin/pair/pending`: every phone waiting on Confirm.
pub fn phone_pending(cx: &App) -> Task<Result<Vec<PhoneClaim>, String>> {
    read("/admin/pair/pending", "PhoneClaims", cx)
}

/// `POST /admin/pair/confirm`: the phone that entered the code, paired.
pub fn phone_confirm(code: String, cx: &App) -> Task<Result<PairedPhone, String>> {
    posted("/admin/pair/confirm", coded(code), "PairedPhone", cx)
}

/// `POST /admin/pair/decline`: 204 ends the claim; a 404 says the code is gone, in the Gateway's words. The caller
/// drops the code here either way.
pub fn phone_decline(code: String, cx: &App) -> Task<Result<(), String>> {
    asked(cx, async move {
        let (status, bytes) = send("POST", "/admin/pair/decline".into(), coded(code)).await?;
        declined(status, &bytes)
    })
}

fn declined(status: u16, bytes: &[u8]) -> Result<(), String> {
    if (200..300).contains(&status) { Ok(()) } else { Err(said_of(status, bytes)) }
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
    /// The code a phone is asked to pair with, and the polls made since.
    phone_code: Option<String>,
    polls: u8,
    /// A Mac whose claim arrives once the list has been read this many more times, as the other Mac's does.
    claim_later: Option<(u8, Mac)>,
    /// Every body `POST /macs/update` took, in order.
    pub updates: Vec<UpdateMac>,
    /// The words the next update is refused with, once: Fleet's own sentence for a Mac that did not answer.
    pub refuse_update: Option<String>,
    /// Updates under way: the Mac, whether this Mac is the one that moves, and the reads left before the row is linked.
    updating: Vec<(String, bool, u8)>,
    /// What `GET /admin/macs/found` lists, and the sentence it gives when the account could not be read.
    pub found: Vec<FoundMac>,
    pub found_why: Option<String>,
    /// Every tunnel `POST /admin/macs/pair/ask` took, in order.
    pub asks: Vec<String>,
}

/// This Mac's address under the domain the sign-in picks (`label_of` the Mac's name).
const MOCK_ADDRESS: &str = "studio.example.com";
/// The address Tailscale Funnel serves this Mac at.
const MOCK_FUNNEL: &str = "https://studio.example.ts.net";
/// A link Start made on another Mac, which the mock's Claim takes.
pub const MOCK_OTHER_LINK: &str = "https://office.example.ts.net/pair/mac?code=7c41e0b2";
/// The code in the link the mock's Start makes.
const MOCK_CODE: &str = "5e0c1f7a";
/// The four digits both Macs show while a pairing the mock asked waits for Confirm.
const MOCK_DIGITS: &str = "4821";
/// The code the mock's phone pairing makes.
const MOCK_PHONE_CODE: &str = "a41f09c2";
/// The phone that enters it, from the second poll.
const MOCK_PHONE: &str = "iPhone 15";

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
            phone_code: None,
            polls: 0,
            claim_later: None,
            updates: Vec::new(),
            refuse_update: None,
            updating: Vec::new(),
            found: Vec::new(),
            found_why: None,
            asks: Vec::new(),
        }
    }

    /// Macs found beside the paired ones: Work MacBook is paired and on this network; Office iMac is on this network;
    /// MacBook Pro is on the account only and does not answer; Old iMac is asleep. Pair on Office iMac waits on its Confirm.
    pub fn nearby() -> MockGateway {
        let found = |tunnel: &str, name: &str, colour: Option<MacColour>, online: bool, paired: bool, nearby: bool| FoundMac {
            tunnel: tunnel.into(),
            name: name.into(),
            colour,
            address: (!nearby).then(|| format!("{}.example.com", tunnel.trim_start_matches("armada-"))),
            online,
            paired,
            build: Some("2930ac0".into()),
            nearby,
        };
        let mut gateway = MockGateway::new();
        gateway.found = vec![
            found("armada-mac-work", "Work MacBook", Some(MacColour::Orange), true, true, true),
            found("armada-mac-office", "Office iMac", Some(MacColour::Blue), true, false, true),
            found("armada-mac-book-pro", "MacBook Pro", Some(MacColour::Teal), true, false, false),
            found("armada-mac-old", "Old iMac", None, false, false, false),
        ];
        gateway
    }

    /// Two Macs on other builds: Mac mini on an older one, Mac Pro on a newer one, each with the Gateway's own why.
    pub fn builds_differ() -> MockGateway {
        let mut gateway = MockGateway::new();
        gateway.macs.retain(|mac| mac.this_mac || mac.id == "mac-work");
        gateway.macs.push(Mac {
            id: "mac-mini".into(),
            name: "Mac mini".into(),
            colour: MacColour::Blue,
            address: Some("mini.example.com".into()),
            this_mac: false,
            build: Some("1d199a1".into()),
            link: Some(MacLink::OlderBuild),
            why: Some("Mac mini runs an older build. Nothing crosses until it updates.".into()),
            ..Default::default()
        });
        gateway.macs.push(Mac {
            id: "mac-pro".into(),
            name: "Mac Pro".into(),
            colour: MacColour::Green,
            address: Some("https://pro.example.ts.net".into()),
            this_mac: false,
            build: Some("8f04be2".into()),
            link: Some(MacLink::NewerBuild),
            why: Some("Mac Pro runs a newer build. Nothing crosses until this Mac updates.".into()),
            ..Default::default()
        });
        gateway
    }

    /// The other Mac claims after the page opened: Mac Pro's claim arrives at the third read of the list.
    pub fn claim_later() -> MockGateway {
        let mut gateway = MockGateway::new();
        let at = gateway.macs.iter().position(|mac| mac.link == Some(MacLink::ClaimWaiting)).expect("the mock has a claim");
        let claim = gateway.macs.remove(at);
        gateway.claim_later = Some((2, claim));
        gateway
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

    /// A code good for five minutes. Cleared by Decline and Confirm; a new one replaces it.
    pub fn phone_start(&mut self) -> Result<PhoneCode, String> {
        self.up()?;
        let address = self.address().ok_or("Tailscale is not signed in on this Mac. Sign in to it, then start pairing again.")?;
        self.phone_code = Some(MOCK_PHONE_CODE.into());
        self.polls = 0;
        Ok(PhoneCode { code: MOCK_PHONE_CODE.into(), address, expires_at: chrono::Utc::now().timestamp() + 5 * 60 })
    }

    /// Empty at the first poll, and the phone's claim from the second.
    pub fn phone_pending(&mut self) -> Result<Vec<PhoneClaim>, String> {
        self.up()?;
        let Some(code) = self.phone_code.clone() else { return Ok(Vec::new()) };
        self.polls = self.polls.saturating_add(1);
        Ok(if self.polls >= 2 { vec![PhoneClaim { code, name: MOCK_PHONE.into(), claimed_at: chrono::Utc::now().timestamp() }] } else { Vec::new() })
    }

    pub fn phone_confirm(&mut self, code: &str) -> Result<PairedPhone, String> {
        self.up()?;
        if self.phone_code.as_deref() != Some(code) {
            return Err("This code is not good any more. Start pairing again.".into());
        }
        if self.polls < 2 {
            return Err("No phone has entered this code yet.".into());
        }
        self.phone_code = None;
        let now = chrono::Utc::now().timestamp();
        let phone = PairedPhone { id: "phone-3".into(), name: MOCK_PHONE.into(), created_at: now, last_seen_at: Some(now) };
        self.phones.push(phone.clone());
        Ok(phone)
    }

    /// Whether no code waits on a claim: Decline and Confirm leave none.
    pub fn phone_pending_is_empty(&self) -> bool {
        self.phone_code.is_none()
    }

    /// Ends the claim: the code is not listed again.
    pub fn phone_decline(&mut self, code: &str) -> Result<(), String> {
        self.up()?;
        if self.phone_code.as_deref() != Some(code) {
            return Err(CODE_GONE.into());
        }
        self.phone_code = None;
        Ok(())
    }

    /// This Mac's address with its scheme, where it has one.
    fn address(&self) -> Option<String> {
        self.setup.address.as_ref().map(|address| if address.starts_with("https://") { address.clone() } else { format!("https://{address}") })
    }

    pub fn read_macs(&mut self) -> Result<Vec<Mac>, String> {
        self.up()?;
        match self.claim_later.take() {
            Some((0, claim)) => self.macs.push(claim),
            Some((reads, claim)) => self.claim_later = Some((reads - 1, claim)),
            None => {}
        }
        self.finish_updates();
        Ok(self.macs.clone())
    }

    pub fn read_found(&mut self) -> Result<FoundMacs, String> {
        self.up()?;
        Ok(FoundMacs { macs: self.found.clone(), why: self.found_why.clone() })
    }

    /// `POST /admin/macs/pair/ask`: a Mac on this network waits on its Confirm, with the four digits both show. One only
    /// on the account does not answer, and one asleep has no address to ask. The Gateway's own sentences.
    pub fn pair_ask(&mut self, tunnel: &str) -> Result<Mac, String> {
        self.up()?;
        self.asks.push(tunnel.to_string());
        let Some(at) = self.found.iter().position(|one| one.tunnel == tunnel) else {
            return Err("That Mac is not on your Cloudflare account or on this network any more.".into());
        };
        let one = self.found[at].clone();
        if one.paired {
            return Err(format!("{} is paired with this Mac already.", one.name));
        }
        if !one.online && !one.nearby {
            return Err(format!("{} is not on this network and has no address on your domain. Pair with a link instead.", one.name));
        }
        if !one.nearby {
            return Err(format!("{} did not answer. Check Armada is open there, then press Pair again.", one.name));
        }
        self.found[at].paired = true;
        let mac = Mac {
            id: tunnel.trim_start_matches("armada-").to_string(),
            name: one.name.clone(),
            colour: one.colour.unwrap_or_default(),
            build: one.build,
            link: Some(MacLink::ConfirmWaiting),
            why: Some(format!("Confirm on {} if it shows {MOCK_DIGITS}.", one.name)),
            code: Some(MOCK_DIGITS.into()),
            ..Default::default()
        };
        self.put_paired(mac.clone());
        Ok(mac)
    }

    /// `POST /macs/update` on Fleet: 202, the restart has begun. The Mac is on the same build once the list has been
    /// read twice more, and its row is linked then. A refusal is Fleet's sentence, said once.
    pub fn update(&mut self, asked: &UpdateMac) -> Result<FleetBuildChanging, String> {
        self.updates.push(asked.clone());
        if let Some(said) = self.refuse_update.take() {
            return Err(said);
        }
        let (id, this_moves) = match (&asked.mac, &asked.like) {
            (Some(id), None) => (id.clone(), false),
            (None, Some(id)) => (id.clone(), true),
            _ => return Err("The body names one Mac to update, or one to update this Mac like.".into()),
        };
        if !self.macs.iter().any(|one| !one.this_mac && one.id == id) {
            return Err(format!("No Mac {id} is paired with this one."));
        }
        self.updating.push((id, this_moves, 2));
        Ok(FleetBuildChanging { build: BuildSource::Main })
    }

    /// Each read counts an update down, and the one that reaches nothing turns its row linked on one build.
    fn finish_updates(&mut self) {
        let mut done = Vec::new();
        self.updating.retain_mut(|(id, this_moves, reads)| {
            *reads -= 1;
            if *reads == 0 {
                done.push((id.clone(), *this_moves));
            }
            *reads > 0
        });
        for (id, this_moves) in done {
            let Some(at) = self.macs.iter().position(|one| one.id == id) else { continue };
            let build = self.macs[at].build.clone();
            let mine = self.macs.iter().find(|one| one.this_mac).and_then(|one| one.build.clone());
            self.macs[at].link = Some(MacLink::Linked);
            self.macs[at].why = None;
            if this_moves {
                if let Some(this) = self.macs.iter_mut().find(|one| one.this_mac) {
                    this.build = build;
                }
            } else {
                self.macs[at].build = mine;
            }
        }
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

    fn ids(gateway: &mut MockGateway) -> Vec<String> {
        gateway.read_macs().expect("read").into_iter().map(|one| one.id).collect()
    }

    #[test]
    fn the_macs_come_this_mac_first_then_paired_then_claims_waiting() {
        let mut gateway = MockGateway::new();
        assert_eq!(ids(&mut gateway), ["mac-studio", "mac-work", "mac-mini", "mac-air", "mac-pro"]);
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
        assert_eq!(ids(&mut gateway), ["mac-studio", "mac-work", "mac-mini", "mac-air", "mac-office", "mac-pro"], "a claim made here is paired, ahead of the claims waiting");
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
    fn pair_ask_waits_on_confirm_for_a_mac_nearby_and_is_refused_in_the_gateways_words_otherwise() {
        let mut gateway = MockGateway::nearby();
        let found = gateway.read_found().expect("read");
        assert_eq!(found.macs.iter().filter(|one| !one.paired).count(), 3);
        assert_eq!(gateway.pair_ask("armada-mac-book-pro"), Err("MacBook Pro did not answer. Check Armada is open there, then press Pair again.".into()));
        assert!(gateway.pair_ask("armada-mac-old").is_err_and(|said| said.contains("Pair with a link instead")));
        assert_eq!(gateway.pair_ask("armada-mac-gone"), Err("That Mac is not on your Cloudflare account or on this network any more.".into()));
        assert_eq!(gateway.pair_ask("armada-mac-work"), Err("Work MacBook is paired with this Mac already.".into()));
        let asked = gateway.pair_ask("armada-mac-office").expect("ask");
        assert_eq!((asked.link, asked.code.as_deref(), asked.why.as_deref()), (Some(MacLink::ConfirmWaiting), Some("4821"), Some("Confirm on Office iMac if it shows 4821.")));
        assert!(gateway.read_found().expect("read").macs.iter().any(|one| one.tunnel == "armada-mac-office" && one.paired), "it is paired from then on");
        assert!(ids(&mut gateway).contains(&"mac-office".to_string()));
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

    #[test]
    fn a_phone_claims_from_the_second_poll_and_confirm_pairs_it() {
        let mut gateway = MockGateway::new();
        let made = gateway.phone_start().expect("start");
        assert_eq!(made.link(), format!("{MOCK_FUNNEL}/pair?code={MOCK_PHONE_CODE}"));
        assert!(made.expires_at > chrono::Utc::now().timestamp() + 4 * 60, "about five minutes");
        assert_eq!(gateway.phone_confirm(&made.code), Err("No phone has entered this code yet.".into()));
        assert_eq!(gateway.phone_pending(), Ok(Vec::new()), "the first poll finds none");
        let claims = gateway.phone_pending().expect("pending");
        assert_eq!(claims.iter().map(|one| (one.code.as_str(), one.name.as_str())).collect::<Vec<_>>(), [(MOCK_PHONE_CODE, MOCK_PHONE)]);
        let paired = gateway.phone_confirm(&made.code).expect("confirm");
        assert_eq!(paired.name, MOCK_PHONE);
        assert!(gateway.phones.iter().any(|one| one.id == paired.id), "the phone is in the list");
        assert_eq!(gateway.phone_confirm(&made.code), Err("This code is not good any more. Start pairing again.".into()), "a spent code");
    }

    #[test]
    fn decline_ends_the_claim_and_says_when_the_code_is_gone() {
        let mut gateway = MockGateway::new();
        let made = gateway.phone_start().expect("start");
        gateway.phone_pending().expect("pending");
        gateway.phone_pending().expect("pending");
        assert_eq!(gateway.phone_decline(&made.code), Ok(()));
        assert_eq!(gateway.phone_pending(), Ok(Vec::new()), "the code is not listed again");
        assert_eq!(gateway.phones.len(), 2, "no phone joined");
        assert_eq!(gateway.phone_decline(&made.code), Err(CODE_GONE.into()));
    }

    #[test]
    fn a_decline_ends_on_204_and_a_refusal_is_said_in_the_gateways_words() {
        assert_eq!(declined(204, b""), Ok(()));
        assert_eq!(declined(404, b"This code is not good any more.\n"), Err(CODE_GONE.into()));
        assert_eq!(declined(404, b""), Err("The Gateway answered 404.".into()));
        assert_eq!(declined(500, b"The store failed."), Err("The store failed.".into()));
    }
}
