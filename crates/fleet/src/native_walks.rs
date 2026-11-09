//! A native walk: a screen of the native Bridge a Job shows with `show_window` on
//! `armada://walk/<name>`, opened by the person's press and run from the Job's own slot.
//!
//! **Only `open_native_walk` starts the app**, and only Bridge calls it, on a press. The binary is
//! the slot's own build, never an installed app. **Fleet owns the process**: it is stopped when the
//! walk ends, when the Job ends, when the slot is given back and when Fleet stops. Never written
//! down, for `crate::servers`' reason, but unlike a server it does not outlive Fleet: it is a
//! window on this Fleet's port.

use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{NativeWalkOpened, OpenNativeWalk, SessionRow};

use crate::daemon::Fleet;

/// What a native walk's address begins with.
pub(crate) const PREFIX: &str = "armada://walk/";

/// Where the slot's build is, from the slot.
const BINARY: &str = "crates/bridge-native/target/debug/bridge-native";

/// The app the walk runs as, beside the build: macOS names the menu bar and the Dock from the
/// bundle a process runs from, so a bare binary said "bridge-native" there.
const BUNDLE: &str = "crates/bridge-native/target/debug/walk/Armada.app";

/// The brand icon, from the slot.
const ICON: &str = "packages/brand/macos/AppIcon.icns";

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

/// The slot's build wrapped as `Armada.app`, its binary a hard link so nothing is copied, or the
/// bare build where the wrapping fails: a walk named wrong in the menu bar still walks.
fn wrapped(slot: &Path, binary: &Path) -> std::path::PathBuf {
    let bundle = slot.join(BUNDLE);
    let executable = bundle.join("Contents/MacOS/Armada");
    let made = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(executable.parent().expect("inside the bundle"))?;
        std::fs::create_dir_all(bundle.join("Contents/Resources"))?;
        std::fs::write(bundle.join("Contents/Info.plist"), INFO)?;
        let icon = slot.join(ICON);
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
    slot: String,
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
        slot: &str,
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
        held.push(Held { session: session.to_string(), url: url.to_string(), slot: slot.to_string(), built, child });
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

    /// Every walk run from the slot at `path`: the slot is being given back.
    pub(crate) fn stopped_in(&self, path: &str) {
        self.stopped_where(|one| one.slot == path);
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

    /// `open_native_walk`: start the Job's own build of the native app for the walk it has open on
    /// `open.url`, **or answer that it is already running** from that build.
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
        let (_, hosting) = self.session_and_hosting(&id).await?;
        if hosting.lease_slot.is_none() {
            return Err(self.hosted_refusal(NOT_BUILT, "this job holds no slot, so it has no build to walk"));
        }
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
        let slot = Self::directory_of(&served, &hosting);
        let binary = Path::new(&slot).join(BINARY);
        let build_first = || self.hosted_refusal(NOT_BUILT, &format!("Build the walk first: cargo build in crates/bridge-native in {slot}"));
        let made = std::fs::metadata(&binary).and_then(|meta| meta.modified()).map_err(|_| build_first())?;
        let committed = {
            let (vcs, slot) = (std::sync::Arc::clone(self.vcs()), slot.clone());
            tokio::task::spawn_blocking(move || vcs.head_committed_at(&slot)).await.ok().flatten()
        };
        let built = made.duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs() as i64).unwrap_or(0);
        if committed.is_some_and(|committed| built < committed) {
            return Err(build_first());
        }
        let port = self.host().port.to_string();
        let program = wrapped(Path::new(&slot), &binary);
        let started = self
            .native_walks()
            .started_once(&id, &url, &slot, made, || {
                tokio::process::Command::new(&program)
                    .args(["--walk", name, "--job", &id, "--url", &url, "--fleet-port", &port])
                    .args(&takes)
                    .current_dir(&slot)
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
