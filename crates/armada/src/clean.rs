//! Giving back what Fleet's own files hold on disk: the shared base checkouts,
//! and with `--all` the machine's store and the files beside it.
//!
//! # It deletes what a derivation names, never what a pattern matches
//!
//! A hand-run `git branch -D` over the `armada/*` glob destroyed nine unmerged
//! branches that were nobody's to delete. So each base checkout is removed through the
//! `BaseSpec` that derives it. **No derivation, no delete.** A directory under
//! `.armada/worktrees/` is reported and left: evidence, not litter.
//!
//! A Session's worktree is a pool slot, which the pool gives back. `armada
//! worktree` is where one is read and released.

use std::path::{Path, PathBuf};

use adapter_traits::{BaseSpec, Vcs};
use config::Manifest;
use fleet::runtime::{self, Presence};

use crate::serve;
use crate::setup::MANIFEST;

/// The machine files `--all` removes, beside the runtime file itself.
///
/// Named individually rather than by wiping the directory: Application Support
/// is a place other things may put files, and a verb that empties a directory
/// is one nobody can predict.
const MACHINE_FILES: &[&str] = &[
    serve::STORE_FILE,
    "armada.db-wal",
    "armada.db-shm",
    runtime::FILE_NAME,
    serve::MCP_FILE,
];

/// How far one clean reaches. `--all` is a different question from `--force`
/// and is kept a different type, so neither can be passed for the other.
///
/// Both scopes refuse while a Fleet is running, because it holds the records
/// being forgotten in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// This repository's base checkouts.
    Repository,
    /// And the machine's store and the files beside it.
    AndTheMachine,
}

/// A machine file `--all` was asked about.
#[derive(Debug, PartialEq, Eq)]
pub enum FileGone {
    Removed(PathBuf),
    Absent(PathBuf),
    NotRemoved { path: PathBuf, why: String },
}

/// Everything the clean touched, and everything it deliberately did not.
#[derive(Debug, Default)]
pub struct Cleaned {
    pub repository: PathBuf,
    pub manifest_id: String,
    /// Directories under `.armada/worktrees/`. **Left alone.**
    pub unclaimed: Vec<PathBuf>,
    /// Only with `--all`.
    pub machine: Vec<FileGone>,
    /// The shared base checkouts this clean took, by commit.
    ///
    /// **Every one of them, and not only the ones the base has moved past.**
    /// Fleet's sweep keeps the current base because a Session is about to want
    /// it; a clean is a person saying give the disk back, and the checkout is
    /// remade by the next run that needs a *before*. Holding one here would be
    /// `armada clean` leaving behind the largest directory under `.armada/`.
    pub bases: Vec<String>,
    /// Things that went wrong and did not stop the rest.
    pub faults: Vec<String>,
}

impl Cleaned {
    /// Whether anything was removed at all. A clean that did nothing says so
    /// rather than printing an empty list.
    pub fn touched_nothing(&self) -> bool {
        self.bases.is_empty()
            && self
                .machine
                .iter()
                .all(|f| matches!(f, FileGone::Absent(_)))
    }
}

/// Why a clean did not happen at all.
#[derive(Debug)]
pub enum CleanRefused {
    /// There is no Manifest here, so there is no repository to clean.
    NotARepository {
        path: PathBuf,
    },
    ManifestRefused(Box<config::LoadError>),
    /// A Fleet holds the store. **Named by pid**, because the next thing the
    /// caller wants is to stop it.
    FleetIsRunning {
        pid: u32,
        port: u16,
    },
    RuntimeUnreadable(Box<runtime::ReadError>),
    /// The path could not be made absolute, so nothing here could derive a
    /// checkout from it.
    RepositoryUnresolvable {
        path: PathBuf,
        why: std::io::Error,
    },
}

/// Clean `root`, as far as `scope` reaches.
///
/// **It gives back the base checkouts and nothing a record holds.** A Session's
/// transcript, log and results sit under the repository root rather than
/// inside a checkout, so they survive with no exemption written here. Anything
/// added here that reaches outside `.armada/bases/` is that defect arriving
/// again.
pub fn clean(root: &Path, machine: &Path, scope: Scope) -> Result<Cleaned, CleanRefused> {
    let manifest = read_the_manifest(root)?;
    refuse_a_live_fleet(&machine.join(runtime::FILE_NAME))?;

    let root = root
        .canonicalize()
        .map_err(|why| CleanRefused::RepositoryUnresolvable {
            path: root.to_path_buf(),
            why,
        })?;
    let mut cleaned = Cleaned {
        repository: root.clone(),
        manifest_id: manifest.id().as_str().to_string(),
        ..Cleaned::default()
    };

    report_the_worktrees_left(&root, &mut cleaned);
    cleaned.bases = give_back_the_base_checkouts(&root, &mut cleaned.faults);

    if scope == Scope::AndTheMachine {
        cleaned.machine = MACHINE_FILES
            .iter()
            .map(|name| remove(machine, name))
            .collect();
    }
    Ok(cleaned)
}

