//! Starting a repository in a folder that is in none, and reading where a
//! repository with no `armada.yml` keeps its work.
//!
//! **The first commit is empty**, so a worktree can branch from something. A
//! folder's own files are left as they are, untracked.

use std::path::Path;

use git2::{Repository, RepositoryInitOptions, Signature};

/// The branch a repository Armada starts is on.
const STARTED_ON: &str = "main";

/// Whether `folder`, or a folder above it, is a git repository's working tree.
pub fn inside_a_repository(folder: &Path) -> bool {
    Repository::discover(folder).is_ok()
}

/// `git init` in `folder` with an empty first commit on `main`, and `.armada/`
/// left out of its status, where Fleet keeps the slots. **Refused where `folder`
/// is already inside a repository**, which is no place to start another.
pub fn start_repository(folder: &Path) -> Result<(), String> {
    if inside_a_repository(folder) {
        return Err(format!("{} is already inside a git repository", folder.display()));
    }
    let said = |why: git2::Error| why.message().to_string();
    let mut options = RepositoryInitOptions::new();
    options.initial_head(STARTED_ON);
    let repo = Repository::init_opts(folder, &options).map_err(said)?;
    let tree = repo
        .treebuilder(None)
        .and_then(|empty| empty.write())
        .and_then(|id| repo.find_tree(id))
        .map_err(said)?;
    // A machine with no `user.name` still starts one.
    let who = repo
        .signature()
        .or_else(|_| Signature::now("Armada", "armada@localhost"))
        .map_err(said)?;
    repo.commit(Some("HEAD"), &who, &who, "Start the repository", &tree, &[])
        .map_err(said)?;
    let info = folder.join(".git").join("info");
    std::fs::create_dir_all(&info).map_err(|why| why.to_string())?;
    std::fs::write(info.join("exclude"), ".armada/\n").map_err(|why| why.to_string())
}

/// The branch the repository at `root` merges into when nothing declares one:
/// what `armada.yml`'s `base:` would otherwise say. `None` where it has none.
pub fn inferred_base(root: &Path) -> Option<String> {
    let repo = Repository::open(root).ok()?;
    let base = crate::base::resolve(&repo, None).ok().flatten()?;
    Some(base.name().to_string())
}
