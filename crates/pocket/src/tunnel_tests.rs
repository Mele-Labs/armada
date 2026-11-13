//! The tunnel's setup rows, its press and its runner, against a `cloudflared` the test names.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

use crate::cloudflared::{account_tunnels, matches, pick, signed_with, zone_named, Release};
use crate::pairing::ADDRESS_FILE;
use crate::tunnel::{connected, label_of, newer, AccountTunnel, Cloudflared, NoZone, SignIn, Tunnel};
use crate::{Gateway, WayIn};

static DIRS: AtomicU32 = AtomicU32::new(0);

fn machine() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pocket-tunnel-{}-{}", std::process::id(), DIRS.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[derive(Default)]
pub(crate) struct Fake {
    version: Mutex<Option<String>>,
    signed_in: AtomicBool,
    /// The sign-in the next press opens: its page, and the wait the test ends by sending.
    sign_in: Mutex<Option<(String, mpsc::Receiver<Result<(), String>>)>>,
    create_fails: Mutex<Option<String>>,
    /// Why the next zone lookup fails. `None` answers `example.com`.
    no_zone: Mutex<Option<NoZone>>,
    created: Mutex<Vec<(String, String)>>,
    /// The shell line the runner runs in place of `cloudflared`.
    runs: Mutex<Option<String>>,
    installs: AtomicU32,
    /// The account's tunnels as Cloudflare would list them, or why it could not be read.
    pub(crate) account: Mutex<Option<Result<Vec<AccountTunnel>, String>>>,
}

impl Cloudflared for Fake {
    fn version(&self) -> Option<String> {
        self.version.lock().unwrap().clone()
    }

    fn latest(&self) -> Result<String, String> {
        Ok("2026.11.0".into())
    }

    fn install(&self) -> Result<String, String> {
        self.installs.fetch_add(1, Ordering::Relaxed);
        *self.version.lock().unwrap() = Some("2026.11.0".into());
        Ok("2026.11.0".into())
    }

    fn signed_in(&self) -> bool {
        self.signed_in.load(Ordering::Relaxed)
    }

    fn sign_in(&self) -> Result<SignIn, String> {
        let (page, wait) = self.sign_in.lock().unwrap().take().ok_or("no sign-in in this test")?;
        Ok(SignIn { page, finished: Box::new(move || wait.recv().unwrap_or(Err("gone".into()))) })
    }

    fn zone(&self) -> Result<String, NoZone> {
        self.no_zone.lock().unwrap().clone().map_or(Ok("example.com".into()), Err)
    }

    fn create(&self, name: &str, host: &str) -> Result<(), String> {
        if let Some(why) = self.create_fails.lock().unwrap().clone() {
            return Err(why);
        }
        self.created.lock().unwrap().push((name.into(), host.into()));
        Ok(())
    }

    fn run(&self, _name: &str, _port: u16, _metrics: u16) -> Command {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(self.runs.lock().unwrap().clone().unwrap_or_else(|| "exit 0".into()));
        command
    }

    fn tunnels(&self) -> Result<Vec<AccountTunnel>, String> {
        self.account.lock().unwrap().clone().unwrap_or_else(|| Err("This Mac is not signed in to Cloudflare yet.".into()))
    }
}

/// A tunnel on `fake`, on a machine where this Mac made the tunnel `own` where it names one.
pub(crate) fn fake_tunnel(fake: Arc<Fake>, own: Option<&str>) -> Tunnel {
    let dir = machine();
    if let Some(own) = own {
        std::fs::create_dir_all(dir.join(crate::tunnel::DIR)).unwrap();
        std::fs::write(dir.join(crate::tunnel::DIR).join("tunnel"), format!("{own}\nmac.example.com\n")).unwrap();
    }
    Tunnel::new(fake, dir, Arc::new(|| 0))
}

pub(crate) fn quiet_tunnel() -> Tunnel {
    Tunnel::new(Arc::new(Fake::default()), machine(), Arc::new(|| 0))
}

