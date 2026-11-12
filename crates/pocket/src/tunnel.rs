//! The Mac's Cloudflare tunnel: a way in Armada sets up itself, on the owner's own Cloudflare account and
//! domain. `cloudflared` holds an outbound connection to Cloudflare and hands whatever arrives at the Mac's
//! address to the Gateway on loopback, so the Mac takes calls without listening on any port.
//!
//! **One press does the whole setup.** It downloads `cloudflared` where Armada has none, opens the sign-in,
//! and once the owner has signed in makes the tunnel and its address and writes the address as the Mac's way
//! in. Fleet runs the tunnel while the Gateway runs, and starts it again when it stops.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ipc::CloudflareState;

use crate::pairing::{Clock, ADDRESS_FILE};

/// Armada's own `cloudflared`, its sign-in and the tunnel it made, under the machine directory.
pub const DIR: &str = "cloudflared";
/// The tunnel this Mac made: its name on the account, then its host, a line each.
const RECORD: &str = "tunnel";

/// How often the runner looks at its `cloudflared`, and so how soon a stop takes.
const TICK: Duration = Duration::from_secs(1);
/// How many ticks between asks of `cloudflared` whether it is connected.
const PROBE_EVERY: u32 = 5;
/// How often a running tunnel asks whether a newer `cloudflared` is out.
const UPDATE_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// The longest wait before starting a `cloudflared` that stopped again.
const LONGEST_REST: Duration = Duration::from_secs(60);

/// What the sign-in press started: the page to sign in on, and the wait for it.
pub struct SignIn {
    pub page: String,
    /// Blocks until the owner has signed in, or the sign-in ended without it.
    pub finished: Box<dyn FnOnce() -> Result<(), String> + Send>,
}

/// `cloudflared` as the Gateway uses it. Blocking. Injected, so a test names the answers.
pub trait Cloudflared: Send + Sync {
    /// The version Armada keeps, or `None` where it has not downloaded one.
    fn version(&self) -> Option<String>;
    /// The newest version released.
    fn latest(&self) -> Result<String, String>;
    /// Download the newest version, check it is Cloudflare's, and keep it. The version kept.
    fn install(&self) -> Result<String, String>;
    fn signed_in(&self) -> bool;
    fn sign_in(&self) -> Result<SignIn, String>;
    /// Make the tunnel `name`, or take this Mac's of that name, and point `host` at it.
    fn create(&self, name: &str, host: &str) -> Result<(), String>;
    /// The command that runs the tunnel `name` in front of the Gateway's `port`, answering whether it is
    /// connected on `metrics`.
    fn run(&self, name: &str, port: u16, metrics: u16) -> Command;
}

/// What a press on the setup row did.
#[derive(Debug, PartialEq, Eq)]
pub enum Pressed {
    /// The tunnel is made and the runner takes it up.
    Made,
    /// The owner signs in on this page, and the tunnel is made once he has.
    SignIn(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// The domain is not one a host can go under. The sentence says what was wrong.
    Domain(String),
    /// A press is already under way.
    Busy,
    Failed(String),
}

/// This Mac as the press needs it: its id names the tunnel, its name the address.
pub struct ForMac<'a> {
    pub id: &'a str,
    pub name: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Made {
    name: String,
    host: String,
}

#[derive(Default)]
struct Live {
    signing_in: Option<String>,
    /// The last press's failure, until a press succeeds.
    failed: Option<String>,
    pressing: bool,
    connected: bool,
    /// When `connected` last changed.
    since: i64,
    /// Why it is not connected, in `cloudflared`'s words where it said.
    why: Option<String>,
}

struct Runner {
    stop: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct Tunnel {
    driver: Arc<dyn Cloudflared>,
    machine: PathBuf,
    clock: Clock,
    live: Arc<Mutex<Live>>,
    running: Arc<Mutex<Option<Runner>>>,
}

/// `example.com` from what the owner typed, which may carry a scheme or a trailing slash.
pub(crate) fn domain_from(typed: &str) -> Result<String, String> {
    let bare = typed.trim().trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_ascii_lowercase();
    let labels: Vec<&str> = bare.split('.').collect();
    let fits = |label: &&str| {
        !label.is_empty() && label.len() <= 63 && !label.starts_with('-') && !label.ends_with('-') && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    if labels.len() >= 2 && labels.iter().all(fits) && bare.len() <= 200 {
        Ok(bare)
    } else {
        Err(format!("{:?} is not a domain such as example.com. Use one whose DNS is on your Cloudflare account.", typed.trim()))
    }
}

/// The first label of this Mac's address, from its name: `studio` from `Studio`, `nicks-mac` from `Nick's Mac`.
pub(crate) fn label_of(name: &str) -> String {
    let mut label = String::new();
    for c in name.trim().to_ascii_lowercase().chars() {
        match c {
            'a'..='z' | '0'..='9' => label.push(c),
            '\'' | '\u{2019}' => {}
            _ if !label.is_empty() && !label.ends_with('-') => label.push('-'),
            _ => {}
        }
    }
    let label: String = label.trim_end_matches('-').chars().take(40).collect();
    match label.trim_end_matches('-') {
        "" => "mac".to_string(),
        fitted => fitted.to_string(),
    }
}

/// Whether `latest` is a newer version than `kept`, read as `cloudflared` numbers them: `2026.11.0`.
pub(crate) fn newer(latest: &str, kept: &str) -> bool {
    let parts = |v: &str| v.split('.').map(|n| n.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parts(latest) > parts(kept)
}

/// Whether the `cloudflared` at `metrics` says it holds a connection to Cloudflare.
pub(crate) fn connected(metrics: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, metrics)), Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    if stream.write_all(b"GET /ready HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").is_err() {
        return false;
    }
    let mut head = [0u8; 16];
    let read = stream.read(&mut head).unwrap_or(0);
    let head = String::from_utf8_lossy(&head[..read]);
    head.starts_with("HTTP/1.0 200") || head.starts_with("HTTP/1.1 200")
}

fn free_port() -> Option<u16> {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).ok()?.local_addr().ok().map(|a| a.port())
}

impl Tunnel {
    pub fn new(driver: Arc<dyn Cloudflared>, machine: PathBuf, clock: Clock) -> Tunnel {
        Tunnel { driver, machine, clock, live: Arc::default(), running: Arc::default() }
    }

