//! What the debug report reads off the machine: versions, whether `armada` resolves, the mod's
//! installed version and the end of Fleet's log. `ipc::debug_report` lays it out.
//!
//! **Every read is bounded**: a CLI that does not answer in time is `None`, and the log is read from
//! its last megabyte. Fleet and `armada report` call the same functions, so a Fleet that is down
//! still has a report to give.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;

/// How much of the end of the log is read before the last lines are taken from it.
const LOG_TAIL_BYTES: u64 = 1024 * 1024;

/// The launchd label Fleet runs under unless `ARMADA_FLEET_LABEL` names another.
const DEFAULT_LABEL: &str = "com.armada.fleet";

/// A version as the CLI prints it: the first word, where it looks like one.
fn version_in(said: &str) -> Option<String> {
    let word = said.split_whitespace().next()?;
    let shaped = word.len() <= 32 && word.starts_with(|c: char| c.is_ascii_digit()) && word.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'));
    shaped.then(|| word.to_string())
}

/// This install as the report names it. **Nothing here assumes the app**: the binary is whichever
/// runs, a `~/.cargo/bin` one as much as a bundle's, and `armada` resolves where `path` finds one.
pub fn install_of(path: &str, claude_code: Option<String>) -> ipc::debug_report::Install {
    ipc::debug_report::Install {
        armada: env!("CARGO_PKG_VERSION").to_string(),
        runs_from: std::env::current_exe().map(|exe| exe.display().to_string()).unwrap_or_default(),
        resolves: resolves_on(path, "armada"),
        claude_code,
    }
}

/// `program --version` on `path`, or `None` where it did not answer within `within`.
pub fn version_of(program: &str, path: &str, within: Duration) -> Option<String> {
    let mut command = Command::new(program);
    command.arg("--version").env("PATH", path);
    crate::agent_mod::run_coded(command, within).ok().and_then(|said| version_in(&said))
}

/// Whether an executable named `program` is in a folder of `path`, which is how a Job finds it.
pub fn resolves_on(path: &str, program: &str) -> bool {
    path.split(':')
        .filter(|folder| !folder.is_empty())
        .any(|folder| std::fs::metadata(Path::new(folder).join(program)).map(|file| file.is_file() && file.permissions().mode() & 0o111 != 0).unwrap_or(false))
}

#[derive(Deserialize)]
struct Plugin {
    #[serde(default)]
    version: Option<String>,
}

#[derive(Deserialize)]
struct Installed {
    #[serde(default)]
    plugins: BTreeMap<String, Vec<Plugin>>,
}

/// The version of the mod the agent CLI has installed, from its own record under `home`.
pub fn mod_installed_version(home: &Path) -> Option<String> {
    let bytes = std::fs::read(home.join(".claude/plugins/installed_plugins.json")).ok()?;
    let installed: Installed = ipc::decode("installed plugins", &bytes).ok()?;
    installed.plugins.get(crate::agent_mod::PLUGIN)?.first()?.version.clone()
}

/// The version of the mod in Fleet's copy at `folder`.
pub fn mod_copy_version(folder: &Path) -> Option<String> {
    let bytes = std::fs::read(folder.join(crate::MOD_INSTALLED_MANIFEST)).ok()?;
    let plugin: Plugin = ipc::decode("the mod's plugin file", &bytes).ok()?;
    plugin.version
}

/// The text of a plist's string for `key`, found by reading it as text: launchd's own plist is XML,
/// and a scan is all that is needed of it.
fn plist_string(plist: &str, key: &str) -> Option<String> {
    let after = plist.split_once(&format!("<key>{key}</key>"))?.1;
    let value = after.split_once("<string>")?.1.split_once("</string>")?.0;
    Some(value.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&"))
}

/// Where Fleet may be writing its log, **every way it starts**: the file the launchd job's plist names
/// for its output, the one `scripts/install.sh` and `scripts/restart` give it under `~/Library/Logs`,
/// and the one `scripts/dev` redirects a direct start to. A Fleet started by hand in a terminal writes
/// to the terminal, and has no file to name.
pub fn fleet_log_candidates(home: &Path) -> Vec<PathBuf> {
    let label = std::env::var("ARMADA_FLEET_LABEL").ok().filter(|label| !label.is_empty()).unwrap_or_else(|| DEFAULT_LABEL.to_string());
    let plist = home.join("Library/Application Support/Armada").join(format!("{label}.plist"));
    let named = std::fs::read_to_string(plist).ok().and_then(|text| plist_string(&text, "StandardErrorPath")).map(PathBuf::from);
    named
        .into_iter()
        .chain([home.join("Library/Logs/Armada").join(format!("{label}.log")), std::env::temp_dir().join("armada-fleet.log")])
        .collect()
}

/// A log file and when it was last written, in epoch milliseconds.
struct Written {
    path: PathBuf,
    millis: i64,
}

/// The candidate written most recently. **The newest, not the first**: a path a plist names may have
/// been left behind by an earlier way of starting Fleet.
fn newest(candidates: Vec<PathBuf>) -> Option<Written> {
    candidates
        .into_iter()
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
            let millis = modified.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as i64;
            Some(Written { path, millis })
        })
        .max_by_key(|file| file.millis)
}

const DAY_MILLIS: i64 = 86_400_000;

/// What Fleet's log says of recent trouble at `now` (RFC3339). `None` where no candidate file exists.
/// **A file last written more than a day ago is named as old, and its lines are not read**: Fleet may
/// be logging somewhere this does not look, and an old file's lines are not recent.
pub fn recent_log(home: &Path, now: &str) -> Option<ipc::debug_report::Log> {
    use ipc::debug_report::Log;
    let file = newest(fleet_log_candidates(home))?;
    let now = core_model::Timestamp::from_rfc3339(now).epoch_millis()?;
    let age = now - file.millis;
    if age >= DAY_MILLIS {
        return Some(Log::Old { days: (age / DAY_MILLIS) as u64 });
    }
    let tail = log_tail(&file.path)?;
    Some(Log::Read(ipc::log_trouble::trouble_in(&tail, now - DAY_MILLIS)))
}

/// The end of the file at `path`, whole lines only. `None` where there is no file to read.
fn log_tail(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let from = length.saturating_sub(LOG_TAIL_BYTES);
    file.seek(SeekFrom::Start(from)).ok()?;
    let mut bytes = Vec::new();
    file.take(LOG_TAIL_BYTES).read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    // A read that began mid file begins mid line.
    Some(match from {
        0 => text,
        _ => text.split_once('\n').map(|(_, rest)| rest.to_string()).unwrap_or_default(),
    })
}
