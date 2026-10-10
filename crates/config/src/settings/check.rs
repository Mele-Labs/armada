//! Whether a settings document is one the table accepts, whole.
//!
//! **The first fault refuses the file**, named by its key, and nothing in it is
//! taken: a file half applied is a machine running on settings nobody wrote.

use std::fmt;

use store::settings_file::{SettingValue, SettingsDocument};

use super::resolve::{options, Env, Supplied};
use super::{entry, Kind};

/// A document every key of which the table accepted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Saved(SettingsDocument);

impl Saved {
    /// Nothing saved: every setting at its default.
    pub fn nothing() -> Saved {
        Saved::default()
    }

    pub fn document(&self) -> &SettingsDocument {
        &self.0
    }

    pub fn get(&self, key: &str) -> Option<&SettingValue> {
        self.0.get(key)
    }
}

/// Why a document was refused. `key` is `None` where the file is not a JSON
/// object at all, so there is no key to blame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub key: Option<String>,
    pub reason: String,
}

impl Refused {
    /// A file that could not be read as a document of keys at all.
    pub fn file(reason: impl Into<String>) -> Refused {
        Refused {
            key: None,
            reason: reason.into(),
        }
    }

    fn at(key: &str, reason: String) -> Refused {
        Refused {
            key: Some(key.to_string()),
            reason,
        }
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.reason)
    }
}

impl std::error::Error for Refused {}

/// `document` as a [`Saved`], or the first key the table refuses. Keys are
/// read in order, so the refusal for a file is the same every time.
pub fn checked(
    document: &SettingsDocument,
    supplied: &Supplied,
    env: &Env,
) -> Result<Saved, Refused> {
    for (key, value) in document {
        let Some(found) = entry(key) else {
            return Err(Refused::at(
                key,
                format!("settings.json names `{key}`, which is not a setting Armada has"),
            ));
        };
        if let Some(why) = fault(&found.kind, value, || {
            options(found, document, supplied, env)
        }) {
            return Err(Refused::at(
                key,
                format!("`{key}` {why}; settings.json has {}", shown(value)),
            ));
        }
    }
    Ok(Saved(document.clone()))
}

/// What is wrong with `value` as `kind`, or `None`.
fn fault(
    kind: &Kind,
    value: &SettingValue,
    choices: impl FnOnce() -> Vec<String>,
) -> Option<String> {
    match (kind, value) {
        (Kind::Integer { min, max, .. }, SettingValue::Integer(n)) if (min..=max).contains(&n) => {
            None
        }
        (Kind::Integer { min, max, unit }, _) => Some(format!(
            "must be a whole number of {unit} from {min} to {max}"
        )),
        (Kind::Seconds { min, max }, SettingValue::Integer(n)) if (min..=max).contains(&n) => None,
        (Kind::Seconds { min, max }, _) => {
            Some(format!("must be whole seconds from {min} to {max}"))
        }
        (Kind::Boolean, SettingValue::Bool(_)) => None,
        (Kind::Boolean, _) => Some("must be true or false".to_string()),
        (Kind::Choice(_), SettingValue::Text(word)) => {
            let choices = choices();
            (!choices.contains(word)).then(|| format!("must be one of {}", choices.join(", ")))
        }
        (Kind::Choice(_), _) => Some(format!("must be one of {}", choices().join(", "))),
        (Kind::Text, SettingValue::Text(_)) => None,
        (Kind::Prompt(keeps), SettingValue::Text(text)) => prompt_fault(keeps, text),
        (Kind::Text | Kind::Prompt(_), _) => Some("must be text".to_string()),
        (Kind::TextList, SettingValue::List(items))
            if !items.is_empty()
                && items
                    .iter()
                    .all(|item| matches!(item, SettingValue::Text(_))) =>
        {
            None
        }
        (Kind::TextList, _) => Some("must be a list of text with at least one item".to_string()),
        (Kind::Json, _) => None,
    }
}

/// What is wrong with `text` as a prompt keeping `keeps`, or `None`. A blank
/// prompt is refused rather than sent: removing the key is how a person goes
/// back to the one that ships.
fn prompt_fault(keeps: &[&str], text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return Some(
            "must not be blank; remove the key to use the prompt Armada ships".to_string(),
        );
    }
    let dropped: Vec<String> = keeps
        .iter()
        .filter(|name| !text.contains(&format!("{{{name}}}")))
        .map(|name| format!("{{{name}}}"))
        .collect();
    (!dropped.is_empty()).then(|| format!("must keep {}, which Fleet fills in", dropped.join(", ")))
}

/// A value as it appears in the file, cut short enough for one line.
fn shown(value: &SettingValue) -> String {
    let text = value.to_json_text();
    match text.char_indices().nth(60) {
        Some((cut, _)) => format!("`{}…`", &text[..cut]),
        None => format!("`{text}`"),
    }
}
