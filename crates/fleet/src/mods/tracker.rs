//! Tracker mods: a `tracker.toml` checked without running it, the try that runs it, and the
//! record of that try beside the mod. `docs/concepts/mods.md`, *Tracker mods*.
//!
//! **Tried before used.** The file passing its checks says nothing about whether the command or
//! the address it names answers, so a Manifest naming the mod reaches it only after a try of the
//! file as it now stands has passed. The record keeps the file's digest, so an edit undoes the try.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use adapter_traits::Tracker;
use adapters::taught_tracker::{self, TaughtTracker, TaughtTrackers, TrackerFile};
use ipc::{Instant, ManifestId, ModKind, TrackerTry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{examine, read_small, slug_problem, Examined, Files, Payload, Source, TRACKER};

/// Beside the mod's own files, and none of them: never promoted, never checked as the mod.
const RECORD: &str = ".tried.toml";
const MOST_RECORD: u64 = 4 * 1024;

/// A try as it is kept, with the digest of the file it tried.
#[derive(Serialize, Deserialize)]
struct Record {
    at: String,
    manifest_id: String,
    passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    listed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    read: Option<String>,
    sha256: String,
}

/// The rest of [`examine`] for a tracker mod: `tracker.toml` read and checked, and its last try
/// where it tried this text. **Runs nothing**, so a scan stays a read of a few small files. A
/// built-in tracker has no folder here to keep a try in, so it has none.
pub(super) fn examined(source: &Source, found: &mut Examined, mut stamps: Vec<i64>, manifest: Option<String>) {
    match source.read(TRACKER, taught_tracker::MOST_BYTES) {
        Ok((text, ms)) => {
            stamps.extend(ms);
            found.problems.extend(taught_tracker::problems(&text));
            (found.tried, found.stale) = match source {
                Source::Folder(dir) => tried_of(dir, &text),
                Source::BuiltIn(_) => (None, false),
            };
            if let (true, Some(manifest)) = (found.problems.is_empty(), manifest) {
                found.files = Some(Files { manifest, payload: Payload::Tracker(text) });
            }
        }
        Err(why) => found.problems.push(why),
    }
    found.changed_ms = stamps.into_iter().max();
}

fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The try kept beside the mod where it tried `text`, and whether one is kept of a `tracker.toml`
/// that has since changed. A mod never tried, or whose record cannot be read, is neither.
fn tried_of(dir: &Path, text: &str) -> (Option<TrackerTry>, bool) {
    let Ok((kept, _)) = read_small(&dir.join(RECORD), MOST_RECORD, RECORD) else { return (None, false) };
    let Ok(record) = toml::from_str::<Record>(&kept) else { return (None, false) };
    if record.sha256 != digest(text) {
        return (None, true);
    }
    let tried = TrackerTry {
        at: Instant::carried(record.at),
        manifest_id: ManifestId::carried(record.manifest_id),
        passed: record.passed,
        reason: record.reason,
        listed: record.listed,
        read: record.read,
    };
    (Some(tried), false)
}

/// Written beside the old record and moved over it, so a reader sees one try or the other.
fn keep(dir: &Path, text: &str, tried: &TrackerTry) -> Result<(), String> {
    let record = Record {
        at: tried.at.as_str().to_string(),
        manifest_id: tried.manifest_id.as_str().to_string(),
        passed: tried.passed,
        reason: tried.reason.clone(),
        listed: tried.listed,
        read: tried.read.clone(),
        sha256: digest(text),
    };
    let written = toml::to_string(&record).map_err(|why| why.to_string())?;
    let fresh = dir.join(format!("{RECORD}.new"));
    std::fs::write(&fresh, written)
        .and_then(|()| std::fs::rename(&fresh, dir.join(RECORD)))
        .map_err(|why| format!("the try could not be kept: {why}"))
}

/// Why a try did not run at all.
pub(crate) enum NotTried {
    /// The mod is not a tracker. A 422.
    NotATracker(String),
    /// The record could not be written. A 500.
    Failed(String),
}

/// Try the tracker mod `name` against the checkout at `root`: its Open query, then a read of the
/// first issue it listed. **Blocking**, and every call in it bounded by the adapter. A mod that is
/// not valid answers why and keeps nothing, since no file passed to be tried.
pub(crate) fn try_it(mods_dir: &Path, name: &str, root: &str, manifest_id: ManifestId, at: Instant) -> Result<TrackerTry, NotTried> {
    let found = examine(mods_dir, name);
    if found.manifest.kind != Some(ModKind::Tracker) {
        return Err(NotTried::NotATracker(format!("`{name}` is not a tracker mod")));
    }
    let mut tried = TrackerTry { at, manifest_id, passed: false, reason: None, listed: 0, read: None };
    let text = match found.files.map(|files| files.payload) {
        Some(Payload::Tracker(text)) => text,
        _ => {
            let first = found.problems.first().cloned().unwrap_or_default();
            tried.reason = Some(format!("it is not valid: {first}"));
            return Ok(tried);
        }
    };
    let file = taught_tracker::read(&text).map_err(|problems| NotTried::Failed(problems.join("; ")))?;
    match outcome(&file, root) {
        Ok((listed, read)) => {
            tried.passed = true;
            tried.listed = listed;
            tried.read = read;
        }
        Err((listed, why)) => {
            tried.listed = listed;
            tried.reason = Some(why);
        }
    }
    keep(&mods_dir.join(name), &text, &tried).map_err(NotTried::Failed)?;
    Ok(tried)
}

/// How many issues the Open query listed and which was read back, or how many and why it failed.
fn outcome(file: &TrackerFile, root: &str) -> Result<(u32, Option<String>), (u32, String)> {
    let tracker = TaughtTracker::over(file.clone());
    let said = |not: adapter_traits::NotTracked| format!("{} did not happen: {}", not.doing, not.said);
    let mut listed = tracker.search_issues(root, &file.guide().open).map_err(|not| (0, said(not)))?;
    if file.lists_apart() {
        listed = tracker.open_issues(root).map_err(|not| (0, said(not)))?;
    }
    let count = u32::try_from(listed.len()).unwrap_or(u32::MAX);
    let Some(first) = listed.first() else { return Ok((count, None)) };
    match tracker.read_issue(root, &first.reference) {
        Ok(Some(_)) => Ok((count, Some(first.reference.clone()))),
        Ok(None) => Err((count, format!("reading `{}`, which the Open query listed, found no issue", first.reference))),
        Err(not) => Err((count, said(not))),
    }
}

/// This machine's mod switches as Fleet last read them from the store. A mod not in it is on.
#[derive(Clone, Debug, Default)]
pub struct ModSwitches(Arc<RwLock<BTreeMap<String, bool>>>);

impl ModSwitches {
    pub(crate) fn replace(&self, now: &BTreeMap<String, bool>) {
        *self.0.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = now.clone();
    }

    fn off(&self, name: &str) -> bool {
        self.0.read().unwrap_or_else(|poisoned| poisoned.into_inner()).get(name) == Some(&false)
    }
}

/// The tracker mods in a machine's `mods/`, as the composition root hands them to the trackers.
pub struct TrackerMods {
    mods_dir: PathBuf,
    switches: ModSwitches,
}

impl TrackerMods {
    pub fn in_dir(mods_dir: PathBuf, switches: ModSwitches) -> TrackerMods {
        TrackerMods { mods_dir, switches }
    }
}

impl TaughtTrackers for TrackerMods {
    fn taught(&self, name: &str) -> Option<Result<TrackerFile, String>> {
        if slug_problem(name).is_some() || !std::fs::symlink_metadata(self.mods_dir.join(name)).is_ok_and(|it| it.is_dir()) {
            return None;
        }
        let found = examine(&self.mods_dir, name);
        if found.manifest.kind != Some(ModKind::Tracker) {
            return Some(Err(format!("the mod `{name}` is not a tracker mod")));
        }
        if self.switches.off(name) {
            return Some(Err(format!("the tracker mod `{name}` is switched off in Mods")));
        }
        let text = match found.files.map(|files| files.payload) {
            Some(Payload::Tracker(text)) => text,
            _ => {
                let first = found.problems.first().cloned().unwrap_or_default();
                return Some(Err(format!("the tracker mod `{name}` is not valid: {first}")));
            }
        };
        Some(match found.tried {
            None => Err(format!(
                "the tracker mod `{name}` has not passed a try since it was last edited; try it with try_tracker_mod"
            )),
            Some(tried) if !tried.passed => Err(format!(
                "the tracker mod `{name}` failed its last try: {}",
                tried.reason.unwrap_or_default()
            )),
            Some(_) => taught_tracker::read(&text).map_err(|problems| problems.join("; ")),
        })
    }
}
