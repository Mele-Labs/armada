//! A repository's `.armada/triggers/` and `.armada/watches/` as a branch holds
//! them, never as the worktree or a Job's branch does. The decision record for
//! Triggers has why, and a watch recipe is kept where a Trigger is.

use std::path::{Path, PathBuf};

use git2::{ObjectType, Repository};

const FOLDER: &str = ".armada/triggers";
const WATCHES: &str = ".armada/watches";

/// Every `.yml` or `.yaml` file in the folder at `base`, or at `HEAD` where the
/// repository names no base, with its text. **Empty on any failure**: a
/// repository whose files cannot be read has no Triggers, which is the same as
/// one that never wrote any.
pub fn triggers_on_base(root: &Path, base: Option<&str>) -> Vec<(PathBuf, String)> {
    read(root, base.unwrap_or("HEAD"), FOLDER).unwrap_or_default()
}

/// Every watch recipe in `.armada/watches/` at `base`, as [`triggers_on_base`]
/// reads Triggers, and empty on any failure for the same reason.
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
