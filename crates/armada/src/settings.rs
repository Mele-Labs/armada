//! settings.json at the composition root: where it is, the defaults only this
//! process can know, the variables that win over it, and the watch that hands
//! a hand edit to Fleet.
//!
//! **The file is beside the store**, in the machine directory, and not under
//! `~/.armada`: `docs/contracts/system-architecture.md` keeps machine data out
//! of the home directory. The watch is a poll of one small file, for
//! [`crate::watching`]'s reasons; unlike `armada.yml` this file may not exist
//! yet, so it is read and compared rather than handed to a watcher that will
//! not watch a path that is not there.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapters::HeadlessAgent;
use config::settings::{self as keys, Env, Supplied};
use fleet::settings::{MachineSettings, Reread};
use tokio::sync::mpsc;

/// The file's name in the machine directory.
pub const SETTINGS_FILE: &str = "settings.json";

/// How often the file is read and compared, and how long it must sit still.
/// `crate::watching`'s numbers, for its measured reasons.
const POLL: Duration = Duration::from_secs(1);
const SETTLE: Duration = Duration::from_millis(1_500);
const LATEST: Duration = Duration::from_secs(10);

/// What ships where only this process knows it: half the cores, the agent
/// CLI and its models, which only `adapters` may spell, and the prompts
/// Fleet owns the words of.
pub fn supplied() -> Supplied {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    fleet::prompts::supplied(Supplied::new())
        .integer(keys::CHECKS_AT_ONCE, fleet::ChecksAtOnce::for_cores(cores).get() as i64)
        .words(keys::HARNESS_AGENT, HeadlessAgent::harness_name())
        .words(keys::HARNESS_BINARY_PATH, HeadlessAgent::on_path().program())
        .list(keys::MODELS_ROSTER, HeadlessAgent::models())
        .words(keys::MODELS_DEFAULT, HeadlessAgent::default_model())
        .words(keys::MODELS_JUDGE, HeadlessAgent::judge_model())
        .words(keys::MODELS_PROPOSER, HeadlessAgent::proposer_model())
        .words(keys::MODELS_RETRO, HeadlessAgent::retro_model())
        .words(keys::MODELS_SECOND_OPINION, HeadlessAgent::second_opinion_model())
        .harnesses(&[HeadlessAgent::harness_name()])
}

/// The variables set in this process that win over the file.
pub fn env() -> Env {
    Env::read(|var| std::env::var(var).ok())
}

/// settings.json in `machine`, read.
pub fn in_machine(machine: &Path) -> MachineSettings {
    MachineSettings::at(machine.join(SETTINGS_FILE), supplied(), env())
}

/// settings.json on this machine, for a verb that has no Fleet: `armada
/// check`. `None` where the machine directory cannot be found.
pub fn on_this_machine() -> Option<MachineSettings> {
    let runtime_file = fleet::runtime::machine_path().ok()?;
    Some(in_machine(runtime_file.parent()?))
}

/// What settings.json holds now, or what ships where the machine directory
/// cannot be found — for a verb with no Fleet to ask.
pub fn now_or_shipped() -> config::settings::Resolved {
    match on_this_machine() {
        Some(settings) => settings.now(),
        None => config::settings::Resolved::new(config::settings::Saved::nothing(), supplied(), env()),
    }
}

/// Say what the file came to, on the console. A refusal leaves the last good
/// settings in force, and somebody watching the terminal is told so.
pub fn say(path: &Path, reread: &Reread) {
    match reread {
        Reread::Quiet => {}
        Reread::Taken => println!("{} changed — in force now, or at the next start", path.display()),
        Reread::Refused(refused) => {
            eprintln!("{} was not taken: {refused}", path.display());
            eprintln!("the settings in force are unchanged; correct the file and save again");
        }
    }
}

/// A watch on settings.json. **Dropping it ends the watch**, which is why the
/// composition root holds it for as long as it serves.
pub struct Watching {
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for Watching {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Watch `path` and hand every settled change to `changed`.
pub fn watch<F, Fut>(path: PathBuf, changed: F) -> Watching
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    watch_every(path, POLL, SETTLE, changed)
}

/// The same at stated intervals, so a test can drive a real file quickly.
pub(crate) fn watch_every<F, Fut>(path: PathBuf, poll: Duration, settle: Duration, mut changed: F) -> Watching
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (told, arrived) = mpsc::unbounded_channel();
    // The contents, not the metadata: `crate::watching` measured a save a
    // quarter of a second after the last one leaving the time unchanged. Read
    // before the task starts, so a save made the moment this returns is a change.
    let mut last = std::fs::read(&path).ok();
    let polling = tokio::spawn(async move {
        loop {
            tokio::time::sleep(poll).await;
            let now = std::fs::read(&path).ok();
            if now != last {
                last = now;
                if told.send(()).is_err() {
                    return;
                }
            }
        }
    });
    // Settled reads are applied one at a time, in order, off the debounce.
    let (apply, mut applied) = mpsc::unbounded_channel::<()>();
    let settling = tokio::spawn(crate::watching::settling(arrived, settle, LATEST, move || {
        let _ = apply.send(());
    }));
    let applying = tokio::spawn(async move {
        while applied.recv().await.is_some() {
            changed().await;
        }
    });
    Watching { tasks: vec![polling, settling, applying] }
}

/// Watch Fleet's own file and hand each change to it, saying on the console
/// what each came to.
pub fn watched_by<H, V, W>(fleet: Arc<fleet::Fleet<H, V, W>>, path: PathBuf) -> Watching
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let shown = path.clone();
    watch(path, move || {
        let fleet = Arc::clone(&fleet);
        let shown = shown.clone();
        async move { say(&shown, &fleet.settings_reread().await) }
    })
}
