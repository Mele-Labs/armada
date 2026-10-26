//! A checkout set up before a run starts on it: the Manifest's `setup.requires`, run at the
//! checkout's root in the file's order, **each Command once for each checkout and again when what
//! it was traced from changed**. `docs/concepts/manifest.md`, *Root armada.yml*.
//!
//! **What Fleet keeps is a mark per checkout**, beside its servers' logs and never in the tree:
//! for each Command that passed, its evidence (the Command's name and line, and the dependency
//! files it read: lockfiles and package manifests, at the root and in each folder it names), and
//! the paths the setup made. It is written as each Command passes, so a setup that failed at the
//! second Command is tried again from the second. A Command is done when its evidence is the same
//! and each path the setup made is still there, so a slot cleaned for its next lease
//! (`git clean -fdx`) is set up again whole, and an unchanged one is not. **The first Command that
//! has to run starts the rest**: a later one may install into what it made.
//!
//! **Setup is a run's first phase and not a server.** It is held in its own book beside the servers
//! (`Setups`), one per checkout, and the runs that asked for it start when it passes. Its output is
//! a log like a server's, read on `observe_server` by the setup's own id. A Command that fails
//! stops there: no process starts, and the run says which Command it was.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Writing;
use config::{Manifest, Preparation};
use ipc::Event;
use tokio::sync::watch;
use verification::Exit;

use crate::checkouts::Checkout;
use crate::daemon::Fleet;
use crate::rehearsing::records;
use crate::servers::{marked, pumped, until};

/// Where a checkout's setup stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetupState {
    SettingUp,
    /// Every Command passed, and the runs that waited on it were started.
    Ready,
    /// A Command failed or was stopped, or a run would not start after it.
    Failed,
}

/// One setup on one checkout, as a run reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetupView {
    /// What `observe_server` takes to read its output.
    pub(crate) id: String,
    /// Its log, absolute.
    pub(crate) log_path: String,
    pub(crate) state: SetupState,
    /// The `setup.requires` entries, in the order they run.
    pub(crate) commands: Vec<String>,
    /// The Command running now.
    pub(crate) running: Option<String>,
    /// The Command that failed. Absent on a run that would not start once setup passed.
    pub(crate) failed: Option<String>,
    /// How it ended, in a sentence, where it did not pass.
    pub(crate) ended: Option<String>,
    /// The code the failed Command exited with, where it exited with one.
    pub(crate) exit_code: Option<i32>,
    /// The runs that asked for it, which it starts when it passes.
    pub(crate) runs: Vec<String>,
}

struct Held {
    view: SetupView,
    /// Its output while it runs. Taken when it ends, which closes a viewer's socket.
    feed: Option<api::RunFeed>,
    dir: PathBuf,
    log: String,
    stop: Arc<watch::Sender<bool>>,
    done: watch::Receiver<bool>,
}

/// Every checkout's last setup, by the checkout's path. **In memory**, for the servers' reason; the
/// mark on disk is what outlives Fleet.
#[derive(Clone, Default)]
pub(crate) struct Setups(Arc<Mutex<BTreeMap<String, Held>>>);

impl Setups {
    fn book(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Held>> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The setup on `path` that `run` asked for, where there is one.
    pub(crate) fn of(&self, path: &str, run: &str) -> Option<SetupView> {
        self.book().get(path).map(|held| held.view.clone()).filter(|view| view.runs.iter().any(|one| one == run))
    }

    /// `id`'s state, its output while it runs, its directory and its log's path.
    pub(crate) fn watching(&self, id: &str) -> Option<(SetupView, Option<api::RunWatch>, PathBuf, String)> {
        let book = self.book();
        let held = book.values().find(|held| held.view.id == id)?;
        Some((held.view.clone(), held.feed.as_ref().map(api::RunFeed::watch), held.dir.clone(), held.log.clone()))
    }

    /// Add `run` to the setup running on `path`, and answer whether there is one.
    fn joined(&self, path: &str, run: &str) -> bool {
        let mut book = self.book();
        let Some(held) = book.get_mut(path).filter(|held| held.view.state == SetupState::SettingUp) else {
            return false;
        };
        if !held.view.runs.iter().any(|one| one == run) {
            held.view.runs.push(run.to_string());
        }
        true
    }

    fn changed(&self, path: &str, change: impl FnOnce(&mut SetupView)) -> Option<SetupView> {
        let mut book = self.book();
        let held = book.get_mut(path)?;
        change(&mut held.view);
        if held.view.state != SetupState::SettingUp {
            held.feed = None;
        }
        Some(held.view.clone())
    }

    /// Stop the setup `run` asked for on `path`, and wait for it to end.
    pub(crate) async fn stopped(&self, path: &str, run: &str) {
        let waiting = {
            let book = self.book();
            book.get(path)
                .filter(|held| held.view.state == SetupState::SettingUp && held.view.runs.iter().any(|one| one == run))
                .map(|held| {
                    let _ = held.stop.send(true);
                    held.done.clone()
                })
        };
        if let Some(mut done) = waiting {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(30), done.wait_for(|done| *done)).await;
        }
    }
}

