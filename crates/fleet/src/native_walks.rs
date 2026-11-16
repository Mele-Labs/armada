//! A native walk: a screen of the native Bridge a Job shows with `show_window` on
//! `armada://walk/<name>`, opened by the person's press and run from the build he runs.
//!
//! **Only `open_native_walk` starts the app**, and only Bridge calls it, on a press. The binary is
//! the native Bridge of the build his preview restart kept, never a Job's slot build: `show_window`
//! refuses a walk that build does not play, so the one press opens it. **Fleet owns the process**:
//! it is stopped when the walk ends, when the Job ends and when Fleet stops. Never written
//! down, for `crate::servers`' reason, but unlike a server it does not outlive Fleet: it is a
//! window on this Fleet's port.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{NativeWalkOpened, OpenNativeWalk, SessionRow};

use crate::daemon::Fleet;

/// What a native walk's address begins with.
pub(crate) const PREFIX: &str = "armada://walk/";

/// The native Bridge of the build he runs, under `Armada/builds/current`: the one his preview
/// restart kept, so a merged preview carries every offered branch's walks. Never a Job's slot.
const APP: &str = "Armada.app/Contents/MacOS/Armada";

/// The walk's own bundle, under the support folder: the same binary, hard-linked, under another
/// bundle id, so macOS keeps a walk's menu bar and Dock apart from his running native Bridge's.
const BUNDLE: &str = "walk/Armada.app";

/// The brand icon, in the kept app.
const ICON: &str = "Armada.app/Contents/Resources/AppIcon.icns";

/// What every refusal of a walk his build cannot play ends with: the way to one it can.
const OFFER: &str = "offer_preview the branch that adds it, and show the walk once he has refreshed his preview";

/// The kept build he runs: `Armada/builds/current`.
pub(crate) fn running_build(home: &str) -> PathBuf {
    crate::runtime::support_dir(home).join(crate::kept_builds::BUILDS).join(crate::kept_builds::CURRENT)
}

/// The walks the native Bridge at `app` plays (`--list-walks`), or why it cannot say.
async fn walks_of(app: &Path) -> Result<Vec<String>, String> {
    if !app.is_file() {
        return Err(format!("the build he runs has no native Bridge at {}", app.display()));
    }
    let listed = tokio::process::Command::new(app)
        .arg("--list-walks")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(std::time::Duration::from_secs(10), listed).await {
        Ok(Ok(out)) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).lines().map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect()),
        Ok(Ok(_)) => Err("the native Bridge in the build he runs is older than walks from his preview, so it cannot name its walks".to_string()),
        Ok(Err(why)) => Err(format!("the native Bridge in the build he runs could not be asked for its walks: {why}")),
        Err(_) => Err("the native Bridge in the build he runs did not name its walks within 10 seconds".to_string()),
    }
}

/// Why the build he runs cannot play `show`'s walk and its takes, or `None` where it can: the
/// Job is told, so he is never handed a walk that does not open. `home` is Fleet's.
pub(crate) async fn unplayable(home: &str, show: &ipc::ShowWindow) -> Option<String> {
    let app = running_build(home).join(APP);
    let known = match walks_of(&app).await {
        Ok(known) => known,
        Err(why) => return Some(format!("{why}: {OFFER}")),
    };
    std::iter::once(show.url.trim())
        .chain(show.variations.iter().map(|take| take.url.trim()))
        .filter_map(walk_name)
        .find(|name| !known.iter().any(|one| one == name))
        .map(|name| format!("the build he runs has no walk named {name}: {OFFER}"))
}

const INFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Armada</string>
  <key>CFBundleDisplayName</key><string>Armada</string>
  <key>CFBundleExecutable</key><string>Armada</string>
  <key>CFBundleIdentifier</key><string>dev.armada.bridge.native.walk</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#;

/// His build's binary wrapped as the walk's own `Armada.app`, a hard link so nothing is copied, or
/// the bare binary where the wrapping fails: a walk named wrong in the menu bar still walks.
fn wrapped(support: &Path, build: &Path, binary: &Path) -> PathBuf {
    let bundle = support.join(BUNDLE);
    let executable = bundle.join("Contents/MacOS/Armada");
    let made = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(executable.parent().expect("inside the bundle"))?;
        std::fs::create_dir_all(bundle.join("Contents/Resources"))?;
        std::fs::write(bundle.join("Contents/Info.plist"), INFO)?;
        let icon = build.join(ICON);
        if icon.is_file() {
            std::fs::copy(&icon, bundle.join("Contents/Resources/AppIcon.icns"))?;
        }
        let _ = std::fs::remove_file(&executable);
        std::fs::hard_link(binary, &executable)
    })();
    if made.is_ok() { executable } else { binary.to_path_buf() }
}

const NOT_NATIVE: &str = "fleet.native_walk_not_native";
const NOT_OPEN: &str = "fleet.native_walk_not_open";
const NOT_BUILT: &str = "fleet.native_walk_not_built";
const NOT_STARTED: &str = "fleet.native_walk_not_started";

/// The walk's name where `url` is a native walk's address: a slug of lowercase letters, digits and `-`.
pub(crate) fn walk_name(url: &str) -> Option<&str> {
    let name = url.strip_prefix(PREFIX)?;
    let slug = !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    slug.then_some(name)
}

/// Why a native walk's address cannot be shown, or `None` for one that can.
pub(crate) fn address_problem(url: &str) -> Option<&'static str> {
    (url.starts_with(PREFIX) && walk_name(url).is_none())
        .then_some("a native walk's address is armada://walk/<name>, the name made of lowercase letters, digits and -")
}

