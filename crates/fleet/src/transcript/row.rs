//! What a line looks like on disk.
//!
//! Two shapes, because there are two files. A transcript row is one
//! `DroneEvent` and the instant Fleet saw it, and it is `ipc::TranscriptRow`
//! rather than a type here — the live view sends the same shape, and two
//! declarations would be two vocabularies. What stayed is the mapping: `ipc`
//! depends on `core-model` and nothing else, so the arm that turns a
//! `DroneEvent` into a row lives beside the loop that holds the event.
//!
//! A Job log line is `core_model::Envelope`, and [`Line`] is its wire shape.
//! The derives are here rather than on `Envelope` because `core-model` has no
//! dependencies at all, deliberately, so serde on it would be serde everywhere.

use std::collections::BTreeMap;

use core_model::{Envelope, FieldValue};
use serde::Serialize;

/// One line of `.armada/logs/<handle>.jsonl`, in the contract's field order.
///
/// **A key is present with a value or absent, never present and null**, which
/// is why every conditional field is skipped rather than written. `target` and
/// `span` are absent from this type entirely: they are supplied by a `tracing`
/// layer, and Fleet has none.
#[derive(Debug, Serialize)]
pub(crate) struct Line<'a> {
    ts: &'a str,
    level: &'static str,
    component: &'static str,
    run_id: &'a str,
    msg: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drone_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    step_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<&'a str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<&'a str, Field>,
}

impl<'a> From<&'a Envelope> for Line<'a> {
    fn from(envelope: &'a Envelope) -> Line<'a> {
        Line {
            ts: envelope.ts().as_str(),
            level: envelope.level().as_wire(),
            component: envelope.component().as_wire(),
            run_id: envelope.run_id().as_str(),
            msg: envelope.msg(),
            job_id: envelope.job_id().map(|id| id.as_str()),
            drone_id: envelope.drone_id().map(|id| id.as_str()),
            step_id: envelope.step_id(),
            workspace: envelope.workspace(),
            fields: envelope
                .fields()
                .iter()
                .map(|(key, value)| (key.as_str(), Field::from(value)))
                .collect(),
        }
    }
}

/// A `FieldValue` on the wire. Untagged, because `fields` is read by a person
/// with `jq` and a tag would put the type of every value in their way.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Field {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Field>),
}

impl From<&FieldValue> for Field {
    fn from(value: &FieldValue) -> Field {
        match value {
            FieldValue::Str(text) => Field::Str(text.clone()),
            FieldValue::Int(number) => Field::Int(*number),
            FieldValue::Float(number) => Field::Float(*number),
            FieldValue::Bool(flag) => Field::Bool(*flag),
            FieldValue::List(values) => Field::List(values.iter().map(Field::from).collect()),
        }
    }
}