/// What a setup leaves on disk for the checkout: the evidence each Command passed at, and the paths
/// the setup made.
#[derive(Debug, Default, PartialEq, Eq)]
struct Mark {
    /// `(name, evidence)` of each Command that passed.
    done: Vec<(String, String)>,
    made: Vec<String>,
}

impl Mark {
    fn read(at: &Path) -> Option<Mark> {
        let text = std::fs::read_to_string(at).ok()?;
        let mut mark = Mark::default();
        for line in text.lines() {
            match line.split_once(' ') {
                Some(("done", rest)) => {
                    if let Some((evidence, name)) = rest.split_once(' ') {
                        mark.done.push((name.to_string(), evidence.to_string()));
                    }
                }
                Some(("made", path)) => mark.made.push(path.to_string()),
                _ => {}
            }
        }
        Some(mark)
    }

    fn written(&self, at: &Path) {
        let mut text = String::new();
        for (name, evidence) in &self.done {
            text.push_str(&format!("done {evidence} {name}\n"));
        }
        for path in &self.made {
            text.push_str(&format!("made {path}\n"));
        }
        if let Some(parent) = at.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(at, text);
    }

    /// Whether `step` passed at `evidence`.
    fn has(&self, step: &Preparation, evidence: &str) -> bool {
        self.done.iter().any(|(name, held)| name == step.name() && held == evidence)
    }

    /// `step` passed at `evidence`, in place of an earlier record of it.
    fn passed(&mut self, step: &Preparation, evidence: String) {
        self.forgotten(step);
        self.done.push((step.name().to_string(), evidence));
    }

    fn forgotten(&mut self, step: &Preparation) {
        self.done.retain(|(name, _)| name != step.name());
    }
}

/// The files a setup is traced from: what an install reads to decide what it installs.
const EVIDENCE: &[&str] = &[
    "package.json",
    "package-lock.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "requirements.txt",
    "requirements-dev.txt",
    "pyproject.toml",
    "poetry.lock",
    "uv.lock",
    "Pipfile",
    "Pipfile.lock",
    "Cargo.lock",
    "Gemfile.lock",
    "go.sum",
    "composer.lock",
];

/// Folders whose contents are what an install made, never what it read.
const MADE_FOLDERS: &[&str] = &["node_modules", "target", "vendor", "__pycache__"];

/// The root, and each folder a Command names, by a path or by a word that is a folder: `frontend`
/// in `npm ci --prefix frontend`, `backend` in `backend/.venv/bin/python`. **Up to the first part
/// that is hidden or made**, so the folders are the same before the setup and after it.
fn traced_folders(checkout: &Path, setup: &[Preparation]) -> Vec<String> {
    let mut folders = vec![String::from(".")];
    for step in setup {
        for word in step.run().split_whitespace() {
            let word = word.split_once('=').map_or(word, |(_, value)| value);
            if word.starts_with('-') || word.starts_with('/') {
                continue;
            }
            let mut at = String::new();
            for part in word.split('/') {
                if part.is_empty() || part.starts_with('.') || MADE_FOLDERS.contains(&part) {
                    break;
                }
                at = if at.is_empty() { part.to_string() } else { format!("{at}/{part}") };
                if !checkout.join(&at).is_dir() {
                    break;
                }
                if !folders.contains(&at) {
                    folders.push(at.clone());
                }
            }
        }
    }
    folders.sort();
    folders
}

/// FNV-1a, 64 bits: the same number for the same bytes on every build, which a mark kept across
/// Fleet's upgrades needs and `std`'s hasher does not promise.
fn fnv(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3))
}

/// The evidence `step` is done at on `checkout`: its name and line, and the path and bytes of each
/// traced file in the root and in each folder its line names.
fn evidence(checkout: &Path, step: &Preparation) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    hash = fnv(hash, format!("{}\0{}\0", step.name(), step.run()).as_bytes());
    for folder in traced_folders(checkout, std::slice::from_ref(step)) {
        for file in EVIDENCE {
            let path = checkout.join(&folder).join(file);
            if let Ok(bytes) = std::fs::read(&path) {
                hash = fnv(hash, format!("{folder}/{file}\0").as_bytes());
                hash = fnv(hash, &bytes);
            }
        }
    }
    format!("{hash:016x}")
}

