//! The Phone Gateway's own routes on its loopback port, for Settings › Macs: `GET /admin/setup`,
//! `POST /admin/setup/cloudflare`, `GET /admin/macs`, `GET /admin/devices` and `DELETE /admin/devices/{id}`
//! (`crates/pocket/src/routes.rs`).
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
use ipc::{CloudflareState, GatewaySetup, Mac, MacColour, MacLink, PairedPhone, Served, TailscaleState};
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

/// The walk's and the tests' Gateway: this Mac signed out of Cloudflare, two other Macs and two phones. The first
/// press is refused, as Cloudflare refuses one; the next asks for the sign-in; the second read after it finds the
/// tunnel running, as if he had signed in; and a read after that finds it down, as when the page is opened later.
pub struct MockGateway {
    pub running: bool,
    pub setup: GatewaySetup,
    pub macs: Vec<Mac>,
    pub phones: Vec<PairedPhone>,
    /// Reads left before the sign-in finishes.
    signing: u8,
    /// Whether the running tunnel was read once.
    seen_running: bool,
    /// Presses taken.
    presses: u8,
}

/// This Mac's address under the domain the sign-in picks (`label_of` the Mac's name).
const MOCK_ADDRESS: &str = "studio.example.com";

impl MockGateway {
    pub fn new() -> MockGateway {
        let now = chrono::Utc::now().timestamp();
        MockGateway {
            running: true,
            setup: GatewaySetup { tailscale: TailscaleState::Running, served: Served::None, named: false, address: None, cloudflare: CloudflareState::SignedOut, cloudflared: Some("2025.11.0".into()) },
            macs: vec![
                Mac { id: "mac-studio".into(), name: "Studio".into(), colour: MacColour::Teal, address: None, this_mac: true, build: Some("2930ac0".into()), link: None },
                Mac { id: "mac-work".into(), name: "Work MacBook".into(), colour: MacColour::Orange, address: Some("work.example.com".into()), this_mac: false, build: Some("2930ac0".into()), link: Some(MacLink::Linked) },
                Mac { id: "mac-mini".into(), name: "Mac mini".into(), colour: MacColour::Blue, address: Some("mini.example.com".into()), this_mac: false, build: Some("1d199a1".into()), link: Some(MacLink::Asleep { since: now - 3 * 3600 }) },
            ],
            phones: vec![
                PairedPhone { id: "phone-1".into(), name: "iPhone".into(), created_at: now - 9 * 86400, last_seen_at: Some(now - 4 * 60) },
                PairedPhone { id: "phone-2".into(), name: "iPad".into(), created_at: now - 2 * 86400, last_seen_at: None },
            ],
            signing: 0,
            seen_running: false,
            presses: 0,
        }
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
            CloudflareState::SigningIn { .. } => Some(CloudflareState::Running { address: self.setup.address.clone().unwrap_or_default() }),
            CloudflareState::Running { address } if self.seen_running => Some(CloudflareState::Down { address: address.clone(), since, why: "Cloudflare closed the connection: the tunnel's credentials were refused.".into() }),
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
            return Err("Cloudflare did not answer: the download of cloudflared timed out.".into());
        }
        let login = "https://dash.cloudflare.com/argotunnel?aud=&callback=https%3A%2F%2Flogin.cloudflareaccess.org%2Fmock".to_string();
        self.setup.address = Some(MOCK_ADDRESS.into());
        self.setup.cloudflare = CloudflareState::SigningIn { login: login.clone() };
        self.signing = 1;
        Ok(Tunnel::SignIn(login))
    }

    pub fn unpair(&mut self, id: &str) -> Result<(), String> {
        self.up()?;
        let before = self.phones.len();
        self.phones.retain(|one| one.id != id);
        if self.phones.len() == before { Err("No phone is paired with that id.".into()) } else { Ok(()) }
    }
}
