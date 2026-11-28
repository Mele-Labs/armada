//! A native walk: a screen of the native Bridge a Job shows with `show_window` on
//! `armada://walk/<name>`, opened by the person's press.
//!
//! **Only `open_native_walk` starts the app**, and only Bridge calls it, on a press. The binary is
//! the native Bridge Fleet built from the Job's own commit (`crate::walk_builds`), so a Job's new
//! screen needs no preview refresh first; `show_window` accepts it and the row says Building until
//! that build opens. A row written before walks were built from a Job's branch has no build, and
//! runs from the kept build his preview restart left. **Fleet owns the process**:
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

/// The kept build he runs: `Armada/builds/current`.
pub(crate) fn running_build(home: &str) -> PathBuf {
    crate::runtime::support_dir(home).join(crate::kept_builds::BUILDS).join(crate::kept_builds::CURRENT)
}

/// The walks the native Bridge at `app` plays (`--list-walks`), or why it cannot say.
pub(crate) async fn walks_of(app: &Path) -> Result<Vec<String>, String> {
    if !app.is_file() {
        return Err(format!("there is no native Bridge at {}", app.display()));
    }
    let listed = tokio::process::Command::new(app)
        .arg("--list-walks")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(std::time::Duration::from_secs(10), listed).await {
        Ok(Ok(out)) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).lines().map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect()),
        Ok(Ok(_)) => Err("the native Bridge built cannot name its walks: it is older than walks".to_string()),
        Ok(Err(why)) => Err(format!("the native Bridge built could not be asked for its walks: {why}")),
        Err(_) => Err("the native Bridge built did not name its walks within 10 seconds".to_string()),
    }
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

/// A commit as the Job and he read it.
fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
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
    /// The build it was started from: a kept build's time, or a commit Fleet built.
    built: String,
    child: tokio::process::Child,
}

/// The native apps Fleet started and has not stopped, one per Job and address.
#[derive(Default)]
pub(crate) struct NativeWalks {
    held: Mutex<Vec<Held>>,
}

impl NativeWalks {
    /// Start the app `spawn` makes for this Job and address, **or leave the one already running**
    /// from this build: `true` where this call started it. One running from another build is
    /// stopped and started again, so a press opens the Job's latest walk without a quit first.
    /// One lock holds the look and the start, so two presses at once start one.
    fn started_once(
        &self,
        session: &str,
        url: &str,
        built: &str,
        spawn: impl FnOnce() -> std::io::Result<tokio::process::Child>,
    ) -> std::io::Result<bool> {
        let mut held = self.held.lock().expect("not poisoned");
        // One the person closed on its own has ended, and is started again by the next press.
        held.retain_mut(|one| one.child.try_wait().map(|ended| ended.is_none()).unwrap_or(false));
        if let Some(at) = held.iter().position(|one| one.session == session && one.url == url) {
            if held[at].built == built {
                return Ok(false);
            }
            stop(held.remove(at));
        }
        let child = spawn()?;
        held.push(Held { session: session.to_string(), url: url.to_string(), built: built.to_string(), child });
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
        // A walk is a tab of the Job, and a Job may hold several open: the newest on this address.
        let Some((variations, built)) = rows.iter().rev().find_map(|row| match row {
            SessionRow::Window { url: shown, ended_at: None, variations, build, .. } if *shown == url => Some((variations, build)),
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
        let kept = running_build(&home);
        // The Job's own build where the row has one, and only once it is Ready: his one press opens it.
        let (binary, directory, made) = match built {
            None => {
                let binary = kept.join(APP);
                let made = std::fs::metadata(&binary).and_then(|meta| meta.modified()).map_err(|_| {
                    self.hosted_refusal(NOT_BUILT, &format!("the build he runs has no native Bridge at {}", binary.display()))
                })?;
                let nanos = made.duration_since(std::time::UNIX_EPOCH).map(|since| since.as_nanos()).unwrap_or(0);
                (binary, kept.clone(), format!("kept:{nanos}"))
            }
            Some(build) => match build.state {
                ipc::WalkBuildState::Building => {
                    return Err(self.hosted_refusal(NOT_BUILT, &format!("{url} is still building from commit {}", short(&build.commit))));
                }
                ipc::WalkBuildState::Failed => {
                    let why = build.why.as_deref().unwrap_or("it did not build");
                    return Err(self.hosted_refusal(NOT_BUILT, &format!("{url} did not build from commit {}: {why}", short(&build.commit))));
                }
                ipc::WalkBuildState::Ready => {
                    let (_, hosting) = self.session_and_hosting(&id).await?;
                    let key = crate::walk_builds::key_of(&hosting.manifest_id);
                    let binary = crate::walk_builds::binary_of(&home, &key, &build.commit);
                    if !binary.is_file() {
                        // Kept no longer: it is built again, and the press waits for that.
                        self.walk_rebuild_asked(&id, &url).await;
                        return Err(self.hosted_refusal(NOT_BUILT, &format!("the build of {} is no longer kept, so {url} is building again", short(&build.commit))));
                    }
                    let directory = binary.parent().expect("inside its folder").to_path_buf();
                    (binary, directory, format!("commit:{}", build.commit))
                }
            },
        };
        let port = self.host().port.to_string();
        let started = self
            .native_walks()
            .started_once(&id, &url, &made, || {
                // Wrapped only when it starts: a press that finds it running must not relink the
                // executable a walk still starting is reading.
                let program = wrapped(&crate::runtime::support_dir(&home), &kept, &binary);
                tokio::process::Command::new(&program)
                    .args(["--walk", name, "--job", &id, "--url", &url, "--fleet-port", &port])
                    .args(&takes)
                    .current_dir(&directory)
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
