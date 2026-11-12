//! Armada's own `cloudflared`: downloaded, checked, kept current and run with a home of its own.
//!
//! **A download is kept only when two things hold.** The archive's SHA-256 is the digest the release
//! publishes for it, and the binary inside is signed with Cloudflare's Developer ID. The second check
//! does not trust the forge the release is published on.
//!
//! **It never touches the owner's own `~/.cloudflared`.** Every `cloudflared` runs with `HOME` set to
//! `cloudflared/home` in the machine directory, so its sign-in certificate and the tunnel's credentials
//! are Armada's alone, and a tunnel the owner runs himself is left as it is.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::setup::page_in;
use crate::tunnel::{Cloudflared, SignIn, DIR};

/// The newest release, as the forge's API describes it.
const LATEST: &str = "https://api.github.com/repos/cloudflare/cloudflared/releases/latest";
/// Cloudflare's Apple team. A binary signed by any other is refused.
const CLOUDFLARE_TEAM: &str = "68WVV388M8";
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

    fn run(&self, name: &str, port: u16, metrics: u16) -> Command {
        let credentials = self.credentials();
        let mut command = self.tunnel(&["--no-autoupdate", "--metrics", &format!("127.0.0.1:{metrics}"), "run"]);
        command.arg("--credentials-file").arg(credentials).arg("--url").arg(format!("http://127.0.0.1:{port}")).arg(name);
        command
    }
}
