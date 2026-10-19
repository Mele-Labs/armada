//! The Kit layout a repository's runs are written in, for `this_mac` and `synced`: one TOML file
//! a person can read, `runs/<key>.toml` under Kit's folder or under the folder they named.
//!
//! **Keyed by the repository's remote** (`owner--name`), so the same repository cloned twice, or
//! on two machines sharing a synced folder, is one file; a repository with no remote is keyed by
//! its path. A write reads the file first and merges, so a fact another machine added stays.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use store::KeptRun;

/// The folder inside Kit's, or a synced folder, that holds the files.
pub(super) const FOLDER: &str = "runs";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    repository: String,
    #[serde(default)]
    fact: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    workspace: String,
    kind: String,
    command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    succeeded: bool,
    /// Milliseconds since the epoch.
    last_ran: u64,
    source: String,
}

/// What names a repository on disk: its remote's `owner--name`, else its path made safe.
pub(super) fn key_of(root: &str) -> String {
    match adapters::remote_of(Path::new(root)) {
        Some(address) => address.slug().replace('/', "--"),
        None => {
            let safe: String = root.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
            format!("path{safe}")
        }
    }
}

pub(super) fn file_for(base: &Path, root: &str) -> PathBuf {
    base.join(FOLDER).join(format!("{}.toml", key_of(root)))
}

/// The runs a file holds. A file that is not there holds none; one that will not read is an error,
/// since writing over it would lose what a person kept.
pub(super) fn read(path: &Path) -> Result<Vec<KeptRun>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{} could not be read: {why}", path.display())),
    };
    let file: File = toml::from_str(&text).map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    if file.version != VERSION {
        return Err(format!("{} is version {}, and this Fleet reads {VERSION}", path.display(), file.version));
    }
    Ok(file
        .fact
        .into_iter()
        .map(|one| KeptRun {
            workspace: one.workspace,
            kind: one.kind,
            command: one.command,
            port: one.port,
            succeeded: one.succeeded,
            last_ran: one.last_ran,
            source: one.source,
            kept_in: None,
            why: None,
        })
        .collect())
}

/// Write `runs` as the file, over a temporary file renamed into place.
pub(super) fn write(path: &Path, root: &str, runs: &[KeptRun]) -> Result<(), String> {
    let file = File {
        version: VERSION,
        repository: root.to_string(),
        fact: runs
            .iter()
            .map(|one| Entry {
                workspace: one.workspace.clone(),
                kind: one.kind.clone(),
                command: one.command.clone(),
                port: one.port,
                succeeded: one.succeeded,
                last_ran: one.last_ran,
                source: one.source.clone(),
            })
            .collect(),
    };
    let text = toml::to_string_pretty(&file).map_err(|why| why.to_string())?;
    let folder = path.parent().ok_or_else(|| format!("{} has no folder", path.display()))?;
    std::fs::create_dir_all(folder).map_err(|why| format!("{} could not be made: {why}", folder.display()))?;
    let staging = path.with_extension("toml.writing");
    std::fs::write(&staging, text).map_err(|why| format!("{} could not be written: {why}", staging.display()))?;
    std::fs::rename(&staging, path).map_err(|why| format!("{} could not be written: {why}", path.display()))
}