fn gateway(tunnel: Tunnel) -> Gateway {
    Gateway { way_in: WayIn { tunnel, ..crate::setup::quiet_way_in() }, ..crate::pairing_tests::rig().gateway }
}

async fn call(gateway: &Gateway, method: Method, path: &str, body: &str) -> (StatusCode, String) {
    let request = Request::builder().method(method).uri(path).header("content-type", "application/json").body(Body::from(body.to_string())).unwrap();
    let answer = crate::router(gateway.clone()).oneshot(request).await.unwrap();
    let status = answer.status();
    (status, String::from_utf8(to_bytes(answer.into_body(), 1 << 16).await.unwrap().to_vec()).unwrap())
}

async fn cloudflare_row(gateway: &Gateway) -> String {
    let (code, body) = call(gateway, Method::GET, "/admin/setup", "").await;
    assert_eq!(code, StatusCode::OK, "{body}");
    body
}

async fn until(mut done: impl FnMut() -> bool) {
    let by = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < by, "waited 10 s");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// The press carries nothing: the domain comes from the sign-in.
const PRESS: &str = "";

#[tokio::test]
async fn the_setup_row_says_each_step_as_the_mac_has_it() {
    let fake = Arc::new(Fake::default());
    let dir = machine();
    let tunnel = Tunnel::new(fake.clone(), dir.clone(), Arc::new(|| 1_000));
    let gateway = gateway(tunnel.clone());
    assert!(cloudflare_row(&gateway).await.contains(r#""cloudflare":{"state":"missing"}"#));

    *fake.version.lock().unwrap() = Some("2026.10.1".into());
    let body = cloudflare_row(&gateway).await;
    assert!(body.contains(r#""cloudflare":{"state":"signed_out"}"#) && body.contains(r#""cloudflared":"2026.10.1""#), "{body}");

    fake.signed_in.store(true, Ordering::Relaxed);
    assert!(cloudflare_row(&gateway).await.contains(r#""cloudflare":{"state":"no_tunnel"}"#));

    std::fs::create_dir_all(dir.join("cloudflared")).unwrap();
    std::fs::write(dir.join("cloudflared/tunnel"), "armada-abc\nstudio.example.com\n").unwrap();
    let body = cloudflare_row(&gateway).await;
    assert!(body.contains(r#""state":"down","address":"https://studio.example.com","since":1000,"why":"Not connected to Cloudflare yet.""#), "{body}");

    tunnel.mark(true, None);
    assert!(cloudflare_row(&gateway).await.contains(r#""cloudflare":{"state":"running","address":"https://studio.example.com"}"#));
}

#[tokio::test]
async fn one_press_downloads_waits_for_the_sign_in_then_makes_the_tunnel_and_its_address() {
    let fake = Arc::new(Fake::default());
    let (signed, wait) = mpsc::channel();
    *fake.sign_in.lock().unwrap() = Some(("https://dash.example/argotunnel?x=1".into(), wait));
    let dir = machine();
    let gateway = gateway(Tunnel::new(fake.clone(), dir.clone(), Arc::new(|| 0)));

    let (code, body) = call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await;
    assert_eq!((code, body.as_str()), (StatusCode::OK, r#"{"login":"https://dash.example/argotunnel?x=1"}"#));
    assert_eq!(fake.installs.load(Ordering::Relaxed), 1, "cloudflared was downloaded first");
    assert!(cloudflare_row(&gateway).await.contains(r#""state":"signing_in","login":"https://dash.example/argotunnel?x=1""#));
    let again = call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await;
    assert_eq!(again.1, body, "a second press shows the same page");

    fake.signed_in.store(true, Ordering::Relaxed);
    signed.send(Ok(())).unwrap();
    until(|| !fake.created.lock().unwrap().is_empty()).await;
    let made = fake.created.lock().unwrap()[0].clone();
    let id = crate::machine::this_mac(&gateway).unwrap().id;
    assert_eq!(made.0, format!("armada-{}", &id[..12]), "named for this Mac's id");
    assert!(made.1.ends_with(".example.com"), "under the domain the sign-in picked: {made:?}");
    until(|| dir.join(ADDRESS_FILE).is_file()).await;
    assert_eq!(std::fs::read_to_string(dir.join(ADDRESS_FILE)).unwrap(), format!("https://{}\n", made.1), "the Mac's way in");
    until(|| !std::fs::read_to_string(dir.join("cloudflared/tunnel")).unwrap_or_default().is_empty()).await;
}

#[tokio::test]
async fn a_signed_in_mac_makes_the_tunnel_at_once_and_a_refusal_says_why() {
    let fake = Arc::new(Fake { version: Mutex::new(Some("2026.11.0".into())), signed_in: AtomicBool::new(true), ..Fake::default() });
    let gateway = gateway(Tunnel::new(fake.clone(), machine(), Arc::new(|| 0)));
    *fake.create_fails.lock().unwrap() = Some("Cloudflare would not point studio.example.com at the tunnel: record exists".into());
    let (code, body) = call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await;
    assert_eq!(code, StatusCode::BAD_GATEWAY);
    assert!(body.contains("record exists"));
    assert!(cloudflare_row(&gateway).await.contains(r#""state":"no_tunnel","why":"Cloudflare would not point studio.example.com at the tunnel: record exists""#));

    *fake.create_fails.lock().unwrap() = None;
    assert_eq!(call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await.0, StatusCode::NO_CONTENT);
    assert_eq!(fake.installs.load(Ordering::Relaxed), 0, "nothing downloaded where Armada has it");
}

#[tokio::test]
async fn a_lookup_that_fails_says_what_to_press_and_an_unusable_sign_in_is_done_again() {
    let fake = Arc::new(Fake { version: Mutex::new(Some("2026.11.0".into())), signed_in: AtomicBool::new(true), ..Fake::default() });
    let gateway = gateway(Tunnel::new(fake.clone(), machine(), Arc::new(|| 0)));

    *fake.no_zone.lock().unwrap() = Some(NoZone::Retry("Cloudflare could not be reached to read the domain.".into()));
    let (code, body) = call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await;
    assert_eq!((code, body.as_str()), (StatusCode::BAD_GATEWAY, "Cloudflare could not be reached to read the domain. Press Set up to try again."));
    assert!(cloudflare_row(&gateway).await.contains(r#""state":"no_tunnel","why":"Cloudflare could not be reached to read the domain. Press Set up to try again.""#));
    assert!(fake.created.lock().unwrap().is_empty());

    *fake.no_zone.lock().unwrap() = Some(NoZone::SignIn("The Cloudflare sign-in on this Mac names no domain.".into()));
    call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await;
    assert!(cloudflare_row(&gateway).await.contains(r#""why":"The Cloudflare sign-in on this Mac names no domain. Press Set up to sign in again.""#));

    // Still signed in as far as cloudflared knows, yet the next press opens the sign-in again.
    let (signed, wait) = mpsc::channel();
    *fake.sign_in.lock().unwrap() = Some(("https://dash.example/argotunnel?again".into(), wait));
    *fake.no_zone.lock().unwrap() = None;
    let (code, body) = call(&gateway, Method::POST, "/admin/setup/cloudflare", PRESS).await;
    assert_eq!((code, body.as_str()), (StatusCode::OK, r#"{"login":"https://dash.example/argotunnel?again"}"#));
    signed.send(Ok(())).unwrap();
    until(|| !fake.created.lock().unwrap().is_empty()).await;
    assert!(fake.created.lock().unwrap()[0].1.ends_with(".example.com"));
}

fn made_tunnel(runs: &str) -> (Arc<Fake>, Tunnel) {
    let fake = Arc::new(Fake { version: Mutex::new(Some("2026.11.0".into())), signed_in: AtomicBool::new(true), ..Fake::default() });
    *fake.runs.lock().unwrap() = Some(runs.into());
    let dir = machine();
    std::fs::create_dir_all(dir.join("cloudflared")).unwrap();
    std::fs::write(dir.join("cloudflared/tunnel"), "armada-abc\nstudio.example.com\n").unwrap();
    let tunnel = Tunnel::new(fake.clone(), dir, Arc::new(|| 5));
    (fake, tunnel)
}

#[tokio::test]
async fn a_tunnel_that_stops_reads_down_with_cloudflareds_own_words() {
    let (_fake, tunnel) = made_tunnel("echo '2026-11-12T10:00:00Z ERR Unauthorized: invalid tunnel secret' >&2; exit 1");
    tunnel.start(8443);
    until(|| matches!(tunnel.state().0, ipc::CloudflareState::Down { ref why, .. } if why == "Unauthorized: invalid tunnel secret")).await;
    tunnel.stop();
}

#[tokio::test]
async fn stopping_the_tunnel_ends_its_cloudflared() {
    let pid_file = machine().join("pid");
    let (_fake, tunnel) = made_tunnel(&format!("echo $$ > '{}'; exec sleep 30", pid_file.display()));
    tunnel.start(8443);
    until(|| std::fs::read_to_string(&pid_file).is_ok_and(|pid| !pid.trim().is_empty())).await;
    let pid = std::fs::read_to_string(&pid_file).unwrap().trim().to_string();
    let alive = |pid: &str| Command::new("/bin/ps").args(["-p", pid]).output().is_ok_and(|out| out.status.success());
    assert!(alive(&pid));
    tunnel.stop();
    until(|| !alive(&pid)).await;
    until(|| matches!(tunnel.state().0, ipc::CloudflareState::Down { ref why, .. } if why == "The Phone Gateway is off.")).await;
}

#[test]
fn connected_is_read_from_cloudflareds_ready_answer() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let serving = std::thread::spawn(move || {
        for answer in ["HTTP/1.1 200 OK\r\n\r\n", "HTTP/1.1 503 Service Unavailable\r\n\r\n"] {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0u8; 256]);
            stream.write_all(answer.as_bytes()).unwrap();
        }
    });
    assert!(connected(port));
    assert!(!connected(port));
    serving.join().unwrap();
    assert!(!connected(port), "nothing listening");
}

#[test]
fn the_address_and_the_version_are_read_as_cloudflare_has_them() {
    assert_eq!(label_of("Nick's MacBook Pro"), "nicks-macbook-pro");
    assert_eq!(label_of("Studio"), "studio");
    assert_eq!(label_of("  ***  "), "mac");
    assert!(newer("2026.11.0", "2026.10.1"));
    assert!(newer("2026.11.10", "2026.11.9"));
    assert!(!newer("2026.11.0", "2026.11.0"));
}

/// A sign-in certificate as `cloudflared tunnel login` writes it, with a made-up token.
fn cert(token: &str) -> String {
    use base64::Engine;
    let block = base64::engine::general_purpose::STANDARD.encode(token);
    let wrapped: Vec<&str> = block.as_bytes().chunks(64).map(|line| std::str::from_utf8(line).unwrap()).collect();
    format!("-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n-----BEGIN ARGO TUNNEL TOKEN-----\n{}\n-----END ARGO TUNNEL TOKEN-----\n", wrapped.join("\n"))
}

#[test]
fn the_domain_is_read_from_the_sign_in_and_cloudflares_answer() {
    let signed = signed_with(&cert(r#"{"zoneID":"0123abcd","accountID":"acc","apiToken":"made-up-token"}"#)).unwrap();
    assert_eq!((signed.zone_id.as_str(), signed.api_token.as_str()), ("0123abcd", "made-up-token"));
    assert!(signed_with("-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n").is_none(), "no token block");
    assert!(signed_with(&cert(r#"{"accountID":"acc","apiToken":"t"}"#)).is_none(), "no zone");
    assert!(signed_with(&cert(r#"{"zoneID":"../x","apiToken":"t"}"#)).is_none(), "a zone id that is not one");

    assert_eq!(zone_named(br#"{"success":true,"result":{"id":"0123abcd","name":"Example.com","status":"active"}}"#).as_deref(), Some("example.com"));
    assert_eq!(zone_named(br#"{"success":false,"errors":[{"code":7003}],"result":null}"#), None);
    assert_eq!(zone_named(b"not json"), None);
}

const RELEASE: &str = r#"{"tag_name":"2026.11.0","assets":[
  {"name":"cloudflared-darwin-amd64.tgz","browser_download_url":"https://dl.example/amd64.tgz","digest":"sha256:8A59D51120CEBBE902F0A8D6E1B7A7FD4F9A5BB4DCB3E0F05872483E75E7DB6B"},
  {"name":"cloudflared-darwin-arm64.tgz","browser_download_url":"https://dl.example/arm64.tgz"}
]}"#;

#[test]
fn a_download_is_taken_only_with_the_sha256_its_release_publishes() {
    let release: Release = ipc::decode("release", RELEASE.as_bytes()).unwrap();
    let (asset, sha) = pick(&release, "x86_64").unwrap();
    assert_eq!(asset.browser_download_url, "https://dl.example/amd64.tgz");
    assert_eq!(sha, "8a59d51120cebbe902f0a8d6e1b7a7fd4f9a5bb4dcb3e0f05872483e75e7db6b");
    assert!(pick(&release, "aarch64").unwrap_err().contains("publishes no SHA-256"));
    assert!(pick(&release, "riscv64").is_err());
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert!(matches(b"abc", abc));
    assert!(!matches(b"abd", abc));
}

#[tokio::test]
async fn the_devices_list_names_this_mac_with_its_one_colour_and_build() {
    let gateway = Gateway {
        way_in: WayIn { build: Some("68b554e7e5".into()), ..crate::setup::quiet_way_in() },
        ..crate::pairing_tests::rig().gateway
    };
    let (code, first) = call(&gateway, Method::GET, "/admin/macs", "").await;
    assert_eq!(code, StatusCode::OK, "{first}");
    let macs: Vec<ipc::Mac> = ipc::decode("macs", first.as_bytes()).unwrap();
    assert_eq!(macs.len(), 1);
    assert!(macs[0].this_mac && macs[0].link.is_none());
    assert_eq!(macs[0].build.as_deref(), Some("68b554e7e5"));
    assert_eq!(macs[0].address.as_deref(), Some("https://mac.tail.ts.net"));
    assert_eq!(call(&gateway, Method::GET, "/admin/macs", "").await.1, first, "minted once");
    assert_eq!(call(&gateway, Method::GET, "/admin/macs", "").await.0, StatusCode::OK);
    let tunnelled = crate::router(gateway).oneshot(Request::get("/admin/macs").header("cf-ray", "1").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(tunnelled.status(), StatusCode::FORBIDDEN, "never through the tunnel");
}

#[test]
fn the_accounts_tunnels_are_read_with_whether_each_is_connected_and_the_address_routed_to_it() {
    let tunnels = br#"{"success":true,"result":[
        {"id":"aaaa-1111","name":"armada-37b968e614c8","status":"healthy","connections":[{}]},
        {"id":"bbbb-2222","name":"armada-0f0f0f0f0f0f","status":"down"},
        {"id":"cccc-3333","name":"armada-123456789abc","status":"degraded"},
        {"id":"dddd-4444","name":"his-own","status":"inactive"}]}"#;
    let records = br#"{"success":true,"result":[
        {"name":"Studio.Example.com","type":"CNAME","content":"aaaa-1111.cfargotunnel.com"},
        {"name":"work.example.com","type":"CNAME","content":"bbbb-2222.cfargotunnel.com"},
        {"name":"www.example.com","type":"CNAME","content":"example.com"}]}"#;
    let read = account_tunnels(tunnels, records).unwrap();
    let tunnel = |name: &str, connected: bool, address: Option<&str>| AccountTunnel { name: name.into(), connected, address: address.map(str::to_string) };
    assert_eq!(
        read,
        vec![
            tunnel("armada-37b968e614c8", true, Some("https://studio.example.com")),
            tunnel("armada-0f0f0f0f0f0f", false, Some("https://work.example.com")),
            tunnel("armada-123456789abc", true, None),
            tunnel("his-own", false, None),
        ]
    );
    assert_eq!(account_tunnels(br#"{"success":false,"result":null}"#, records), None, "a refusal lists nothing");
    let signed = signed_with(&cert(r#"{"zoneID":"0123abcd","accountID":"acc","apiToken":"made-up-token"}"#)).unwrap();
    assert_eq!(signed.account_id, "acc", "the account its tunnels are listed on");
}

#[test]
fn the_macs_setup_says_its_step_and_the_one_press_that_moves_it_on() {
    use ipc::{MacSetup, MacSetupStep};
    use crate::mac_setup::setup_of;
    let fake = Arc::new(Fake::default());
    let dir = machine();
    let tunnel = Tunnel::new(fake.clone(), dir.clone(), Arc::new(|| 1_000));
    let setup = |step, working, login: Option<&str>, why: Option<&str>, address: Option<&str>| MacSetup {
        step,
        working,
        login: login.map(str::to_string),
        why: why.map(str::to_string),
        address: address.map(str::to_string),
    };
    assert_eq!(setup_of(&tunnel, 1_000), setup(MacSetupStep::SignIn, false, None, None, None), "not pressed yet: press Set up");

    tunnel.live.lock().unwrap().pressing = true;
    assert_eq!(setup_of(&tunnel, 1_000), setup(MacSetupStep::SignIn, true, None, None, None), "downloading: nothing to press");
    tunnel.live.lock().unwrap().pressing = false;
    tunnel.live.lock().unwrap().failed = Some("cloudflared could not be downloaded: offline".into());
    let stuck = setup_of(&tunnel, 1_000);
    assert_eq!(stuck, setup(MacSetupStep::SignIn, false, None, Some("cloudflared could not be downloaded: offline"), None));

    *fake.version.lock().unwrap() = Some("2026.11.0".into());
    tunnel.live.lock().unwrap().failed = None;
    tunnel.live.lock().unwrap().signing_in = Some("https://dash.example/argotunnel".into());
    let login = setup(MacSetupStep::SignIn, false, Some("https://dash.example/argotunnel"), None, None);
    assert_eq!(setup_of(&tunnel, 1_000), login, "signing in: open the page");

    tunnel.live.lock().unwrap().signing_in = None;
    tunnel.live.lock().unwrap().making = true;
    fake.signed_in.store(true, Ordering::Relaxed);
    assert_eq!(setup_of(&tunnel, 1_000), setup(MacSetupStep::Tunnel, true, None, None, None), "making the tunnel after the sign-in");
    tunnel.live.lock().unwrap().making = false;

    std::fs::create_dir_all(dir.join("cloudflared")).unwrap();
    std::fs::write(dir.join("cloudflared/tunnel"), "armada-abc\nstudio.example.com\n").unwrap();
    tunnel.mark(false, Some("dial tcp: timeout".into()));
    assert_eq!(setup_of(&tunnel, 1_010), setup(MacSetupStep::Tunnel, true, None, None, None), "connecting, never red at once");
    let late = 1_000 + crate::mac_setup::CONNECTING;
    assert_eq!(setup_of(&tunnel, late), setup(MacSetupStep::Tunnel, false, None, Some("dial tcp: timeout"), None), "stuck: press again");

    tunnel.mark(true, None);
    assert_eq!(setup_of(&tunnel, late), setup(MacSetupStep::Done, false, None, None, Some("https://studio.example.com")));
}
