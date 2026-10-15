//! Where Bridge's annotation layer writes its notes under a repository root. Here because one of
//! those places is the agent harness's own worktree directory, whose name is the vendor's.

use std::path::{Path, PathBuf};

/// Every directory a note may be in, in the order looked: the root's own checkout, the preview's,
/// each slot's, then each of the harness's worktrees.
pub fn annotation_places(root: &Path) -> Vec<PathBuf> {
    let notes = |checkout: &Path| checkout.join(".armada").join("annotations");
    let mut places = vec![notes(root), notes(&root.join(".armada").join("preview"))];
    for parent in [root.join(".armada").join("slots"), root.join(".claude").join("worktrees")] {
        let mut checkouts: Vec<PathBuf> = std::fs::read_dir(&parent)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        checkouts.sort();
        places.extend(checkouts.iter().map(|checkout| notes(checkout)));
    }
    places
}
