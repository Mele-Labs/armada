//! Listing the checkout for `search_files` — `ipc::FilesFound`'s own notes
//! carry the rest of the reasoning.
//!
//! Git names the files, so `.gitignore` decides what is a candidate: a slot
//! under `.armada/` or a `.venv` is a copy of files that would otherwise answer
//! every query. A folder that is not a repository is walked instead.
//!
//! Matches are ranked rather than filtered: the query's letters in order, with
//! the file name and word starts counting for most.
//!
//! **The listing is kept, not rerun.** Bridge asks on every keystroke after
//! `@`; measured on this repository, `git ls-files` was about 90% of a query
//! (18 ms of 20) and scoring the paths was about 2 ms. So `FileIndex` holds
//! each root's paths in memory and a query only scores them. A listing is
//! retaken in the background once it is `REFRESH_AFTER` old, or at once where
//! git's index or `HEAD` has changed, and the old list answers until the new
//! one lands. Only a root's first query waits for git. Both git and scoring
//! run on the blocking pool, so no keystroke holds an executor thread.
//! Two `Vec<String>` per root, its files and its folders, are the whole memory cost.
//!
//! **Folders are derived from the listing, once per listing**: every ancestor
//! directory of a listed path, kept beside the paths and ranked the same way, so
//! a keystroke that asks for folders reads memory like one that asks for files.
//! A root Fleet does not serve is kept the same way, but only the few asked most
//! recently.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

/// How old a listing may be before a query retakes it in the background. A
/// file created without git knowing (untracked) shows within this long.
const REFRESH_AFTER: Duration = Duration::from_secs(5);

/// Every root this process has been asked about.
static INDEX: LazyLock<FileIndex> = LazyLock::new(|| FileIndex::new(REFRESH_AFTER));

/// Directories the fallback walk never descends into. Each is either
/// enormous, generated, or both — and `.armada` holds a full copy of the
/// repository per slot.
const EXCLUDED: &[&str] = &[".git", ".armada", "node_modules", "target"];

/// How many paths `search` answers with at most.
const RESULT_CAP: usize = 50;

/// How many roots Fleet does not serve keep a listing at once: the most recently asked.
const UNSERVED_KEPT: usize = 4;

/// How many files the fallback walk reads before it stops, so one keystroke
/// in a folder nobody has sized costs a bounded read.
const WALK_CAP: usize = 20_000;

/// What a search answers with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wanted {
    Files,
    Folders,
}

/// The files (or folders) under `root` whose path, relative to `root`, holds
/// `query`'s letters in order, best match first and at most `RESULT_CAP` of
/// them. An empty `query` matches everything.
///
/// **Never fails.** A subdirectory this cannot read is skipped rather than
/// failing the search: `ipc::FilesFound`'s own notes carry the reasoning.
pub(crate) async fn search(root: &Path, query: &str, wanted: Wanted) -> Vec<String> {
    INDEX.answer(root, query, wanted).await
}

/// [`search`] under a root Fleet does not serve, kept like a served one but
/// among only the `UNSERVED_KEPT` asked most recently.
pub(crate) async fn search_elsewhere(root: &Path, query: &str, wanted: Wanted) -> Vec<String> {
    INDEX.elsewhere(root, query, wanted).await
}

/// `paths` narrowed to `query` and ranked.
fn rank(paths: &[String], query: &str) -> Vec<String> {
    let query: Vec<char> = query.chars().map(lower).collect();
    let mut ranked: Vec<(i32, &String)> = paths
        .iter()
        .filter_map(|path| Some((score(path, &query)?, path)))
        .collect();
    // Bridge shows the rows in the order given, so the cut keeps the best.
    ranked.sort_by(|(a_score, a), (b_score, b)| {
        b_score.cmp(a_score).then(a.len().cmp(&b.len())).then(a.cmp(b))
    });
    ranked.into_iter().take(RESULT_CAP).map(|(_, path)| path.clone()).collect()
}

/// What git or the walk names under `root`.
fn list(root: &Path) -> Vec<String> {
    listed(root).unwrap_or_else(|| walked(root))
}

/// One listing per root, answered from memory.
struct FileIndex {
    window: Duration,
    roots: Mutex<HashMap<PathBuf, Arc<Root>>>,
    /// The roots Fleet does not serve, oldest ask first, so the oldest can be let go.
    recent: Mutex<VecDeque<PathBuf>>,
}

