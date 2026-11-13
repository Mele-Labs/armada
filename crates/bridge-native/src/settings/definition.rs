//! A workflow definition's text, read for its steps and changed in the one place Settings edits, a step's
//! `instructions` (`packages/components/src/compositions/WorkflowManager/definition.ts`). The text is read with its
//! keys in the order the file wrote them, so what goes back to `save_workflow` is the file as it was but for those
//! words.

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

/// A JSON value whose objects keep their keys in the file's order.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    Text(String),
    List(Vec<Json>),
    Map(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Map(fields) => fields.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Json::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Set a key, where it was or at the end.
    fn set(&mut self, key: &str, value: Json) {
        let Json::Map(fields) = self else { return };
        match fields.iter_mut().find(|(name, _)| name == key) {
            Some((_, was)) => *was = value,
            None => fields.push((key.to_string(), value)),
        }
    }

    fn unset(&mut self, key: &str) {
        if let Json::Map(fields) = self {
            fields.retain(|(name, _)| name != key);
        }
    }

    fn id(&self) -> Option<&str> {
        self.get("id").and_then(Json::text)
    }
}

impl Serialize for Json {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Json::Null => serializer.serialize_unit(),
            Json::Bool(on) => serializer.serialize_bool(*on),
            Json::Number(number) => number.serialize(serializer),
            Json::Text(text) => serializer.serialize_str(text),
            Json::List(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Json::Map(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (key, value) in fields {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

struct Reading;

impl<'de> Visitor<'de> for Reading {
    type Value = Json;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("JSON")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Json, D::Error> {
        Json::deserialize(deserializer)
    }

    fn visit_bool<E: de::Error>(self, on: bool) -> Result<Json, E> {
        Ok(Json::Bool(on))
    }

    fn visit_i64<E: de::Error>(self, n: i64) -> Result<Json, E> {
        Ok(Json::Number(n.into()))
    }

    fn visit_u64<E: de::Error>(self, n: u64) -> Result<Json, E> {
        Ok(Json::Number(n.into()))
    }

    fn visit_f64<E: de::Error>(self, n: f64) -> Result<Json, E> {
        Ok(serde_json::Number::from_f64(n).map_or(Json::Null, Json::Number))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Json, E> {
        Ok(Json::Text(text.to_string()))
    }

    fn visit_string<E: de::Error>(self, text: String) -> Result<Json, E> {
        Ok(Json::Text(text))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Json::List(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut fields = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Json>()? {
            fields.push((key, value));
        }
        Ok(Json::Map(fields))
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(Reading)
    }
}

/// The definition read, where it is a workflow: an object with `steps`.
pub fn tree_of(text: &str) -> Option<Json> {
    let tree: Json = ipc::decode("workflow definition", text.as_bytes()).ok()?;
    matches!(tree.get("steps"), Some(Json::List(_))).then_some(tree)
}

/// Written back as Classic writes it, two spaces a level.
pub fn text_of(tree: &Json) -> String {
    serde_json::to_string_pretty(tree).unwrap_or_default()
}

/// The definition with one step's words replaced and every other key as it was, or none where the step is not
/// there (`withWords`).
pub fn with_words(text: &str, step: &str, words: &str) -> Option<String> {
    let mut tree = tree_of(text)?;
    let Json::Map(fields) = &mut tree else { return None };
    let (_, Json::List(steps)) = fields.iter_mut().find(|(name, _)| name == "steps")? else { return None };
    let one = steps.iter_mut().find(|one| one.get("id").and_then(Json::text) == Some(step))?;
    one.set("instructions", Json::Text(words.to_string()));
    Some(text_of(&tree))
}

/// The definition with one step's label replaced and every other key as it was, or none where the step is not there.
pub fn with_label(text: &str, step: &str, label: &str) -> Option<String> {
    let mut tree = tree_of(text)?;
    let Json::Map(fields) = &mut tree else { return None };
    let (_, Json::List(steps)) = fields.iter_mut().find(|(name, _)| name == "steps")? else { return None };
    let one = steps.iter_mut().find(|one| one.get("id").and_then(Json::text) == Some(step))?;
    one.set("label", Json::Text(label.to_string()));
    Some(text_of(&tree))
}

/// `text` with what changed between `before` and `after` carried into it, and the rest of `text` as it was: the
/// top-level keys and each step's fields (by id) that changed, steps removed, and steps added or moved laid after
/// the step they follow in `after`. A step whose predecessor in `text` is not its predecessor in `before` was moved
/// since, and keeps that place. None where any of the three is not a workflow.
pub fn with_delta(text: &str, before: &str, after: &str) -> Option<String> {
    let (mut tree, was, now) = (tree_of(text)?, tree_of(before)?, tree_of(after)?);
    carry(&mut tree, &was, &now, "steps");
    let (Some(Json::List(was_steps)), Some(Json::List(now_steps))) = (was.get("steps"), now.get("steps")) else { return None };
    let Json::Map(fields) = &mut tree else { return None };
    let (_, Json::List(steps)) = fields.iter_mut().find(|(name, _)| name == "steps")? else { return None };
    let moved_since: Vec<String> = steps
        .iter()
        .filter_map(Json::id)
        .filter(|id| place_of(was_steps, id).is_some() && follows(steps, id) != follows(was_steps, id))
        .map(String::from)
        .collect();
    steps.retain(|one| one.id().is_none_or(|id| place_of(was_steps, id).is_none() || place_of(now_steps, id).is_some()));
    for (at, one) in now_steps.iter().enumerate() {
        let Some(id) = one.id() else { continue };
        match (place_of(was_steps, id), place_of(steps, id)) {
            (Some(was_at), Some(here)) => {
                carry(&mut steps[here], &was_steps[was_at], one, "id");
                if follows(now_steps, id) != follows(was_steps, id) && !moved_since.iter().any(|his| his == id) {
                    let step = steps.remove(here);
                    let place = place_after(steps, &now_steps[..at]);
                    steps.insert(place, step);
                }
            }
            (Some(_), None) => {}
            (None, Some(here)) => steps[here] = one.clone(),
            (None, None) => {
                let place = place_after(steps, &now_steps[..at]);
                steps.insert(place, one.clone());
            }
        }
    }
    Some(text_of(&tree))
}

fn place_of(steps: &[Json], id: &str) -> Option<usize> {
    steps.iter().position(|one| one.id() == Some(id))
}

/// The id of the step before `id`: none where `id` is not listed, `Some(None)` where it is first.
fn follows<'a>(steps: &'a [Json], id: &str) -> Option<Option<&'a str>> {
    place_of(steps, id).map(|at| at.checked_sub(1).and_then(|prev| steps[prev].id()))
}

/// Just after the last of `preceding` that is in `steps`, or first where none is.
fn place_after(steps: &[Json], preceding: &[Json]) -> usize {
    preceding.iter().rev().filter_map(Json::id).find_map(|prev| place_of(steps, prev)).map_or(0, |prev| prev + 1)
}

/// The keys of an object that differ between `was` and `now` set on `into`, and those `now` dropped taken out of it.
fn carry(into: &mut Json, was: &Json, now: &Json, except: &str) {
    let (Json::Map(was), Json::Map(now)) = (was, now) else { return };
    for (key, value) in now.iter().filter(|(key, _)| key != except) {
        if !was.iter().any(|(name, old)| name == key && old == value) {
            into.set(key, value.clone());
        }
    }
    for (key, _) in was.iter().filter(|(key, _)| key != except) {
        if !now.iter().any(|(name, _)| name == key) {
            into.unset(key);
        }
    }
}

/// The steps a definition lists, in order, as their ids and labels (a step with no label is named by its id).
pub fn steps_of(text: &str) -> Vec<(String, String)> {
    let Some(Json::List(steps)) = tree_of(text).and_then(|tree| tree.get("steps").cloned()) else { return Vec::new() };
    steps
        .iter()
        .filter_map(|one| {
            let id = one.get("id").and_then(Json::text)?.to_string();
            let label = one.get("label").and_then(Json::text).unwrap_or(&id).to_string();
            Some((id, label))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r#"{"version":1,"workflow_id":"bug","name":"bug","steps":[{"id":"fix","label":"Fix","instructions":"Fix it.","rules":{"z":1,"a":2.5}},{"id":"tests","label":"Tests"}]}"#;

    #[test]
    fn a_steps_words_change_and_every_key_keeps_its_place() {
        let changed = with_words(TEXT, "fix", "Make the smallest change.").expect("fix is a step");
        let tree = tree_of(&changed).expect("still a workflow");
        let Some(Json::Map(top)) = Some(&tree) else { panic!("an object") };
        assert_eq!(top.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["version", "workflow_id", "name", "steps"]);
        let Some(Json::List(steps)) = tree.get("steps") else { panic!("steps") };
        assert_eq!(steps[0].get("instructions").and_then(Json::text), Some("Make the smallest change."));
        let Some(Json::Map(rules)) = steps[0].get("rules") else { panic!("rules") };
        assert_eq!(rules.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["z", "a"]);
    }

    #[test]
    fn a_step_with_no_words_gets_them_and_a_missing_step_is_none() {
        let changed = with_words(TEXT, "tests", "Write the test.").expect("tests is a step");
        let tree = tree_of(&changed).expect("a workflow");
        let Some(Json::List(steps)) = tree.get("steps") else { panic!("steps") };
        assert_eq!(steps[1].get("instructions").and_then(Json::text), Some("Write the test."));
        assert_eq!(with_words(TEXT, "deliver", "x"), None);
        assert_eq!(with_words("steps: []", "fix", "x"), None);
    }

    #[test]
    fn a_steps_label_changes_and_every_other_key_keeps_its_place() {
        let changed = with_label(TEXT, "fix", "Repair").expect("fix is a step");
        let tree = tree_of(&changed).expect("still a workflow");
        let Some(Json::List(steps)) = tree.get("steps") else { panic!("steps") };
        let Json::Map(fix) = &steps[0] else { panic!("a step") };
        assert_eq!(fix.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["id", "label", "instructions", "rules"]);
        assert_eq!(steps[0].get("label").and_then(Json::text), Some("Repair"));
        assert_eq!(steps[1].get("label").and_then(Json::text), Some("Tests"));
        assert_eq!(with_label(TEXT, "deliver", "x"), None);
        assert_eq!(steps_of(&changed), [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Tests".to_string())]);
    }
}
