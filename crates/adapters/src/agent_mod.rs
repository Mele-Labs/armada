//! Installing the Armada mod into the person's agent CLI: two CLI commands on Fleet's own copy of
//! the mod, and the copy made when it is missing. `docs/concepts/session/installing-the-claude-code-mod.md`.
//!
//! **The install cannot be called without a request.** [`install`] takes an [`Asked`], and the only
//! way to make one is [`Asked::when`] with `true`, so a caller that never saw a person's press has
//! nothing to pass. A CLI's own failure text is returned as it was printed.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The CLI's program name, looked up on the `PATH` Fleet gives it.
pub const PROGRAM: &str = "claude";

/// The plugin, as the CLI names it: the mod's name and the marketplace Fleet's copy declares.
pub const PLUGIN: &str = "armada@armada-local";

/// Where Fleet's script that fills the copy sits in a tree, and what a tree must hold for it.
pub const SYNC_SCRIPT: &str = "scripts/sync-mod";
pub const SOURCE_MARKETPLACE: &str = "plugins/.claude-plugin/marketplace.json";
pub const COPY_MARKETPLACE: &str = ".claude-plugin/marketplace.json";

/// How often a running command is asked whether it has finished.
const POLL: Duration = Duration::from_millis(50);

/// A person's request to install. **No other way to hold one**: it is not `Default` and not
/// `Clone`.
#[derive(Debug)]
pub struct Asked(());

impl Asked {
    /// `Some` only where the person asked.
    pub fn when(requested: bool) -> Option<Asked> {
        requested.then_some(Asked(()))
    }
}

/// Which step stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Populate,
    AddMarketplace,
    Install,
}

/// How the install went, in the words the command printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Installed {
    Done { said: String },
    Failed { step: Step, said: String },
}

/// Whether Fleet's copy of the mod is there to be installed from.
pub fn is_populated(folder: &Path) -> bool {
    folder.join(COPY_MARKETPLACE).is_file()
}

/// Whether `tree` can fill the copy: it holds the mod and the script.
pub fn can_populate_from(tree: &Path) -> bool {
    tree.join(SYNC_SCRIPT).is_file() && tree.join(SOURCE_MARKETPLACE).is_file()
}

/// Fill Fleet's copy from `tree`, with the script that fills it on a restart. Its failure text on
/// `Err`.
pub fn populate(tree: &Path, home: &Path, within: Duration) -> Result<(), String> {
    let mut script = Command::new(tree.join(SYNC_SCRIPT));
    script.arg(tree).env("HOME", home);
    run(script, within).map(|_| ())
}

/// Fleet's copy of the mod is there, and is the app's where `bundle` names one: filled from it when
/// missing or of another plugin version. With none, a copy that is there is left, and a missing one is
/// filled from the first of `trees` that holds the mod. Blocking: it copies, or runs the script.
pub fn populated(home: &Path, folder: &Path, trees: &[PathBuf], bundle: Option<&Path>, within: Duration) -> Result<(), String> {
    if let Some(bundle) = bundle {
        return filled_from_bundle(bundle, folder).map(|_| ());
    }
    if is_populated(folder) {
        return Ok(());
    }
    let Some(tree) = trees.iter().find(|tree| can_populate_from(tree)) else {
        return Err(format!(
            "{} holds no copy of the mod, and none of the repositories this Fleet serves has one to copy",
            folder.display()
        ));
    };
    populate(tree, home, within)?;
    match is_populated(folder) {
        true => Ok(()),
        false => Err(format!("{} is still empty after copying from {}", folder.display(), tree.display())),
    }
}

/// The mod an app carries, beside the binary it runs: `<x>/Contents/Resources/bin/armada` has it at
/// `<x>/Contents/Resources/mod`. `None` for a binary anywhere else, such as one in `~/.cargo/bin`, or
/// where the app holds no mod, so a Fleet that is not an app's keeps the paths it had.
pub fn bundle_beside(exe: &Path) -> Option<PathBuf> {
    let bin = exe.parent().filter(|bin| bin.file_name() == Some(OsStr::new("bin")))?;
    let resources = bin.parent().filter(|one| one.file_name() == Some(OsStr::new("Resources")))?;
    resources.parent().filter(|one| one.file_name() == Some(OsStr::new("Contents")))?;
    let bundle = resources.join("mod");
    is_populated(&bundle).then_some(bundle)
}

