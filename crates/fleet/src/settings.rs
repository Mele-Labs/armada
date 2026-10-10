//! settings.json as Fleet holds it: the file's path, what was in force at
//! start, and the last read that was taken — or the refusal of the last one
//! that was not.
//!
//! **The last good settings stay in force.** A hand edit `config::settings`
//! refuses is recorded and served, and changes nothing; a save it refuses is
//! answered and writes nothing. [`migrated`] carries the store's saved rows
//! into the file once, at the first start that finds no file, and after that
//! the rows are never read again. [`serving`] is the part that speaks for Fleet.

mod serving;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use config::settings::{self as keys, checked, entries, Applies, Env, Key, Kind, Options, Refused, Resolved, Saved, Supplied};
use ipc::{SettingApplies, SettingKind, SettingsList, SettingsRefused};
use store::settings_file::{self, SettingValue, SettingsDocument};

pub use serving::Reread;

/// The Machine's settings file, read.
pub struct MachineSettings {
    path: PathBuf,
    at_start: Resolved,
    held: Mutex<Held>,
    /// One save at a time, so two saves cannot each write the file over what
    /// the other read.
    saving: tokio::sync::Mutex<()>,
}

struct Held {
    now: Resolved,
    refused: Option<Refused>,
}

impl MachineSettings {
    /// The file at `path` read against the table. **A file refused at start is
    /// every default**, there being no last good read to keep, and the refusal
    /// is held to be served and said.
    pub fn at(path: PathBuf, supplied: Supplied, env: Env) -> MachineSettings {
        let base = Resolved::new(Saved::nothing(), supplied, env);
        let (saved, refused) = match read_checked(&path, &base) {
            Ok(saved) => (saved, None),
            Err(refused) => (Saved::nothing(), Some(refused)),
        };
        let at_start = base.with(saved);
        MachineSettings {
            path,
            held: Mutex::new(Held { now: at_start.clone(), refused }),
            at_start,
            saving: tokio::sync::Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What Fleet started with: every `at_restart` setting's value in force.
    pub fn at_start(&self) -> &Resolved {
        &self.at_start
    }

    /// What ships on this machine: the defaults and the variables set, with
    /// nothing from the file. What the composition root hands a live dial in as.
    pub fn shipped(&self) -> Resolved {
        self.at_start.with(Saved::nothing())
    }

    /// The last read that was taken.
    pub fn now(&self) -> Resolved {
        self.held().now.clone()
    }

    /// The refusal of the last read, where it was not taken.
    pub fn refused(&self) -> Option<Refused> {
        self.held().refused.clone()
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Every setting as the wire describes it.
    pub fn listed(&self) -> SettingsList {
        let held = self.held();
        SettingsList {
            path: self.path.display().to_string(),
            refused: held
                .refused
                .as_ref()
                .map(|refused| SettingsRefused { key: refused.key.clone(), reason: refused.reason.clone() }),
            settings: entries()
                .map(|entry| {
                    let now = held.now.effective(entry.key);
                    let started = self.at_start.effective(entry.key);
                    let restart = entry.applies == Applies::AtRestart;
                    ipc::Setting {
                        key: entry.key.to_string(),
                        group: entry.section.group().title().to_string(),
                        section: entry.section.title().to_string(),
                        title: entry.title.to_string(),
                        description: entry.description.to_string(),
                        kind: wire_kind(&entry.kind, || held.now.options_of(entry)),
                        default: wire_value(&held.now.default_of(entry)),
                        value: wire_value(if restart { &started } else { &now }),
                        saved: held.now.saved().get(entry.key).map(wire_value),
                        applies: match entry.applies {
                            Applies::Live => SettingApplies::Live,
                            Applies::AtRestart => SettingApplies::AtRestart,
                        },
                        pending_restart: restart && now != started,
                        overridden_by_env: held.now.overridden_by_env(entry).map(str::to_string),
                    }
                })
                .collect(),
        }
    }
}

/// The file read and checked whole, Bridge's own rules included.
fn read_checked(path: &Path, base: &Resolved) -> Result<Saved, Refused> {
    let document = settings_file::read(path)
        .map_err(|why| Refused::file(why.to_string()))?
        .unwrap_or_default();
    checked_whole(&document, base)
}

/// `config`'s table, then the three rules a Bridge preference has always been
/// held to — a theme's name, a layout.json, key bindings Bridge can read.
pub(crate) fn checked_whole(document: &SettingsDocument, base: &Resolved) -> Result<Saved, Refused> {
    let saved = checked(document, base.supplied(), base.env())?;
    let refused = |key: &str, why: String| Refused { key: Some(key.to_string()), reason: format!("`{key}` {why}") };
    if let Some(SettingValue::Text(theme)) = saved.get(keys::THEME.name()) {
        if let Some(why) = crate::mods::theme_id_problem(theme) {
            return Err(refused(keys::THEME.name(), format!("is not a theme's name: {why}")));
        }
    }
    if let Some(layout) = saved.get(keys::LAYOUT.name()).filter(|value| **value != SettingValue::Null) {
        if let Some(first) = ipc::layout::problems(&layout.to_json_text()).first() {
            return Err(refused(keys::LAYOUT.name(), format!("is not a layout.json: {first}")));
        }
    }
    if let Some(bindings) = saved.get(keys::KEY_BINDINGS.name()).filter(|value| **value != SettingValue::Null) {
        if let Some(first) = ipc::key_bindings::problems(&bindings.to_json_text()).first() {
            return Err(refused(keys::KEY_BINDINGS.name(), format!("could not be read: {first}")));
        }
    }
    Ok(saved)
}

fn wire_kind(kind: &Kind, options: impl FnOnce() -> Vec<String>) -> SettingKind {
    match *kind {
        Kind::Integer { min, max, unit } => SettingKind::Integer { min, max, unit: Some(unit.to_string()) },
        Kind::Seconds { min, max } => SettingKind::Seconds { min, max },
        Kind::Boolean => SettingKind::Boolean,
        Kind::Choice(Options::Fixed(_) | Options::Models | Options::Harnesses) => {
            SettingKind::Choice { options: options() }
        }
        Kind::Text => SettingKind::Text,
        Kind::Prompt(_) => SettingKind::Prompt,
        Kind::TextList => SettingKind::TextList,
        Kind::Json => SettingKind::Json,
    }
}

/// A value the file holds, as the wire carries it.
pub(crate) fn wire_value(value: &SettingValue) -> ipc::SettingValue {
    match value {
        SettingValue::Null => ipc::SettingValue::Null,
        SettingValue::Bool(flag) => ipc::SettingValue::Bool(*flag),
        SettingValue::Integer(n) => ipc::SettingValue::Integer(*n),
        SettingValue::Float(n) => ipc::SettingValue::Float(*n),
        SettingValue::Text(text) => ipc::SettingValue::Text(text.clone()),
        SettingValue::List(items) => ipc::SettingValue::List(items.iter().map(wire_value).collect()),
        SettingValue::Map(keys) => {
            ipc::SettingValue::Map(keys.iter().map(|(key, value)| (key.clone(), wire_value(value))).collect())
        }
    }
}

/// A value off the wire, as the file will hold it.
pub(crate) fn file_value(value: &ipc::SettingValue) -> SettingValue {
    match value {
        ipc::SettingValue::Null => SettingValue::Null,
        ipc::SettingValue::Bool(flag) => SettingValue::Bool(*flag),
        ipc::SettingValue::Integer(n) => SettingValue::Integer(*n),
        ipc::SettingValue::Float(n) => SettingValue::Float(*n),
        ipc::SettingValue::Text(text) => SettingValue::Text(text.clone()),
        ipc::SettingValue::List(items) => SettingValue::List(items.iter().map(file_value).collect()),
        ipc::SettingValue::Map(keys) => {
            SettingValue::Map(keys.iter().map(|(key, value)| (key.clone(), file_value(value))).collect())
        }
    }
}

/// Why the store's rows could not be carried into the file.
#[derive(Debug)]
pub enum NotMigrated {
    Reading(store::DatabaseFault),
    Writing(std::io::Error),
}

impl std::fmt::Display for NotMigrated {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotMigrated::Reading(why) => write!(out, "the saved limits and preferences could not be read: {why}"),
            NotMigrated::Writing(why) => write!(out, "settings.json could not be written: {why}"),
        }
    }
}

impl std::error::Error for NotMigrated {}

/// Carry what the store saved into settings.json, **where there is no file
/// yet**, and answer with the keys written. A store with nothing saved writes
/// no file, and a file that is there is never touched: the rows are not read.
pub fn migrated(store: &store::Store, path: &Path) -> Result<Vec<String>, NotMigrated> {
    if path.exists() {
        return Ok(Vec::new());
    }
    let limits = store.saved_limits().map_err(NotMigrated::Reading)?;
    let preferences = store.saved_preferences().map_err(NotMigrated::Reading)?;
    let mut document = SettingsDocument::new();
    let mut whole = |key: keys::Integer, value: Option<u32>| {
        if let Some(value) = value {
            document.insert(key.name().to_string(), SettingValue::Integer(i64::from(value)));
        }
    };
    whole(keys::DRONES_AT_ONCE, limits.concurrency);
    whole(keys::MEMORY_SPARE_PERCENT, limits.memory_spare_percent);
    whole(keys::DISK_FLOOR_GIB, limits.disk_floor_gib);
    whole(keys::CHECKS_AT_ONCE, limits.checks_at_once);
    let mut put = |key: &str, value: Option<SettingValue>| {
        if let Some(value) = value {
            document.insert(key.to_string(), value);
        }
    };
    put(keys::WHERE_THINGS_ARE_OPEN.name(), preferences.where_things_are_open.map(SettingValue::Bool));
    put(keys::DRAFT_PULL_REQUESTS.name(), preferences.draft_pull_requests.map(SettingValue::Bool));
    put(keys::THEME.name(), preferences.theme.map(SettingValue::Text));
    let json = |text: Option<String>| text.and_then(|text| SettingValue::from_json_text(&text).ok());
    put(keys::LAYOUT.name(), json(preferences.layout_choices));
    put(keys::KEY_BINDINGS.name(), json(preferences.key_bindings));
    // A row only a hand-edited database could hold out of range was ignored
    // before; carried, it would have the file refused at the next start.
    document.retain(|key, value| {
        let one = SettingsDocument::from([(key.clone(), value.clone())]);
        checked_whole(&one, &Resolved::default()).is_ok()
    });
    if document.is_empty() {
        return Ok(Vec::new());
    }
    settings_file::write(path, &document).map_err(NotMigrated::Writing)?;
    Ok(document.into_keys().collect())
}
