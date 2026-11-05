//! The builds a restart keeps in `Armada/builds/`, and the swap back to the one before.
//!
//! `scripts/keep-build` makes a folder per build, holding the `armada` binary, Bridge's `bridge/`
//! and a `build.toml` of what went in, and points `current` at it with the old one as `previous`.
//! launchd runs Fleet and Bridge through `current`, so **a revert changes two links and boots the
//! two jobs out and back in**; no plist is written. `docs/concepts/fleet.md`, *Kept builds*.
//!
//! **Run by the build reverted to**, `builds/previous/armada build revert`, so a broken build
//! never has to run its own undo. It outlives the Fleet it stops, as `scripts/restart-build` does.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;

/// Under the support folder.
pub const BUILDS: &str = "builds";
/// The build launchd runs.
pub const CURRENT: &str = "current";
/// The build a revert starts.
pub const PREVIOUS: &str = "previous";
/// What went into a build, in its folder.
pub const RECORD: &str = "build.toml";
/// The store copies, one per swap, beside the ones `scripts/restart` takes.
const BACKUPS: &str = "backups";
/// How long a job booted out has to be gone, and a new Fleet has to write its runtime file.
const WITHIN: Duration = Duration::from_secs(20);

/// One kept build, as its folder and `build.toml` say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBuild {
    /// The folder's name: when it was built, then its short commit.
    pub name: String,
    pub built_at: String,
    pub commit: String,
    /// `main` or `preview`.
    pub source: String,
    /// A hash of Bridge's `out/`: two builds with the same one run the same Bridge.
    pub bridge: Option<String>,
}

#[derive(Deserialize)]
struct Record {
    built_at: String,
    commit: String,
    source: String,
    bridge: Option<String>,
}

/// The build `link` names under `builds`, or none where the link or its record is missing.
pub fn kept_at(builds: &Path, link: &str) -> Option<KeptBuild> {
    let name = std::fs::read_link(builds.join(link)).ok()?;
    let name = name.file_name()?.to_string_lossy().into_owned();
    let text = std::fs::read_to_string(builds.join(&name).join(RECORD)).ok()?;
    let record: Record = toml::from_str(&text).ok()?;
    Some(KeptBuild {
        name,
        built_at: record.built_at,
        commit: record.commit,
        source: record.source,
        bridge: record.bridge,
    })
}

/// Point `link` at `name`, by a link made beside it and renamed over it, so no reader sees none.
fn pointed(builds: &Path, link: &str, name: &str) -> std::io::Result<()> {
    let made = builds.join(format!(".{link}.tmp"));
    let _ = std::fs::remove_file(&made);
    std::os::unix::fs::symlink(name, &made)?;
    std::fs::rename(&made, builds.join(link))
}

/// The two launchd jobs and the store copy, which a test stands in for.
pub trait Services {
    /// Boot the job `label` out, wait until launchd holds it no more, and bootstrap it from `plist`.
    fn rebooted(&self, label: &str, plist: &Path) -> Result<(), String>;
    /// A copy of the store at `db` in `to`, safe while Fleet writes it.
    fn store_copied(&self, db: &Path, to: &Path) -> Result<(), String>;
    /// A Fleet other than `old` has written its runtime file in `support`.
    fn fleet_came_up(&self, support: &Path, old: Option<u32>) -> Result<(), String>;
}

/// launchd and `sqlite3`, as `scripts/restart` drives them.
pub struct OnThisMac;