/// Fleet's copy at `folder` made the bundle's, when it is not already: missing, or of another
/// plugin version. `Ok(true)` where it was replaced, `Ok(false)` where it was left alone.
///
/// **The copy is made beside the folder and renamed over it**, as `scripts/sync-mod` does, so a Job
/// reading the mod sees the old copy or the new one and never half of either. The versions are the
/// plugin manifests' bytes, which a release changes with the version and nothing here has to parse.
pub fn filled_from_bundle(bundle: &Path, folder: &Path) -> Result<bool, String> {
    let manifest = |root: &Path| std::fs::read(root.join(crate::MOD_INSTALLED_MANIFEST)).ok();
    let had = manifest(bundle);
    if is_populated(folder) && had.is_some() && had == manifest(folder) {
        return Ok(false);
    }
    if !is_populated(bundle) {
        return Err(format!("{} holds no copy of the mod", bundle.display()));
    }
    let support = folder.parent().filter(|one| !one.as_os_str().is_empty()).ok_or_else(|| format!("{} has no folder to sit in", folder.display()))?;
    std::fs::create_dir_all(support).map_err(said("could not make", support))?;
    let id = std::process::id();
    let staging = support.join(format!(".mod-new.{id}"));
    let old = support.join(format!(".mod-old.{id}"));
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(&old);
    let filled = (|| {
        copied(bundle, &staging)?;
        // rename(2) cannot replace a folder that holds anything, so the old copy steps aside first.
        if folder.exists() {
            std::fs::rename(folder, &old).map_err(said("could not move aside", folder))?;
        }
        std::fs::rename(&staging, folder).map_err(|why| {
            // The old copy goes back, so a failure leaves what was there.
            let _ = std::fs::rename(&old, folder);
            said("could not put the copy at", folder)(why)
        })
    })();
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(&old);
    filled.map(|()| true)
}

/// `from` and everything under it copied to `to`, which is made. Symlinks are copied as what they point at.
fn copied(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(said("could not make", to))?;
    for entry in std::fs::read_dir(from).map_err(said("could not read", from))? {
        let path = entry.map_err(said("could not read", from))?.path();
        let target = to.join(path.file_name().unwrap_or_default());
        match std::fs::metadata(&path).map_err(said("could not read", &path))?.is_dir() {
            true => copied(&path, &target)?,
            false => {
                std::fs::copy(&path, &target).map_err(said("could not copy", &path))?;
            }
        }
    }
    Ok(())
}

fn said<'a>(what: &'a str, at: &'a Path) -> impl FnOnce(std::io::Error) -> String + 'a {
    move |why| format!("{what} {}: {why}", at.display())
}

/// Add the marketplace and install the plugin with `program`, the CLI. **Both commands are tried**:
/// a marketplace the CLI already holds is refused by `add` and does not stop the install.
pub fn install(_asked: Asked, program: &OsStr, path: &str, home: &Path, folder: &Path, within: Duration) -> Installed {
    let cli = |args: &[&OsStr]| {
        let mut command = Command::new(program);
        command.args(args).env("HOME", home).env("PATH", path);
        run(command, within)
    };
    let added = cli(&[OsStr::new("plugin"), OsStr::new("marketplace"), OsStr::new("add"), folder.as_os_str()]);
    match (cli(&[OsStr::new("plugin"), OsStr::new("install"), OsStr::new(PLUGIN)]), added) {
        (Ok(said), _) => Installed::Done { said },
        (Err(said), Ok(_)) => Installed::Failed { step: Step::Install, said },
        (Err(_), Err(said)) => Installed::Failed { step: Step::AddMarketplace, said },
    }
}

/// Run to the end within `within`. What it printed on success; on failure what it said, or why it
/// would not start or stop.
fn run(command: Command, within: Duration) -> Result<String, String> {
    run_coded(command, within).map_err(|(_, said)| said)
}

/// [`run`], keeping the exit code beside what a failure said. `-1` where there is none: the
/// command would not start, did not stop in time, or was ended by a signal.
pub(crate) fn run_coded(mut command: Command, within: Duration) -> Result<String, (i32, String)> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| (-1, why.to_string()))?;
    // Read on threads of their own so a chatty command cannot fill a pipe and stall.
    let mut out = child.stdout.take().expect("stdout is piped");
    let mut err = child.stderr.take().expect("stderr is piped");
    let reading_out = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = out.read_to_string(&mut text);
        text
    });
    let reading_err = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = err.read_to_string(&mut text);
        text
    });
    let deadline = Instant::now() + within;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err((-1, format!("did not finish within {} seconds", within.as_secs())));
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(why) => return Err((-1, why.to_string())),
        }
    };
    let printed = reading_out.join().unwrap_or_default();
    let complained = reading_err.join().unwrap_or_default();
    match status.success() {
        true => Ok(printed.trim().to_string()),
        false => {
            let said = if complained.trim().is_empty() { printed } else { complained };
            let said = said.trim();
            let said = if said.is_empty() { "exited with an error and printed nothing".to_string() } else { said.to_string() };
            Err((status.code().unwrap_or(-1), said))
        }
    }
}
