//! Listing the checkout for `search_files` — `ipc::FilesFound`'s own notes
//! carry the rest of the reasoning.
//!
//! Git names the files, so `.gitignore` decides what is a candidate: a slot
//! under `.armada/` or a `.venv` is a copy of files that would otherwise answer
//! every query. A folder that is not a repository is walked instead.

use std::path::Path;
use std::process::Command;

/// Directories the fallback walk never descends into. Each is either
/// enormous, generated, or both — and `.armada` holds a full copy of the
/// repository per slot.
const EXCLUDED: &[&str] = &[".git", ".armada", "node_modules", "target"];

/// How many paths `search` answers with at most.
const RESULT_CAP: usize = 50;

/// How many files the fallback walk reads before it stops, so one keystroke
/// in a folder nobody has sized costs a bounded read.
const WALK_CAP: usize = 20_000;

/// Every file under `root` whose path, relative to `root`, contains `query`
/// case-insensitively — `query` empty matches everything, up to the cap.
///
/// **Never fails.** A subdirectory this cannot read is skipped rather than
/// failing the search: `ipc::FilesFound`'s own notes carry the reasoning.
pub(crate) fn search(root: &Path, query: &str) -> Vec<String> {
    let needle = query.to_lowercase();
    let mut found: Vec<String> = listed(root)
        .unwrap_or_else(|| walked(root))
        .into_iter()
        .filter(|path| needle.is_empty() || path.to_lowercase().contains(needle.as_str()))
        .collect();
    found.sort();
    found.truncate(RESULT_CAP);
    found
}

/// Tracked and untracked-but-not-ignored files, or `None` where `root` is not
/// a repository or git cannot be run.
fn listed(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        out.stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect(),
    )
}

/// Every file under `root` outside `EXCLUDED`, up to `WALK_CAP` of them.
fn walked(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![std::path::PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            if found.len() >= WALK_CAP {
                return found;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if EXCLUDED.contains(&name) {
                continue;
            }
            let child = relative.join(name);
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(child);
            } else if file_type.is_file() {
                // Forward-slashed regardless of host, so the path a Windows Fleet
                // walks reads the same as the one every other platform in this
                // workspace already assumes — `docs/practices/protocol.md`'s
                // same-machine rule for a staged path draws the same line.
                found.push(child.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::tmp::TempDir;

    /// A checkout with a nested file, one of the three excluded directories,
    /// and a file that only matches by case.
    fn a_checkout() -> TempDir {
        let home = TempDir::new();
        std::fs::create_dir_all(home.path().join("src/log")).expect("a nested directory");
        std::fs::write(home.path().join("src/log/reader.rs"), b"").expect("a file");
        std::fs::write(home.path().join("README.md"), b"").expect("a file");
        std::fs::create_dir_all(home.path().join("node_modules/left-pad")).expect("excluded");
        std::fs::write(home.path().join("node_modules/left-pad/index.js"), b"")
            .expect("a file under an excluded directory");
        home
    }

    /// An empty query matches every file under the root, not none of them —
    /// this is what lets a popup opened on a bare `@` show something to pick
    /// from rather than an empty list.
    #[test]
    fn an_empty_query_matches_everything_excluded_directories_aside() {
        let home = a_checkout();
        let found = search(home.path(), "");
        assert_eq!(found, vec!["README.md", "src/log/reader.rs"]);
    }

    /// `.git`, `node_modules` and `target` are never descended into, whatever
    /// is typed — the walk never reads what is under them at all.
    #[test]
    fn the_three_excluded_directories_are_never_descended_into() {
        let home = a_checkout();
        assert!(search(home.path(), "left-pad").is_empty());
        assert!(search(home.path(), "index").is_empty());
    }

    /// The match is case-insensitive, against the path relative to the root —
    /// a person typing `@REA` still finds `README.md`.
    #[test]
    fn the_match_is_case_insensitive() {
        let home = a_checkout();
        assert_eq!(search(home.path(), "reAdMe"), vec!["README.md"]);
    }

    /// Narrowed to nothing is an empty list, not every path — the two must
    /// stay distinguishable, or a popup narrowed to nothing would draw as one
    /// nobody has typed into yet.
    #[test]
    fn a_query_matching_nothing_answers_empty() {
        let home = a_checkout();
        assert!(search(home.path(), "zzz-nothing-matches-this").is_empty());
    }

    /// A path is joined with `/` and never `\`, whatever the host — the same
    /// rule a staged path already follows on this seam.
    #[test]
    fn nested_paths_are_forward_slashed() {
        let home = a_checkout();
        let found = search(home.path(), "reader");
        assert_eq!(found, vec!["src/log/reader.rs"]);
        assert!(!found[0].contains('\\'));
    }

    /// More files exist than the cap, and what comes back is cut to it.
    #[test]
    fn the_result_is_capped() {
        let home = TempDir::new();
        for n in 0..RESULT_CAP + 10 {
            std::fs::write(home.path().join(format!("file-{n:03}.txt")), b"").expect("a file");
        }
        assert_eq!(search(home.path(), "").len(), RESULT_CAP);
    }

    /// One git command in `root`, with the user's own configuration off.
    fn git(root: &Path, args: &[&str]) {
        let done = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("git runs");
        assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
    }

    /// A repository ignoring `.armada/` and `.venv/`, holding one tracked file,
    /// one untracked file and a copy of the tracked one under each ignored
    /// directory, the way every slot is a copy of the repository.
    fn a_repository() -> TempDir {
        let home = TempDir::new();
        git(home.path(), &["init", "--quiet"]);
        let files = [
            ".gitignore",
            "src/tracked.rs",
            "src/untracked.rs",
            ".armada/slots/slot-1/src/tracked.rs",
            ".venv/lib/site-packages/tracked.py",
        ];
        for file in files {
            let path = home.path().join(file);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, if file == ".gitignore" { ".armada/\n.venv/\n" } else { "" })
                .expect("a file");
        }
        git(home.path(), &["add", ".gitignore", "src/tracked.rs"]);
        home
    }

    /// What `.gitignore` names never shows, tracked or untracked files do —
    /// each slot under `.armada/` is a full copy of the repository, and would
    /// otherwise answer every query with copies of the same file.
    #[test]
    fn a_repository_lists_what_git_does_not_ignore() {
        let home = a_repository();
        let found = search(home.path(), "tracked");
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.contains(&"src/tracked.rs".to_string()), "{found:?}");
        assert!(found.contains(&"src/untracked.rs".to_string()), "{found:?}");
    }

    /// A folder that is not a repository is walked, and `.armada` is skipped
    /// there too — a Session can run in a folder holding Armada's own slots.
    #[test]
    fn a_folder_without_git_is_walked_and_skips_armada_slots() {
        let home = TempDir::new();
        std::fs::create_dir_all(home.path().join(".armada/slots/slot-1")).expect("slots");
        std::fs::write(home.path().join(".armada/slots/slot-1/copy.rs"), b"").expect("a file");
        std::fs::write(home.path().join("copy.rs"), b"").expect("a file");
        assert_eq!(search(home.path(), "copy"), vec!["copy.rs"]);
    }

    /// A directory this cannot read at all — the root itself missing — is
    /// skipped rather than failing the whole search.
    #[test]
    fn a_root_that_cannot_be_read_answers_empty_rather_than_failing() {
        let missing = std::env::temp_dir().join("armada-files-search-missing-root");
        assert!(search(&missing, "").is_empty());
    }
}