    fn record(&self) -> PathBuf {
        self.machine.join(DIR).join(RECORD)
    }

    fn made(&self) -> Option<Made> {
        let text = std::fs::read_to_string(self.record()).ok()?;
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        Some(Made { name: lines.next()?.to_string(), host: lines.next()?.to_string() })
    }

    /// The setup row as it stands, and the version of `cloudflared` Armada keeps.
    pub fn state(&self) -> (CloudflareState, Option<String>) {
        let version = self.driver.version();
        let live = self.live.lock().unwrap();
        let state = match (&version, &live.signing_in) {
            (None, _) => CloudflareState::Missing,
            (_, Some(page)) => CloudflareState::SigningIn { login: page.clone() },
            _ if !self.driver.signed_in() => CloudflareState::SignedOut,
            _ => match self.made() {
                None => CloudflareState::NoTunnel { why: live.failed.clone() },
                Some(made) if live.connected => CloudflareState::Running { address: format!("https://{}", made.host) },
                Some(made) => CloudflareState::Down {
                    address: format!("https://{}", made.host),
                    since: if live.since == 0 { (self.clock)() } else { live.since },
                    why: live.why.clone().unwrap_or_else(|| "Not connected to Cloudflare yet.".to_string()),
                },
            },
        };
        (state, version)
    }

    /// The setup press: download, sign in and make the tunnel, as far as each is not done.
    pub fn press(&self, typed_domain: &str, mac: ForMac<'_>) -> Result<Pressed, Refused> {
        let domain = domain_from(typed_domain).map_err(Refused::Domain)?;
        {
            let mut live = self.live.lock().unwrap();
            if let Some(page) = &live.signing_in {
                return Ok(Pressed::SignIn(page.clone()));
            }
            if live.pressing {
                return Err(Refused::Busy);
            }
            live.pressing = true;
        }
        let done = self.pressed(domain, mac.id.to_string(), mac.name.to_string());
        let mut live = self.live.lock().unwrap();
        live.pressing = false;
        live.failed = done.as_ref().err().cloned();
        done.map_err(Refused::Failed)
    }

    fn pressed(&self, domain: String, id: String, name: String) -> Result<Pressed, String> {
        if self.driver.version().is_none() {
            self.driver.install()?;
        }
        if self.driver.signed_in() {
            return self.make(&domain, &id, &name).map(|()| Pressed::Made);
        }
        let SignIn { page, finished } = self.driver.sign_in()?;
        self.live.lock().unwrap().signing_in = Some(page.clone());
        let tunnel = self.clone();
        std::thread::spawn(move || {
            let made = finished().and_then(|()| tunnel.make(&domain, &id, &name));
            let mut live = tunnel.live.lock().unwrap();
            live.signing_in = None;
            live.failed = made.err();
        });
        Ok(Pressed::SignIn(page))
    }

    /// The tunnel is named for this Mac's id, so two Macs that share a name never take each other's.
    fn make(&self, domain: &str, id: &str, name: &str) -> Result<(), String> {
        let made = Made { name: format!("armada-{}", &id[..id.len().min(12)]), host: format!("{}.{domain}", label_of(name)) };
        self.driver.create(&made.name, &made.host)?;
        let dir = self.machine.join(DIR);
        let kept = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(self.record(), format!("{}\n{}\n", made.name, made.host)))
            .and_then(|()| std::fs::write(self.machine.join(ADDRESS_FILE), format!("https://{}\n", made.host)));
        kept.map_err(|why| format!("The tunnel is made, but Armada could not keep it in {}: {why}", dir.display()))
    }

