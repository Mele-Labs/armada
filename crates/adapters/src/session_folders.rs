//! The folders a person has already worked in with their agent CLI, found from the sessions the
//! CLI kept.
//!
//! The CLI keeps one directory per working folder under its projects directory, each holding one
//! `*.jsonl` record per session, and every line of a record names the folder it ran in. **The folder
//! is read from those lines.** The directory's name is the folder's path with every character that
//! is not a letter or digit turned into `-`, which cannot be turned back (`a-b` and `a/b` collide),
//! so the name only breaks a tie between folders a session visited.
//!
//! Read only and bounded: the head and tail of a record, never the whole of it, and no process runs
//! — a folder's git state is whether `.git` is there.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Deserialize;

/// Where the CLI keeps its sessions, under a person's home.
pub const PROJECTS: &str = ".claude/projects";

/// The CLI, in its own name, for a screen to show.
pub const AGENT: &str = "Claude Code";

/// The most folders one read returns, newest first. A person with more has stopped looking.
pub const MOST_FOLDERS: usize = 500;

/// How much of the start of a record is read for the folder it names.
const HEAD_BYTES: u64 = 256 * 1024;
/// How much of its end, for a session that began elsewhere and moved.
const TAIL_BYTES: u64 = 128 * 1024;
/// How many of a directory's newest records are looked in before giving up on it.
const RECORDS_LOOKED_IN: usize = 2;

/// Whether a folder is a git repository, from the folder alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderGit {
    Root,
    /// Under a root, which is named.
    Inside(String),
    NotARepository,
    /// The folder is gone.
    Unknown,
}

/// One folder the sessions name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFolder {
    pub path: String,
    pub exists: bool,
    /// When the newest record was last written, in milliseconds since the epoch.
    pub newest_ms: u64,
    pub sessions: usize,
    pub git: FolderGit,
    /// The directories under the projects directory that name it, newest record first within each.
    pub kept_in: Vec<PathBuf>,
}

impl SessionFolder {
    /// What a command's working directory is made relative to: the repository's root when the
    /// folder is inside one, otherwise the folder.
    pub fn root(&self) -> &str {
        match &self.git {
            FolderGit::Inside(root) => root,
            _ => &self.path,
        }
    }
}

/// A directory's name for a folder: every character but an ASCII letter or digit becomes `-`.
pub fn directory_name(folder: &str) -> String {
    folder
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Every folder with sessions under `projects`, newest session first.
pub fn list_folders(projects: &Path) -> Vec<SessionFolder> {
    let Ok(entries) = fs::read_dir(projects) else {
        return Vec::new();
    };
    let mut found: Vec<SessionFolder> = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let records = records_in(&dir);
        let Some(newest) = records.first() else {
            continue;
        };
        let Some(path) = folder_named_by(&dir, &records) else {
            continue;
        };
        match found.iter_mut().find(|one| one.path == path) {
            Some(same) => {
                same.newest_ms = same.newest_ms.max(newest.1);
                same.sessions += records.len();
                same.kept_in.push(dir);
            }
            None => found.push(SessionFolder {
                exists: Path::new(&path).is_dir(),
                git: git_of(Path::new(&path)),
                path,
                newest_ms: newest.1,
                sessions: records.len(),
                kept_in: vec![dir],
            }),
        }
    }
    found.sort_by(|a, b| b.newest_ms.cmp(&a.newest_ms).then_with(|| a.path.cmp(&b.path)));
    found.truncate(MOST_FOLDERS);
    found
}

/// Every record in a directory with its last write, newest first. Sub-directories hold a session's
/// helpers and are not sessions.
pub fn records_in(dir: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut records: Vec<(PathBuf, u64)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|entry| {
            let meta = entry.metadata().ok().filter(|meta| meta.is_file())?;
            let since = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
            Some((entry.path(), since.as_millis() as u64))
        })
        .collect();
    records.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    records
}

/// The folder a directory's records name: the one whose name the directory carries, and failing
/// that the first one seen.
fn folder_named_by(dir: &Path, records: &[(PathBuf, u64)]) -> Option<String> {
    let name = dir.file_name()?.to_string_lossy().to_string();
    let mut first: Option<String> = None;
    for (record, _) in records.iter().take(RECORDS_LOOKED_IN) {
        for seen in folders_in(record) {
            if directory_name(&seen) == name {
                return Some(seen);
            }
            first.get_or_insert(seen);
        }
    }
    first
}

/// The one field read from a line.
#[derive(Deserialize)]
struct Cwd {
    cwd: Option<String>,
}

/// Each distinct folder a record's head and tail name, in the order the lines come.
fn folders_in(record: &Path) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for chunk in head_and_tail(record) {
        for line in chunk.split(|byte| *byte == b'\n') {
            if !contains(line, b"\"cwd\":\"") {
                continue;
            }
            if let Ok(Cwd { cwd: Some(cwd) }) = ipc::decode::<Cwd>("a session record line", line) {
                if cwd.starts_with('/') && !seen.contains(&cwd) {
                    seen.push(cwd);
                }
            }
        }
    }
    seen
}

/// The whole lines in the first and last stretch of a file. A stretch cut at a line's middle loses
/// that line, never reads half of it.
fn head_and_tail(record: &Path) -> Vec<Vec<u8>> {
    let Ok(mut file) = File::open(record) else {
        return Vec::new();
    };
    let Ok(len) = file.metadata().map(|meta| meta.len()) else {
        return Vec::new();
    };
    let mut stretches = Vec::new();
    let mut head = Vec::new();
    if (&mut file).take(HEAD_BYTES).read_to_end(&mut head).is_err() {
        return stretches;
    }
    if len > HEAD_BYTES {
        if let Some(end) = head.iter().rposition(|byte| *byte == b'\n') {
            head.truncate(end);
        }
    }
    stretches.push(head);
    if len > HEAD_BYTES {
        let from = len.saturating_sub(TAIL_BYTES).max(HEAD_BYTES);
        let mut tail = Vec::new();
        if file.seek(SeekFrom::Start(from)).is_ok() && file.read_to_end(&mut tail).is_ok() {
            if let Some(start) = tail.iter().position(|byte| *byte == b'\n') {
                tail.drain(..=start);
            }
            stretches.push(tail);
        }
    }
    stretches
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

/// A folder's git state: the nearest `.git` at or above it.
fn git_of(folder: &Path) -> FolderGit {
    if !folder.is_dir() {
        return FolderGit::Unknown;
    }
    for above in folder.ancestors() {
        if above.join(".git").exists() {
            return match above == folder {
                true => FolderGit::Root,
                false => FolderGit::Inside(above.to_string_lossy().to_string()),
            };
        }
    }
    FolderGit::NotARepository
}
