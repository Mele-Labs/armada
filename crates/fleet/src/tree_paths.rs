//! Where a path an agent named lies in a repository: which worktree, and the path from that
//! worktree's root. A file is named that way on the timeline and in the write table, so the Field
//! can match it to the repository's tree whichever slot or worktree it was written in.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, SLOT_ROOT};

use crate::daemon::Fleet;
use crate::timeline::{bare, DETAIL};

/// Which kind of worktree a file lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tree {
    /// The repository's own checkout.
    Main,
    /// A slot of the pool.
    Slot(u32),
    /// Any other linked worktree of the repository.
    Linked,
}

/// A file inside a repository, as its own worktree names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InTree {
    /// The worktree's root, absolute.
    pub(crate) worktree: String,
    /// From that root, with no `.` or `..` left in it.
    pub(crate) path: String,
    pub(crate) tree: Tree,
}

/// `path` as an agent standing in `cwd` named it (absolute, from `~/`, or from `cwd`), read against
/// the repository at `root`. `linked` says whether a directory is a linked worktree's root; the
/// pool's slots are known by their place. None where it lies outside the repository.
pub(crate) fn in_tree(root: &str, cwd: Option<&str>, home: &str, path: &str, linked: &dyn Fn(&str) -> bool) -> Option<InTree> {
    let whole = absolute(cwd, home, path)?;
    let root = root.trim_end_matches('/');
    let rest = whole.strip_prefix(root)?.strip_prefix('/')?;
    let parts: Vec<&str> = rest.split('/').collect();
    let pool: Vec<&str> = SLOT_ROOT.split('/').collect();
    let slot = parts
        .get(pool.len())
        .filter(|_| parts.len() > pool.len() + 1 && parts[..pool.len()] == pool[..])
        .and_then(|name| name.strip_prefix("slot-")?.parse::<u32>().ok());
    if let Some(slot) = slot {
        return Some(InTree {
            worktree: adapter_traits::slot_path(root, slot),
            path: parts[pool.len() + 1..].join("/"),
            tree: Tree::Slot(slot),
        });
    }
    // The shallowest wins, so a submodule or a checkout inside a worktree is read as part of it.
    for depth in 1..parts.len() {
        let dir = format!("{root}/{}", parts[..depth].join("/"));
        if linked(&dir) {
            return Some(InTree { worktree: dir, path: parts[depth..].join("/"), tree: Tree::Linked });
        }
    }
    Some(InTree { worktree: root.to_string(), path: rest.to_string(), tree: Tree::Main })
}

/// `path` made absolute and normalised by reading it, never by touching disk. None for a relative
/// path with no directory to read it from, or a `~/` one with no home.
fn absolute(cwd: Option<&str>, home: &str, path: &str) -> Option<String> {
    let whole = match path.strip_prefix("~/") {
        Some(rest) if !home.is_empty() => format!("{}/{rest}", home.trim_end_matches('/')),
        Some(_) => return None,
        None if path.starts_with('/') => path.to_string(),
        None => format!("{}/{path}", cwd?.trim_end_matches('/')),
    };
    let mut kept: Vec<&str> = Vec::new();
    for part in whole.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                kept.pop();
            }
            other => kept.push(other),
        }
    }
    (!kept.is_empty()).then(|| format!("/{}", kept.join("/")))
}

/// Whether `dir` is a linked worktree's root: its `.git` is a file naming a directory under a
/// repository's `.git/worktrees/`. A submodule's names `.git/modules/`, and is not one.
pub(crate) fn linked_worktree(dir: &str) -> bool {
    std::fs::read_to_string(Path::new(dir).join(".git"))
        .ok()
        .and_then(|said| said.lines().find_map(|line| line.strip_prefix("gitdir:")).map(|to| to.trim().contains("/.git/worktrees/")))
        .unwrap_or(false)
}

/// A file as a timeline entry names it. **Only [`file_name`] makes one**, so no entry carries a path
/// spelled from wherever the agent happened to stand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileName(String);

impl FileName {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

/// `path` from its worktree's root, whole, where it lies in one of the repositories `roots` serves;
/// else as it came, cut to a line as any detail is. A line break, which an entry's item cannot hold,
/// is the one thing changed in a whole path.
pub(crate) fn file_name(roots: &[(String, String)], cwd: Option<&str>, home: &str, path: &str, linked: &dyn Fn(&str) -> bool) -> FileName {
    let mut deepest: Vec<&str> = roots.iter().map(|(_, root)| root.trim_end_matches('/')).collect();
    deepest.sort_by_key(|root| std::cmp::Reverse(root.len()));
    let inside = deepest.into_iter().find_map(|root| in_tree(root, cwd, home, path, linked));
    FileName(match inside {
        Some(InTree { path, .. }) if path.contains(['\n', '\r']) => bare(&path, usize::MAX),
        Some(InTree { path, .. }) => path,
        None => bare(path, DETAIL),
    })
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A file an agent standing in `cwd` named, as the timeline names it.
    pub(crate) fn file_named(&self, cwd: Option<&str>, path: &str) -> FileName {
        file_name(&self.ledger_roots(), cwd, &self.host().home, path, &linked_worktree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> bool {
        false
    }

    #[test]
    fn a_slot_path_is_named_from_the_slot_and_a_main_one_from_the_root() {
        let at = |cwd: Option<&str>, path: &str| in_tree("/r", cwd, "/home/me", path, &none);
        let slot = at(None, "/r/.armada/slots/slot-2/crates/x/src/y.rs").expect("inside");
        assert_eq!((slot.path.as_str(), slot.tree, slot.worktree.as_str()), ("crates/x/src/y.rs", Tree::Slot(2), "/r/.armada/slots/slot-2"));
        let from_root = at(Some("/r"), ".armada/slots/slot-2/./a/../b.rs").expect("inside");
        assert_eq!((from_root.path.as_str(), from_root.tree), ("b.rs", Tree::Slot(2)));
        let deep = at(Some("/r/.armada/slots/slot-3/crates"), "x.rs").expect("inside");
        assert_eq!((deep.path.as_str(), deep.tree), ("crates/x.rs", Tree::Slot(3)));
        let main = at(Some("/r"), "src/a.rs").expect("inside");
        assert_eq!((main.path.as_str(), main.tree), ("src/a.rs", Tree::Main));
        assert!(at(Some("/r"), "../elsewhere/a.rs").is_none(), "climbs out");
        assert!(at(None, "/rr/a.rs").is_none(), "a sibling that shares the prefix");
        assert!(at(None, "a.rs").is_none(), "relative with nowhere to read it from");
    }

    #[test]
    fn a_linked_worktree_is_named_from_its_own_root() {
        let linked = |dir: &str| dir == "/r/.wt/x";
        let at = in_tree("/r", Some("/r"), "", ".wt/x/src/a.rs", &linked).expect("inside");
        assert_eq!((at.path.as_str(), at.tree, at.worktree.as_str()), ("src/a.rs", Tree::Linked, "/r/.wt/x"));
    }

    #[test]
    fn a_file_outside_every_repository_is_cut_as_a_detail_is() {
        let roots = [("m".to_string(), "/r".to_string())];
        let long = format!("/tmp/{}", "d/".repeat(100));
        assert_eq!(file_name(&roots, None, "", &long, &none).into_string().chars().count(), DETAIL);
        assert_eq!(file_name(&roots, None, "", "/r/a\nb.rs", &none).into_string(), "a b.rs");
    }
}