impl Services for OnThisMac {
    fn rebooted(&self, label: &str, plist: &Path) -> Result<(), String> {
        // The person whose launchd domain the job is in owns its plist, which they wrote.
        let uid = std::os::unix::fs::MetadataExt::uid(&std::fs::metadata(plist).map_err(|why| format!("{}: {why}", plist.display()))?);
        let domain = format!("gui/{uid}");
        let service = format!("{domain}/{label}");
        let held = || {
            std::process::Command::new("launchctl")
                .args(["print", &service])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        };
        // **Out and back in, never kickstart**: `scripts/restart` says why.
        let _ = std::process::Command::new("launchctl").args(["bootout", &service]).status();
        let since = Instant::now();
        while held() {
            if since.elapsed() > WITHIN {
                return Err(format!("{label} has not stopped after {} seconds", WITHIN.as_secs()));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let booted = std::process::Command::new("launchctl")
            .args(["bootstrap", &domain])
            .arg(plist)
            .status()
            .map_err(|why| format!("launchctl would not run: {why}"))?;
        match booted.success() {
            true => Ok(()),
            false => Err(format!("launchd would not load {}", plist.display())),
        }
    }

    fn store_copied(&self, db: &Path, to: &Path) -> Result<(), String> {
        let backup = format!(".backup '{}'", to.display());
        let copied = std::process::Command::new("sqlite3")
            .arg(db)
            .arg(backup)
            .status()
            .map_err(|why| format!("sqlite3 would not run: {why}"))?;
        match copied.success() {
            true => Ok(()),
            false => Err(format!("the store could not be copied to {}", to.display())),
        }
    }

    fn fleet_came_up(&self, support: &Path, old: Option<u32>) -> Result<(), String> {
        let since = Instant::now();
        loop {
            if let Ok(crate::runtime::Presence::Running(file)) = crate::runtime::read(&support.join(crate::runtime::FILE_NAME)) {
                if Some(file.pid) != old {
                    return Ok(());
                }
            }
            if since.elapsed() > WITHIN {
                return Err(format!(
                    "No new Fleet wrote its runtime file within {} seconds. Read ~/Library/Logs/Armada/",
                    WITHIN.as_secs()
                ));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

/// Now, as a store copy is named: `20261105T162000Z`.
pub fn stamp_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX));
    crate::clock::rfc3339_utc(now).chars().take(19).filter(char::is_ascii_alphanumeric).collect::<String>() + "Z"
}

/// What a revert did.
#[derive(Debug, PartialEq, Eq)]
pub struct Reverted {
    pub from: KeptBuild,
    pub to: KeptBuild,
    /// Bridge was booted onto the build reverted to: its `out/` differs, or a record lacks the hash.
    pub bridge_reopened: bool,
    /// The store as it stood before the swap.
    pub store_copy: Option<PathBuf>,
}

/// Swap onto the previous build: copy the store, point `current` at `previous` and `previous` at
/// what was current, boot Fleet out and in, and Bridge where its `out/` differs. `label` is the
/// Fleet's launchd job, `com.armada.fleet` unless a scratch Fleet's override names another. `stamp`
/// names the store copy. **Refused before anything changes** where there is no previous build, or
/// launchd's Fleet does not run from `builds/current`, since then the links move nothing.
pub fn revert(support: &Path, label: &str, stamp: &str, services: &dyn Services) -> Result<Reverted, String> {
    let builds = support.join(BUILDS);
    let from = kept_at(&builds, CURRENT).ok_or("No kept build is running, so there is nothing to revert from")?;
    let to = kept_at(&builds, PREVIOUS).ok_or("No build before this one is kept")?;
    let plist = support.join(format!("{label}.plist"));
    let bridge_plist = support.join(format!("{label}.bridge.plist"));
    let runs_from = builds.join(CURRENT).join("armada");
    let named = std::fs::read_to_string(&plist).unwrap_or_default();
    if !named.contains(&format!("<string>{}</string>", runs_from.display())) {
        return Err(format!(
            "Fleet's launchd job does not run {}, so a revert would change nothing it runs. Restart once with scripts/restart",
            runs_from.display()
        ));
    }
    let db = support.join("armada.db");
    let store_copy = match db.is_file() {
        true => {
            let to = support.join(BACKUPS).join(format!("armada-{stamp}.db"));
            std::fs::create_dir_all(support.join(BACKUPS)).map_err(|why| why.to_string())?;
            services.store_copied(&db, &to)?;
            Some(to)
        }
        false => None,
    };
    let was = crate::runtime::read(&support.join(crate::runtime::FILE_NAME)).ok();
    let old_pid = match was {
        Some(crate::runtime::Presence::Running(file)) => Some(file.pid),
        _ => None,
    };
    pointed(&builds, CURRENT, &to.name).map_err(|why| format!("current could not be pointed at {}: {why}", to.name))?;
    pointed(&builds, PREVIOUS, &from.name).map_err(|why| format!("previous could not be pointed at {}: {why}", from.name))?;
    services.rebooted(label, &plist)?;
    services.fleet_came_up(support, old_pid)?;
    let bridge_reopened = from.bridge.is_none() || from.bridge != to.bridge;
    if bridge_reopened && bridge_plist.is_file() {
        services.rebooted(&format!("{label}.bridge"), &bridge_plist)?;
    }
    Ok(Reverted { from, to, bridge_reopened: bridge_reopened && bridge_plist.is_file(), store_copy })
}
