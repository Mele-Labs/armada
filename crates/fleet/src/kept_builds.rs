//! The builds a restart keeps in `Armada/builds/`, and the swap back to the one before.
//!
//! `scripts/keep-build` makes a folder per build, holding the `armada` binary, Bridge's `bridge/`
//! and a `build.toml` of what went in, and points `current` at it with the old one as `previous`.
//! launchd runs Fleet and Bridge through `current`, so **a revert changes two links and boots the
//! two jobs out and back in**; no plist is written. `docs/practices/running-locally.md`, *Kept builds*.
//!
//! **Run by the build reverted to**, `builds/previous/armada build revert`, so a broken build
//! never has to run its own undo. It outlives the Fleet it stops, as `scripts/restart-build` does.
//!
//! **`Armada/revert.status` is the record, as `restart-build.status` is a restart's**: Fleet writes
//! `running` as it starts the undo, and `armada build revert` removes the file on success or writes
//! `failed`, the build `current` names, and one line on failure. The Fleet that starts a revert is not the one that reads it back.
//!
//! **A restart onto a new build is checked twice.** Before anything stops, `armada build rehearse`
//! has the new build open a copy of the store. After the swap, `armada build settle` gives the new
//! Fleet a minute to answer `/health`, and otherwise marks the build `unhealthy` and swaps back.
//!
//! **A snapshot is a build kept and not swapped onto**: `scripts/restart --keep-only` names it in
//! `Armada/snapshot`, and `armada build swap` moves onto it as a revert moves back, settling as a
//! restart does. `Armada/preview-picked` is the branches the person picked for the next one.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

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
/// How long a job booted out has to be gone.
const WITHIN: Duration = Duration::from_secs(20);
/// How long a new Fleet has to answer `/health` before a swap onto it is undone.
pub const HEALTHY_WITHIN: Duration = Duration::from_secs(60);
/// The store copy a rehearsal opens, under `backups/`. Removed once opened.
const REHEARSAL: &str = "rehearsal.db";
/// What a revert is doing, or why it stopped. Under the support folder.
pub const STATUS: &str = "revert.status";
/// What a swap onto the snapshot is doing, or why it stopped, as `revert.status` is a revert's.
pub const SWAP_STATUS: &str = "swap.status";
/// The kept build `scripts/restart --keep-only` made last, which Swap moves onto.
pub const SNAPSHOT: &str = "snapshot";
/// The branches the person picked for the next snapshot, one a line.
pub const PICKED: &str = "preview-picked";
/// The build the person pressed Keep on, so its Jobs are no longer being tried.
pub const KEPT_TRIED: &str = "kept-tried";
/// A `running` status older than this is a revert that died, not one working.
const RUNNING_FOR_AT_MOST: Duration = Duration::from_secs(2 * 60);
/// Fleet's launchd job: `ARMADA_FLEET_LABEL`, the override `scripts/restart` reads, or the shipped name.
pub fn fleet_label() -> String {
    std::env::var("ARMADA_FLEET_LABEL").unwrap_or_else(|_| String::from("com.armada.fleet"))
}

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
    /// The branches merged into it, which only a preview build has.
    pub jobs: Vec<String>,
    /// Why a swap onto it was undone: its Fleet did not answer in time.
    pub unhealthy: Option<String>,
    /// The tree it was built from.
    pub tree: Option<String>,
}

#[derive(Deserialize)]
struct Record {
    built_at: String,
    commit: String,
    source: String,
    bridge: Option<String>,
    unhealthy: Option<String>,
    tree: Option<String>,
    #[serde(default, rename = "job")]
    jobs: Vec<JobRow>,
}

/// One `[[job]]` row of `build.toml`. Its `tip` is not read.
#[derive(Deserialize)]
struct JobRow {
    branch: String,
}

/// The build `link` names under `builds`, or none where the link or its record is missing.
pub fn kept_at(builds: &Path, link: &str) -> Option<KeptBuild> {
    let name = std::fs::read_link(builds.join(link)).ok()?;
    kept_named(builds, &name.file_name()?.to_string_lossy())
}

