//! `layout.json`: what a layout mod, or the owner's own choice, may say about how Bridge arranges
//! its tabs, panels and rail. `docs/concepts/layout-mods.md`.
//!
//! **Here for `codec`'s reason**: reading JSON nobody typed is bytes entering the process, and
//! `serde_json::from_*` may appear only in `ipc` and `store`. The rules are the ones `parseLayout`
//! in `packages/shell/src/layout.tsx` applies to the same text, and Bridge parses again what
//! Fleet returned, so the two must agree on what is invalid. Whether an id exists is Bridge's
//! registry and never a reason to refuse a file: an id or a region this build does not have is
//! ignored by both.

use serde_json::Value;

/// The most a `layout.json`, or the owner's saved choice, may weigh.
pub const MOST_BYTES: usize = 4096;

const MOST_IDS: usize = 32;
const MOST_ID: usize = 40;

/// The regions Bridge arranges, from the file both sides check. `packages/shell/layout-registry.json`
/// is the shipped list; Bridge's registry has a test that it agrees with it.
const REGISTRY: &str = include_str!("../../../packages/shell/layout-registry.json");

/// Every region a layout may name, in the registry's order.
pub fn regions() -> Vec<String> {
    let registry: Value = serde_json::from_str(REGISTRY).expect("the shipped registry is JSON");
    registry["regions"]
        .as_object()
        .map(|all| all.keys().cloned().collect())
        .unwrap_or_default()
}

/// Every problem in `text`, or empty where Bridge may apply it.
pub fn problems(text: &str) -> Vec<String> {
    if text.len() > MOST_BYTES {
        return vec![format!("larger than {MOST_BYTES} bytes")];
    }
    let Ok(raw) = serde_json::from_str::<Value>(text) else {
        return vec!["not JSON".to_string()];
    };
    let Value::Object(top) = raw else {
        return vec!["not an object".to_string()];
    };
    let mut found = Vec::new();
    if top.get("version").and_then(Value::as_f64) != Some(1.0) {
        found.push("version is not 1".to_string());
    }
    let known = regions();
    for (key, value) in &top {
        if key == "version" || !known.contains(key) {
            continue;
        }
        let Value::Object(region) = value else {
            found.push(format!("{key} is not an object"));
            continue;
        };
        for (field, body) in region {
            match field.as_str() {
                "order" | "hidden" => {
                    if !is_ids(body) {
                        found.push(format!("{key}.{field} is not a list of up to {MOST_IDS} ids"));
                    }
                }
                "first" => {
                    if !body.as_str().is_some_and(is_id) {
                        found.push(format!("{key}.first is not an id"));
                    }
                }
                _ => found.push(format!("{key}.{field} is not a field")),
            }
        }
    }
    found
}

fn is_ids(body: &Value) -> bool {
    body.as_array()
        .is_some_and(|ids| ids.len() <= MOST_IDS && ids.iter().all(|one| one.as_str().is_some_and(is_id)))
}

/// A slug: a lowercase letter, then lowercase letters, digits and `-`, at most 40 in all.
fn is_id(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && text.len() <= MOST_ID
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
