//! A repository's `armada.yml` as a person keeps it in Kit, and what it does to the
//! repository's own. `docs/concepts/manifest.md`, *Kit's copy*.
//!
//! **Kit's is the Manifest where the repository has none.** Where both exist, each Check and
//! Command Kit's names replaces the repository's of the same name, Kit's `tracker` and `ci_gate`
//! replace the repository's whole, and the rest of the repository's stands: a person's own wins
//! over a teammate's commit. A name Kit's holds that the repository does not declare is added, and
//! is named as added.
//!
//! **A Check only Kit's declares is available and gates nothing** unless Kit's writes `gate: true`
//! on it. `gate` is read on no other Check: on one the repository declares it is refused, and Kit's
//! copy is then left out as any that will not merge is.
//!
//! **Merged as YAML, then parsed once**, so every rule a Manifest is held to (a Check's
//! `requires` naming a declared Command, no name in both registries) is checked on the document
//! that runs. **A Kit file that will not merge is left out and named**, and the repository's own
//! stands, as a Kit workflow that does not fit is.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_yaml_ng::{Mapping, Value};

use super::Manifest;
use crate::error::LoadError;

/// The registries Kit's copy may replace by name. **A run and its ports too**, so a person's own way
/// to run the app can bring the servers and the `${port.NAME}` they start on, whether or not the
/// repository's file declares any.
const REGISTRIES: [(&str, KitKind); 4] = [
    ("checks", KitKind::Check),
    ("commands", KitKind::Command),
    ("ports", KitKind::Port),
    ("runs", KitKind::Run),
];

/// The top-level keys Kit's copy may replace whole: where a person's issues and gate are kept is
/// theirs to say, as their Checks are.
const WHOLE_KEYS: [&str; 2] = ["tracker", "ci_gate"];

/// Which registry a name is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitKind {
    Check,
    Command,
    Port,
    Run,
}

impl KitKind {
    pub fn as_wire(self) -> &'static str {
        match self {
            KitKind::Check => "check",
            KitKind::Command => "command",
            KitKind::Port => "port",
            KitKind::Run => "run",
        }
    }
}

/// One Check or Command Kit's copy supplied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KitName {
    pub kind: KitKind,
    pub name: String,
    /// The repository declares this name and says something else for it. **A copy that is the
    /// same hides nothing.**
    pub hides: bool,
    /// The repository declares no such name, and Kit's added it. It hides nothing.
    pub added: bool,
    /// Whether the Check is part of a gate. **`Some` for a Check added to a repository's Manifest,
    /// and for each Check of a copy that is the whole Manifest**, `None` for a Command and for a
    /// Check that replaces the repository's, which gates as the repository's did.
    pub gates: Option<bool>,
}

impl KitName {
    fn read(kind: KitKind, name: String) -> KitName {
        KitName { kind, name, hides: false, added: false, gates: None }
    }
}

/// What Kit's copy did to a repository's Manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FromKit {
    /// Kit's file, as it was read.
    pub file: PathBuf,
    /// The repository has no `armada.yml`, and Kit's is the Manifest.
    pub whole: bool,
    /// The names Kit's supplied, in the order it writes them.
    pub names: Vec<KitName>,
    /// Kit's file would not merge, and the repository's own stands: the sentence that says why.
    pub left_out: Option<String>,
}

/// What reading Kit's copy found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KitRead {
    Text(String),
    /// Kit holds none for this repository.
    Absent,
    /// The place Kit's copy is kept in would not answer: the sentence that says why. **Reads as
    /// no copy**, and is said where a copy would have been named.
    Unread(String),
}

/// Where Kit's copy of one repository's `armada.yml` is read from: a file on this Mac, or the
/// place the repository's tiles are kept in. **Read on each call, never cached.**
pub trait KitCopy: Send + Sync + std::fmt::Debug {
    /// What the copy is called where it is said: its path, or its object's address.
    fn label(&self) -> PathBuf;
    fn read(&self) -> KitRead;
}

/// Kit's copy as a file on this Mac.
#[derive(Debug)]
pub struct KitFile(pub PathBuf);

impl KitCopy for KitFile {
    fn label(&self) -> PathBuf {
        self.0.clone()
    }

    fn read(&self) -> KitRead {
        match std::fs::read_to_string(&self.0) {
            Ok(text) => KitRead::Text(text),
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => KitRead::Absent,
            Err(cause) => KitRead::Unread(cause.to_string()),
        }
    }
}

impl Manifest {
    /// Read and validate an `armada.yml` with Kit's copy at `kit` laid over it, and hand back the
    /// one handle that may re-read both. `kit` is where Kit keeps it, whether or not a file is
    /// there. A repository with no `armada.yml` and no Kit copy is `Unreadable`, as
    /// [`Manifest::reloadable`] answers it.
    pub fn reloadable_over(path: &Path, kit: &Path) -> Result<(Manifest, crate::Reloads), LoadError> {
        Manifest::reloadable_over_copy(path, Arc::new(KitFile(kit.to_path_buf())))
    }

