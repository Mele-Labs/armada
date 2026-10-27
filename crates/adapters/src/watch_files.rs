//! A repository's `.armada/watches/` as a branch holds it, never as the worktree or a Job's branch
//! does.

use std::path::{Path, PathBuf};

use git2::{ObjectType, Repository};

const FOLDER: &str = ".armada/triggers";
const WATCHES: &str = ".armada/watches";

/// Goes with the Trigger callers in the commit that removes them.
pub fn triggers_on_base(root: &Path, base: Option<&str>) -> Vec<(PathBuf, String)> {
    read(root, base.unwrap_or("HEAD"), FOLDER).unwrap_or_default()
}

/// Every watch recipe in `.armada/watches/` at `base`, or at `HEAD` where the repository names no
/// base, with its text. **Empty on any failure**: a repository whose files cannot be read has no
/// recipes, which is the same as one that never wrote any.
pub fn watch_recipes_on_base(root: &Path, base: Option<&str>) -> Vec<(PathBuf, String)> {
    read(root, base.unwrap_or("HEAD"), WATCHES).unwrap_or_default()
}

fn read(root: &Path, base: &str, folder: &str) -> Result<Vec<(PathBuf, String)>, git2::Error> {
    let repo = Repository::open(root)?;
    let tree = repo.revparse_single(base)?.peel_to_commit()?.tree()?;
    let found = tree
        .get_path(Path::new(folder))?
        .to_object(&repo)?
        .peel_to_tree()?;
    let mut out = Vec::new();
    for entry in found.iter() {
        let Some(name) = entry.name() else { continue };
        let yaml = name.ends_with(".yml") || name.ends_with(".yaml");
        if !yaml || entry.kind() != Some(ObjectType::Blob) {
            continue;
        }
        let blob = repo.find_blob(entry.id())?;
        if let Ok(text) = std::str::from_utf8(blob.content()) {
            out.push((Path::new(folder).join(name), text.to_string()));
        }
    }
    Ok(out)
}
