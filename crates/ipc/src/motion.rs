//! How fast the owner has Bridge move: today the seconds one turn of the busy ring takes, written by
//! Settings, Motion as `{"ringSeconds":6}` and read back by Bridge.
//!
//! **Here for `time_format`'s reason**: JSON nobody typed is bytes entering the process, and
//! `serde_json::from_*` may appear only in `ipc` and `store`. A value outside the closed range, or one
//! that is not a whole number of seconds, is refused on a save and reads as the default on a read.

use serde::Serialize;
use serde_json::Value;

/// The slowest and the fastest a turn may be, in seconds.
pub const RING_SECONDS: std::ops::RangeInclusive<u32> = 2..=20;

/// What ships: one turn in six seconds.
pub const DEFAULT_RING_SECONDS: u32 = 6;

const NAME: &str = "ringSeconds";

/// Every problem in `text`, or empty where Bridge may apply it.
pub fn problems(text: &str) -> Vec<String> {
    let Ok(raw) = serde_json::from_str::<Value>(text) else {
        return vec!["not JSON".to_string()];
    };
    let Value::Object(top) = raw else {
        return vec!["not an object".to_string()];
    };
    let mut found = Vec::new();
    match top.get(NAME) {
        None => found.push(format!("{NAME} is missing")),
        Some(value) => match value.as_u64().and_then(|n| u32::try_from(n).ok()) {
            Some(n) if RING_SECONDS.contains(&n) => {}
            Some(_) => found.push(format!("{NAME} is outside {} to {} seconds", RING_SECONDS.start(), RING_SECONDS.end())),
            None => found.push(format!("{NAME} is not a whole number of seconds")),
        },
    }
    for name in top.keys().filter(|name| name.as_str() != NAME) {
        found.push(format!("{name} is not a motion setting"));
    }
    found
}

/// The motion settings as Bridge draws by them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Motion {
    #[serde(rename = "ringSeconds")]
    pub ring_seconds: u32,
}

impl Default for Motion {
    fn default() -> Self {
        Motion { ring_seconds: DEFAULT_RING_SECONDS }
    }
}

impl Motion {
    /// The text Settings, Motion saves.
    pub fn text(&self) -> String {
        serde_json::to_string(self).expect("one number is JSON")
    }
}

/// What `text` sets, or what ships where it is empty or has a problem: never half of one.
pub fn read(text: &str) -> Motion {
    if text.trim().is_empty() || !problems(text).is_empty() {
        return Motion::default();
    }
    let raw: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    let seconds = raw.get(NAME).and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()).unwrap_or(DEFAULT_RING_SECONDS);
    Motion { ring_seconds: seconds }
}