    /// [`reloadable_over`](Manifest::reloadable_over) with Kit's copy read from wherever the
    /// repository's own pieces are kept.
    pub fn reloadable_over_copy(path: &Path, kit: Arc<dyn KitCopy>) -> Result<(Manifest, crate::Reloads), LoadError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => None,
            Err(cause) => return Err(LoadError::Unreadable { path: path.to_path_buf(), cause }),
        };
        let manifest = Manifest::read_over(path, text.as_deref(), Some(kit.as_ref()))?;
        let reloads = crate::Reloads::of(path.to_path_buf(), manifest.live.clone(), &manifest).over(kit);
        Ok((manifest, reloads))
    }

    /// Read an `armada.yml` with Kit's copy at `kit` laid over it, as
    /// [`reloadable_over`](Manifest::reloadable_over) reads it, for a reader that will not re-read.
    pub fn load_over(path: &Path, kit: &Path) -> Result<Manifest, LoadError> {
        Manifest::load_over_copy(path, Arc::new(KitFile(kit.to_path_buf())))
    }

    /// [`load_over`](Manifest::load_over) with Kit's copy read from wherever it is kept.
    pub fn load_over_copy(path: &Path, kit: Arc<dyn KitCopy>) -> Result<Manifest, LoadError> {
        Manifest::reloadable_over_copy(path, kit).map(|(manifest, _)| manifest)
    }

    /// The repository's text, or none, with Kit's copy read and laid over it where it is there.
    /// A copy that cannot be read is left out, as one that will not merge is.
    pub(crate) fn read_over(path: &Path, text: Option<&str>, kit: Option<&dyn KitCopy>) -> Result<Manifest, LoadError> {
        let copy = kit.map(|kit| (kit.label(), kit.read()));
        match (text, copy) {
            (None, Some((label, KitRead::Text(kit_text)))) => Manifest::from_kit_alone(path, &label, &kit_text),
            (None, Some((label, KitRead::Unread(why)))) => {
                // No copy and no file: the repository is not set up, and the reason is the one thing said.
                let cause = std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} was not read: {why}", label.display()));
                Err(LoadError::Unreadable { path: path.to_path_buf(), cause })
            }
            (None, _) => Err(LoadError::Unreadable {
                path: path.to_path_buf(),
                cause: std::io::Error::from(std::io::ErrorKind::NotFound),
            }),
            (Some(text), Some((label, KitRead::Text(kit_text)))) => Manifest::with_kit(path, text, &label, &kit_text),
            (Some(text), Some((label, KitRead::Unread(why)))) => {
                let mut manifest = Manifest::parse(path, text)?;
                let said = format!("{} was left out, and the repository's armada.yml is read alone: {why}", label.display());
                manifest.kit = Some(FromKit { file: label, whole: false, names: Vec::new(), left_out: Some(said) });
                Ok(manifest)
            }
            (Some(text), _) => Manifest::parse(path, text),
        }
    }

    /// What Kit's copy did to this Manifest, where one was read.
    pub fn from_kit(&self) -> Option<&FromKit> {
        self.kit.as_ref()
    }

    /// Kit's copy as the Manifest of a repository that has none. `file` is the repository's
    /// `armada.yml`, which does not exist: the file a person would write, as
    /// [`Manifest::implicit`] has it, so everything that finds the repository from the Manifest
    /// finds the repository and not Kit's folder.
    pub fn from_kit_alone(file: &Path, kit: &Path, text: &str) -> Result<Manifest, LoadError> {
        // Every Check of a whole copy is Kit's own, so `gate` is read on each: absent gates, as a
        // Manifest's Checks do, and `gate: false` keeps one out.
        let (mut manifest, names) = match serde_yaml_ng::from_str::<Value>(text) {
            Ok(mut document) => {
                let marks = ungated(&mut document);
                let names = written(&document, &marks);
                match (marks.is_empty(), serde_yaml_ng::to_string(&document)) {
                    (false, Ok(stripped)) => (Manifest::parse(kit, &stripped)?, names),
                    _ => (Manifest::parse(kit, text)?, names),
                }
            }
            Err(_) => (Manifest::parse(kit, text)?, Vec::new()),
        };
        for one in names.iter().filter(|one| one.gates == Some(false)) {
            if let Some(check) = manifest.checks.get_mut(&one.name) {
                check.gates = false;
            }
        }
        manifest.path = file.to_path_buf();
        manifest.kit = Some(FromKit { file: kit.to_path_buf(), whole: true, names, left_out: None });
        Ok(manifest)
    }

    /// The repository's `armada.yml` with Kit's copy laid over it by name.
    ///
    /// **Only the repository's own refusal refuses.** Kit's that is not YAML, or whose merge
    /// will not parse, is left out with the reason and the repository's file is read alone.
    pub fn with_kit(file: &Path, text: &str, kit: &Path, kit_text: &str) -> Result<Manifest, LoadError> {
        let alone = |why: String| -> Result<Manifest, LoadError> {
            let mut manifest = Manifest::parse(file, text)?;
            manifest.kit = Some(FromKit { file: kit.to_path_buf(), whole: false, names: Vec::new(), left_out: Some(why) });
            Ok(manifest)
        };
        let theirs: Value = match serde_yaml_ng::from_str(text) {
            Ok(value) => value,
            Err(cause) => return Err(LoadError::NotYaml { path: file.to_path_buf(), cause }),
        };
        let mine: Value = match serde_yaml_ng::from_str(kit_text) {
            Ok(value) => value,
            Err(cause) => return alone(left_out(kit, &LoadError::NotYaml { path: kit.to_path_buf(), cause })),
        };
        let (merged, names) = match merged(theirs, &mine) {
            Ok(merged) => merged,
            Err(why) => return alone(format!("{} was left out, because {why}", kit.display())),
        };
        let merged = match serde_yaml_ng::to_string(&merged) {
            Ok(merged) => merged,
            Err(cause) => return alone(format!("{} was left out, because {cause}", kit.display())),
        };
        match Manifest::parse(kit, &merged) {
            Ok(mut manifest) => {
                manifest.path = file.to_path_buf();
                for one in names.iter().filter(|one| one.gates == Some(false)) {
                    if let Some(check) = manifest.checks.get_mut(&one.name) {
                        check.gates = false;
                    }
                }
                manifest.kit = Some(FromKit { file: kit.to_path_buf(), whole: false, names, left_out: None });
                Ok(manifest)
            }
            Err(why) => alone(left_out(kit, &why)),
        }
    }
}

