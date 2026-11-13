//! Armada's own `cloudflared`: downloaded, checked, kept current and run with a home of its own.
//!
//! **A download is kept only when two things hold.** The archive's SHA-256 is the digest the release
//! publishes for it, and the binary inside is signed with Cloudflare's Developer ID. The second check
//! does not trust the forge the release is published on.
//!
//! **It never touches the owner's own `~/.cloudflared`.** Every `cloudflared` runs with `HOME` set to
//! `cloudflared/home` in the machine directory, so its sign-in certificate and the tunnel's credentials
//! are Armada's alone, and a tunnel the owner runs himself is left as it is.
//!
//! **The domain is read from the sign-in.** The certificate `cloudflared tunnel login` writes carries the zone
//! the owner picked and a token that may read it. The token is read for the one call that names the zone and
//! is kept nowhere else.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::setup::page_in;
use crate::tunnel::{AccountTunnel, Cloudflared, NoZone, SignIn, DIR};

/// The newest release, as the forge's API describes it.
const LATEST: &str = "https://api.github.com/repos/cloudflare/cloudflared/releases/latest";
/// Cloudflare's Apple team. A binary signed by any other is refused.
const CLOUDFLARE_TEAM: &str = "68WVV388M8";
/// Cloudflare's zones, each read as `<ZONES>/<zone id>`.
const ZONES: &str = "https://api.cloudflare.com/client/v4/zones";
/// Cloudflare's accounts, whose tunnels are read as `<ACCOUNTS>/<account id>/cfd_tunnel`.
const ACCOUNTS: &str = "https://api.cloudflare.com/client/v4/accounts";
/// What a host routed to a tunnel names, after the tunnel's id.
const TUNNEL_TARGET: &str = ".cfargotunnel.com";
/// The sign-in certificate's block that carries the zone and its token, as base64 JSON.
const TOKEN_BLOCK: &str = "ARGO TUNNEL TOKEN";
/// How long the press waits for Cloudflare to name the zone.
const ZONE_WAIT: Duration = Duration::from_secs(20);
/// How long the sign-in press waits for `cloudflared` to name the page.
const SIGN_IN_WAIT: Duration = Duration::from_secs(20);
/// How long a download may take.
const DOWNLOAD_WAIT: Duration = Duration::from_secs(5 * 60);

#[derive(Deserialize)]
pub(crate) struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Asset {
    pub name: String,
    pub browser_download_url: String,
    /// `sha256:<hex>`. Absent on releases published before the forge kept digests, which are refused.
    #[serde(default)]
    pub digest: Option<String>,
}

/// What the sign-in certificate carries. No `Debug`, so the token is never printed.
#[derive(Deserialize)]
pub(crate) struct Signed {
    #[serde(rename = "zoneID")]
    pub zone_id: String,
    #[serde(rename = "apiToken")]
    pub api_token: String,
    /// The account the sign-in is on. Older sign-ins may not carry it.
    #[serde(rename = "accountID", default)]
    pub account_id: String,
}

/// The zone and its token from a sign-in certificate's text, where it carries a zone id that is one.
pub(crate) fn signed_with(cert: &str) -> Option<Signed> {
    use base64::Engine;
    let begin = format!("-----BEGIN {TOKEN_BLOCK}-----");
    let end = format!("-----END {TOKEN_BLOCK}-----");
    let block: String = cert.lines().skip_while(|line| line.trim() != begin).skip(1).take_while(|line| line.trim() != end).map(str::trim).collect();
    let json = base64::engine::general_purpose::STANDARD.decode(block).ok()?;
    let signed: Signed = ipc::decode("the Cloudflare sign-in's token", &json).ok()?;
    let fits = !signed.zone_id.is_empty() && signed.zone_id.bytes().all(|b| b.is_ascii_alphanumeric()) && !signed.api_token.is_empty();
    fits.then_some(signed)
}

#[derive(Deserialize)]
struct ZoneAnswer {
    result: Option<ZoneName>,
}