/// Why a native walk takes what `show_window` was passed, or `None` where it takes it.
pub(crate) fn takes_no(show: &ipc::ShowWindow) -> Option<&'static str> {
    if !show.origins.is_empty() {
        Some("a native walk takes no origins: it is not a web page")
    } else if !show.sign_in.is_empty() {
        Some("a native walk takes no sign_in: it is not a web page")
    } else if !show.steps.is_empty() {
        Some("a native walk's steps live in its walk file in crates/bridge-native")
    } else {
        None
    }
}

fn stop(mut one: Held) {
    if let Some(group) = one.child.id() {
        // The group `process_group(0)` made, so what the app started goes with it.
        crate::session_shell::signal(group, "KILL");
    }
    let _ = one.child.start_kill();
}

struct Held {
    session: String,
    url: String,
    /// When the build it was started from was made.
    built: std::time::SystemTime,
    child: tokio::process::Child,
}

/// The native apps Fleet started and has not stopped, one per Job and address.
#[derive(Default)]
pub(crate) struct NativeWalks {
    held: Mutex<Vec<Held>>,
}

impl NativeWalks {
    /// Start the app `spawn` makes for this Job and address, **or leave the one already running**
    /// from this build: `true` where this call started it. One running from an older build is
    /// stopped and started again, so a press opens the Job's latest walk without a quit first.
    /// One lock holds the look and the start, so two presses at once start one.
    fn started_once(
        &self,
        session: &str,
        url: &str,
        built: std::time::SystemTime,
        spawn: impl FnOnce() -> std::io::Result<tokio::process::Child>,
    ) -> std::io::Result<bool> {
        let mut held = self.held.lock().expect("not poisoned");
        // One the person closed on its own has ended, and is started again by the next press.
        held.retain_mut(|one| one.child.try_wait().map(|ended| ended.is_none()).unwrap_or(false));
        if let Some(at) = held.iter().position(|one| one.session == session && one.url == url) {
            if held[at].built >= built {
                return Ok(false);
            }
            stop(held.remove(at));
        }
        let child = spawn()?;
        held.push(Held { session: session.to_string(), url: url.to_string(), built, child });
        Ok(true)
    }

    fn stopped_where(&self, which: impl Fn(&Held) -> bool) {
        let gone: Vec<Held> = {
            let mut held = self.held.lock().expect("not poisoned");
            let (gone, kept) = std::mem::take(&mut *held).into_iter().partition(|one| which(one));
            *held = kept;
            gone
        };
        gone.into_iter().for_each(stop);
    }

    /// The walk on `url` ended: closed, withdrawn or approved.
    pub(crate) fn stopped_for_walk(&self, session: &str, url: &str) {
        self.stopped_where(|one| one.session == session && one.url == url);
    }

    /// Every walk of the Job: it ended, or showed another window.
    pub(crate) fn stopped_for_job(&self, session: &str) {
        self.stopped_where(|one| one.session == session);
    }

    /// Every walk: Fleet is stopping.
    pub(crate) fn stopped_all(&self) {
        self.stopped_where(|_| true);
    }

    #[cfg(test)]
    pub(crate) fn running(&self, session: &str, url: &str) -> bool {
        let mut held = self.held.lock().expect("not poisoned");
        held.retain_mut(|one| one.child.try_wait().map(|ended| ended.is_none()).unwrap_or(false));
        held.iter().any(|one| one.session == session && one.url == url)
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Fleet is stopping: every native walk it started goes with it.
    pub fn native_walks_stopped(&self) {
        self.native_walks().stopped_all();
    }

    /// `open_native_walk`: start the native Bridge of the build he runs on the walk the Job has open
    /// on `open.url`, **or answer that it is already running** from that build.
    pub(crate) async fn native_walk_opened(&self, open: OpenNativeWalk) -> Result<NativeWalkOpened, Refusal> {
        let (id, url) = (open.session_id.as_str().trim().to_string(), open.url.trim().to_string());
        let Some(name) = walk_name(&url) else {
            return Err(self.hosted_refusal(NOT_NATIVE, &format!("{url} is not a native walk: its address is armada://walk/<name>")));
        };
        let rows = self.rows_of(&id).await?;
        // A walk is a tab of the Job, and a Job may hold several open: any of them on this address.
        let Some(variations) = rows.iter().find_map(|row| match row {
            SessionRow::Window { url: shown, ended_at: None, variations, .. } if *shown == url => Some(variations),
            _ => None,
        }) else {
            return Err(self.hosted_refusal(NOT_OPEN, &format!("this job has no open walk on {url}. Show it with show_window first")));
        };
        // Each take as `--take <name>=<walk>`, for the bar's takes dropdown.
        let takes: Vec<String> = variations
            .iter()
            .filter_map(|take| walk_name(&take.url).map(|walk| format!("{}={walk}", take.name)))
            .flat_map(|take| ["--take".to_string(), take])
            .collect();
        let home = self.host().home.clone();
        let build = running_build(&home);
        let binary = build.join(APP);
        let made = std::fs::metadata(&binary).and_then(|meta| meta.modified()).map_err(|_| {
            self.hosted_refusal(NOT_BUILT, &format!("the build he runs has no native Bridge at {}", binary.display()))
        })?;
        let port = self.host().port.to_string();
        let program = wrapped(&crate::runtime::support_dir(&home), &build, &binary);
        let started = self
            .native_walks()
            .started_once(&id, &url, made, || {
                tokio::process::Command::new(&program)
                    .args(["--walk", name, "--job", &id, "--url", &url, "--fleet-port", &port])
                    .args(&takes)
                    .current_dir(&build)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0)
                    .kill_on_drop(true)
                    .spawn()
            })
            .map_err(|why| self.hosted_refusal(NOT_STARTED, &format!("the walk could not be started: {why}")))?;
        Ok(NativeWalkOpened { started })
    }
}
