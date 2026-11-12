//! Starting a terminal Job, from the dispatch sheet's Terminal, ⌘T or the palette's New terminal: Fleet
//! starts the Job with its first shell open, and nothing runs in its thread until it is written to.
//! Where it starts is kept, so ⌘T with nothing in view starts in the same place again.

use gpui::{App, Task};

use super::shells::{self, Place, Start, Started};

/// Where the last terminal Job started: `repo:<name>` or `dir:<path>`.
const KEEP_PLACE: &str = "terminal:place";

/// The place a terminal Job last started in.
pub fn kept_place(cx: &App) -> Option<Place> {
    let kept = crate::drafts::kept(KEEP_PLACE, cx)?;
    if let Some(name) = kept.strip_prefix("repo:") {
        return Some(Place::Repository(name.to_string()));
    }
    kept.strip_prefix("dir:").map(|path| Place::Folder(path.to_string()))
}

pub fn keep_place(place: &Place, cx: &mut App) {
    let kept = match place {
        Place::Repository(name) => format!("repo:{name}"),
        Place::Folder(path) => format!("dir:{path}"),
    };
    crate::drafts::keep(KEEP_PLACE, &kept, cx);
}

/// Ask for the Job. None where no shells are reachable: no Fleet, and no walk.
pub fn start(start: Start, cx: &mut App) -> Task<Result<Started, String>> {
    match shells::of(cx) {
        Some(shells) => shells.start(start, cx),
        None => Task::ready(Err("Fleet is not connected".into())),
    }
}