#[derive(Default)]
struct Root {
    snapshot: Mutex<Option<Arc<Snapshot>>>,
    /// Held while the first listing runs, so queries arriving meanwhile wait
    /// for that one rather than each running git.
    first: tokio::sync::Mutex<()>,
    refreshing: AtomicBool,
}

/// Git's index and `HEAD` file times: what moves when files are added,
/// removed or renamed, or another branch is checked out.
type Stamp = [Option<SystemTime>; 2];

struct Snapshot {
    paths: Vec<String>,
    /// `paths`' ancestor directories, worked out once with the listing.
    folders: Vec<String>,
    taken: Instant,
    /// Where `stamp` was read, or `None` outside a repository, where only age
    /// retakes the listing.
    git_dir: Option<PathBuf>,
    stamp: Stamp,
}

impl Snapshot {
    /// Blocking: runs git, or walks.
    fn take(root: &Path) -> Self {
        let git_dir = git_dir(root);
        // Read before listing, so a change made meanwhile reads as stale.
        let stamp = git_dir.as_deref().map(stamp).unwrap_or_default();
        let paths = list(root);
        Self {
            taken: Instant::now(),
            folders: folders_of(&paths),
            paths,
            git_dir,
            stamp,
        }
    }

    fn is_stale(&self, window: Duration) -> bool {
        self.taken.elapsed() >= window
            || self.git_dir.as_deref().is_some_and(|dir| stamp(dir) != self.stamp)
    }
}

fn stamp(git_dir: &Path) -> Stamp {
    ["index", "HEAD"].map(|name| std::fs::metadata(git_dir.join(name)).and_then(|m| m.modified()).ok())
}