    /// Run the tunnel in front of the Gateway's `port` until [`Tunnel::stop`], once there is one to run.
    pub fn start(&self, port: u16) {
        let mut running = self.running.lock().unwrap();
        if running.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let (tunnel, flag) = (self.clone(), stop.clone());
        std::thread::spawn(move || tunnel.keep_up(port, &flag));
        *running = Some(Runner { stop });
    }

    /// Stop running the tunnel. Its `cloudflared` is ended within a tick.
    pub fn stop(&self) {
        if let Some(runner) = self.running.lock().unwrap().take() {
            runner.stop.store(true, Ordering::Relaxed);
        }
    }

    pub(crate) fn mark(&self, connected: bool, why: Option<String>) {
        let mut live = self.live.lock().unwrap();
        if live.connected != connected || live.since == 0 {
            live.since = (self.clock)();
        }
        live.connected = connected;
        live.why = why;
    }

    /// Rest `for_`, a tick at a time so a stop still takes within one. `false` once stopped.
    fn rest(stop: &AtomicBool, for_: Duration) -> bool {
        let until = Instant::now() + for_;
        while Instant::now() < until {
            if stop.load(Ordering::Relaxed) {
                return false;
            }
            std::thread::sleep(TICK.min(until.saturating_duration_since(Instant::now())));
        }
        !stop.load(Ordering::Relaxed)
    }

    /// Installs a newer `cloudflared` where one is out. `true` where it did, so the runner restarts on it.
    fn update(&self) -> bool {
        let (Some(kept), Ok(latest)) = (self.driver.version(), self.driver.latest()) else { return false };
        if !newer(&latest, &kept) {
            return false;
        }
        match self.driver.install() {
            Ok(_) => true,
            Err(why) => {
                eprintln!("cloudflared {latest} not installed: {why}");
                false
            }
        }
    }

    fn keep_up(&self, port: u16, stop: &AtomicBool) {
        let mut rest = Duration::from_secs(1);
        let mut checked: Option<Instant> = None;
        while !stop.load(Ordering::Relaxed) {
            let Some(made) = self.made().filter(|_| self.driver.version().is_some()) else {
                if !Self::rest(stop, PROBE_EVERY * TICK) {
                    break;
                }
                continue;
            };
            if checked.is_none_or(|at| at.elapsed() >= UPDATE_EVERY) {
                checked = Some(Instant::now());
                self.update();
            }
            let connected_once = self.run_once(&made, port, stop, &mut checked);
            rest = if connected_once { Duration::from_secs(1) } else { (rest * 2).min(LONGEST_REST) };
            if !Self::rest(stop, rest) {
                break;
            }
        }
        self.mark(false, Some("The Phone Gateway is off.".to_string()));
    }

    /// One `cloudflared`, from start to its end. `true` where it connected at some point.
    fn run_once(&self, made: &Made, port: u16, stop: &AtomicBool, checked: &mut Option<Instant>) -> bool {
        let Some(metrics) = free_port() else {
            self.mark(false, Some("No free port on loopback for cloudflared to answer on.".to_string()));
            return false;
        };
        let mut command = self.driver.run(&made.name, port, metrics);
        let spawned = command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(why) => {
                self.mark(false, Some(format!("cloudflared could not start: {why}")));
                return false;
            }
        };
        let said = Arc::new(Mutex::new(None::<String>));
        if let Some(pipe) = child.stderr.take() {
            let said = said.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    if line.contains(" ERR ") {
                        *said.lock().unwrap() = Some(line.split_once(" ERR ").map_or(line.as_str(), |(_, rest)| rest).trim().to_string());
                    }
                }
            });
        }
        self.mark(false, None);
        let mut ever = false;
        let mut ticks = 0u32;
        loop {
            if stop.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return ever;
            }
            if let Ok(Some(status)) = child.try_wait() {
                let words = said.lock().unwrap().clone().unwrap_or_else(|| format!("cloudflared stopped ({status})."));
                self.mark(false, Some(words));
                return ever;
            }
            if ticks % PROBE_EVERY == 0 {
                let up = connected(metrics);
                ever |= up;
                self.mark(up, if up { None } else { said.lock().unwrap().clone() });
            }
            ticks = ticks.wrapping_add(1);
            if checked.is_some_and(|at| at.elapsed() >= UPDATE_EVERY) {
                *checked = Some(Instant::now());
                if self.update() {
                    let _ = child.kill();
                    let _ = child.wait();
                    return true;
                }
            }
            std::thread::sleep(TICK);
        }
    }
}