#[derive(Deserialize)]
struct ZoneName {
    name: String,
}

/// The domain in Cloudflare's answer about a zone, where it names one.
pub(crate) fn zone_named(answer: &[u8]) -> Option<String> {
    let answer: ZoneAnswer = ipc::decode("Cloudflare's zone", answer).ok()?;
    Some(answer.result?.name.trim().to_ascii_lowercase()).filter(|name| name.contains('.'))
}

#[derive(Deserialize)]
struct TunnelsAnswer {
    result: Option<Vec<TunnelRow>>,
}

#[derive(Deserialize)]
struct TunnelRow {
    id: String,
    name: String,
    #[serde(default)]
    status: String,
}

#[derive(Deserialize)]
struct RecordsAnswer {
    result: Option<Vec<RecordRow>>,
}

#[derive(Deserialize)]
struct RecordRow {
    name: String,
    content: String,
}

/// The tunnels in Cloudflare's answer about an account's tunnels, each with the host its answer about the
/// zone's records routes to it. `healthy` and `degraded` are connected; `inactive` and `down` are not.
pub(crate) fn account_tunnels(tunnels: &[u8], records: &[u8]) -> Option<Vec<AccountTunnel>> {
    let tunnels: TunnelsAnswer = ipc::decode("Cloudflare's tunnels", tunnels).ok()?;
    let records: RecordsAnswer = ipc::decode("Cloudflare's records", records).ok()?;
    let records = records.result.unwrap_or_default();
    let host = |id: &str| {
        let target = format!("{id}{TUNNEL_TARGET}");
        let record = records.iter().find(|record| record.content.eq_ignore_ascii_case(&target))?;
        Some(format!("https://{}", record.name.trim().to_ascii_lowercase()))
    };
    let rows = tunnels.result?;
    Some(
        rows.into_iter()
            .map(|row| AccountTunnel { address: host(&row.id), connected: matches!(row.status.as_str(), "healthy" | "degraded"), name: row.name })
            .collect(),
    )
}

/// One read of Cloudflare's API with the sign-in's token: the status and the bytes.
fn read_api(url: String, token: String) -> Result<(u16, Vec<u8>), String> {
    blocking(async move {
        let unreached = |why: reqwest::Error| format!("Cloudflare could not be reached: {}.", why.without_url());
        let client = reqwest::Client::builder().user_agent("armada").timeout(ZONE_WAIT).build().map_err(unreached)?;
        let answer = client.get(&url).bearer_auth(&token).send().await.map_err(unreached)?;
        let status = answer.status().as_u16();
        Ok((status, answer.bytes().await.map(|b| b.to_vec()).map_err(unreached)?))
    })
}

/// The archive built for this Mac's processor.
pub(crate) fn asset_name(arch: &str) -> Option<&'static str> {
    match arch {
        "aarch64" => Some("cloudflared-darwin-arm64.tgz"),
        "x86_64" => Some("cloudflared-darwin-amd64.tgz"),
        _ => None,
    }
}

/// The archive for `arch` in `release`, and the SHA-256 the release publishes for it, as hex.
pub(crate) fn pick<'a>(release: &'a Release, arch: &str) -> Result<(&'a Asset, String), String> {
    let wanted = asset_name(arch).ok_or_else(|| format!("cloudflared publishes no build for this Mac's processor ({arch})."))?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == wanted)
        .ok_or_else(|| format!("cloudflared {} has no {wanted}.", release.tag_name))?;
    let digest = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .filter(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| format!("cloudflared {} publishes no SHA-256 for {wanted}, so Armada cannot check it.", release.tag_name))?;
    Ok((asset, digest.to_ascii_lowercase()))
}

/// Whether `bytes` are the ones the release vouched for.
pub(crate) fn matches(bytes: &[u8], sha256_hex: &str) -> bool {
    crate::pairing::to_hex(&Sha256::digest(bytes)) == sha256_hex
}

