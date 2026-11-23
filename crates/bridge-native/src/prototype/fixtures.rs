//! The data a scene binds: each fixture's text decoded through `ipc`, the way a live answer is.
//!
//! A fixture's `type` names the wire type the `--mock` scenarios use for the same data. The text
//! must decode as that type, so a fixture that has drifted from the wire fails here and not
//! on his screen. It is then read a second time as plain JSON, for the path lookup a binding
//! makes: `{jobs.sessions.0.title}`. Both reads go through `ipc::decode`.

use std::collections::BTreeMap;

use serde_json::Value;

use super::markup::FixtureDecl;

/// The wire types a fixture may name, by the word a scene writes.
pub const TYPES: [&str; 6] = ["jobs", "thread", "needs-you", "issues", "preview-offers", "json"];

/// A decoded fixture, or why it did not.
#[derive(Debug, Default)]
pub struct Fixtures {
    held: BTreeMap<String, Result<Value, String>>,
}

impl Fixtures {
    /// Each fixture the scene declares, decoded from the text that came with it. `given` is
    /// `(name, text)` as Fleet sends it; a fixture is found by its `src`, else its `name`.
    pub fn read(decls: &[FixtureDecl], given: &[(String, String)]) -> (Fixtures, Vec<String>) {
        let mut held = BTreeMap::new();
        let mut trouble = Vec::new();
        for decl in decls {
            let found = decl.src.iter().chain(std::iter::once(&decl.name)).find_map(|key| given.iter().find(|(name, _)| name == key));
            let outcome = match found {
                None => Err(format!("no file came with the scene for {}", decl.src.as_deref().unwrap_or(&decl.name))),
                Some((_, text)) => decode(decl.kind.as_deref(), text),
            };
            if let Err(why) = &outcome {
                trouble.push(format!("Fixture {}: {why}", decl.name));
            }
            held.insert(decl.name.clone(), outcome);
        }
        (Fixtures { held }, trouble)
    }

    pub fn value(&self, name: &str) -> Option<&Value> {
        self.held.get(name).and_then(|held| held.as_ref().ok())
    }

    pub fn has(&self, name: &str) -> bool {
        self.held.contains_key(name)
    }
}

fn decode(kind: Option<&str>, text: &str) -> Result<Value, String> {
    let bytes = text.as_bytes();
    let said = |why: ipc::Undecodable| why.to_string();
    match kind.unwrap_or("json") {
        "jobs" => ipc::decode::<ipc::SessionList>("SessionList", bytes).map(|_| ()).map_err(said)?,
        "thread" => ipc::decode::<ipc::JobThread>("JobThread", bytes).map(|_| ()).map_err(said)?,
        "needs-you" => ipc::decode::<ipc::NeedsYou>("NeedsYou", bytes).map(|_| ()).map_err(said)?,
        "issues" => ipc::decode::<ipc::IssueList>("IssueList", bytes).map(|_| ()).map_err(said)?,
        "preview-offers" => ipc::decode::<ipc::PreviewOffers>("PreviewOffers", bytes).map(|_| ()).map_err(said)?,
        "json" => {}
        other => return Err(format!("type {other} is none of {}", TYPES.join(", "))),
    }
    ipc::decode::<Value>("fixture", bytes).map_err(said)
}

/// Why a path named no words: nothing is there, or what is there is more than one value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gap {
    Nothing,
    Many(String),
}

/// The words at `path` in `value`: object keys and array places, joined by dots.
pub fn at(value: &Value, path: &str) -> Result<String, Gap> {
    let mut here = value;
    for step in path.split('.').filter(|step| !step.is_empty()) {
        here = match here {
            Value::Object(map) => map.get(step).ok_or(Gap::Nothing)?,
            Value::Array(items) => step.parse::<usize>().ok().and_then(|place| items.get(place)).ok_or(Gap::Nothing)?,
            _ => return Err(Gap::Nothing),
        };
    }
    match here {
        Value::String(words) => Ok(words.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(yes) => Ok(yes.to_string()),
        Value::Null => Ok(String::new()),
        Value::Array(_) | Value::Object(_) => Err(Gap::Many(format!("{path} holds more than one value"))),
    }
}

/// The value at `path`, whole: an array a list walks.
pub fn value_at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut here = value;
    for step in path.split('.').filter(|step| !step.is_empty()) {
        here = match here {
            Value::Object(map) => map.get(step)?,
            Value::Array(items) => items.get(step.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(here)
}

/// What a scene's bindings read from: the fixtures, and the names a list gave each of its items.
#[derive(Clone, Copy)]
pub struct Scope<'a> {
    pub fixtures: &'a Fixtures,
    pub locals: &'a [(String, &'a Value)],
}

impl<'a> Scope<'a> {
    /// `{name.path}`: the root a path starts at, a list's item or a fixture, and what is left of it.
    fn root_of<'w>(&self, whole: &'w str) -> Option<(&'a Value, &'w str)> {
        let (name, rest) = whole.split_once('.').unwrap_or((whole, ""));
        if let Some((_, item)) = self.locals.iter().rev().find(|(local, _)| local == name) {
            return Some((item, rest));
        }
        self.fixtures.value(name).map(|value| (value, rest))
    }

    pub fn value(&self, whole: &str) -> Option<&'a Value> {
        let (root, rest) = self.root_of(whole)?;
        value_at(root, rest)
    }

    /// The words `text` holds with each `{name.path}` replaced by what it names. A binding that
    /// names nothing is replaced by nothing, and each one is listed in `reads` for the caller to judge.
    pub fn bind(&self, text: &str, reads: &mut Vec<Read>) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let tail = &rest[open + 1..];
            match tail.find('}') {
                Some(close) if is_path(&tail[..close]) => {
                    let whole = &tail[..close];
                    let name = whole.split('.').next().unwrap_or_default();
                    let local = self.locals.iter().any(|(local, _)| local == name).then(|| name.to_string());
                    let words = match self.root_of(whole) {
                        None => Err(Gap::Many(self.why_no_root(whole))),
                        Some((root, path)) => at(root, path),
                    };
                    reads.push(Read { whole: whole.to_string(), local, fixture: name.to_string(), gap: words.as_ref().err().cloned() });
                    out.push_str(&words.unwrap_or_default());
                    rest = &tail[close + 1..];
                }
                _ => {
                    out.push('{');
                    rest = tail;
                }
            }
        }
        out.push_str(rest);
        out
    }

    fn why_no_root(&self, whole: &str) -> String {
        let name = whole.split('.').next().unwrap_or_default();
        if self.fixtures.has(name) {
            format!("reads fixture {name}, which did not decode")
        } else {
            format!("names no fixture or item called {name}")
        }
    }
}

/// One binding read: its path, the list item it starts at (if it starts at one), the name it
/// starts with, and why it named nothing.
#[derive(Debug)]
pub struct Read {
    pub whole: String,
    pub local: Option<String>,
    pub fixture: String,
    pub gap: Option<Gap>,
}

/// A binding reads as `word.word.0`: letters, digits, dashes and underscores between dots. Braces
/// round anything else are the scene's own words.
fn is_path(text: &str) -> bool {
    !text.is_empty() && text.split('.').all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
}
