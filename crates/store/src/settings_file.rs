//! `settings.json`: the bytes of the Machine's settings file, in and out.
//!
//! **This crate parses and writes the file and knows nothing else about it.**
//! Which keys exist, what kind each is and what range it allows is `config`'s
//! registry; this module hands back a flat object of dotted keys to untyped
//! values, which is what a person can type into the file. It is here because
//! this is one of the two crates allowed to read JSON (`docs/practices/rust.md`
//! section 4).
//!
//! **A write is a temp file and a rename**, pretty-printed with its keys
//! sorted and a trailing newline, so a hand edit and a save read alike in a
//! diff and a reader never meets a file half written.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write as _;
use std::path::Path;

/// One value as the file holds it, before anything has said what it should be.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    List(Vec<SettingValue>),
    Map(BTreeMap<String, SettingValue>),
}

/// The whole file: dotted key to value. A key absent is the shipped default.
pub type SettingsDocument = BTreeMap<String, SettingValue>;

/// Why the file could not be taken as a settings document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsFileError {
    /// The file is there and could not be read.
    Unreadable { why: String },
    /// The bytes are not JSON. Line and column are the parser's, from one.
    NotJson { line: usize, column: usize, why: String },
    /// JSON, but not one object of keys.
    NotAnObject,
}

impl fmt::Display for SettingsFileError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsFileError::Unreadable { why } => write!(out, "settings.json could not be read: {why}"),
            SettingsFileError::NotJson { line, column, why } => {
                write!(out, "settings.json is not JSON at line {line}, column {column}: {why}")
            }
            SettingsFileError::NotAnObject => {
                out.write_str("settings.json is JSON but not one object of keys")
            }
        }
    }
}

impl std::error::Error for SettingsFileError {}

impl SettingValue {
    /// `text` as one value — a Bridge preference that arrives as JSON text,
    /// such as the layout choices. The parser's message on refusal.
    pub fn from_json_text(text: &str) -> Result<SettingValue, String> {
        serde_json::from_str::<serde_json::Value>(text)
            .map(|value| from_json(&value))
            .map_err(|why| why.to_string())
    }

    /// The value as compact JSON text, the shape the older preference
    /// operations carry it in.
    pub fn to_json_text(&self) -> String {
        to_json(self).to_string()
    }
}

/// Read the file at `path`. **No file is `None`**, the shipped defaults, and
/// is not an error.
pub fn read(path: &Path) -> Result<Option<SettingsDocument>, SettingsFileError> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text).map(Some),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(why) => Err(SettingsFileError::Unreadable { why: why.to_string() }),
    }
}

/// The text of a settings file, as a document.
pub fn parse(text: &str) -> Result<SettingsDocument, SettingsFileError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|why| SettingsFileError::NotJson {
            line: why.line(),
            column: why.column(),
            why: plain(&why),
        })?;
    match from_json(&value) {
        SettingValue::Map(keys) => Ok(keys),
        _ => Err(SettingsFileError::NotAnObject),
    }
}

/// The document as the file is written: two-space indent, keys sorted, a
/// trailing newline.
pub fn render(document: &SettingsDocument) -> String {
    let object = serde_json::Value::Object(
        document
            .iter()
            .map(|(key, value)| (key.clone(), to_json(value)))
            .collect(),
    );
    let mut text = serde_json::to_string_pretty(&object).unwrap_or_else(|_| String::from("{}"));
    text.push('\n');
    text
}

/// Write `document` to `path` whole: beside it first, then renamed over it, so
/// a reader sees the old file or the new one and never part of either.
pub fn write(path: &Path, document: &SettingsDocument) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let beside = dir.join(format!(".settings.json.{}.tmp", std::process::id()));
    let written = (|| {
        let mut file = std::fs::File::create(&beside)?;
        file.write_all(render(document).as_bytes())?;
        file.sync_all()
    })();
    match written.and_then(|()| std::fs::rename(&beside, path)) {
        Ok(()) => Ok(()),
        Err(why) => {
            let _ = std::fs::remove_file(&beside);
            Err(why)
        }
    }
}

/// The parser's sentence without the position it appends, which the error
/// carries apart.
fn plain(why: &serde_json::Error) -> String {
    let said = why.to_string();
    match said.rfind(" at line ") {
        Some(cut) => said[..cut].to_string(),
        None => said,
    }
}

fn from_json(value: &serde_json::Value) -> SettingValue {
    match value {
        serde_json::Value::Null => SettingValue::Null,
        serde_json::Value::Bool(flag) => SettingValue::Bool(*flag),
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(whole) => SettingValue::Integer(whole),
            None => SettingValue::Float(number.as_f64().unwrap_or(f64::NAN)),
        },
        serde_json::Value::String(text) => SettingValue::Text(text.clone()),
        serde_json::Value::Array(items) => SettingValue::List(items.iter().map(from_json).collect()),
        serde_json::Value::Object(keys) => SettingValue::Map(
            keys.iter()
                .map(|(key, value)| (key.clone(), from_json(value)))
                .collect(),
        ),
    }
}

fn to_json(value: &SettingValue) -> serde_json::Value {
    match value {
        SettingValue::Null => serde_json::Value::Null,
        SettingValue::Bool(flag) => serde_json::Value::Bool(*flag),
        SettingValue::Integer(whole) => serde_json::Value::from(*whole),
        SettingValue::Float(number) => serde_json::Number::from_f64(*number)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        SettingValue::Text(text) => serde_json::Value::String(text.clone()),
        SettingValue::List(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        SettingValue::Map(keys) => serde_json::Value::Object(
            keys.iter()
                .map(|(key, value)| (key.clone(), to_json(value)))
                .collect(),
        ),
    }
}