/// Every Check and Command a document names, in the order it writes them. A Check in `marks` gates
/// as it says.
fn written(document: &Value, marks: &BTreeMap<String, bool>) -> Vec<KitName> {
    REGISTRIES
        .iter()
        .flat_map(|(key, kind)| match document.get(key) {
            Some(Value::Mapping(entries)) => entries
                .keys()
                .filter_map(Value::as_str)
                .map(|name| {
                    let mut one = KitName::read(*kind, name.to_string());
                    if *kind == KitKind::Check {
                        one.gates = Some(marks.get(name).copied().unwrap_or(true));
                    }
                    one
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// Every command line the Checks and Commands of an `armada.yml` run, in the order it writes them,
/// or why it is not a mapping of keys. For a reader that shows what a copy would run before it is
/// added: a copy that names only what it changes has no `id` to be a Manifest by.
pub fn manifest_run_lines(text: &str) -> Result<Vec<String>, String> {
    let document: Value = serde_yaml_ng::from_str(text).map_err(|why| format!("is not YAML: {why}"))?;
    if !document.is_mapping() {
        return Err(String::from("is not a mapping of keys"));
    }
    Ok(REGISTRIES
        .iter()
        .filter_map(|(key, _)| match document.get(key) {
            Some(Value::Mapping(entries)) => Some(entries),
            _ => None,
        })
        .flat_map(|entries| {
            entries.values().flat_map(|entry| ["run", "serve"].into_iter().filter_map(|key| entry.get(key).and_then(Value::as_str).map(str::to_string)))
        })
        .collect())
}

/// Why `text` is not a person's own `armada.yml` to keep in Kit, or none where it is: **a whole
/// Manifest, or a copy naming only what it changes.** The first is read as the repository's file
/// would be. The second is a mapping holding only `checks`, `commands`, `ports` and `runs`, each Check with a `run`
/// line, each Command with a `run` or `serve` line, each run with a `commands` list, and `tracker` and `ci_gate`. The reason is a plain sentence a person can read.
pub fn kit_copy_fault(text: &str) -> Option<String> {
    let document: Value = match serde_yaml_ng::from_str(text) {
        Ok(document) => document,
        Err(why) => return Some(plain(&format!("The text is not YAML: {why}"))),
    };
    let Value::Mapping(keys) = &document else {
        return Some(String::from("The text is not a mapping of keys, so it is not an armada.yml."));
    };
    // `gate` is read on a Check by where Kit's copy lands, which is not known here, so it is not judged.
    let mut bare = document.clone();
    ungated(&mut bare);
    let whole = match serde_yaml_ng::to_string(&bare).map(|bare| Manifest::parse(Path::new("armada.yml"), &bare)) {
        Ok(Ok(_)) => return None,
        Ok(Err(why)) => why,
        Err(why) => return Some(plain(&format!("The text is not YAML: {why}"))),
    };
    let overlay = !keys.contains_key("id")
        && !keys.is_empty()
        && keys.keys().all(|key| {
            key.as_str().is_some_and(|key| REGISTRIES.iter().any(|(held, _)| *held == key) || WHOLE_KEYS.contains(&key))
        });
    if !overlay {
        return Some(plain(&format!(
            "The text is not a whole armada.yml: {whole}. A copy that is not whole holds only `checks`, `commands`, `ports`, `runs`, `tracker` and `ci_gate`."
        )));
    }
    for (registry, _) in REGISTRIES {
        let Some(entries) = keys.get(registry) else { continue };
        let Value::Mapping(entries) = entries else {
            return Some(format!("`{registry}` is not a mapping of names to entries."));
        };
        for (name, entry) in entries {
            let name = name.as_str().unwrap_or("?");
            if registry == "ports" {
                if !entry.is_mapping() {
                    return Some(format!("`ports` entry `{name}` is not a mapping, `{{}}` where it declares nothing."));
                }
                continue;
            }
            // A run lists servers, a server is a Command with `serve`, and the rest run a line.
            let (held, line) = match registry {
                "runs" => (entry.get("commands").is_some(), "`commands` list"),
                "commands" => (entry.get("run").or_else(|| entry.get("serve")).and_then(Value::as_str).is_some(), "`run` or `serve` line"),
                _ => (entry.get("run").and_then(Value::as_str).is_some(), "`run` line"),
            };
            if !held {
                return Some(format!("`{registry}` entry `{name}` has no {line}."));
            }
        }
    }
    None
}

/// A sentence with no dash in it, as Fleet says it to a person.
fn plain(said: &str) -> String {
    said.replace(" \u{2014} ", ", ").replace('\u{2014}', ",")
}

fn left_out(kit: &Path, why: &LoadError) -> String {
    format!("{} was left out, and the repository's armada.yml is read alone: {why}", kit.display())
}

type Merged = (Value, Vec<KitName>);

/// Take `gate` off every Check of a copy that is the whole Manifest, and say what each marked.
/// A mark that is not true or false stays where it is, and the parse refuses it.
fn ungated(document: &mut Value) -> BTreeMap<String, bool> {
    let mut marks = BTreeMap::new();
    let Some(Value::Mapping(checks)) = document.get_mut("checks") else { return marks };
    for (name, entry) in checks.iter_mut() {
        let (Some(name), Value::Mapping(entry)) = (name.as_str(), entry) else { continue };
        if let Some(Value::Bool(mark)) = entry.get("gate") {
            marks.insert(name.to_string(), *mark);
            entry.remove("gate");
        }
    }
    marks
}

/// The repository's document with each Check and Command Kit's names replaced or added, and what
/// each did. **A Check added gates only where it says `gate: true`**, and the key is taken off it
/// here: the parse refuses `gate` on every other Check. Why not a mapping of keys, or a `gate` that is
/// not true or false, as a sentence.
fn merged(mut theirs: Value, mine: &Value) -> Result<Merged, String> {
    let mine = match mine {
        Value::Mapping(mine) => mine,
        Value::Null => return Ok((theirs, Vec::new())),
        _ => return Err(String::from("it is not a mapping of keys")),
    };
    let mut names = Vec::new();
    if let Some(top) = theirs.as_mapping_mut() {
        for key in WHOLE_KEYS {
            if let Some(value) = mine.get(key) {
                top.insert(Value::from(key), value.clone());
            }
        }
    }
    for (key, kind) in REGISTRIES {
        let Some(Value::Mapping(entries)) = mine.get(key) else { continue };
        let Some(top) = theirs.as_mapping_mut() else { continue };
        if top.get(key).is_none_or(Value::is_null) {
            top.insert(Value::from(key), Value::Mapping(Mapping::new()));
        }
        let Some(Value::Mapping(held)) = top.get_mut(key) else { continue };
        for (name, entry) in entries {
            let Some(named) = name.as_str().map(str::to_string) else { continue };
            match held.get_mut(name) {
                Some(theirs) => {
                    let hides = theirs != entry;
                    *theirs = entry.clone();
                    names.push(KitName { hides, ..KitName::read(kind, named) });
                }
                None => {
                    let mut added = entry.clone();
                    let mut gates = None;
                    if kind == KitKind::Check {
                        gates = Some(match added.get("gate") {
                            None => false,
                            Some(Value::Bool(mark)) => *mark,
                            Some(_) => return Err(format!("`checks.{named}.gate` is not true or false")),
                        });
                        if let Value::Mapping(entry) = &mut added {
                            entry.remove("gate");
                        }
                    }
                    held.insert(name.clone(), added);
                    names.push(KitName { added: true, gates, ..KitName::read(kind, named) });
                }
            }
        }
    }
    Ok((theirs, names))
}