/// The build in the folder `name` under `builds`, or none where its record is missing.
pub fn kept_named(builds: &Path, name: &str) -> Option<KeptBuild> {
    let name = name.to_string();
    let text = std::fs::read_to_string(builds.join(&name).join(RECORD)).ok()?;
    let record: Record = toml::from_str(&text).ok()?;
    Some(KeptBuild {
        name,
        built_at: record.built_at,
        commit: record.commit,
        source: record.source,
        bridge: record.bridge,
        jobs: record.jobs.into_iter().map(|row| row.branch).collect(),
        unhealthy: record.unhealthy,
        tree: record.tree,
    })
}

/// Bridge is booted onto the build reverted to: its `out/` differs, or a record lacks the hash.
fn bridge_reopens(from: &KeptBuild, to: &KeptBuild) -> bool {
    from.bridge.is_none() || from.bridge != to.bridge
}

/// The build running and the one a revert would go to, or why there is none to go to. **The one
/// check `revert` and the report share**: launchd's Fleet must run from `builds/current`, since
/// otherwise moving the links changes nothing it runs.
pub fn revertible(support: &Path, label: &str) -> Result<(KeptBuild, KeptBuild), String> {
    let builds = support.join(BUILDS);
    let from = kept_at(&builds, CURRENT).ok_or("No kept build is running, so there is nothing to revert from")?;
    let to = kept_at(&builds, PREVIOUS).ok_or("No build before this one is kept")?;
    if let Some(why) = &to.unhealthy {
        return Err(format!("The build before this one was swapped back from, since it did not come up: {why}"));
    }
    runs_from_current(support, label)?;
    Ok((from, to))
}

/// launchd's Fleet runs `builds/current/armada`, so moving the links moves what it runs.
fn runs_from_current(support: &Path, label: &str) -> Result<(), String> {
    let runs_from = support.join(BUILDS).join(CURRENT).join("armada");
    let named = std::fs::read_to_string(support.join(format!("{label}.plist"))).unwrap_or_default();
    match named.contains(&format!("<string>{}</string>", runs_from.display())) {
        true => Ok(()),
        false => Err(format!(
            "Fleet's launchd job does not run {}, so a swap would change nothing it runs. Restart once with scripts/restart",
            runs_from.display()
        )),
    }
}

/// The build running and the snapshot Swap would move onto, or why there is none to move onto.
pub fn swappable(support: &Path, label: &str) -> Result<(KeptBuild, KeptBuild), String> {
    let builds = support.join(BUILDS);
    let from = kept_at(&builds, CURRENT).ok_or("No kept build is running, so there is nothing to swap from")?;
    let named = std::fs::read_to_string(support.join(SNAPSHOT)).unwrap_or_default();
    let to = kept_named(&builds, named.trim()).ok_or("No build is kept to swap onto")?;
    if to.name == from.name {
        return Err(String::from("The kept build is the one running"));
    }
    if let Some(why) = &to.unhealthy {
        return Err(format!("The kept build was swapped back from, since it did not come up: {why}"));
    }
    runs_from_current(support, label)?;
    Ok((from, to))
}

/// The branches picked for the next snapshot: the file's lines, or the running build's own where
/// the person has picked nothing yet.
pub fn picked_in(support: &Path) -> Vec<String> {
    match std::fs::read_to_string(support.join(PICKED)) {
        Ok(text) => text.lines().map(str::trim).filter(|one| !one.is_empty()).map(String::from).collect(),
        Err(_) => kept_at(&support.join(BUILDS), CURRENT).map(|kept| kept.jobs).unwrap_or_default(),
    }
}

/// `branch` added to the picked branches, or taken out, and the file written.
pub fn picked_changed(support: &Path, branch: &str, picked: bool) -> std::io::Result<Vec<String>> {
    let mut branches = picked_in(support);
    branches.retain(|one| one != branch);
    if picked {
        branches.push(branch.to_string());
    }
    std::fs::create_dir_all(support)?;
    let text: String = branches.iter().map(|one| format!("{one}\n")).collect();
    std::fs::write(support.join(PICKED), text)?;
    Ok(branches)
}

/// The branches of the running build the person is trying: none once Keep was pressed on it.
pub fn trying_in(support: &Path) -> Vec<String> {
    let Some(running) = kept_at(&support.join(BUILDS), CURRENT) else {
        return Vec::new();
    };
    let kept = std::fs::read_to_string(support.join(KEPT_TRIED)).unwrap_or_default();
    match kept.trim() == running.name {
        true => Vec::new(),
        false => running.jobs,
    }
}

