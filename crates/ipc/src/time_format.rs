//! How the owner has Bridge draw a time: three switches, `hour12`, `seconds` and `date`, written by
//! Settings, Time and read back by Bridge.
//!
//! **Here for `key_bindings`' reason**: JSON nobody typed is bytes entering the process, and
//! `serde_json::from_*` may appear only in `ipc` and `store`. All three switches are required and
//! nothing else is allowed, so a half-written value is refused and never read as a default.

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
