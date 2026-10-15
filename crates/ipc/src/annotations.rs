//! `mark_annotation_done`: a session marks a note Bridge sent it as done. The note is one JSON
//! file Bridge's annotation layer writes (`Annotation` in `apps/desktop/src/shared/annotations.ts`);
//! Fleet does the file I/O, and the bytes are read and rewritten here because this is where
//! bytes may be parsed.

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SessionId;

/// `mark_annotation_done`: mark the note `note_id` done, where it was sent to this session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkAnnotationDone {
    /// The note's id, which is also its file name less `.json`.
    pub note_id: String,
    /// Which session. **Read only where the connection places no hosted session**, as `FinishUp`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `mark_annotation_done` wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationMarked {
    /// The note's file, absolute.
    pub path: String,
}

/// Whether `id` can only name a file in the annotations directory: letters, digits and `-`.
pub fn is_note_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// Why a note was not marked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotMarked {
    /// The file is not a JSON object.
    Unreadable(String),
    /// Its `sent.sessionId` is absent or another session's.
    NotSentHere,
}

/// The note in `bytes` with `status` `"done"` and `updatedAt` `at`, every other field as it was.
/// **Keys keep their order**, because Bridge writes them so the note comes first for whoever reads it.
pub fn marked_done(bytes: &[u8], session_id: &str, at: &str) -> Result<Vec<u8>, NotMarked> {
    let note: Ordered =
        serde_json::from_slice(bytes).map_err(|why| NotMarked::Unreadable(why.to_string()))?;
    let Ordered::Object(mut fields) = note else {
        return Err(NotMarked::Unreadable(String::from("not a JSON object")));
    };
    let sent_here = fields.iter().any(|(key, value)| {
        key == "sent"
            && matches!(value, Ordered::Object(sent) if sent.iter().any(|(key, value)| {
                key == "sessionId" && matches!(value, Ordered::Leaf(Value::String(id)) if id == session_id)
            }))
    });
    if !sent_here {
        return Err(NotMarked::NotSentHere);
    }
    set(&mut fields, "status", "done");
    set(&mut fields, "updatedAt", at);
    let mut out = serde_json::to_vec_pretty(&Ordered::Object(fields))
        .map_err(|why| NotMarked::Unreadable(why.to_string()))?;
    out.push(b'\n');
    Ok(out)
}

fn set(fields: &mut Vec<(String, Ordered)>, key: &str, to: &str) {
    let value = Ordered::Leaf(Value::String(to.to_string()));
    match fields.iter_mut().find(|(named, _)| named == key) {
        Some((_, slot)) => *slot = value,
        None => fields.push((key.to_string(), value)),
    }
}

/// JSON whose objects keep their keys in the order they were read. `serde_json::Value` sorts them,
/// and turning on `preserve_order` would change it for the whole workspace.
enum Ordered {
    Object(Vec<(String, Ordered)>),
    Array(Vec<Ordered>),
    Leaf(Value),
}

impl Serialize for Ordered {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Ordered::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (key, value) in fields {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Ordered::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Ordered::Leaf(value) => value.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OrderedVisitor)
    }
}

struct OrderedVisitor;

impl<'de> Visitor<'de> for OrderedVisitor {
    type Value = Ordered;

    fn expecting(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str("JSON")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::from(value)))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::from(value)))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::from(value)))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::String(value.to_string())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::Null))
    }

    fn visit_none<E: de::Error>(self) -> Result<Ordered, E> {
        Ok(Ordered::Leaf(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Ordered, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Ordered::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered, A::Error> {
        let mut fields = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Ordered>()? {
            fields.push((key, value));
        }
        Ok(Ordered::Object(fields))
    }
}
