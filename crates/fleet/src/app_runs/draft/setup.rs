//! What a checkout needs installed before its app runs, proposed from the files it tracks: a
//! lockfile beside a `package.json` is that folder's install, and a `requirements.txt` or
//! `pyproject.toml` is a `.venv` made beside it and filled. **Read, never run.**
//!
//! **Each line runs at the checkout's root**, as every `setup.requires` Command does, so a folder
//! is named by a flag (`--prefix`, `--dir`, `--cwd`) or in a path, never by a `dir`: a Command that
//! exits has none, and Verify and `armada run` would run it at the root regardless.
//!
//! **A remembered command survives only if what it names will be there.** Run memory is what a Job
//! started in its own worktree, which held what that Job installed and what it wrote and never
//! committed. A program path or a file a command names is kept where the checkout tracks it or a
//! proposed setup Command makes it, and the rest is not proposed.

use std::path::Path;

use super::{safe_name, DraftSetup};

/// One install for each folder whose files say how its dependencies are installed, in the walk's
/// order: the root first.
pub(super) fn proposed(folders: &[(String, std::path::PathBuf)]) -> Vec<DraftSetup> {
    let mut out = Vec::new();
    for (relative, path) in folders {
        let suffix = match relative.as_str() {
            "." => String::new(),
            below => format!("_{}", safe_name(below)),
        };
        let at = |file: &str| match relative.as_str() {
            "." => file.to_string(),
            below => format!("{below}/{file}"),
        };
        if path.join("package.json").is_file() {
            if let Some(run) = javascript_install(path, relative) {
                out.push(DraftSetup { name: format!("install{suffix}"), run, creates: Some(at("node_modules")) });
            }
        }
        let requirements = path.join("requirements.txt").is_file();
        if requirements || path.join("pyproject.toml").is_file() {
            let venv = at(".venv");
            out.push(DraftSetup {
                name: format!("venv{suffix}"),
                run: format!("python3 -m venv {venv}"),
                creates: Some(venv.clone()),
            });
            let filled = match requirements {
                true => format!("-r {}", at("requirements.txt")),
                false => format!("-e {}", if relative == "." { "." } else { relative.as_str() }),
            };
            out.push(DraftSetup {
                name: format!("pip{suffix}"),
                run: format!("{venv}/bin/python -m pip install {filled}"),
                creates: None,
            });
        }
    }
    out
}

/// The install a lockfile beside `package.json` names, run from the root.
fn javascript_install(folder: &Path, relative: &str) -> Option<String> {
    let below = (relative != ".").then_some(relative);
    let with = |line: &str, flag: &str| match below {
        Some(dir) => format!("{line} {flag} {dir}"),
        None => line.to_string(),
    };
    if folder.join("pnpm-lock.yaml").is_file() {
        return Some(with("pnpm install --frozen-lockfile", "--dir"));
    }
    if folder.join("yarn.lock").is_file() {
        return Some(match below {
            Some(dir) => format!("yarn --cwd {dir} install --frozen-lockfile"),
            None => String::from("yarn install --frozen-lockfile"),
        });
    }
    if folder.join("bun.lock").is_file() || folder.join("bun.lockb").is_file() {
        return Some(with("bun install --frozen-lockfile", "--cwd"));
    }
    if folder.join("package-lock.json").is_file() {
        return Some(with("npm ci", "--prefix"));
    }
    None
}

/// The extensions a word ends in that make it a file a command reads.
const FILE_ENDINGS: &[&str] = &[
    "js", "mjs", "cjs", "ts", "mts", "cts", "jsx", "tsx", "json", "py", "toml", "yaml", "yml", "cfg", "ini",
    "conf", "env", "sh", "txt", "html",
];

/// The paths `line` names, relative to the root, for a process that starts in `dir`: its program
/// where that is a path, and each argument that is a path or a file name.
pub(super) fn named_paths(line: &str, dir: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for (at, word) in line.split_whitespace().enumerate() {
        let word = match word.split_once('=') {
            Some((flag, value)) if flag.starts_with('-') => value,
            _ => word,
        };
        let word = word.trim_matches(['"', '\'']);
        let refused = word.is_empty()
            || word.starts_with('-')
            || word.starts_with('/')
            || word.contains('$')
            || word.contains("://")
            || word.contains(':')
            || word.contains('@');
        if refused {
            continue;
        }
        let file = word.rsplit_once('.').is_some_and(|(stem, ending)| {
            !stem.is_empty() && FILE_ENDINGS.contains(&ending.to_ascii_lowercase().as_str())
        });
        let path = word.contains('/');
        if !(file || path) || (at == 0 && !path) {
            continue;
        }
        if let Some(joined) = joined(dir, word) {
            out.push(joined);
        }
    }
    out
}

/// `word` under `dir`, with `.` and `..` worked out. `None` where it climbs out of the checkout.
fn joined(dir: Option<&str>, word: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in dir.unwrap_or(".").split('/').chain(word.split('/')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Which of `paths` the checkout at `root` tracks: a file in its index, or a folder holding one.
/// **Where git will not answer, a path on disk counts**: a folder that is no repository has no
/// Jobs to have left anything in it.
pub(super) fn tracked(root: &Path, paths: &[String]) -> Vec<bool> {
    if paths.is_empty() {
        return Vec::new();
    }
    let listed = std::process::Command::new("git")
        .args(["ls-files", "-z", "--cached", "--"])
        .args(paths)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()
        .filter(|out| out.status.success());
    let Some(listed) = listed else {
        return paths.iter().map(|path| root.join(path).exists()).collect();
    };
    let files: Vec<String> = listed
        .stdout
        .split(|byte| *byte == 0)
        .filter(|file| !file.is_empty())
        .map(|file| String::from_utf8_lossy(file).into_owned())
        .collect();
    paths
        .iter()
        .map(|path| files.iter().any(|file| file == path || file.starts_with(&format!("{path}/"))))
        .collect()
}

/// Whether `path` is inside something a proposed setup Command makes.
pub(super) fn made_by(path: &str, setup: &[DraftSetup]) -> bool {
    setup
        .iter()
        .filter_map(|one| one.creates.as_deref())
        .any(|made| path == made || path.starts_with(&format!("{made}/")))
}