/// Runs `fetch` on a runtime of its own, on a thread of its own, so it may be called from anywhere.
fn blocking<T: Send + 'static>(fetch: impl std::future::Future<Output = Result<T, String>> + Send + 'static) -> Result<T, String> {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|why| format!("Armada could not start a download: {why}"))?
            .block_on(fetch)
    })
    .join()
    .unwrap_or_else(|_| Err("The download stopped unexpectedly.".to_string()))
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("armada")
        .timeout(DOWNLOAD_WAIT)
        .build()
        .map_err(|why| format!("Armada could not start a download: {why}"))
}

fn latest_release() -> Result<Release, String> {
    let bytes = blocking(async {
        let answer = client()?.get(LATEST).send().await.map_err(|why| format!("cloudflared's releases could not be read: {why}"))?;
        if !answer.status().is_success() {
            return Err(format!("cloudflared's releases answered {}.", answer.status()));
        }
        answer.bytes().await.map(|b| b.to_vec()).map_err(|why| format!("cloudflared's releases could not be read: {why}"))
    })?;
    ipc::decode("cloudflared's latest release", &bytes).map_err(|why| why.to_string())
}

fn said(output: &std::process::Output) -> String {
    let mut words = String::from_utf8_lossy(&output.stdout).into_owned();
    words.push_str(&String::from_utf8_lossy(&output.stderr));
    words.trim().to_string()
}

/// The `cloudflared` Armada keeps under `machine`.
pub struct MacCloudflared {
    dir: PathBuf,
}

impl MacCloudflared {
    pub fn new(machine: &Path) -> MacCloudflared {
        MacCloudflared { dir: machine.join(DIR) }
    }

    fn binary(&self) -> PathBuf {
        self.dir.join("bin").join("cloudflared")
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn cert(&self) -> PathBuf {
        self.home().join(".cloudflared").join("cert.pem")
    }

    fn credentials(&self) -> PathBuf {
        self.dir.join("tunnel.json")
    }

    /// `cloudflared tunnel <args>` with Armada's home and sign-in.
    fn tunnel(&self, args: &[&str]) -> Command {
        let mut command = Command::new(self.binary());
        command.env("HOME", self.home()).arg("tunnel").arg("--origincert").arg(self.cert()).args(args);
        command
    }

    fn ran(&self, args: &[&str]) -> Result<String, String> {
        let output = self
            .tunnel(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|why| format!("cloudflared could not start: {why}"))?;
        match output.status.success() {
            true => Ok(said(&output)),
            false => Err(said(&output)),
        }
    }

    /// Checks `archive` against `sha256`, unpacks it in a folder of its own, checks the signature, and
    /// moves the binary into place. A `cloudflared` already running keeps the file it opened.
    fn keep(&self, archive: &[u8], sha256: &str, version: &str) -> Result<(), String> {
        if !matches(archive, sha256) {
            return Err(format!("The cloudflared {version} download does not match the SHA-256 its release publishes, so Armada did not keep it."));
        }
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let unpack = self.dir.join(format!("download-{}-{stamp}", std::process::id()));
        let kept = (|| {
            std::fs::create_dir_all(&unpack).map_err(|why| format!("Armada could not make {}: {why}", unpack.display()))?;
            let tgz = unpack.join("cloudflared.tgz");
            std::fs::write(&tgz, archive).map_err(|why| format!("Armada could not write the download: {why}"))?;
            let untar = Command::new("/usr/bin/tar").arg("-xzf").arg(&tgz).arg("-C").arg(&unpack).output().map_err(|why| format!("tar could not start: {why}"))?;
            if !untar.status.success() {
                return Err(format!("The cloudflared download could not be unpacked: {}", said(&untar)));
            }
            let fresh = unpack.join("cloudflared");
            if !std::fs::symlink_metadata(&fresh).is_ok_and(|m| m.is_file()) {
                return Err("The cloudflared download holds no cloudflared.".to_string());
            }
            let requirement = format!("=anchor apple generic and certificate leaf[subject.OU] = \"{CLOUDFLARE_TEAM}\"");
            let signed = Command::new("/usr/bin/codesign").args(["--verify", "--strict", "-R"]).arg(&requirement).arg(&fresh).output().map_err(|why| format!("codesign could not start: {why}"))?;
            if !signed.status.success() {
                return Err(format!("The cloudflared {version} download is not signed by Cloudflare, so Armada did not keep it: {}", said(&signed)));
            }
            let bin = self.binary();
            let folder = bin.parent().unwrap_or(&self.dir);
            std::fs::create_dir_all(folder).map_err(|why| format!("Armada could not make {}: {why}", folder.display()))?;
            std::fs::rename(&fresh, &bin).map_err(|why| format!("Armada could not move cloudflared into place: {why}"))?;
            std::fs::write(folder.join("version"), format!("{version}\n")).map_err(|why| format!("Armada could not note cloudflared's version: {why}"))
        })();
        let _ = std::fs::remove_dir_all(&unpack);
        kept
    }
}

impl Cloudflared for MacCloudflared {
    fn version(&self) -> Option<String> {
        if !self.binary().is_file() {
            return None;
        }
        let text = std::fs::read_to_string(self.dir.join("bin").join("version")).ok()?;
        Some(text.trim().to_string()).filter(|v| !v.is_empty())
    }

