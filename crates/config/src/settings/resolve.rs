//! What each setting is in force: an environment variable, then the file,
//! then what ships — and what ships where only the composition root knows.

use std::collections::BTreeMap;

use store::settings_file::{SettingValue, SettingsDocument};

use super::check::Saved;
use super::{
    entries, entry, Entry, Integer, Key, Kind, List, Options, Shipped, Words, MODELS_DEFAULT,
    MODELS_ROSTER,
};

/// The defaults only known when Fleet starts: half the cores, the models and
/// the agent CLI only `adapters` may spell, a prompt Fleet owns.
#[derive(Clone, Debug, Default)]
pub struct Supplied {
    defaults: BTreeMap<&'static str, SettingValue>,
    harnesses: Vec<String>,
}

impl Supplied {
    pub fn new() -> Supplied {
        Supplied::default()
    }

    pub fn integer(mut self, key: Integer, value: i64) -> Supplied {
        self.defaults
            .insert(key.name(), SettingValue::Integer(value));
        self
    }

    pub fn words(mut self, key: Words, value: impl Into<String>) -> Supplied {
        self.defaults
            .insert(key.name(), SettingValue::Text(value.into()));
        self
    }

    pub fn list(mut self, key: List, items: &[impl AsRef<str>]) -> Supplied {
        let items = items
            .iter()
            .map(|item| SettingValue::Text(item.as_ref().to_string()));
        self.defaults
            .insert(key.name(), SettingValue::List(items.collect()));
        self
    }

    /// The harnesses this build runs, which `harness.agent` chooses from.
    pub fn harnesses(mut self, held: &[impl AsRef<str>]) -> Supplied {
        self.harnesses = held.iter().map(|one| one.as_ref().to_string()).collect();
        self
    }

    /// Every supplied entry nothing was handed in for. **Empty at a sound
    /// composition root**; a key here would have no default to fall back to.
    pub fn missing(&self) -> Vec<&'static str> {
        entries()
            .filter(|entry| {
                entry.shipped == Shipped::Supplied && !self.defaults.contains_key(entry.key)
            })
            .map(|entry| entry.key)
            .collect()
    }
}

/// The environment variables set on this machine that win over the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Env {
    set: BTreeMap<&'static str, (&'static str, String)>,
}

impl Env {
    /// No variable set.
    pub fn none() -> Env {
        Env::default()
    }

    /// Each entry's variable as `lookup` answers it. **Blank is unset**, as it
    /// always was for these variables.
    pub fn read(lookup: impl Fn(&str) -> Option<String>) -> Env {
        let mut set = BTreeMap::new();
        for entry in entries() {
            let Some(var) = entry.env else { continue };
            if let Some(value) = lookup(var).map(|value| value.trim().to_string()) {
                if !value.is_empty() {
                    set.insert(entry.key, (var, value));
                }
            }
        }
        Env { set }
    }

    fn value(&self, key: &str) -> Option<SettingValue> {
        self.set
            .get(key)
            .map(|(_, value)| SettingValue::Text(value.clone()))
    }
}

/// Every setting as it stands: the file that was accepted, the supplied
/// defaults and the variables set.
#[derive(Clone, Debug, Default)]
pub struct Resolved {
    saved: Saved,
    supplied: Supplied,
    env: Env,
}

impl Resolved {
    pub fn new(saved: Saved, supplied: Supplied, env: Env) -> Resolved {
        Resolved {
            saved,
            supplied,
            env,
        }
    }

    /// The same defaults and variables over another accepted file.
    pub fn with(&self, saved: Saved) -> Resolved {
        Resolved {
            saved,
            ..self.clone()
        }
    }

    pub fn saved(&self) -> &Saved {
        &self.saved
    }

    pub fn supplied(&self) -> &Supplied {
        &self.supplied
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    /// What `key` is in force.
    pub fn get<K: Key>(&self, key: K) -> K::Value {
        K::read(&self.effective(key.name()))
    }

    /// What the file says for `key`, **where no variable overrides it** — the
    /// value a dial handed in at assembly gives way to.
    pub fn chosen<K: Key>(&self, key: K) -> Option<K::Value> {
        match self.env.set.contains_key(key.name()) {
            true => None,
            false => self.saved.get(key.name()).map(K::read),
        }
    }

    /// The value in force for the entry named `key`: variable, file, default.
    pub fn effective(&self, key: &str) -> SettingValue {
        self.env
            .value(key)
            .or_else(|| self.saved.get(key).cloned())
            .or_else(|| entry(key).map(|found| self.default_of(found)))
            .unwrap_or(SettingValue::Null)
    }

    /// What `entry` is when nothing is saved and nothing set.
    pub fn default_of(&self, entry: &Entry) -> SettingValue {
        shipped(entry, &self.supplied)
    }

    /// The variable that wins over the file for `entry` on this machine.
    pub fn overridden_by_env(&self, entry: &Entry) -> Option<&'static str> {
        self.env.set.get(entry.key).map(|(var, _)| *var)
    }

    /// The words a [`Kind::Choice`] takes, and nothing for any other kind.
    pub fn options_of(&self, entry: &Entry) -> Vec<String> {
        options(entry, self.saved.document(), &self.supplied, &self.env)
    }

    /// The models a Job may name: the roster in force, led by the default
    /// model's variable where it names one outside it.
    pub fn models(&self) -> Vec<String> {
        roster(self.saved.document(), &self.supplied, &self.env)
    }
}

fn shipped(entry: &Entry, supplied: &Supplied) -> SettingValue {
    match entry.shipped {
        Shipped::Integer(n) | Shipped::Seconds(n) => SettingValue::Integer(n),
        Shipped::Boolean(flag) => SettingValue::Bool(flag),
        Shipped::Text(text) => SettingValue::Text(text.to_string()),
        Shipped::TextList(items) => SettingValue::List(
            items
                .iter()
                .map(|item| SettingValue::Text(item.to_string()))
                .collect(),
        ),
        Shipped::Nothing => SettingValue::Null,
        Shipped::Supplied => supplied
            .defaults
            .get(entry.key)
            .cloned()
            .unwrap_or(SettingValue::Null),
    }
}

/// The words `entry` chooses from, read against `document`'s own roster so a
/// file that changes both is checked against itself.
pub(super) fn options(
    entry: &Entry,
    document: &SettingsDocument,
    supplied: &Supplied,
    env: &Env,
) -> Vec<String> {
    match entry.kind {
        Kind::Choice(Options::Fixed(words)) => words.iter().map(|word| word.to_string()).collect(),
        Kind::Choice(Options::Models) => roster(document, supplied, env),
        Kind::Choice(Options::Harnesses) => supplied.harnesses.clone(),
        _ => Vec::new(),
    }
}

fn roster(document: &SettingsDocument, supplied: &Supplied, env: &Env) -> Vec<String> {
    let listed = document
        .get(MODELS_ROSTER.name())
        .cloned()
        .unwrap_or_else(|| {
            entry(MODELS_ROSTER.name()).map_or(SettingValue::Null, |found| shipped(found, supplied))
        });
    let mut models = List::read(&listed);
    if let Some(SettingValue::Text(named)) = env.value(MODELS_DEFAULT.name()) {
        if !models.contains(&named) {
            models.insert(0, named);
        }
    }
    models
}
