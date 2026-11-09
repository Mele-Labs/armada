//! Every line this app writes is a Log Envelope line (`docs/concepts/log-envelope.md`).
//!
//! One JSON object per line on standard error, the way Fleet's `note_aloud` writes the lines that
//! belong to no Job: `ts`, `level`, `component`, `run_id`, `msg`, and everything structured in
//! `fields`. The type is `core_model::Envelope`; the wire shape of a line is Fleet's `Line`, which
//! lives in `fleet` and cannot be linked from here, so it is restated below and nothing else is.
//!
//! **`msg` carries no id and no number.** A port, a pid, a count is a field, so a query reads the
//! field and nothing greps the sentence.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::OnceLock;

use core_model::{Component, Envelope, FieldValue, Level, Timestamp, Ulid};
use serde::Serialize;

/// The emitting process's instance, minted at its first line. The one id this app mints; every
/// other id it carries was handed to it by Fleet.
fn run_id() -> &'static Ulid {
    static RUN: OnceLock<Ulid> = OnceLock::new();
    RUN.get_or_init(|| {
        let mut random = [0u8; 10];
        // A machine with no entropy source still logs; its run id is then the clock alone.
        let _ = getrandom::fill(&mut random);
        Ulid::carried(ulid(chrono::Utc::now().timestamp_millis().max(0) as u64, random))
    })
}

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A ULID: 48 bits of milliseconds then 80 of randomness, 26 Crockford base32 characters, most
/// significant first, so a string sort is a time sort.
fn ulid(millis: u64, random: [u8; 10]) -> String {
    let tail = random.iter().fold(0u128, |all, byte| (all << 8) | u128::from(*byte));
    let bits = ((u128::from(millis) & ((1 << 48) - 1)) << 80) | tail;
    (0..26).rev().map(|place| CROCKFORD[((bits >> (place * 5)) & 0x1F) as usize] as char).collect()
}

/// A value in `fields`.
#[derive(Clone, Debug, PartialEq)]
pub struct Field(FieldValue);

impl From<&str> for Field {
    fn from(text: &str) -> Field {
        Field(FieldValue::Str(text.to_string()))
    }
}
impl From<String> for Field {
    fn from(text: String) -> Field {
        Field(FieldValue::Str(text))
    }
}
impl From<i64> for Field {
    fn from(number: i64) -> Field {
        Field(FieldValue::Int(number))
    }
}
impl From<u64> for Field {
    fn from(number: u64) -> Field {
        Field(FieldValue::Int(number as i64))
    }
}
impl From<u32> for Field {
    fn from(number: u32) -> Field {
        Field(FieldValue::Int(i64::from(number)))
    }
}
impl From<usize> for Field {
    fn from(number: usize) -> Field {
        Field(FieldValue::Int(number as i64))
    }
}
impl From<bool> for Field {
    fn from(flag: bool) -> Field {
        Field(FieldValue::Bool(flag))
    }
}

/// One line before it is written, so a test reads what a fold said without a stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    level: Level,
    msg: &'static str,
    fields: BTreeMap<String, FieldValue>,
}

impl Note {
    pub fn info(msg: &'static str) -> Note {
        Note { level: Level::Info, msg, fields: BTreeMap::new() }
    }

    pub fn warn(msg: &'static str) -> Note {
        Note { level: Level::Warn, msg, fields: BTreeMap::new() }
    }

    pub fn error(msg: &'static str) -> Note {
        Note { level: Level::Error, msg, fields: BTreeMap::new() }
    }

    pub fn field(mut self, key: &str, value: impl Into<Field>) -> Note {
        self.fields.insert(key.to_string(), value.into().0);
        self
    }

    #[cfg(test)]
    pub fn msg(&self) -> &'static str {
        self.msg
    }

    #[cfg(test)]
    pub fn get(&self, key: &str) -> Option<&FieldValue> {
        self.fields.get(key)
    }

    fn envelope(&self) -> Envelope {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        self.fields.iter().fold(
            Envelope::new(Timestamp::from_rfc3339(now), self.level, Component::BridgeMain, run_id().clone(), self.msg),
            |envelope, (key, value)| envelope.with_field(key.clone(), value.clone()),
        )
    }

    /// Write the line. A stream that will not take it loses it and nothing else.
    pub fn emit(&self) {
        if let Ok(mut line) = ipc::encode(&Line::from(&self.envelope())) {
            line.push('\n');
            let _ = std::io::stderr().lock().write_all(line.as_bytes());
        }
    }
}

/// `Note::warn(msg).field(..).emit()` in one call, for the common case.
pub fn warn<const N: usize>(msg: &'static str, fields: [(&str, Field); N]) {
    fields.into_iter().fold(Note::warn(msg), |note, (key, value)| note.field(key, value)).emit();
}

/// A line's JSON, in the contract's field order. A key is present with a value or absent, never
/// null, which is why each optional is skipped. `target` and `span` are Fleet's alone.
#[derive(Serialize)]
struct Line<'a> {
    ts: &'a str,
    level: &'static str,
    component: &'static str,
    run_id: &'a str,
    msg: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<&'a str, Value>,
}

/// A `FieldValue` as a person with `jq` reads it: no tag around the value.
#[derive(Serialize)]
#[serde(untagged)]
enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
}

impl From<&FieldValue> for Value {
    fn from(value: &FieldValue) -> Value {
        match value {
            FieldValue::Str(text) => Value::Str(text.clone()),
            FieldValue::Int(number) => Value::Int(*number),
            FieldValue::Float(number) => Value::Float(*number),
            FieldValue::Bool(flag) => Value::Bool(*flag),
            FieldValue::List(all) => Value::List(all.iter().map(Value::from).collect()),
        }
    }
}

impl<'a> From<&'a Envelope> for Line<'a> {
    fn from(envelope: &'a Envelope) -> Line<'a> {
        Line {
            ts: envelope.ts().as_str(),
            level: envelope.level().as_wire(),
            component: envelope.component().as_wire(),
            run_id: envelope.run_id().as_str(),
            msg: envelope.msg(),
            fields: envelope.fields().iter().map(|(key, value)| (key.as_str(), Value::from(value))).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ulid_is_26_characters_and_sorts_by_time() {
        let early = ulid(1, [0xFF; 10]);
        let late = ulid(2, [0; 10]);
        assert_eq!(early.len(), 26);
        assert!(early < late);
        assert_eq!(ulid(0, [0; 10]), "00000000000000000000000000");
        assert_eq!(ulid(0, [0, 0, 0, 0, 0, 0, 0, 0, 0, 1]), "00000000000000000000000001");
    }

    #[test]
    fn a_line_is_the_envelope_with_its_fields_and_no_null() {
        let note = Note::warn("the Plex fonts did not load").field("why", "no such font").field("faces", 5usize);
        let line = ipc::encode(&Line::from(&note.envelope())).expect("a line");
        assert!(line.starts_with("{\"ts\":\""), "{line}");
        assert!(line.contains("\"level\":\"warn\""), "{line}");
        assert!(line.contains("\"component\":\"bridge-main\""), "{line}");
        assert!(line.contains("\"msg\":\"the Plex fonts did not load\""), "{line}");
        assert!(line.contains("\"fields\":{\"faces\":5,\"why\":\"no such font\"}"), "{line}");
        assert!(!line.contains("null"), "{line}");
    }

    #[test]
    fn a_line_without_fields_leaves_the_key_out() {
        let line = ipc::encode(&Line::from(&Note::info("started").envelope())).expect("a line");
        assert!(!line.contains("fields"), "{line}");
    }
}