/// Keep pressed on the running build: its branches are no longer being tried.
pub fn kept_tried(support: &Path) -> std::io::Result<Vec<String>> {
    let branches = trying_in(support);
    if let Some(running) = kept_at(&support.join(BUILDS), CURRENT) {
        std::fs::write(support.join(KEPT_TRIED), format!("{}\n", running.name))?;
    }
    Ok(branches)
}

/// What a revert would change, as the Fleet panel asks it before the press.
#[derive(Debug, PartialEq, Eq)]
pub struct Undo {
    pub built_at: String,
    pub commit: String,
    pub bridge_reopens: bool,
    /// Branches the running build holds and the one reverted to does not.
    pub takes_out: Vec<String>,
    /// Branches the one reverted to holds and the running build does not.
    pub brings_back: Vec<String>,
}

/// What a revert from `from` onto `to` changes.
pub fn undoing(from: &KeptBuild, to: &KeptBuild) -> Undo {
    let missing_from = |these: &[String], those: &[String]| these.iter().filter(|one| !those.contains(one)).cloned().collect();
    Undo {
        built_at: to.built_at.clone(),
        commit: to.commit.clone(),
        bridge_reopens: bridge_reopens(from, to),
        takes_out: missing_from(&from.jobs, &to.jobs),
        brings_back: missing_from(&to.jobs, &from.jobs),
    }
}

/// What the status file says.
#[derive(Debug, PartialEq, Eq)]
pub enum RevertStatus {
    Running,
    /// Stopped, in one line of plain facts.
    Failed(String),
}

/// The revert's status: `running` for as long as it is young enough to be working, or the line it failed with.
pub fn revert_status_in(support: &Path, now: SystemTime) -> Option<RevertStatus> {
    status_in(support, STATUS, now)
}

/// The swap's status, as [`revert_status_in`] reads the revert's.
pub fn swap_status_in(support: &Path, now: SystemTime) -> Option<RevertStatus> {
    status_in(support, SWAP_STATUS, now)
}

fn status_in(support: &Path, file: &str, now: SystemTime) -> Option<RevertStatus> {
    let path = support.join(file);
    let text = std::fs::read_to_string(&path).ok()?;
    let mut lines = text.lines();
    match lines.next()?.trim() {
        "running" => {
            let written = std::fs::metadata(&path).ok()?.modified().ok()?;
            let age = now.duration_since(written).unwrap_or_default();
            (age <= RUNNING_FOR_AT_MOST).then_some(RevertStatus::Running)
        }
        // A failure belongs to the build `current` named when it failed; once a restart moves it, it is history.
        "failed" => {
            let on = lines.next()?.trim();
            let line = lines.next()?.trim();
            let current = std::fs::read_link(support.join(BUILDS).join(CURRENT)).ok()?;
            let now_on = current.file_name()?.to_string_lossy().into_owned();
            (!line.is_empty() && on == now_on).then(|| RevertStatus::Failed(line.to_string()))
        }
        _ => None,
    }
}

/// Say a revert is under way, before the undo has done anything.
pub fn revert_begun(support: &Path) -> std::io::Result<()> {
    status_begun(support, STATUS)
}

/// Say a swap onto the snapshot is under way.
pub fn swap_begun(support: &Path) -> std::io::Result<()> {
    status_begun(support, SWAP_STATUS)
}

fn status_begun(support: &Path, file: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(support)?;
    std::fs::write(support.join(file), "running\n")
}

/// Keep what a revert came to: the file goes on success, and otherwise holds `failed`, the build `current` names, and the error's one line.
pub fn revert_ended(support: &Path, outcome: &Result<Reverted, String>) {
    status_ended(support, STATUS, outcome.as_ref().map(drop).map_err(String::as_str));
}

/// Keep what a swap came to, as [`revert_ended`] does. A swap undone by itself failed.
pub fn swap_ended(support: &Path, outcome: &Result<Settled, String>) {
    let undone = match outcome {
        Ok(Settled::SwappedBack { why, .. }) => Some(format!("Swapped back, since the new Fleet did not answer: {why}")),
        _ => None,
    };
    let said = match (outcome, &undone) {
        (_, Some(undone)) => Err(undone.as_str()),
        (Ok(_), None) => Ok(()),
        (Err(why), None) => Err(why.as_str()),
    };
    status_ended(support, SWAP_STATUS, said);
}

