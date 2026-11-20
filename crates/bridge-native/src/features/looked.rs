//! When the owner last looked at each feature's page, kept on this Mac beside the list's own files
//! (`jobs::deck_file`): `native-features-looked`, a line a feature, `project<TAB>feature<TAB>time`.
//!
//! The live app reads the file the first time a live page is made and writes it a moment after each
//! change, off the main thread. A walk, a mock and a test keep it in memory, so they never read or write his.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use gpui::{App, AppContext as _, Global};

use crate::log::Note;

const FILE: &str = "native-features-looked";

#[derive(Default)]
struct Looked {
    at: BTreeMap<(String, String), DateTime<Utc>>,
    /// Where it is kept between runs, once a live page has asked.
    file: Option<PathBuf>,
}

impl Global for Looked {}

/// A word that fits on a line of the file.
fn plain(word: &str) -> bool {
    !word.is_empty() && !word.contains(['\t', '\n', '\r'])
}

fn text_of(at: &BTreeMap<(String, String), DateTime<Utc>>) -> String {
    at.iter().filter(|((project, slug), _)| plain(project) && plain(slug)).map(|((project, slug), at)| format!("{project}\t{slug}\t{}", at.to_rfc3339())).collect::<Vec<_>>().join("\n")
}

fn from_text(text: &str) -> BTreeMap<(String, String), DateTime<Utc>> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let (project, slug, at) = (parts.next()?, parts.next()?, parts.next()?);
            let at = DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&Utc);
            (plain(project) && plain(slug) && parts.next().is_none()).then(|| ((project.to_string(), slug.to_string()), at))
        })
        .collect()
}

fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name(FILE))
}

/// Keep what is kept in the file from here on, reading it the first time. Called as a live page is made.
pub fn persist(cx: &mut App) {
    if cx.try_global::<Looked>().is_some_and(|held| held.file.is_some()) {
        return;
    }
    let Some(path) = file() else { return };
    let at = std::fs::read_to_string(&path).map(|text| from_text(&text)).unwrap_or_default();
    let held = cx.default_global::<Looked>();
    for (key, was) in at {
        held.at.entry(key).and_modify(|now| *now = (*now).max(was)).or_insert(was);
    }
    held.file = Some(path);
}

/// When the feature was last looked at, where it ever was.
pub fn before(project: &str, slug: &str, cx: &App) -> Option<DateTime<Utc>> {
    cx.try_global::<Looked>().and_then(|held| held.at.get(&(project.to_string(), slug.to_string())).copied())
}

/// He looked at the feature at `at`: what a later look compares with.
pub fn stamp(project: &str, slug: &str, at: DateTime<Utc>, cx: &mut App) {
    let held = cx.default_global::<Looked>();
    held.at.insert((project.to_string(), slug.to_string()), at);
    let Some(path) = held.file.clone() else { return };
    let text = text_of(&held.at);
    cx.background_spawn(async move {
        if let Err(why) = crate::crash::write_atomic(&path, &text) {
            Note::warn("when a feature was last looked at was not kept").field("why", why.to_string()).emit();
        }
    })
    .detach();
}

/// The slug the project's own look is kept under, beside the features': a `~` starts no feature's slug.
const PROJECT_LOOK: &str = "~since-you-looked";

/// When he last marked the project's Since you looked page read, where he ever did.
pub fn before_project(project: &str, cx: &App) -> Option<DateTime<Utc>> {
    before(project, PROJECT_LOOK, cx)
}

/// He marked the project's Since you looked page read at `at`.
pub fn stamp_project(project: &str, at: DateTime<Utc>, cx: &mut App) {
    stamp(project, PROJECT_LOOK, at, cx);
}

/// Forget what is held, in memory only.
#[cfg(test)]
pub fn forget_all(cx: &mut App) {
    cx.set_global(Looked::default());
}
