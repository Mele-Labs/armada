//! The Logbook written where a repository's Armada configuration is kept: the place chosen for its
//! remembered runs, `crate::run_memory`. `docs/concepts/logbook.md`.
//!
//! | section | kept in |
//! |---|---|
//! | `repo` | `logbook/<key>.toml` at the repository's place; `repository` keeps it on this Mac |
//! | `habits`, `machine` | `logbook/habits.toml` and `logbook/machine.toml` under Kit's folder |
//!
//! **Never committed into the repository.** A write reads the file first and takes in what it holds
//! that the store lacks and has not forgotten, so another Mac sharing a synced folder loses nothing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use serde::{Deserialize, Serialize};
use store::{logbook_words, LogEntry, LogEvidence, LogHow, LogKind, LogSection, LogState};

use crate::daemon::Fleet;
use crate::run_memory::{cloud, places};

/// The folder beside `runs/` that holds the Logbook's files.
pub(crate) const FOLDER: &str = "logbook";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    section: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    repository: String,
    #[serde(default)]
    entry: Vec<Line>,
}

#[derive(Serialize, Deserialize)]
struct Line {
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    paths: Vec<String>,
    state: String,
    /// Written before the rename as `pinned`; still read.
    #[serde(default, alias = "pinned")]
    locked: bool,
    #[serde(default)]
    edited_by_person: bool,
    #[serde(default)]
    tripped: u32,
    /// Milliseconds since the epoch.
    first_seen: u64,
    last_confirmed: u64,
    /// The Sessions that bore it out.
    #[serde(default)]
    sessions: Vec<String>,
}

fn line_of(entry: &LogEntry) -> Line {
    let mut sessions: Vec<String> = entry
        .evidence
        .iter()
        .filter(|one| one.how != LogHow::Contradicted)
        .map(|one| one.session_id.clone())
        .collect();
    sessions.sort();
    sessions.dedup();
    Line {
        kind: entry.kind.as_text().to_string(),
        title: entry.title.clone(),
        text: entry.text.clone(),
        paths: entry.paths.clone(),
        state: entry.state.as_text().to_string(),
        locked: entry.locked,
        edited_by_person: entry.edited_by_person,
        tripped: entry.tripped,
        first_seen: entry.first_seen,
        last_confirmed: entry.last_confirmed,
        sessions,
    }
}

fn entry_of(line: Line, section: LogSection, repository: &str) -> Option<LogEntry> {
    Some(LogEntry {
        id: None,
        section,
        repository: repository.to_string(),
        kind: LogKind::of_text(&line.kind)?,
        evidence: line
            .sessions
            .into_iter()
            .map(|session_id| LogEvidence { session_id, cites: Vec::new(), how: LogHow::Confirmed, hard: false, at: line.last_confirmed })
            .collect(),
        title: line.title,
        text: line.text,
        paths: line.paths,
        first_seen: line.first_seen,
        last_confirmed: line.last_confirmed,
        last_served: None,
        state: LogState::of_text(&line.state)?,
        locked: line.locked,
        edited_by_person: line.edited_by_person,
        tripped: line.tripped,
        history: Vec::new(),
    })
}

/// The entries a file's text holds; none for no text. One that will not read is an error, since
/// writing over it would lose what it held.
fn decode(text: Option<&str>, section: LogSection, repository: &str) -> Result<Vec<LogEntry>, String> {
    let Some(text) = text else { return Ok(Vec::new()) };
    let file: File = toml::from_str(text).map_err(|why| why.to_string())?;
    if file.version != VERSION {
        return Err(format!("it is version {}, and this Fleet reads {VERSION}", file.version));
    }
    Ok(file.entry.into_iter().filter_map(|line| entry_of(line, section, repository)).collect())
}

fn encode(section: LogSection, root: &str, entries: &[LogEntry]) -> Result<String, String> {
    let file = File {
        version: VERSION,
        section: section.as_text().to_string(),
        repository: root.to_string(),
        entry: entries.iter().filter(|entry| entry.section == section).map(line_of).collect(),
    };
    toml::to_string_pretty(&file).map_err(|why| why.to_string())
}

/// Where one section's file goes.
enum At {
    File(PathBuf),
    Object(adapters::BucketAddress, String),
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Write a repository's section to its place and the habits and machine sections to Kit's
    /// folder. Quiet on every failure: the store holds the record, and the next write tries again.
    pub(crate) async fn logbook_written(&self, root: &str) {
        let kit = PathBuf::from(&self.host().kit_home);
        let held = self.kept_place(root).await.ok().flatten();
        let repo = match places::where_kept(&kit, held.as_ref()).1 {
            places::Where::Folder(base) => Some(At::File(places::file_for(&base, FOLDER, root))),
            places::Where::Bucket(address, _) => Some(At::Object(address, places::object_for(FOLDER, root))),
            places::Where::Nowhere(_) => None,
        };
        let shared = |name: &str| At::File(kit.join(FOLDER).join(format!("{name}.toml")));
        let sections = [
            (LogSection::Repo, repo),
            (LogSection::Habits, Some(shared("habits"))),
            (LogSection::Machine, Some(shared("machine"))),
        ];
        for (section, at) in sections {
            if let Some(at) = at {
                let _ = self.logbook_section_written(root, section, at).await;
            }
        }
    }

