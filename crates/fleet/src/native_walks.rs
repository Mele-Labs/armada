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
use ipc::{NativeWalkOpened, OpenNativeWalk};

use crate::daemon::Fleet;

/// What a native walk's address begins with.
pub(crate) const PREFIX: &str = "armada://walk/";

/// Where the slot's build is, from the slot.
const BINARY: &str = "crates/bridge-native/target/debug/bridge-native";

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
    if !show.variations.is_empty() {
        Some("a native walk takes no variations: it is one screen")
    } else if !show.origins.is_empty() {
        Some("a native walk takes no origins: it is not a web page")
    } else if !show.sign_in.is_empty() {
        Some("a native walk takes no sign_in: it is not a web page")
    } else if !show.steps.is_empty() {
        Some("a native walk's steps live in its walk file in crates/bridge-native")
    } else {
        None
    }
}

struct Held {
    session: String,
    url: String,
    slot: String,
    child: tokio::process::Child,
}

/// The native apps Fleet started and has not stopped, one per Job and address.
#[derive(Default)]
pub(crate) struct NativeWalks {
    held: Mutex<Vec<Held>>,
}

impl NativeWalks {
    /// Start the app `spawn` makes for this Job and address, **or leave the one already running**:
    /// `true` where this call started it. One lock holds the look and the start, so two presses at
    /// once start one.
    fn started_once(
        &self,
        session: &str,
        url: &str,
        slot: &str,
        spawn: impl FnOnce() -> std::io::Result<tokio::process::Child>,
    ) -> std::io::Result<bool> {
        let mut held = self.held.lock().expect("not poisoned");
        // One the person closed on its own has ended, and is started again by the next press.
        held.retain_mut(|one| one.child.try_wait().map(|ended| ended.is_none()).unwrap_or(false));
        if held.iter().any(|one| one.session == session && one.url == url) {
            return Ok(false);
        }
        let child = spawn()?;
        held.push(Held { session: session.to_string(), url: url.to_string(), slot: slot.to_string(), child });
        Ok(true)
    }

    fn stopped_where(&self, which: impl Fn(&Held) -> bool) {
        let gone: Vec<Held> = {
            let mut held = self.held.lock().expect("not poisoned");
            let (gone, kept) = std::mem::take(&mut *held).into_iter().partition(|one| which(one));
            *held = kept;
            gone
        };
        for mut one in gone {
            if let Some(group) = one.child.id() {
                // The group `process_group(0)` made, so what the app started goes with it.
                crate::session_shell::signal(group, "KILL");
            }
            let _ = one.child.start_kill();
        }
    }

    /// The walk on `url` ended: closed, withdrawn, approved or replaced.
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
    /// `open.url`, **or answer that it is already running**.
    pub(crate) async fn native_walk_opened(&self, open: OpenNativeWalk) -> Result<NativeWalkOpened, Refusal> {
        let (id, url) = (open.session_id.as_str().trim().to_string(), open.url.trim().to_string());
        let Some(name) = walk_name(&url) else {
            return Err(self.hosted_refusal(NOT_NATIVE, &format!("{url} is not a native walk: its address is armada://walk/<name>")));
        };
        let rows = self.rows_of(&id).await?;
        if crate::session_host::walking::open_walk(&rows) != Some(url.as_str()) {
            return Err(self.hosted_refusal(NOT_OPEN, &format!("this job has no open walk on {url}. Show it with show_window first")));
        }
        let (_, hosting) = self.session_and_hosting(&id).await?;
        if hosting.lease_slot.is_none() {
            return Err(self.hosted_refusal(NOT_BUILT, "this job holds no slot, so it has no build to walk"));
        }
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
        let slot = Self::directory_of(&served, &hosting);
        let binary = Path::new(&slot).join(BINARY);
        let build_first = || self.hosted_refusal(NOT_BUILT, &format!("Build the walk first: cargo build in crates/bridge-native in {slot}"));
        let built = std::fs::metadata(&binary).and_then(|meta| meta.modified()).map_err(|_| build_first())?;
        let committed = {
            let (vcs, slot) = (std::sync::Arc::clone(self.vcs()), slot.clone());
            tokio::task::spawn_blocking(move || vcs.head_committed_at(&slot)).await.ok().flatten()
        };
        let built = built.duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs() as i64).unwrap_or(0);
        if committed.is_some_and(|committed| built < committed) {
            return Err(build_first());
        }
        let port = self.host().port.to_string();
        let started = self
            .native_walks()
            .started_once(&id, &url, &slot, || {
                tokio::process::Command::new(&binary)
                    .args(["--walk", name, "--job", &id, "--url", &url, "--fleet-port", &port])
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