fn read_the_manifest(root: &Path) -> Result<Manifest, CleanRefused> {
    let path = root.join(MANIFEST);
    Manifest::load(&path).map_err(|why| match &why {
        config::LoadError::Unreadable { cause, .. }
            if cause.kind() == std::io::ErrorKind::NotFound =>
        {
            CleanRefused::NotARepository { path }
        }
        _ => CleanRefused::ManifestRefused(Box::new(why)),
    })
}

/// **The refusal that matters.** Removing the store out from under a live Fleet
/// leaves it holding records that no longer exist.
fn refuse_a_live_fleet(runtime_file: &Path) -> Result<(), CleanRefused> {
    match runtime::read(runtime_file) {
        Ok(Presence::Running(live)) => Err(CleanRefused::FleetIsRunning {
            pid: live.pid,
            port: live.port,
        }),
        Ok(_) => Ok(()),
        Err(why) => Err(CleanRefused::RuntimeUnreadable(Box::new(why))),
    }
}

/// Directories under `.armada/worktrees/`, which nothing makes any more.
///
/// **Reported, never removed.** A Session works in a pool slot, so what is here
/// was cut before that and nothing derives it. A directory a person made by
/// hand is the other shape it can be, and both are worth looking at before
/// anything deletes them.
fn report_the_worktrees_left(root: &Path, cleaned: &mut Cleaned) {
    let parent = root.join(".armada").join("worktrees");
    let Ok(entries) = std::fs::read_dir(&parent) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            cleaned.unclaimed.push(path);
        }
    }
    cleaned.unclaimed.sort();
}

/// Take back every shared base checkout, whatever commit it is at.
///
/// A base checkout belongs to a commit and is shared by every run on it. The
/// directory is a whole checkout plus whatever `setup.requires` installed,
/// which on this repository is the largest thing under `.armada/`.
///
/// **Through a `BaseSpec` rather than by removing the path the walk read**,
/// which is `fleet::holding`'s rule and the reason a hand-run glob destroyed
/// nine branches: what gets deleted is what a derivation names. A directory
/// here whose name is not a commit id is not something Armada made, so it is
/// left — evidence, not litter, exactly as an unclaimed worktree is.
fn give_back_the_base_checkouts(root: &Path, faults: &mut Vec<String>) -> Vec<String> {
    let Some(at) = root.to_str() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(root.join(".armada").join("bases")) else {
        return Vec::new();
    };
    let vcs = adapters::GitVcs::new();
    let mut taken = Vec::new();
    for entry in entries.flatten() {
        let named = entry.file_name().to_string_lossy().into_owned();
        let Ok(spec) = BaseSpec::at(at, &named) else {
            continue;
        };
        match vcs.drop_base_checkout(&spec) {
            Ok(()) => taken.push(named),
            Err(why) => faults.push(format!("a base checkout was not removed: {why}")),
        }
    }
    taken.sort();
    taken
}

fn remove(machine: &Path, name: &str) -> FileGone {
    let path = machine.join(name);
    if !path.exists() {
        return FileGone::Absent(path);
    }
    match std::fs::remove_file(&path) {
        Ok(()) => FileGone::Removed(path),
        Err(why) => FileGone::NotRemoved {
            path,
            why: why.to_string(),
        },
    }
}

impl std::fmt::Display for CleanRefused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanRefused::NotARepository { path } => write!(
                out,
                "there is no {} here — `armada clean` cleans the repository you are standing in",
                path.display()
            ),
            CleanRefused::ManifestRefused(why) => write!(out, "{why}"),
            CleanRefused::FleetIsRunning { pid, port } => write!(
                out,
                "Fleet is running as pid {pid} on port {port}, and it is holding the \
                 store you are asking to clean. Stop it first — `kill {pid}` — then \
                 run this again."
            ),
            CleanRefused::RuntimeUnreadable(why) => write!(out, "{why}"),
            CleanRefused::RepositoryUnresolvable { path, why } => {
                write!(out, "{} could not be resolved: {why}", path.display())
            }
        }
    }
}

impl std::error::Error for CleanRefused {}
