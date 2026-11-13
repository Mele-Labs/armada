//! How the owner has Bridge draw a time: three switches, `hour12`, `seconds` and `date`, written by
//! Settings, Time and read back by Bridge.
//!
//! **Here for `key_bindings`' reason**: JSON nobody typed is bytes entering the process, and
//! `serde_json::from_*` may appear only in `ipc` and `store`. All three switches are required and
//! nothing else is allowed, so a half-written value is refused and never read as a default.

use serde::Serialize;
use serde_json::Value;

/// The switches, in the order a problem names them.
const SWITCHES: [&str; 3] = ["hour12", "seconds", "date"];

/// Every problem in `text`, or empty where Bridge may apply it.
pub fn problems(text: &str) -> Vec<String> {
    let Ok(raw) = serde_json::from_str::<Value>(text) else {
        return vec!["not JSON".to_string()];
    };
    let Value::Object(top) = raw else {
        return vec!["not an object".to_string()];
    };
    let mut found = Vec::new();
    for name in SWITCHES {
        match top.get(name) {
            None => found.push(format!("{name} is missing")),
            Some(Value::Bool(_)) => {}
            Some(_) => found.push(format!("{name} is not true or false")),
        }
    }
    for name in top.keys().filter(|name| !SWITCHES.contains(&name.as_str())) {
        found.push(format!("{name} is not a switch"));
    }
    found
}

/// The three switches as Bridge draws a time by them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TimeFormat {
    pub hour12: bool,
    pub seconds: bool,
    pub date: bool,
}

/// What ships: a 24-hour clock to the second, with no date.
impl Default for TimeFormat {
    fn default() -> Self {
        TimeFormat { hour12: false, seconds: true, date: false }
    }
}

impl TimeFormat {
    /// The text Settings, Time saves.
    pub fn text(&self) -> String {
        serde_json::to_string(self).expect("three switches are JSON")
    }
}

/// What `text` sets, or what ships where it is empty or has a problem: never half of one.
pub fn read(text: &str) -> TimeFormat {
    if text.trim().is_empty() || !problems(text).is_empty() {
        return TimeFormat::default();
    }
    let raw: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    let switch = |name: &str| raw.get(name).and_then(Value::as_bool).unwrap_or_default();
    TimeFormat { hour12: switch("hour12"), seconds: switch("seconds"), date: switch("date") }
}
