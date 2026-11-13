//! Where the notes are kept: one `<id>.json` for each under `<repository root>/.armada/annotations`,
//! the files Electron's layer writes and an agent reads (`M/annotations.ts`, `shared/annotations.ts`).
//! A note is held as the JSON it was written as, so a field this layer does not draw comes back
//! unchanged when it saves the note again. **Every function here blocks on the disk**: the layer calls
//! them on the background executor.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const DIR: [&str; 2] = [".armada", "annotations"];

/// The first directory at or above `start` holding both `pnpm-workspace.yaml` and `Cargo.toml`.
fn repository_root(start: &Path) -> Option<PathBuf> {
    start.ancestors().find(|at| at.join("pnpm-workspace.yaml").exists() && at.join("Cargo.toml").exists()).map(Path::to_path_buf)
}

/// The repository this Bridge writes its notes in. A kept build has none above it, so the tree it was
/// built from is read from the `tree` line of its `build.toml`, which stands beside the executable, or
/// beside the `.app` it is bundled in (`<build>/Armada.app/Contents/MacOS/Armada`).
fn annotation_root(exe: &Path) -> Option<PathBuf> {
    let folder = exe.parent()?;
    if let Some(root) = repository_root(folder) {
        return Some(root);
    }
    folder.ancestors().take(4).map(|at| at.join("build.toml")).find_map(|record| {
        let record = fs::read_to_string(record).ok()?;
        let line = record.lines().find_map(|line| line.strip_prefix("tree = "))?;
        let tree: String = ipc::decode("the build record's tree", line.as_bytes()).ok()?;
        repository_root(Path::new(&tree))
    })
}

/// The directory this Bridge's notes are in, where a repository can be found for it.
pub fn dir() -> Option<PathBuf> {
    let root = annotation_root(&std::env::current_exe().ok()?)?;
    Some(DIR.iter().fold(root, |path, part| path.join(part)))
}

/// A note's id is its file name: `20260916-222105-k3f9`, which sorts by when it was written.
pub fn new_id(at: chrono::DateTime<chrono::Utc>) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.subsec_nanos() as u64 ^ since.as_secs());
    let mut rest = nanos % 36u64.pow(4);
    let mut suffix = [b'0'; 4];
    for place in suffix.iter_mut().rev() {
        *place = b"0123456789abcdefghijklmnopqrstuvwxyz"[(rest % 36) as usize];
        rest /= 36;
    }
    format!("{}-{}", at.format("%Y%m%d-%H%M%S"), String::from_utf8_lossy(&suffix))
}

/// When a note was written or changed, as Electron's `toISOString` says it.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn is_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Every note in the directory, oldest first. A file that does not parse or is not a note is skipped, so
/// one hand-edited file takes no pin off the screen.
pub fn list(dir: &Path) -> Vec<Value> {
    let Ok(names) = fs::read_dir(dir) else { return Vec::new() };
    let mut notes: Vec<Value> = names
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json")?.to_string();
            let note: Value = ipc::decode("a note", fs::read_to_string(entry.path()).ok()?.as_bytes()).ok()?;
            (note.get("id").and_then(Value::as_str) == Some(id.as_str()) && is_id(&id) && note.get("status").is_some()).then_some(note)
        })
        .collect();
    notes.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    notes
}

/// Written through a rename, so a reader never sees half of it. The partial is named per write and does not
/// end in `.json`, so `list` never reads one a crash left.
fn write(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    let partial = dir.join(format!("{name}.{}-{unique}.partial", std::process::id()));
    fs::write(&partial, bytes)?;
    fs::rename(&partial, path)
}

pub fn save(dir: &Path, note: &Value) -> io::Result<()> {
    let id = note.get("id").and_then(Value::as_str).filter(|id| is_id(id)).ok_or_else(|| io::Error::other("not an annotation"))?;
    let mut text = serde_json::to_string_pretty(note).map_err(io::Error::other)?;
    text.push('\n');
    write(dir, &format!("{id}.json"), text.as_bytes())
}

/// A note's picture, `<id>.png`, or one of its example pictures, `<id>-example-<n>.png`.
pub fn save_png(dir: &Path, name: &str, png: &[u8]) -> io::Result<()> {
    write(dir, name, png)
}

/// The note, its picture and its example pictures.
pub fn remove(dir: &Path, id: &str) {
    if !is_id(id) {
        return;
    }
    let _ = fs::remove_file(dir.join(format!("{id}.json")));
    let _ = fs::remove_file(dir.join(format!("{id}.png")));
    let prefix = format!("{id}-example-");
    for entry in fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.strip_prefix(&prefix).is_some_and(|rest| rest.ends_with(".png")) {
            let _ = fs::remove_file(entry.path());
        }
    }
}