    async fn logbook_section_written(&self, root: &str, section: LogSection, at: At) -> Result<(), String> {
        let key = match section {
            LogSection::Repo => places::key_of(root),
            LogSection::Habits | LogSection::Machine => String::new(),
        };
        let text = match &at {
            At::File(path) => read_file(path)?,
            At::Object(address, object) => {
                let (reaching, address, object) = (Arc::clone(self.reaching()), address.clone(), object.clone());
                let bytes = cloud::blocking(move || reaching.bucket(&address)?.get(&object)).await.map_err(|fault| fault.said().to_string())?;
                bytes.map(|bytes| String::from_utf8(bytes).map_err(|_| String::from("it is not text"))).transpose()?
            }
        };
        let theirs = decode(text.as_deref(), section, &key)?;
        let entries = {
            let mut store = self.store().lock().await;
            let ours = store.logbook_of(&key).map_err(|why| why.to_string())?;
            for entry in theirs {
                let words = logbook_words(&entry.text);
                // An earlier text of an entry is held too, or a rewrite would come back from the file as a second entry.
                let held = ours.iter().any(|one| {
                    one.section == section
                        && one.repository == entry.repository
                        && (logbook_words(&one.text) == words || one.history.iter().any(|was| logbook_words(&was.text) == words))
                });
                if !held {
                    store.save_logbook_entry(&entry).map_err(|why| why.to_string())?;
                }
            }
            store.logbook_of(&key).map_err(|why| why.to_string())?
        };
        let text = encode(section, if section == LogSection::Repo { root } else { "" }, &entries)?;
        match at {
            At::File(path) => write_file(&path, &text),
            At::Object(address, object) => {
                let reaching = Arc::clone(self.reaching());
                cloud::blocking(move || reaching.bucket(&address)?.put(&object, text.as_bytes()))
                    .await
                    .map_err(|fault| fault.said().to_string())
            }
        }
    }
}

fn read_file(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(why) => Err(format!("{} could not be read: {why}", path.display())),
    }
}

/// Write over a temporary file renamed into place.
fn write_file(path: &Path, text: &str) -> Result<(), String> {
    let folder = path.parent().ok_or_else(|| format!("{} has no folder", path.display()))?;
    std::fs::create_dir_all(folder).map_err(|why| format!("{} could not be made: {why}", folder.display()))?;
    let staging = path.with_extension("toml.writing");
    std::fs::write(&staging, text).map_err(|why| format!("{} could not be written: {why}", staging.display()))?;
    std::fs::rename(&staging, path).map_err(|why| format!("{} could not be written: {why}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "version = 1\nsection = \"habits\"\n\n[[entry]]\nkind = \"habit\"\ntext = \"Chrome is in /Applications.\"\nstate = \"standing\"\n";
    const TIMES: &str = "first_seen = 1\nlast_confirmed = 2\n";

    #[test]
    fn a_title_is_written_to_the_file_and_read_back_and_a_file_without_one_reads_untitled() {
        let old = format!("{LINE}{TIMES}");
        let mut entries = decode(Some(&old), LogSection::Habits, "").expect("reads");
        assert_eq!(entries[0].title, None);
        assert!(!encode(LogSection::Habits, "", &entries).expect("encodes").contains("title"));
        entries[0].title = Some("Chrome location".into());
        let written = encode(LogSection::Habits, "", &entries).expect("encodes");
        assert_eq!(decode(Some(&written), LogSection::Habits, "").expect("reads back")[0].title.as_deref(), Some("Chrome location"));
    }

    #[test]
    fn a_file_written_with_pinned_still_loads_as_locked_and_is_written_back_as_locked() {
        let old = format!("{LINE}pinned = true\n{TIMES}");
        let entries = decode(Some(&old), LogSection::Habits, "").expect("the old key reads");
        assert!(entries.len() == 1 && entries[0].locked, "pinned reads as locked");

        let written = encode(LogSection::Habits, "", &entries).expect("encodes");
        assert!(written.contains("locked = true") && !written.contains("pinned"), "{written}");
        assert!(decode(Some(&written), LogSection::Habits, "").expect("reads back")[0].locked);

        let new = format!("{LINE}locked = true\n{TIMES}");
        assert!(decode(Some(&new), LogSection::Habits, "").expect("the new key reads")[0].locked);
    }
}