fn status_ended(support: &Path, file: &str, outcome: Result<(), &str>) {
    let path = support.join(file);
    match outcome {
        Ok(()) => {
            let _ = std::fs::remove_file(&path);
        }
        Err(why) => {
            let line = why.split_whitespace().collect::<Vec<_>>().join(" ");
            let on = std::fs::read_link(support.join(BUILDS).join(CURRENT)).ok().and_then(|to| Some(to.file_name()?.to_string_lossy().into_owned()));
            let _ = std::fs::write(&path, format!("failed\n{}\n{line}\n", on.unwrap_or_default()));
        }
    }
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
    /// A Fleet other than `old` named by the runtime file in `support` answers `/health`, within
    /// [`HEALTHY_WITHIN`].
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
            // A Fleet that crashes as it boots leaves a runtime file naming a dead pid, which
            // answers nothing, so the file alone is no proof.
            if let Ok(crate::runtime::Presence::Running(file)) = crate::runtime::read(&support.join(crate::runtime::FILE_NAME)) {
                if Some(file.pid) != old && answers_health(file.port) {
                    return Ok(());
                }
            }
            if since.elapsed() > HEALTHY_WITHIN {
                return Err(format!(
                    "No new Fleet answered /health within {} seconds. Read ~/Library/Logs/Armada/",
                    HEALTHY_WITHIN.as_secs()
                ));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

/// A Fleet on `port` answers `GET /health` with 200.
pub fn answers_health(port: u16) -> bool {
    let asked = || -> std::io::Result<bool> {
        let mut stream = std::net::TcpStream::connect_timeout(&crate::runtime::listener_address(port), Duration::from_secs(1))?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")?;
        let mut head = [0u8; 12];
        stream.read_exact(&mut head)?;
        Ok(head.starts_with(b"HTTP/1.1 200"))
    };
    asked().unwrap_or(false)
}

/// Open a copy of the store with this build's migrations, as its Fleet will open the store itself,
/// so a build that cannot is turned away while the running Fleet is still up. **Not a Fleet on
/// the copy**: one would resume the copy's Jobs and sweep processes as its own. No store, nothing
/// to open.
pub fn rehearse(support: &Path, services: &dyn Services) -> Result<(), String> {
    let db = support.join("armada.db");
    if !db.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(support.join(BACKUPS)).map_err(|why| why.to_string())?;
    let copy = support.join(BACKUPS).join(REHEARSAL);
    removed(&copy);
    services.store_copied(&db, &copy)?;
    let opened = store::Store::open(&copy).map(drop).map_err(|why| format!("This build could not open a copy of the store: {why}"));
    removed(&copy);
    opened
}

/// A store file and the two SQLite keeps beside it, gone.
fn removed(db: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

/// How a swap onto a new build came to rest.
#[derive(Debug, PartialEq, Eq)]
pub enum Settled {
    Healthy,
    /// The new Fleet did not answer, so the swap was undone: why, and the swap back.
    SwappedBack { why: String, reverted: Reverted },
}

/// After `scripts/restart` boots Fleet onto `current`, replacing the Fleet at `old`: it answers
/// within [`HEALTHY_WITHIN`], or the build is marked `unhealthy` and Fleet swaps back. Bridge is
/// left as it is, since the restart moves it only once Fleet answers.
pub fn settle(support: &Path, label: &str, old: Option<u32>, stamp: &str, services: &dyn Services) -> Result<Settled, String> {
    let Err(why) = services.fleet_came_up(support, old) else {
        return Ok(Settled::Healthy);
    };
    let builds = support.join(BUILDS);
    let on = kept_at(&builds, CURRENT).ok_or_else(|| format!("{why}. No kept build is running, so there is none to swap back from"))?;
    if on.unhealthy.is_none() {
        marked_unhealthy(&builds.join(&on.name).join(RECORD), &why).map_err(|mark| format!("{why}. {} could not be marked: {mark}", on.name))?;
    }
    match swap(support, label, stamp, services, false) {
        Ok(reverted) => Ok(Settled::SwappedBack { why, reverted }),
        Err(back) => Err(format!("{why}. The swap back failed too: {back}")),
    }
}

/// `unhealthy = "<why>"` added to a build's record, **above its first table**: appended after a
/// preview's `[[job]]` rows it would be a key of the last one.
fn marked_unhealthy(record: &Path, why: &str) -> std::io::Result<()> {
    let text = std::fs::read_to_string(record)?;
    let line = format!("unhealthy = {}\n", toml::Value::String(why.to_string()));
    let at = text.find("\n[").map_or(text.len(), |at| at + 1);
    let mut marked = text[..at].to_string();
    if !marked.is_empty() && !marked.ends_with('\n') {
        marked.push('\n');
    }
    marked.push_str(&line);
    marked.push_str(&text[at..]);
    std::fs::write(record, marked)
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
    swap(support, label, stamp, services, true)
}

/// [`revert`], with Bridge booted where `bridge` and the builds' `out/` differ.
fn swap(support: &Path, label: &str, stamp: &str, services: &dyn Services, bridge: bool) -> Result<Reverted, String> {
    let (from, to) = revertible(support, label)?;
    let (store_copy, old_pid) = flipped(support, label, stamp, services, &from, &to)?;
    services.fleet_came_up(support, old_pid)?;
    let bridge_reopened = bridge && bridged(support, label, services, &from, &to)?;
    recorded(support, &to);
    Ok(Reverted { from, to, bridge_reopened, store_copy })
}

/// Swap onto the snapshot `scripts/restart --keep-only` kept, settling as a restart does: a Fleet
/// that does not answer within [`HEALTHY_WITHIN`] has its build marked `unhealthy` and is swapped
/// back from, with Bridge left where it is. Bridge moves only once the new Fleet answers, and only
/// where the builds' `out/` differ. **Refused before anything changes** where there is no snapshot.
pub fn swap_onto(support: &Path, label: &str, stamp: &str, services: &dyn Services) -> Result<Settled, String> {
    let builds = support.join(BUILDS);
    let (from, to) = swappable(support, label)?;
    let (_, old_pid) = flipped(support, label, stamp, services, &from, &to)?;
    let _ = std::fs::remove_file(support.join(SNAPSHOT));
    if let Err(why) = services.fleet_came_up(support, old_pid) {
        marked_unhealthy(&builds.join(&to.name).join(RECORD), &why).map_err(|mark| format!("{why}. {} could not be marked: {mark}", to.name))?;
        return match swap(support, label, stamp, services, false) {
            Ok(reverted) => Ok(Settled::SwappedBack { why, reverted }),
            Err(back) => Err(format!("{why}. The swap back failed too: {back}")),
        };
    }
    bridged(support, label, services, &from, &to)?;
    recorded(support, &to);
    Ok(Settled::Healthy)
}

/// Bridge booted onto `to` where its `out/` differs from `from`'s and its job is there to boot.
fn bridged(support: &Path, label: &str, services: &dyn Services, from: &KeptBuild, to: &KeptBuild) -> Result<bool, String> {
    let bridge_plist = support.join(format!("{label}.bridge.plist"));
    let reopens = bridge_reopens(from, to) && bridge_plist.is_file();
    if reopens {
        services.rebooted(&format!("{label}.bridge"), &bridge_plist)?;
    }
    Ok(reopens)
}

/// The records a restart writes, written for the build now running, so the Fleet panel reads it
/// as the one it runs: its commit, the tree a preview came from, and what a snapshot held.
fn recorded(support: &Path, to: &KeptBuild) {
    let _ = std::fs::write(support.join("restart-commit"), format!("{}\n", to.commit));
    match (to.source.as_str(), &to.tree) {
        ("preview", Some(tree)) => {
            let _ = std::fs::write(support.join("restart-source"), format!("{tree}\n"));
        }
        _ => {
            let _ = std::fs::remove_file(support.join("restart-source"));
        }
    }
    for (snapshot, preview) in [("snapshot-held", "preview-held"), ("snapshot-left-out", "preview-left-out")] {
        let held = std::fs::read_to_string(support.join(snapshot)).unwrap_or_default();
        if held.lines().next() == Some(to.commit.as_str()) {
            let _ = std::fs::write(support.join(preview), held);
        }
    }
}

/// Copy the store, point `current` at `to` and `previous` at `from`, and boot Fleet out and in:
/// the store copy, and the pid of the Fleet that was running.
fn flipped(
    support: &Path,
    label: &str,
    stamp: &str,
    services: &dyn Services,
    from: &KeptBuild,
    to: &KeptBuild,
) -> Result<(Option<PathBuf>, Option<u32>), String> {
    let builds = support.join(BUILDS);
    let plist = support.join(format!("{label}.plist"));
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
    Ok((store_copy, old_pid))
}
