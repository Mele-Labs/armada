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
const SOURCE_MARKETPLACE: &str = "plugins/.claude-plugin/marketplace.json";
const COPY_MARKETPLACE: &str = ".claude-plugin/marketplace.json";

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

/// Fleet's copy of the mod is there, filled from the first of `trees` that holds the mod when it
/// was not. Blocking: it runs the script.
pub fn populated(home: &Path, folder: &Path, trees: &[PathBuf], within: Duration) -> Result<(), String> {
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
