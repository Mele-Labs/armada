//! Reading the disk into [`BuildFolder`]s: which checkouts a repository has, which
//! build folders each holds, and whose each is.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{BuildFolder, Role};
use crate::leasing::git::{git_ok, read};
use crate::leasing::{Pool, SlotState};

/// Where a repository's other checkouts sit under its root.
const CHECKOUT_PARENTS: [&str; 5] = [
    ".armada/slots",
    ".armada/bases",
    ".armada/land",
    ".armada/worktrees",
    ".claude/worktrees",
];

/// The build folders in `checkout`: every directory named `target` that holds the `CACHEDIR.TAG`
/// Cargo writes, within three levels. **Never into `node_modules`, a dot directory, or a build
/// folder once found**, so another checkout's folder (`.armada/…`) is its own checkout's.
pub fn build_folders_in(checkout: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut level = vec![checkout.to_path_buf()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    continue;
                }
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name == "node_modules" || name.starts_with('.') {
                    continue;
                }
                let path = entry.path();
                if name == "target" && path.join("CACHEDIR.TAG").is_file() {
                    found.push(path);
                } else {
                    next.push(path);
                }
            }
        }
        level = next;
    }
    found
}

/// Every checkout of the repository at `root`, by canonical path and once each: its own, the
/// folders it keeps slots, bases, land trees and Job worktrees in, the preview, and every
/// worktree git lists for it, wherever that sits.
pub fn every_checkout(root: &Path) -> Vec<PathBuf> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut all = vec![root.clone()];
    for parent in CHECKOUT_PARENTS {
        if let Ok(entries) = fs::read_dir(root.join(parent)) {
            all.extend(entries.flatten().map(|entry| entry.path()));
        }
    }
    all.push(root.join(".armada/preview"));
    if let Ok(listed) = read(&root, &["worktree", "list", "--porcelain"]) {
        all.extend(listed.lines().filter_map(|line| line.strip_prefix("worktree ")).map(PathBuf::from));
    }
    let mut seen = HashSet::new();
    all.into_iter()
        .filter_map(|checkout| checkout.canonicalize().ok())
        .filter(|checkout| checkout.is_dir() && seen.insert(checkout.clone()))
        .collect()
}

/// Every build folder of every checkout of `root`, with whose it is. A checkout in `held` is a
/// person's and is left out. `base` is the branch slots lease from.
pub fn inventory(root: &Path, base: &str, held: &[PathBuf]) -> Vec<BuildFolder> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let held: Vec<PathBuf> = held.iter().filter_map(|checkout| checkout.canonicalize().ok()).collect();
    let preview = root.join(".armada/preview").canonicalize().ok();
    let slots = slot_roles(&root, base);
    let mut seen = HashSet::new();
    let mut folders = Vec::new();
    for checkout in every_checkout(&root) {
        if held.contains(&checkout) {
            continue;
        }
        let role = if checkout == root {
            Role::Main
        } else if Some(&checkout) == preview.as_ref() {
            Role::Preview
        } else {
            slots.get(&checkout).copied().unwrap_or(Role::Other)
        };
        for path in build_folders_in(&checkout) {
            let Ok(path) = path.canonicalize() else {
                continue;
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            folders.push(BuildFolder {
                last_built: last_built(&path),
                has_incremental: profiles(&path).iter().any(|profile| profile.join("incremental").is_dir()),
                path,
                checkout: checkout.clone(),
                role,
                landed: false,
            });
        }
    }
    folders
}

/// Slot checkouts by canonical path: free, or held. A slot nobody holds but whose holder is gone
/// and which is clean and landed reads as free. Anything else, stranded or mid-lease, is held.
fn slot_roles(root: &Path, base: &str) -> HashMap<PathBuf, Role> {
    let count = fs::read_dir(root.join(".armada/slots"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().strip_prefix("slot-")?.parse::<usize>().ok())
        .max()
        .unwrap_or(1);
    Pool::at(root, count, base, Vec::new())
        .status()
        .into_iter()
        .filter_map(|slot| {
            let role = match slot.state {
                SlotState::Unmade => return None,
                SlotState::Free | SlotState::Abandoned { .. } => Role::FreeSlot,
                _ => Role::HeldSlot,
            };
            Some((slot.path.canonicalize().ok()?, role))
        })
        .collect()
}

/// Sets [`BuildFolder::landed`] on every folder a whole removal for that reason could reach.
/// One `git merge-base --is-ancestor` per checkout, so it is for a pass under pressure only.
/// A checkout git cannot answer for reads as not landed.
pub fn mark_landed(folders: &mut [BuildFolder], base: &str) {
    let remote = format!("refs/remotes/origin/{base}");
    let mut answered: HashMap<PathBuf, bool> = HashMap::new();
    for folder in folders.iter_mut().filter(|folder| matches!(folder.role, Role::Other | Role::FreeSlot)) {
        folder.landed = *answered
            .entry(folder.checkout.clone())
            .or_insert_with(|| git_ok(&folder.checkout, &["merge-base", "--is-ancestor", "HEAD", &remote]));
    }
}

/// The profile directories of `target`: each of `target/<profile>` and `target/<triple>/<profile>`
/// that holds `deps` or `.fingerprint`.
fn profiles(target: &Path) -> Vec<PathBuf> {
    let children = |dir: &Path| -> Vec<PathBuf> {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect()
    };
    let mut found = Vec::new();
    for child in children(target) {
        for dir in std::iter::once(child.clone()).chain(children(&child)) {
            if dir.join("deps").is_dir() || dir.join(".fingerprint").is_dir() {
                found.push(dir);
            }
        }
    }
    found
}

/// When a build last wrote: the newest mtime of `deps`, `.fingerprint` and `incremental` and the
/// entries directly in them, in each profile directory. Cheap, since a build adds a file or a
/// directory in each; the folder's own mtime where it has no profile.
fn last_built(target: &Path) -> SystemTime {
    let modified = |path: &Path| fs::metadata(path).and_then(|meta| meta.modified()).ok();
    let mut newest = None::<SystemTime>;
    for profile in profiles(target) {
        for part in ["deps", ".fingerprint", "incremental"] {
            let dir = profile.join(part);
            let entries = fs::read_dir(&dir).into_iter().flatten().flatten().filter_map(|entry| modified(&entry.path()));
            newest = newest.max(modified(&dir).into_iter().chain(entries).max());
        }
    }
    newest.or_else(|| modified(target)).unwrap_or(SystemTime::UNIX_EPOCH)
}