/// The directory holding `root`'s index and `HEAD` — a worktree's own, not the
/// shared one.
fn git_dir(root: &Path) -> Option<PathBuf> {
    let out = git_in(root).args(["rev-parse", "--absolute-git-dir"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(PathBuf::from(String::from_utf8(out.stdout).ok()?.trim_end()))
}

async fn taken(root: PathBuf) -> Snapshot {
    tokio::task::spawn_blocking(move || Snapshot::take(&root))
        .await
        .unwrap_or_else(|_| Snapshot {
            paths: Vec::new(),
            folders: Vec::new(),
            taken: Instant::now(),
            git_dir: None,
            stamp: Stamp::default(),
        })
}

impl FileIndex {
    fn new(window: Duration) -> Self {
        Self {
            window,
            roots: Mutex::default(),
            recent: Mutex::default(),
        }
    }

    #[cfg(test)]
    async fn search(&self, root: &Path, query: &str) -> Vec<String> {
        self.answer(root, query, Wanted::Files).await
    }

    #[cfg(test)]
    async fn folders(&self, root: &Path, query: &str) -> Vec<String> {
        self.answer(root, query, Wanted::Folders).await
    }

    async fn elsewhere(&self, root: &Path, query: &str, wanted: Wanted) -> Vec<String> {
        self.remember(root);
        self.answer(root, query, wanted).await
    }

    /// Note an ask of an unserved `root`, and let go the listing of the oldest one past the few kept.
    fn remember(&self, root: &Path) {
        let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        recent.retain(|kept| kept != root);
        recent.push_back(root.to_path_buf());
        while recent.len() > UNSERVED_KEPT {
            if let Some(oldest) = recent.pop_front() {
                self.roots.lock().unwrap_or_else(PoisonError::into_inner).remove(&oldest);
            }
        }
    }

    async fn answer(&self, root: &Path, query: &str, wanted: Wanted) -> Vec<String> {
        let held = self
            .roots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(root.to_path_buf())
            .or_default()
            .clone();
        let snapshot = match held.current() {
            Some(snapshot) => snapshot,
            None => held.first(root).await,
        };
        if snapshot.is_stale(self.window) {
            held.refresh(root);
        }
        let query = query.to_owned();
        tokio::task::spawn_blocking(move || {
            rank(
                match wanted {
                    Wanted::Files => &snapshot.paths,
                    Wanted::Folders => &snapshot.folders,
                },
                &query,
            )
        })
        .await
            .unwrap_or_default()
    }
}

impl Root {
    fn current(&self) -> Option<Arc<Snapshot>> {
        self.snapshot.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn keep(&self, snapshot: Arc<Snapshot>) {
        *self.snapshot.lock().unwrap_or_else(PoisonError::into_inner) = Some(snapshot);
    }

    async fn first(&self, root: &Path) -> Arc<Snapshot> {
        let _listing = self.first.lock().await;
        if let Some(snapshot) = self.current() {
            return snapshot;
        }
        let snapshot = Arc::new(taken(root.to_path_buf()).await);
        self.keep(snapshot.clone());
        snapshot
    }

    /// Retake the listing in the background, unless one is already running.
    fn refresh(self: &Arc<Self>, root: &Path) {
        if self.refreshing.swap(true, Ordering::AcqRel) {
            return;
        }
        let this = self.clone();
        let root = root.to_path_buf();
        tokio::spawn(async move {
            this.keep(Arc::new(taken(root).await));
            this.refreshing.store(false, Ordering::Release);
        });
    }
}

/// Every directory above a path in `paths`, once each: relative, `/`-separated,
/// no trailing `/`, and never the root itself.
fn folders_of(paths: &[String]) -> Vec<String> {
    let mut folders = BTreeSet::new();
    for path in paths {
        // Each `/` ends one ancestor, so a path costs one slice per directory above it.
        for (at, _) in path.match_indices('/').filter(|(at, _)| *at > 0) {
            folders.insert(&path[..at]);
        }
    }
    folders.into_iter().map(str::to_owned).collect()
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// How well `query` (lowercased) matches `path`, or `None` where its letters
/// do not appear in order.
///
/// Matched from the end of the path backwards, so the letters land in the file
/// name where they can, not on a directory that happens to hold them earlier.
fn score(path: &str, query: &[char]) -> Option<i32> {
    let chars: Vec<char> = path.chars().collect();
    let name_start = chars.iter().rposition(|c| *c == '/').map_or(0, |i| i + 1);
    let mut at = Vec::with_capacity(query.len());
    let mut end = chars.len();
    for q in query.iter().rev() {
        end = chars[..end].iter().rposition(|c| lower(*c) == *q)?;
        at.push(end);
    }
    // `at` runs last letter first, so the letter before `at[n]` is `at[n + 1]`.
    let mut score = -(chars.len() as i32 / 8);
    for (n, &i) in at.iter().enumerate() {
        score += 1;
        if i >= name_start {
            score += 3;
        }
        let word_start = i == 0
            || matches!(chars[i - 1], '/' | '-' | '_' | '.' | ' ')
            || (chars[i - 1].is_lowercase() && chars[i].is_uppercase());
        if word_start {
            score += 8;
        }
        if at.get(n + 1).is_some_and(|before| before + 1 == i) {
            score += 5;
        }
    }
    let name: String = chars[name_start..].iter().map(|c| lower(*c)).collect();
    if name.contains(&query.iter().collect::<String>()) {
        score += 40;
    }
    Some(score)
}

/// Tracked and untracked-but-not-ignored files, or `None` where `root` is not
/// a repository or git cannot be run.
fn listed(root: &Path) -> Option<Vec<String>> {
    let out = git_in(root)
        .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
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

/// `git` run in `root`, whichever repository the caller's own environment names.
fn git_in(root: &Path) -> Command {
    let mut git = Command::new("git");
    git.current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    git
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

    /// Listed afresh and ranked, the way every query was answered before the
    /// listing was kept — the ranking tests below do not care about the index.
    fn search(root: &Path, query: &str) -> Vec<String> {
        rank(&list(root), query)
    }

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

    /// More files match than the cap: the answer is ranked and then cut, so
    /// the best match survives rather than the first ones listed.
    #[test]
    fn the_result_is_ranked_and_then_capped() {
        let home = TempDir::new();
        for n in 0..RESULT_CAP + 10 {
            std::fs::create_dir_all(home.path().join(format!("mod-{n:03}"))).expect("a directory");
            std::fs::write(home.path().join(format!("mod-{n:03}/x.rs")), b"").expect("a file");
        }
        std::fs::create_dir_all(home.path().join("src")).expect("a directory");
        std::fs::write(home.path().join("src/mod.rs"), b"").expect("a file");
        let found = search(home.path(), "mod");
        assert_eq!(found.len(), RESULT_CAP);
        assert_eq!(found[0], "src/mod.rs");
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

    /// Letters in order are enough, so a half-remembered name finds the file:
    /// `sesscomp` is not a substring of anything here.
    #[test]
    fn a_query_matches_as_a_subsequence() {
        let home = TempDir::new();
        let file = "packages/components/src/compositions/SessionComposer/SessionComposer.tsx";
        let path = home.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(path, b"").expect("a file");
        std::fs::write(home.path().join("README.md"), b"").expect("a file");
        assert_eq!(search(home.path(), "sesscomp"), vec![file]);
    }

    /// A hit in the file name outranks one buried in a directory name, even
    /// where the other sorts first alphabetically.
    #[test]
    fn a_file_name_hit_ranks_above_a_deep_path_hit() {
        let home = TempDir::new();
        let deep = "src/a/reader-tools/deep/nested/other.rs";
        for file in [deep, "src/log/reader.rs"] {
            let path = home.path().join(file);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, b"").expect("a file");
        }
        assert_eq!(search(home.path(), "reader"), vec!["src/log/reader.rs", deep]);
    }

    /// A directory this cannot read at all — the root itself missing — is
    /// skipped rather than failing the whole search.
    #[test]
    fn a_root_that_cannot_be_read_answers_empty_rather_than_failing() {
        let missing = std::env::temp_dir().join("armada-files-search-missing-root");
        assert!(search(&missing, "").is_empty());
    }

    /// Asked until `path` is among the answers, or ten seconds pass.
    async fn appears(index: &FileIndex, root: &Path, query: &str, path: &str) -> bool {
        for _ in 0..400 {
            if index.search(root, query).await.contains(&path.to_string()) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        false
    }

    /// A file made after the first listing is not seen by a second query
    /// within the window: the answer came from memory, not from git.
    #[tokio::test]
    async fn a_second_search_does_not_list_again() {
        let home = a_repository();
        let index = FileIndex::new(Duration::from_secs(3600));
        assert_eq!(index.search(home.path(), "tracked").await.len(), 2);
        std::fs::write(home.path().join("src/late.rs"), b"").expect("a file");
        assert!(index.search(home.path(), "late").await.is_empty());
        assert_eq!(index.search(home.path(), "tracked").await.len(), 2);
    }

    /// Past the window a query still answers from the old list, and the next
    /// ones see the new file once the background listing lands.
    #[tokio::test]
    async fn a_new_file_appears_after_the_refresh_window() {
        let home = a_repository();
        let index = FileIndex::new(Duration::ZERO);
        assert_eq!(index.search(home.path(), "tracked").await.len(), 2);
        std::fs::write(home.path().join("src/late.rs"), b"").expect("a file");
        assert!(index.search(home.path(), "late").await.is_empty());
        assert!(appears(&index, home.path(), "late", "src/late.rs").await);
    }

    /// A change to git's index retakes the listing at once, however far the
    /// window still has to run.
    #[tokio::test]
    async fn a_change_to_gits_index_retakes_the_listing_inside_the_window() {
        let home = a_repository();
        let index = FileIndex::new(Duration::from_secs(3600));
        assert_eq!(index.search(home.path(), "tracked").await.len(), 2);
        git(home.path(), &["mv", "src/tracked.rs", "src/moved.rs"]);
        assert!(appears(&index, home.path(), "moved", "src/moved.rs").await);
    }

    /// A folder that is not a repository is walked through the index too, and
    /// answers from the kept list afterwards.
    #[tokio::test]
    async fn a_folder_without_git_is_walked_into_the_index() {
        let home = a_checkout();
        let index = FileIndex::new(Duration::from_secs(3600));
        assert_eq!(index.search(home.path(), "reader").await, vec!["src/log/reader.rs"]);
        std::fs::write(home.path().join("reader2.rs"), b"").expect("a file");
        assert_eq!(index.search(home.path(), "reader").await, vec!["src/log/reader.rs"]);
    }

    /// A folder is every directory above a listed file and never the root, so
    /// the excluded directories, which list no file, are not folders either.
    #[test]
    fn folders_are_every_ancestor_of_the_indexed_files_and_never_the_root() {
        let home = a_checkout();
        assert_eq!(folders_of(&list(home.path())), vec!["src", "src/log"]);
        assert!(folders_of(&["README.md".to_string()]).is_empty());
        let deep = ["a/b/c/d.rs".to_string(), "a/b/e.rs".to_string()];
        assert_eq!(folders_of(&deep), vec!["a", "a/b", "a/b/c"], "each once, with no trailing slash");
    }

    /// More folders match than the cap: ranked and then cut, so the best
    /// survives, and the shorter of two equal scores comes first.
    #[test]
    fn a_folder_query_ranks_subsequence_matches_best_first_and_caps_them() {
        let home = TempDir::new();
        for n in 0..RESULT_CAP + 10 {
            std::fs::create_dir_all(home.path().join(format!("mod-{n:03}"))).expect("a directory");
            std::fs::write(home.path().join(format!("mod-{n:03}/x.rs")), b"").expect("a file");
        }
        std::fs::create_dir_all(home.path().join("mod")).expect("a directory");
        std::fs::write(home.path().join("mod/x.rs"), b"").expect("a file");
        let found = rank(&folders_of(&list(home.path())), "mod");
        assert_eq!(found.len(), RESULT_CAP);
        assert_eq!(found[0], "mod");
        let letters = rank(&folders_of(&list(home.path())), "m-0");
        assert!(letters.iter().all(|folder| folder.starts_with("mod-")), "a subsequence matches: {letters:?}");
        assert!(rank(&folders_of(&list(home.path())), "zzz-nothing").is_empty());
    }

    /// A folder is found by its last segments, however deep it is.
    #[test]
    fn a_deep_folder_is_found_by_its_last_segments() {
        let home = TempDir::new();
        for file in ["crates/fleet/src/session_host/modding.rs", "crates/fleet/src/files.rs", "docs/concepts/session.md"] {
            let path = home.path().join(file);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
            std::fs::write(path, b"").expect("a file");
        }
        let found = rank(&folders_of(&list(home.path())), "sesshost");
        assert_eq!(found, vec!["crates/fleet/src/session_host"]);
    }

    /// Folders are kept beside the paths and answered from the same snapshot, so a
    /// folder made after the listing is not seen within the window.
    #[tokio::test]
    async fn folders_answer_from_the_kept_listing() {
        let home = a_checkout();
        let index = FileIndex::new(Duration::from_secs(3600));
        assert_eq!(index.folders(home.path(), "log").await, vec!["src/log"]);
        std::fs::create_dir_all(home.path().join("late")).expect("a directory");
        std::fs::write(home.path().join("late/x.rs"), b"").expect("a file");
        assert!(index.folders(home.path(), "late").await.is_empty());
    }

    /// A root Fleet does not serve is listed once and kept: after a file is
    /// deleted, the next keystroke still answers from the first listing, which
    /// proves it read memory and not the disk.
    #[tokio::test]
    async fn a_root_not_served_is_searched_and_kept_for_the_next_keystroke() {
        let home = a_checkout();
        let index = FileIndex::new(Duration::from_secs(3600));
        assert_eq!(index.elsewhere(home.path(), "reader", Wanted::Files).await, vec!["src/log/reader.rs"]);
        std::fs::remove_file(home.path().join("src/log/reader.rs")).expect("deleted");
        assert_eq!(index.elsewhere(home.path(), "reader", Wanted::Files).await, vec!["src/log/reader.rs"]);
        assert_eq!(index.elsewhere(home.path(), "log", Wanted::Folders).await, vec!["src/log"]);
    }

    /// Only the roots asked most recently are kept, so the cache cannot grow without bound.
    #[tokio::test]
    async fn only_the_most_recently_asked_unserved_roots_are_kept() {
        let index = FileIndex::new(Duration::from_secs(3600));
        let homes: Vec<TempDir> = (0..UNSERVED_KEPT + 2).map(|_| a_checkout()).collect();
        for home in &homes {
            index.elsewhere(home.path(), "", Wanted::Files).await;
        }
        let kept = index.roots.lock().expect("roots").len();
        assert_eq!(kept, UNSERVED_KEPT);
        assert!(!index.roots.lock().expect("roots").contains_key(homes[0].path()), "the oldest was let go");
        assert!(index.roots.lock().expect("roots").contains_key(homes[UNSERVED_KEPT + 1].path()));
    }
}
