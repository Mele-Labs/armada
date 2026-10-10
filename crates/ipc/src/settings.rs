//! settings.json on the wire: every setting as Fleet resolved it, the save, and
//! the refusal Bridge draws when a hand edit was not taken.
//!
//! **Self-describing**, so Bridge draws the form from what arrives rather than
//! from a second copy of the table: each setting carries its kind with bounds
//! or choices, its default, the value in force and what the file holds.
//! `config::settings` is the table and Fleet converts field by field.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One value as JSON carries it. Its setting's kind says which of these it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    List(Vec<SettingValue>),
    Map(BTreeMap<String, SettingValue>),
}

// JSON has no NaN, so a float that arrived on the wire is equal to itself and
// the `Event` this rides in may stay `Eq`.
impl Eq for SettingValue {}

/// What kind of value a setting holds, with its bounds or its choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SettingKind {
    Integer { min: i64, max: i64, unit: Option<String> },
    /// Stored in seconds; Bridge reads it out in minutes or hours as well.
    Seconds { min: i64, max: i64 },
    Boolean,
    Choice { options: Vec<String> },
    Text,
    /// Several lines of text, whose default is a prompt Fleet ships.
    Prompt,
    TextList,
    /// A value Bridge owns and draws on a pane of its own.
    Json,
}

/// When a saved change reaches what it governs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingApplies {
    Live,
    AtRestart,
}

/// One setting, as Fleet resolved it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    /// Dotted camelCase, `limits.dronesAtOnce`: the name in settings.json.
    pub key: String,
    /// The group in Bridge's index: Agents, Prompts, Fleet, Features, Tools or
    /// Appearance. `section` is the sub-heading inside it.
    pub group: String,
    pub section: String,
    pub title: String,
    pub description: String,
    pub kind: SettingKind,
    pub default: SettingValue,
    /// What is in force: an environment variable, then the file, then the
    /// default. For an `at_restart` setting, what Fleet started with.
    pub value: SettingValue,
    /// What settings.json holds for it, or `null` where the key is absent.
    #[serde(deserialize_with = "stated")]
    pub saved: Option<SettingValue>,
    pub applies: SettingApplies,
    /// The saved value is not the one in force until Fleet restarts.
    pub pending_restart: bool,
    /// The environment variable that wins over the file on this machine.
    #[serde(deserialize_with = "stated")]
    pub overridden_by_env: Option<String>,
}

/// Why the last read of settings.json was refused. The last good settings
/// stay in force.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsRefused {
    /// The key at fault, or `null` where the file is not a JSON object at all.
    #[serde(deserialize_with = "stated")]
    pub key: Option<String>,
    pub reason: String,
}

/// `get_settings`, the answer to `save_settings`, and `settings.changed`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsList {
    /// settings.json's absolute path. It need not exist yet.
    pub path: String,
    #[serde(deserialize_with = "stated")]
    pub refused: Option<SettingsRefused>,
    pub settings: Vec<Setting>,
}

/// A save — `POST /settings/save`. **`null` removes the key**, so the shipped
/// default is back in force; a key the save does not name is left as it was.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSettings {
    pub changes: BTreeMap<String, Option<SettingValue>>,
}

/// An `Option` whose key must be present: `null` is the answer — nothing saved,
/// no variable, no refusal — and Bridge's mirror reads it as one rather than
/// as a field a Fleet before it left out.
fn stated<'de, D, T>(input: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(input)
}
