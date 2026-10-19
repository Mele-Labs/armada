//! Starting a repository in a folder that is in none, and reading where a
//! repository with no `armada.yml` keeps its work.
//!
//! **The first commit holds the folder's current files**, less what its own
//! `.gitignore` leaves out, so a slot branched from it has them. An empty folder
//! gets an empty commit, so a worktree can still branch from something.

use std::path::Path;

use git2::{Repository, RepositoryInitOptions, Signature};

/// The branch a repository Armada starts is on.
const STARTED_ON: &str = "main";

/// Whether `folder`, or a folder above it, is a git repository's working tree.
pub fn inside_a_repository(folder: &Path) -> bool {
    Repository::discover(folder).is_ok()
}

/// `git init` in `folder` with a first commit of its current files on `main`, and `.armada/`
/// left out of its status, where Fleet keeps the slots. **Whether `folder` should
/// be a repository when it is inside one is the caller's to decide**: serving a
/// folder says no, and a new project made inside one says yes.
pub fn start_repository(folder: &Path) -> Result<(), String> {
    let said = |why: git2::Error| why.message().to_string();
    let mut options = RepositoryInitOptions::new();
    options.initial_head(STARTED_ON);
    let repo = Repository::init_opts(folder, &options).map_err(said)?;
    // Before the files are read, so the slots directory is never in the first commit.
    let info = folder.join(".git").join("info");
    std::fs::create_dir_all(&info).map_err(|why| why.to_string())?;
    std::fs::write(info.join("exclude"), ".armada/\n").map_err(|why| why.to_string())?;
    // What the folder already holds, less what its own `.gitignore` leaves out.
    let tree = repo
        .index()
        .and_then(|mut index| {
            index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
            index.write()?;
            index.write_tree()
        })
        .and_then(|id| repo.find_tree(id))
        .map_err(said)?;
    // A machine with no `user.name` still starts one.
    let who = repo
        .signature()
        .or_else(|_| Signature::now("Armada", "armada@localhost"))
        .map_err(said)?;
    repo.commit(Some("HEAD"), &who, &who, "Start the repository", &tree, &[])
        .map_err(said)?;
    Ok(())
}

/// The branch the repository at `root` merges into when nothing declares one:
/// what `armada.yml`'s `base:` would otherwise say. **The branch it is on where
/// it has neither `main` nor `master`**, since a repository with no `armada.yml`
/// has said nothing else. `None` where it is on no branch.
pub fn inferred_base(root: &Path) -> Option<String> {
    let repo = Repository::open(root).ok()?;
    if let Some(base) = crate::base::resolve(&repo, None).ok().flatten() {
        return Some(base.name().to_string());
    }
    let head = repo.head().ok()?;
    head.is_branch().then(|| head.shorthand().map(str::to_string)).flatten()
}
