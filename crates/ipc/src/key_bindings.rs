//! The owner's key bindings: which keys an act answers in place of the ones `actions.toml` gives it.
//! Settings → Keyboard writes it and Bridge reads it back. `docs/concepts/key-bindings.md`.
//!
//! **Here for `layout`'s reason**: JSON nobody typed is bytes entering the process, and
//! `serde_json::from_*` may appear only in `ipc` and `store`. The rules are the ones
//! `parseKeyBindings` in `packages/components/src/keymap.ts` applies. Whether an act or a key exists
//! is Bridge's registry and never a reason to refuse: an id this build does not have is ignored.

use serde_json::Value;

/// The most the saved bindings may weigh. The store's `CHECK` is the same figure.
pub const MOST_BYTES: usize = 16384;

/// The most keys one act may answer to.
const MOST_KEYS: usize = 8;
/// The most characters one key may be spelled in, `⌃⌥⇧⌘Backspace` with room.
const MOST_KEY: usize = 24;

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
    match top.get("bindings") {
        None => {}
        Some(Value::Object(bindings)) => {
            for (id, keys) in bindings {
                let fits = keys.as_array().is_some_and(|all| {
                    all.len() <= MOST_KEYS
                        && all.iter().all(|one| one.as_str().is_some_and(|key| key.chars().count() <= MOST_KEY))
                });
                if !fits {
                    found.push(format!("bindings.{id} is not a list of up to {MOST_KEYS} keys"));
                }
            }
        }
        Some(_) => found.push("bindings is not an object".to_string()),
    }
    found
}
