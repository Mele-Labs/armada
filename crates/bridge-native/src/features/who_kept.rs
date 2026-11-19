//! The layout Who's on what is drawn in, kept on this Mac beside the list's own files (`jobs::deck_file`):
//! `native-features-who-view`, one line holding the layout's words.
//!
//! The live app reads the file the first time a live page is made and writes it a moment after each change, off the main
//! thread. A walk, a mock and a test keep it in memory, so they never read or write his.

use std::path::PathBuf;

use gpui::{App, AppContext as _, Global};

use super::who_model::WhoView;
use crate::log::Note;

const FILE: &str = "native-features-who-view";

#[derive(Default)]
struct Kept {
    view: Option<WhoView>,
    /// Where it is kept between runs, once a live page has asked.
    file: Option<PathBuf>,
}

impl Global for Kept {}

fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name(FILE))
}

/// Keep what is chosen in the file from here on, reading it the first time. Called as a live page is made.
pub fn persist(cx: &mut App) {
    if let Some(path) = file() {
        persist_at(path, cx);
    }
}

/// The same, with the file named: what a test points at a folder of its own.
pub fn persist_at(path: PathBuf, cx: &mut App) {
    if cx.try_global::<Kept>().is_some_and(|held| held.file.is_some()) {
        return;
    }
    let was = std::fs::read_to_string(&path).ok().and_then(|text| WhoView::of_title(text.trim()));
    let held = cx.default_global::<Kept>();
    if held.view.is_none() {
        held.view = was;
    }
    held.file = Some(path);
}

/// The layout he chose last, where he ever did.
pub fn get(cx: &App) -> Option<WhoView> {
    cx.try_global::<Kept>().and_then(|held| held.view)
}

/// He chose `view`: what the next page and the next start show.
pub fn set(view: WhoView, cx: &mut App) {
    let held = cx.default_global::<Kept>();
    held.view = Some(view);
    let Some(path) = held.file.clone() else { return };
    let text = view.title().to_string();
    cx.background_spawn(async move {
        if let Err(why) = crate::crash::write_atomic(&path, &text) {
            Note::warn("the layout of Who's on what was not kept").field("why", why.to_string()).emit();
        }
    })
    .detach();
}

/// Forget what is held, in memory only.
#[cfg(test)]
pub fn forget_all(cx: &mut App) {
    cx.set_global(Kept::default());
}
