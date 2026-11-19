//! The files in a floor plan's rooms, each with its size. **Sizes are git's**, from one `git ls-tree` at the checkout's
//! `HEAD`, so no file is opened. A plan carries at most `MOST_FILES` entries: the files the feature owns or a live Job
//! wrote stand alone, then the largest, and the rest are taken together by folder. `docs/concepts/feature-catalog.md`,
//! *Floor plan*.

use std::collections::BTreeMap;
use std::path::Path;

use ipc::FloorFile;

/// The most entries a plan carries, single files and folders together. More than this does not draw.
pub(crate) const MOST_FILES: usize = 1500;

/// Endings of files that would fill a room without saying what it does: lock files, images, fonts and test snapshots.
const SKIPPED_ENDINGS: &[&str] =
    &[".lock", ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".icns", ".svg", ".woff", ".woff2", ".ttf", ".otf", ".snap", "package-lock.json", "pnpm-lock.yaml", "go.sum"];

/// Whether a tracked file is left out of every room.
pub(super) fn skipped(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    SKIPPED_ENDINGS.iter().any(|ending| lower.ends_with(ending)) || lower.split('/').any(|segment| segment == "__snapshots__")
}

/// Every file tracked at `HEAD` in the checkout at `root` and its size in bytes, skipped ones left out. Empty where `root`
/// is no repository or git cannot be run. **Blocking**: the caller runs it off the main thread.
pub(crate) fn tracked(root: &Path) -> Vec<(String, u64)> {
    let Ok(out) = crate::files::git_in(root).args(["ls-tree", "-r", "-l", "-z", "HEAD"]).output() else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    out.stdout
        .split(|byte| *byte == 0)
        .filter_map(|line| {
            // `<mode> blob <object> <padded size>\t<path>`; a submodule is a `commit` with `-` for its size.
            let line = String::from_utf8_lossy(line);
            let (head, path) = line.split_once('\t')?;
            let mut fields = head.split_whitespace();
            let kind = fields.nth(1)?;
            let bytes = fields.nth(1)?.parse().ok()?;
            (kind == "blob" && !skipped(path)).then(|| (path.to_string(), bytes))
        })
        .collect()
}

/// A file placed in a room, before the cap.
pub(super) struct Placed {
    pub(super) room: usize,
    pub(super) path: String,
    pub(super) bytes: u64,
    pub(super) lit: bool,
    pub(super) changing: bool,
}

impl Placed {
    fn on_the_wire(self) -> FloorFile {
        FloorFile { path: self.path, bytes: self.bytes, lit: self.lit, changing: self.changing, grouped: 0 }
    }
}

/// The folder a file is taken into at `depth`: its own folder cut to `depth` segments, never above its room's root.
fn folder_at(path: &str, root: &str, depth: usize) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let root_depth = root.split('/').filter(|segment| !segment.is_empty()).count();
    let keep = depth.max(root_depth).min(segments.len() - 1);
    segments[..keep].join("/")
}

/// Each room's entries, largest first, from `placed` and the rooms' roots, at most `most` of them across every room where
/// the files the feature owns or a Job wrote leave room. **Deterministic**: the deepest folder level whose groups take at
/// most half of what is left is chosen, and the largest of the rest fill the other half.
pub(super) fn capped(placed: Vec<Placed>, roots: &[&str], most: usize) -> Vec<Vec<FloorFile>> {
    let mut rooms: Vec<Vec<FloorFile>> = vec![Vec::new(); roots.len()];
    let (kept, mut rest): (Vec<Placed>, Vec<Placed>) = placed.into_iter().partition(|one| one.lit || one.changing);
    let left = most.saturating_sub(kept.len());
    for one in kept {
        rooms[one.room].push(one.on_the_wire());
    }
    rest.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    let groups_at = |depth: usize| rest.iter().map(|one| (one.room, folder_at(&one.path, roots[one.room], depth))).collect::<std::collections::BTreeSet<_>>().len();
    let (singles, depth) = if rest.len() <= left {
        (rest.len(), 0)
    } else {
        let deepest = rest.iter().map(|one| one.path.matches('/').count()).max().unwrap_or(0);
        let depth = (0..=deepest).rev().find(|depth| groups_at(*depth) <= left / 2).unwrap_or(0);
        (left.saturating_sub(groups_at(depth)), depth)
    };
    let grouped = rest.split_off(singles.min(rest.len()));
    for one in rest {
        rooms[one.room].push(one.on_the_wire());
    }
    let mut folders: BTreeMap<(usize, String), Vec<Placed>> = BTreeMap::new();
    for one in grouped {
        folders.entry((one.room, folder_at(&one.path, roots[one.room], depth))).or_default().push(one);
    }
    for ((room, folder), mut files) in folders {
        if files.len() == 1 {
            rooms[room].extend(files.pop().map(Placed::on_the_wire));
            continue;
        }
        let path = if folder.is_empty() { "./".to_string() } else { format!("{folder}/") };
        let bytes = files.iter().map(|one| one.bytes).sum();
        rooms[room].push(FloorFile { path, bytes, lit: false, changing: false, grouped: u32::try_from(files.len()).unwrap_or(u32::MAX) });
    }
    for room in &mut rooms {
        room.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    }
    rooms
}