    fn latest(&self) -> Result<String, String> {
        latest_release().map(|release| release.tag_name)
    }

    fn install(&self) -> Result<String, String> {
        let release = latest_release()?;
        let (asset, sha256) = pick(&release, std::env::consts::ARCH)?;
        let url = asset.browser_download_url.clone();
        let archive = blocking(async move {
            let answer = client()?.get(&url).send().await.map_err(|why| format!("cloudflared could not be downloaded: {why}"))?;
            if !answer.status().is_success() {
                return Err(format!("cloudflared's download answered {}.", answer.status()));
            }
            answer.bytes().await.map(|b| b.to_vec()).map_err(|why| format!("cloudflared could not be downloaded: {why}"))
        })?;
        self.keep(&archive, &sha256, &release.tag_name)?;
        Ok(release.tag_name)
    }

    fn signed_in(&self) -> bool {
        self.cert().is_file()
    }

    /// `cloudflared tunnel login`, with no way to open a browser itself: the page goes to Bridge's button,
    /// so it opens once, where the owner pressed.
    fn sign_in(&self) -> Result<SignIn, String> {
        // `cloudflared tunnel login` will not write over a certificate, and a sign-in is asked for only to
        // replace one that is missing or names no usable domain.
        let _ = std::fs::remove_file(self.cert());
        let no_browser = self.dir.join("no-browser");
        std::fs::create_dir_all(&no_browser).and_then(|()| std::fs::create_dir_all(self.home())).map_err(|why| format!("Armada could not make {}: {why}", self.dir.display()))?;
        let mut child = Command::new(self.binary())
            .env("HOME", self.home())
            .env("PATH", &no_browser)
            .args(["tunnel", "login"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|why| format!("cloudflared could not start: {why}"))?;
        let (lines, heard) = mpsc::channel::<String>();
        let pipes: [Option<Box<dyn Read + Send>>; 2] =
            [child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>)];
        for pipe in pipes.into_iter().flatten() {
            let lines = lines.clone();
            std::thread::spawn(move || BufReader::new(pipe).lines().map_while(Result::ok).for_each(|line| drop(lines.send(line))));
        }
        drop(lines);
        let until = Instant::now() + SIGN_IN_WAIT;
        let mut words = String::new();
        let page = loop {
            if let Some(page) = page_in(&words, None) {
                break page;
            }
            match heard.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Ok(line) => (words.push_str(&line), words.push('\n')).1,
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(match words.trim() {
                        "" => "cloudflared did not name a page to sign in on.".to_string(),
                        said => format!("cloudflared did not name a page to sign in on: {said}"),
                    });
                }
            }
        };
        // The readers keep draining the pipes once no one listens.
        drop(heard);
        let cert = self.cert();
        let finished = Box::new(move || {
            match child.wait() {
                Ok(status) if status.success() && cert.is_file() => Ok(()),
                _ => Err("The Cloudflare sign-in ended before it finished. Press Set up to sign in again.".to_string()),
            }
        });
        Ok(SignIn { page, finished })
    }

    fn zone(&self) -> Result<String, NoZone> {
        let cert = std::fs::read_to_string(self.cert()).unwrap_or_default();
        let Some(signed) = signed_with(&cert) else {
            return Err(NoZone::SignIn("The Cloudflare sign-in on this Mac names no domain.".to_string()));
        };
        let url = format!("{ZONES}/{}", signed.zone_id);
        let (status, answer) = blocking(async move {
            let unreached = |why: reqwest::Error| format!("Cloudflare could not be reached to read the domain: {}.", why.without_url());
            let client = reqwest::Client::builder().user_agent("armada").timeout(ZONE_WAIT).build().map_err(unreached)?;
            let answer = client.get(&url).bearer_auth(&signed.api_token).send().await.map_err(unreached)?;
            let status = answer.status();
            Ok((status, answer.bytes().await.map(|b| b.to_vec()).map_err(unreached)?))
        })
        .map_err(NoZone::Retry)?;
        match zone_named(&answer) {
            Some(domain) if status.is_success() => Ok(domain),
            _ if matches!(status.as_u16(), 400 | 401 | 403 | 404) => {
                Err(NoZone::SignIn(format!("Cloudflare would not name the domain this Mac signed in with ({status}).")))
            }
            _ => Err(NoZone::Retry(format!("Cloudflare answered {status} when asked for the domain."))),
        }
    }

    fn create(&self, name: &str, host: &str) -> Result<(), String> {
        let credentials = self.credentials();
        let credentials = credentials.to_string_lossy();
        if let Err(said) = self.ran(&["create", "--credentials-file", &credentials, name]) {
            if !said.contains("already exists") {
                return Err(format!("Cloudflare would not make the tunnel: {said}"));
            }
            // Made by this Mac before, since its name is this Mac's id: fetch its credentials again.
            self.ran(&["token", "--cred-file", &credentials, name]).map_err(|said| format!("Cloudflare would not give the tunnel's credentials: {said}"))?;
        }
        self.ran(&["route", "dns", name, host]).map(drop).map_err(|said| format!("Cloudflare would not point {host} at the tunnel: {said}"))
    }

    fn tunnels(&self) -> Result<Vec<AccountTunnel>, String> {
        let cert = std::fs::read_to_string(self.cert()).unwrap_or_default();
        let signed = signed_with(&cert).ok_or_else(|| "This Mac is not signed in to Cloudflare yet.".to_string())?;
        if signed.account_id.is_empty() || !signed.account_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err("The Cloudflare sign-in on this Mac names no account. Press Set up to sign in again.".to_string());
        }
        let refused = |what: &str, status: u16| format!("Cloudflare would not list {what} ({status}).");
        let (status, tunnels) =
            read_api(format!("{ACCOUNTS}/{}/cfd_tunnel?is_deleted=false&per_page=100", signed.account_id), signed.api_token.clone())?;
        if !(200..300).contains(&status) {
            return Err(refused("the tunnels on this account", status));
        }
        let (status, records) = read_api(format!("{ZONES}/{}/dns_records?type=CNAME&per_page=500", signed.zone_id), signed.api_token)?;
        if !(200..300).contains(&status) {
            return Err(refused("the addresses in this domain", status));
        }
        account_tunnels(&tunnels, &records).ok_or_else(|| "Cloudflare answered in a way Armada cannot read.".to_string())
    }

    fn run(&self, name: &str, port: u16, metrics: u16) -> Command {
        let credentials = self.credentials();
        let mut command = self.tunnel(&["--no-autoupdate", "--metrics", &format!("127.0.0.1:{metrics}"), "run"]);
        command.arg("--credentials-file").arg(credentials).arg("--url").arg(format!("http://127.0.0.1:{port}")).arg(name);
        command
    }
}
