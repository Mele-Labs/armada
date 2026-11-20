//! The project the Features tab shows, kept on this Mac beside the list's own files (`jobs::deck_file`):
//! `native-features-project`, one line holding the project's id.
//!
//! The live app reads the file the first time a live screen is made and writes it a moment after each pick, off the
//! main thread. A walk, a mock and a test keep it in memory, so they never read or write his.

use std::path::PathBuf;

use gpui::{App, AppContext as _, Global};

use crate::log::Note;

const FILE: &str = "native-features-project";

#[derive(Default)]
struct Chosen {
    project: Option<String>,
    /// Where it is kept between runs, once a live screen has asked.
    file: Option<PathBuf>,
}

impl Global for Chosen {}

/// A word that fits on the line of the file.
fn plain(word: &str) -> bool {
    !word.trim().is_empty() && !word.contains(['\t', '\n', '\r'])
}

fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name(FILE))
}

/// Keep what is chosen in the file from here on, reading it the first time. Called as a live screen is made.
pub fn persist(cx: &mut App) {
    if cx.try_global::<Chosen>().is_some_and(|held| held.file.is_some()) {
        return;
    }
    let Some(path) = file() else { return };
    let was = std::fs::read_to_string(&path).ok().map(|text| text.trim().to_string()).filter(|text| plain(text));
    let held = cx.default_global::<Chosen>();
    if held.project.is_none() {
        held.project = was;
    }
    held.file = Some(path);
}

/// The project he chose last, where he ever did.
pub fn get(cx: &App) -> Option<String> {
    cx.try_global::<Chosen>().and_then(|held| held.project.clone())
}

/// He chose `project`: what the next start shows.
pub fn set(project: &str, cx: &mut App) {
    let held = cx.default_global::<Chosen>();
    held.project = Some(project.to_string());
    let Some(path) = held.file.clone().filter(|_| plain(project)) else { return };
    let text = project.to_string();
    cx.background_spawn(async move {
        if let Err(why) = crate::crash::write_atomic(&path, &text) {
            Note::warn("the project the Features tab shows was not kept").field("why", why.to_string()).emit();
        }
    })
    .detach();
}

/// Forget what is held, in memory only.
#[cfg(test)]
pub fn forget_all(cx: &mut App) {
    cx.set_global(Chosen::default());
}
