//! The tunnel's setup rows, its press and its runner, against a `cloudflared` the test names.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

use crate::cloudflared::{matches, pick, Release};
use crate::pairing::ADDRESS_FILE;
use crate::tunnel::{connected, domain_from, label_of, newer, Cloudflared, SignIn, Tunnel};
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
    created: Mutex<Vec<(String, String)>>,
    /// The shell line the runner runs in place of `cloudflared`.
    runs: Mutex<Option<String>>,
    installs: AtomicU32,
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

const DOMAIN: &str = r#"{"domain":"https://Example.com/"}"#;

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

    let (code, body) = call(&gateway, Method::POST, "/admin/setup/cloudflare", DOMAIN).await;
    assert_eq!((code, body.as_str()), (StatusCode::OK, r#"{"login":"https://dash.example/argotunnel?x=1"}"#));
    assert_eq!(fake.installs.load(Ordering::Relaxed), 1, "cloudflared was downloaded first");
    assert!(cloudflare_row(&gateway).await.contains(r#""state":"signing_in","login":"https://dash.example/argotunnel?x=1""#));
    let again = call(&gateway, Method::POST, "/admin/setup/cloudflare", DOMAIN).await;
    assert_eq!(again.1, body, "a second press shows the same page");

    fake.signed_in.store(true, Ordering::Relaxed);
    signed.send(Ok(())).unwrap();
    until(|| !fake.created.lock().unwrap().is_empty()).await;
    let made = fake.created.lock().unwrap()[0].clone();
    let id = crate::machine::this_mac(&gateway).unwrap().id;
    assert_eq!(made.0, format!("armada-{}", &id[..12]), "named for this Mac's id");
    assert!(made.1.ends_with(".example.com"), "{made:?}");
    until(|| dir.join(ADDRESS_FILE).is_file()).await;
    assert_eq!(std::fs::read_to_string(dir.join(ADDRESS_FILE)).unwrap(), format!("https://{}\n", made.1), "the Mac's way in");
    until(|| !std::fs::read_to_string(dir.join("cloudflared/tunnel")).unwrap_or_default().is_empty()).await;
}

#[tokio::test]
async fn a_signed_in_mac_makes_the_tunnel_at_once_and_a_refusal_says_why() {
    let fake = Arc::new(Fake { version: Mutex::new(Some("2026.11.0".into())), signed_in: AtomicBool::new(true), ..Fake::default() });
    let gateway = gateway(Tunnel::new(fake.clone(), machine(), Arc::new(|| 0)));
    assert_eq!(call(&gateway, Method::POST, "/admin/setup/cloudflare", r#"{"domain":"not a domain"}"#).await.0, StatusCode::UNPROCESSABLE_ENTITY);

    *fake.create_fails.lock().unwrap() = Some("Cloudflare would not point studio.example.com at the tunnel: record exists".into());
    let (code, body) = call(&gateway, Method::POST, "/admin/setup/cloudflare", DOMAIN).await;
    assert_eq!(code, StatusCode::BAD_GATEWAY);
    assert!(body.contains("record exists"));
    assert!(cloudflare_row(&gateway).await.contains(r#""state":"no_tunnel","why":"Cloudflare would not point studio.example.com at the tunnel: record exists""#));

    *fake.create_fails.lock().unwrap() = None;
    assert_eq!(call(&gateway, Method::POST, "/admin/setup/cloudflare", DOMAIN).await.0, StatusCode::NO_CONTENT);
    assert_eq!(fake.installs.load(Ordering::Relaxed), 0, "nothing downloaded where Armada has it");
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
fn the_domain_the_address_and_the_version_are_read_as_cloudflare_has_them() {
    assert_eq!(domain_from(" https://Armada.Example/ ").as_deref(), Ok("armada.example"));
    for wrong in ["example", "exa mple.com", "-a.com", "a..com", ""] {
        assert!(domain_from(wrong).is_err(), "{wrong:?}");
    }
    assert_eq!(label_of("Nick's MacBook Pro"), "nicks-macbook-pro");
    assert_eq!(label_of("Studio"), "studio");
    assert_eq!(label_of("  ***  "), "mac");
    assert!(newer("2026.11.0", "2026.10.1"));
    assert!(newer("2026.11.10", "2026.11.9"));
    assert!(!newer("2026.11.0", "2026.11.0"));
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
