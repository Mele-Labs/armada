//! Making a mod: the folder, its two files, and a git repository with one commit.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use ipc::ModKind;

use super::{manifest, slug_problem, LAYOUT, MANIFEST, THEME};

/// Why a scaffold did not happen.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// The request names something that cannot be a mod. A 422.
    Unacceptable(String),
    /// Something under it failed. Nothing is left behind.
    Failed(String),
}

/// The starter stylesheet: the shipped dark values for the tokens most worth
/// changing, so it validates and draws as before until someone edits it.
const STARTER: &str = "\
/* Token values only, set in :root. Check it with validate_mod. */
:root {
  --bg-base: #0F1419;
  --bg-raised: #161C23;
  --fg-default: #E4E9EF;
  --accent: #4A9EDB;
}
";

/// The starter layout: the version and no region, so it validates and changes nothing until
/// someone names a region.
const STARTER_LAYOUT: &str = "{\n  \"version\": 1\n}\n";

/// Make `mods_dir/name` and commit it. **The folder is made with `create_dir`**,
/// which refuses one that exists in the same step it makes this one, so a
/// concurrent scaffold of the same name cannot get into the other's folder.
pub(crate) fn make(
    mods_dir: &Path,
    name: &str,
    description: Option<&str>,
    kind: ModKind,
) -> Result<PathBuf, Refused> {
    if let Some(why) = slug_problem(name) {
        return Err(Refused::Unacceptable(why));
    }
    let description = description.map(str::trim).unwrap_or_default();
    if description.chars().count() > manifest::MOST_DESCRIPTION
        || description.chars().any(char::is_control)
    {
        return Err(Refused::Unacceptable(format!(
            "a description is at most {} characters on one line",
            manifest::MOST_DESCRIPTION
        )));
    }
    std::fs::create_dir_all(mods_dir)
        .map_err(|why| Refused::Failed(format!("the mods folder could not be made: {why}")))?;
    let dir = mods_dir.join(name);
    std::fs::create_dir(&dir).map_err(|why| match why.kind() {
        std::io::ErrorKind::AlreadyExists => Refused::Unacceptable(format!("a mod called `{name}` already exists")),
        _ => Refused::Failed(format!("the mod's folder could not be made: {why}")),
    })?;
    match written(&dir, name, description, kind) {
        Ok(()) => Ok(dir),
        Err(why) => {
            // Ours, made a moment ago and holding only what this wrote.
            let _ = std::fs::remove_dir_all(&dir);
            Err(Refused::Failed(why))
        }
    }
}

fn written(dir: &Path, name: &str, description: &str, kind: ModKind) -> Result<(), String> {
    let escaped = description.replace('\\', "\\\\").replace('"', "\\\"");
    let (word, file, starter) = match kind {
        ModKind::Theme => ("theme", THEME, STARTER),
        ModKind::Layout => ("layout", LAYOUT, STARTER_LAYOUT),
    };
    let toml = format!(
        "name = \"{name}\"\nkind = \"{word}\"\nversion = \"0.1.0\"\ndescription = \"{escaped}\"\n"
    );
    for (file, text) in [(MANIFEST, toml.as_str()), (file, starter)] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(file))
            .and_then(|mut made| made.write_all(text.as_bytes()))
            .map_err(|why| format!("{file} could not be written: {why}"))?;
    }
    git(dir, &["init", "--quiet", "--initial-branch=main"])?;
    git(dir, &["add", "--", MANIFEST, file])?;
    git(dir, &["commit", "--quiet", "-m", &format!("Start the {name} {word}")])
}

/// One git command in `dir`, with no configuration but what is passed here.
///
/// **The user's own git configuration is turned off**, so a global hook, a
/// signing requirement or a template cannot decide whether a scaffold works,
/// and the identity is Armada's rather than whatever the owner has set.
fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Armada",
            "-c",
            "user.email=armada@localhost",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|why| format!("git could not be run: {why}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "git {} failed: {}",
        args.first().copied().unwrap_or_default(),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}