/// What is in each traced folder now, as paths from the checkout's root.
fn entries(checkout: &Path, setup: &[Preparation]) -> Vec<String> {
    let mut out = Vec::new();
    for folder in traced_folders(checkout, setup) {
        let Ok(listing) = std::fs::read_dir(checkout.join(&folder)) else { continue };
        for entry in listing.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            out.push(match folder.as_str() {
                "." => name,
                below => format!("{below}/{name}"),
            });
        }
    }
    out
}

/// The mark's file for `checkout`: under the records root, by the name its servers' logs go under.
fn mark_path(checkout: &Checkout) -> PathBuf {
    Path::new(checkout.served().records_root()).join(".armada").join("setup").join(checkout.under()).join("ran")
}

/// What a checkout's mark holds that is still true: no Command passed where something the setup
/// made has gone, as a slot's clean takes it. The paths still there stay its own.
fn kept(checkout: &Checkout) -> Mark {
    let root = Path::new(checkout.path());
    let Some(mut mark) = Mark::read(&mark_path(checkout)) else { return Mark::default() };
    if mark.made.iter().any(|path| !root.join(path).exists()) {
        mark.done.clear();
        mark.made.retain(|path| root.join(path).exists());
    }
    mark
}

/// Whether `setup` has to run on `checkout`: a Command that never passed there, passed when it was
/// traced from other files or was another line, or something the setup made has gone.
fn needed(checkout: &Checkout, setup: &[Preparation]) -> bool {
    let root = Path::new(checkout.path());
    let mark = kept(checkout);
    setup.iter().any(|step| !mark.has(step, &evidence(root, step)))
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
    /// Set up `checkout` for `run` where it needs it: **`true` where a setup now stands between the
    /// run and its processes**, started here or already running, and `false` where the checkout is
    /// set up and the processes may start at once. The setup starts the run when it passes.
    pub(crate) fn set_up_first(self: &Arc<Self>, checkout: &Checkout, manifest: &Manifest, run: &str) -> bool {
        let setups = self.servers().setups();
        if setups.joined(checkout.path(), run) {
            return true;
        }
        let setup = manifest.prepared_by().to_vec();
        if !needed(checkout, &setup) {
            return false;
        }
        let id = self.mint().ulid().as_str().to_string();
        let under = checkout.under();
        let dir = Path::new(checkout.served().records_root()).join(".armada").join("servers").join(&under).join(&id);
        if std::fs::create_dir_all(&dir).and_then(|()| std::fs::File::create(dir.join(records::LOG)).map(|_| ())).is_err() {
            return false;
        }
        let (stop, stopped) = watch::channel(false);
        let (finished, done) = watch::channel(false);
        let feed = api::RunFeed::new();
        let view = SetupView {
            id: id.clone(),
            log_path: dir.join(records::LOG).display().to_string(),
            state: SetupState::SettingUp,
            commands: setup.iter().map(|one| one.name().to_string()).collect(),
            running: None,
            failed: None,
            ended: None,
            exit_code: None,
            runs: vec![run.to_string()],
        };
        setups.book().insert(
            checkout.path().to_string(),
            Held {
                view,
                feed: Some(feed.clone()),
                dir: dir.clone(),
                log: format!(".armada/servers/{under}/{id}/{}", records::LOG),
                stop: Arc::new(stop),
                done,
            },
        );
        let fleet = Arc::clone(self);
        let (checkout, manifest) = (checkout.clone(), manifest.clone());
        tokio::spawn(async move {
            let log = dir.join(records::LOG);
            let (over, ending) = watch::channel(false);
            let ran = async {
                let ran = fleet.setup_ran(&checkout, &manifest, &setup, &log, stopped).await;
                let _ = over.send(true);
                ran
            };
            let (passed, ()) = tokio::join!(ran, pumped(&feed, &log, ending, |_| {}));
            if passed {
                fleet.runs_after_setup(&checkout).await;
            }
            let _ = finished.send(true);
        });
        true
    }

    /// Each Command in order that is not done, its output appended to `log`. **`true` where every
    /// one passed**. The mark is written as each does, and before it runs.
    async fn setup_ran(
        &self,
        checkout: &Checkout,
        manifest: &Manifest,
        setup: &[Preparation],
        log: &Path,
        stopped: watch::Receiver<bool>,
    ) -> bool {
        let root = Path::new(checkout.path());
        let before = entries(root, setup);
        // What an earlier setup made is still its own, though this one found it there already.
        let mut mark = kept(checkout);
        // Once a Command runs, each one after it does: it may install into what that one made.
        let mut rerunning = false;
        self.setup_told(checkout, manifest, |_| {}).await;
        for step in setup {
            if *stopped.borrow() {
                break;
            }
            if !rerunning && mark.has(step, &evidence(root, step)) {
                continue;
            }
            rerunning = true;
            // Written before it runs, so a Command that fails is not taken for one that passed.
            mark.forgotten(step);
            mark.written(&mark_path(checkout));
            self.setup_told(checkout, manifest, |view| view.running = Some(step.name().to_string())).await;
            marked(log, &format!("--- {}: {} ---", step.name(), step.run()));
            let attempt = checks_runner::run_until(
                step.run(),
                root,
                self.budget().duration(),
                Writing::Appending(log),
                &[],
                until(stopped.clone()),
                checks_runner::Priority::Normal,
            )
            .await;
            if attempt.exit != Exit::Code(0) || *stopped.borrow() {
                let code = match attempt.exit {
                    Exit::Code(code) if !*stopped.borrow() => Some(code),
                    _ => None,
                };
                let ended = match *stopped.borrow() {
                    true => String::from("setup was stopped"),
                    false => crate::preparing::NotPrepared {
                        command: step.name().to_string(),
                        run: step.run().to_string(),
                        exit: attempt.exit,
                        output: attempt.output,
                    }
                    .to_string(),
                };
                marked(log, &ended);
                self.setup_told(checkout, manifest, |view| {
                    view.state = SetupState::Failed;
                    view.running = None;
                    view.failed = Some(step.name().to_string());
                    view.ended = Some(ended);
                    view.exit_code = code;
                })
                .await;
                return false;
            }
            for path in entries(root, setup) {
                if !before.contains(&path) && !mark.made.contains(&path) {
                    mark.made.push(path);
                }
            }
            mark.passed(step, evidence(root, step));
            mark.written(&mark_path(checkout));
        }
        if *stopped.borrow() {
            self.setup_told(checkout, manifest, |view| {
                view.state = SetupState::Failed;
                view.running = None;
                view.ended = Some(String::from("setup was stopped"));
            })
            .await;
            return false;
        }
        marked(log, "--- set up ---");
        self.setup_told(checkout, manifest, |view| {
            view.state = SetupState::Ready;
            view.running = None;
        })
        .await;
        true
    }

    /// Start every run that waited on the setup. One that will not start says why on the setup.
    async fn runs_after_setup(self: &Arc<Self>, checkout: &Checkout) {
        let runs = self.servers().setups().book().get(checkout.path()).map(|held| held.view.runs.clone()).unwrap_or_default();
        for run in runs {
            if let Err(why) = Arc::clone(self).run_started_on(checkout.clone(), &run).await {
                let said = match why {
                    api::Refusal::Unacceptable(why)
                    | api::Refusal::IllegalMove(why)
                    | api::Refusal::NotFound(why)
                    | api::Refusal::Fault(why) => why.message,
                };
                self.servers().setups().changed(checkout.path(), |view| {
                    view.state = SetupState::Failed;
                    view.ended = Some(format!("`{run}` did not start: {said}"));
                });
                if let Some(manifest) = self.servers().declared_at(checkout.path()) {
                    self.setup_published(checkout, &manifest).await;
                }
            }
        }
    }

    /// Change the setup on `checkout`, then tell each run that waits on it.
    async fn setup_told(&self, checkout: &Checkout, manifest: &Manifest, change: impl FnOnce(&mut SetupView)) {
        self.servers().setups().changed(checkout.path(), change);
        self.setup_published(checkout, manifest).await;
    }

    /// `app_run.changed` for each run waiting on the setup on `checkout`.
    async fn setup_published(&self, checkout: &Checkout, manifest: &Manifest) {
        let runs = self.servers().setups().book().get(checkout.path()).map(|held| held.view.runs.clone()).unwrap_or_default();
        let id = ipc::ManifestId::from(checkout.served().manifest().id());
        for run in runs {
            let Some(view) = super::viewed(manifest, self.servers(), &id, checkout.path(), &run) else { continue };
            if let Some(wired) = self.app_run_wired(view, None).await {
                self.publish(Event::AppRunChanged(wired));
            }
        }
    }
}
