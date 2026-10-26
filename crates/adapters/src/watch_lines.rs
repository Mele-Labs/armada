//! The watch recipes Armada ships, and the readers `armada watch-line` runs for them.
//! `docs/concepts/watch.md`, *Recipes*.
//!
//! **Here and not in `config`**, because each names a provider. `config` parses them as it parses
//! any recipe, from [`carried_watch_recipes`], at the Armada level.
//!
//! **A reader prints one line or fails.** What it read becomes `core_model::Reading`, written by
//! `ipc::watch_line`, so Fleet reads it as any command's line; a reader that could not read says
//! why on standard error and exits non-zero, which Fleet keeps as a broken reading.

pub(crate) mod buildkite;
pub(crate) mod github;

use std::path::{Path, PathBuf};

use core_model::{Part, Reading, ReadingState};

/// Where a shipped recipe says it came from, as a path.
const CARRIED_AT: &str = "armada/watch-recipes";

const CARRIED: &[(&str, &str)] = &[
    ("buildkite-build.yml", include_str!("../watch-recipes/buildkite-build.yml")),
    ("github-checks.yml", include_str!("../watch-recipes/github-checks.yml")),
];

/// Every recipe Armada ships, as files.
pub fn carried_watch_recipes() -> Vec<(PathBuf, String)> {
    CARRIED.iter().map(|(file, text)| (Path::new(CARRIED_AT).join(file), (*text).to_string())).collect()
}

/// The readers, each with the values it takes in order.
pub const WATCH_READERS: &[(&str, &[&str])] = &[
    (github::READER, github::TAKES),
    (buildkite::READER, buildkite::TAKES),
];

/// Read once with `reader`, given `values` in the order it takes them, in the repository at `cwd`.
/// **Blocking**: it calls the provider over HTTPS, so it runs off an async worker.
pub fn read_watch(reader: &str, values: &[String], cwd: &Path) -> Result<Reading, String> {
    let Some((_, takes)) = WATCH_READERS.iter().find(|(name, _)| *name == reader) else {
        let named: Vec<&str> = WATCH_READERS.iter().map(|(name, _)| *name).collect();
        return Err(format!("No watch reader is named {reader}. There are: {}.", named.join(", ")));
    };
    if values.len() != takes.len() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(format!("{reader} takes {}, each one argument and none blank.", takes.join(", ")));
    }
    match reader {
        github::READER => github::read(&values[0], &values[1], cwd),
        _ => buildkite::read(&values[0], &values[1], &values[2]),
    }
}

/// One part as a reader found it, with where to look at it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Found {
    pub(crate) name: String,
    pub(crate) state: ReadingState,
    pub(crate) link: Option<String>,
}

/// Up to three names, then how many more.
fn named(found: &[&Found]) -> String {
    let mut names: Vec<String> = found.iter().take(3).map(|one| one.name.clone()).collect();
    if found.len() > 3 {
        names.push(format!("{} more", found.len() - 3));
    }
    names.join(", ")
}

/// `found` as one reading: failed where any part failed, running while any is waiting or running,
/// passed once all have. `of` names what was read, for `says`.
pub(crate) fn folded(found: Vec<Found>, of: &str, finished: bool) -> Reading {
    let failed: Vec<&Found> = found.iter().filter(|one| one.state == ReadingState::Failed).collect();
    let going: Vec<&Found> =
        found.iter().filter(|one| matches!(one.state, ReadingState::Waiting | ReadingState::Running)).collect();
    let (state, says) = match (failed.is_empty(), going.is_empty()) {
        _ if found.is_empty() => (ReadingState::Waiting, format!("Nothing has started for {of}.")),
        (false, _) => (ReadingState::Failed, format!("{} failed for {of}.", named(&failed))),
        (true, false) => (
            ReadingState::Running,
            format!("{} of {} have passed for {of}.", found.len() - going.len(), found.len()),
        ),
        (true, true) => (ReadingState::Passed, format!("All {} passed for {of}.", found.len())),
    };
    let link = failed.first().copied().or(going.first().copied()).or(found.first()).and_then(|one| one.link.clone());
    Reading {
        state,
        finished: finished && going.is_empty() && !found.is_empty(),
        says,
        link,
        parts: found.into_iter().map(|one| Part { name: one.name, state: one.state }).collect(),
    }
}
