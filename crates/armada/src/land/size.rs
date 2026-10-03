//! How many waiting branches a turn takes: halved after a red turn, doubled
//! after a green one, between 1 and the ceiling `ARMADA_LAND_BATCH` sets.
//! Kept in the state directory, so a new runner goes on from it, with the
//! ceiling the runner took it under, so `--status` says the runner's and not
//! its own environment's.
//! `docs/capabilities/merge-line.md`, *Batching*.

use std::path::PathBuf;

use fleet::clock::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

use super::codec;
use super::dir::StateDir;

/// How a turn went, as far as the size is concerned. A turn that only
/// conflicted with the base, stopped or was withdrawn is neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Went {
    /// A group went red: alone, by blame, or by a split to find whose.
    Red,
    /// A group landed whole, and none went red.
    Green,
}

/// The size the last turn that counted left, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Size {
    size: usize,
    was: usize,
    after: Went,
    /// RFC3339, UTC.
    at: String,
}

/// `batch.json`: the ceiling a runner last turned under, and the size the last
/// turn that counted left. Either may be missing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Held {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ceiling: Option<usize>,
    #[serde(flatten)]
    last: Option<Size>,
}

fn path(state: &StateDir) -> PathBuf {
    state.path().join("batch.json")
}

fn read(state: &StateDir) -> Held {
    codec::read("batch size", &path(state))
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// What the next turn takes. An unreadable file starts again at the ceiling.
pub fn current(state: &StateDir, ceiling: usize) -> usize {
    read(state)
        .last
        .map_or(ceiling, |held| held.size.clamp(1, ceiling))
}

/// Record the ceiling this runner takes its turns under. Best effort, as
/// [`after`] is.
pub fn uses(state: &StateDir, ceiling: usize) {
    let held = read(state);
    if held.ceiling != Some(ceiling) {
        let _ = codec::write(
            &path(state),
            &Held {
                ceiling: Some(ceiling),
                ..held
            },
        );
    }
}

/// The size after a turn that went `went`.
fn next(size: usize, went: Went, ceiling: usize) -> usize {
    match went {
        Went::Red => (size / 2).max(1),
        Went::Green => size.saturating_mul(2).min(ceiling),
    }
}

/// Record the size the next turn takes. Best effort: a size that cannot be
/// written leaves the last one, and the line goes on.
pub fn after(state: &StateDir, ceiling: usize, went: Went) {
    let was = current(state, ceiling);
    let size = Size {
        size: next(was, went, ceiling),
        was,
        after: went,
        at: SystemClock::new().now().as_str().to_string(),
    };
    let held = Held {
        ceiling: Some(ceiling),
        last: Some(size),
    };
    let _ = codec::write(&path(state), &held);
}

/// `taking up to 4 — halved after a red at 14:02 UTC`, for `--status`, under
/// the ceiling the runner recorded, or `unrecorded` where none has.
pub fn describe(state: &StateDir, unrecorded: usize) -> String {
    let Held { ceiling, last } = read(state);
    let ceiling = ceiling.unwrap_or(unrecorded);
    let Some(held) = last else {
        return format!("taking up to {ceiling} — the ceiling");
    };
    let size = held.size.clamp(1, ceiling);
    let why = if size < held.size {
        "the ceiling ARMADA_LAND_BATCH sets,"
    } else {
        match (held.after, size.cmp(&held.was)) {
            (Went::Red, std::cmp::Ordering::Less) => "halved",
            (Went::Green, std::cmp::Ordering::Greater) => "doubled",
            (Went::Red, _) => "the floor,",
            (Went::Green, _) => "the ceiling,",
        }
    };
    let went = match held.after {
        Went::Red => "red",
        Went::Green => "green",
    };
    // `2026-10-02T14:02:31.123Z` -> `14:02`.
    let clock = held.at.get(11..16).unwrap_or(&held.at);
    format!("taking up to {size} — {why} after a {went} at {clock} UTC")
}

#[cfg(test)]
mod tests {
    use super::{after, current, describe, next, uses, Went};
    use crate::land::dir::StateDir;
    use crate::tests::TempDir;

    #[test]
    fn the_size_halves_and_doubles_between_one_and_the_ceiling() {
        assert_eq!(next(8, Went::Red, 8), 4);
        assert_eq!(next(1, Went::Red, 8), 1);
        assert_eq!(next(3, Went::Red, 8), 1);
        assert_eq!(next(4, Went::Green, 8), 8);
        assert_eq!(next(8, Went::Green, 8), 8);
        assert_eq!(next(3, Went::Green, 4), 4);
    }

    #[test]
    fn the_size_is_read_back_and_held_under_a_lowered_ceiling() {
        let dir = TempDir::new();
        let state = StateDir::for_testing(dir.path().to_path_buf());
        assert_eq!(current(&state, 8), 8, "the ceiling before any turn");
        after(&state, 8, Went::Red);
        assert_eq!(current(&state, 8), 4);
        assert!(describe(&state, 8).starts_with("taking up to 4 — halved after a red at "));
        assert_eq!(current(&state, 2), 2, "a lowered ceiling holds it");
        uses(&state, 2);
        assert!(describe(&state, 8).starts_with("taking up to 2 — the ceiling ARMADA_LAND_BATCH"));
    }

    #[test]
    fn status_reads_the_ceiling_the_runner_recorded_not_its_own() {
        let dir = TempDir::new();
        let state = StateDir::for_testing(dir.path().to_path_buf());
        assert_eq!(
            describe(&state, 8),
            "taking up to 8 — the ceiling",
            "none recorded yet"
        );
        uses(&state, 1);
        assert_eq!(describe(&state, 8), "taking up to 1 — the ceiling");
        after(&state, 4, Went::Red);
        assert!(describe(&state, 8).starts_with("taking up to 2 — halved after a red at "));
    }
}
