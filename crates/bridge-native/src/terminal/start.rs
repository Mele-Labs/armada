//! Starting Fleet's quick Job, from ⌘T or the palette's New Job here: it opens on its thread, and nothing
//! runs there until it is written to. Where it starts is kept, so ⌘T with nothing in view starts in the
//! same place again.

use gpui::{App, Task};

use super::screen::Cells;
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

/// The size a terminal pane last measured, so a new Job's first shell opens at the size it will be drawn.
struct Measured(Cells);

impl gpui::Global for Measured {}

pub(super) fn measured(cells: Cells, cx: &mut App) {
    cx.set_global(Measured(cells));
}

/// The first shell's size: the last pane's, else 80 by 24 before any pane is drawn.
pub fn size(cx: &App) -> Cells {
    cx.try_global::<Measured>().map_or(Cells::OPENED, |measured| measured.0)
}

/// Ask for the Job. None where no shells are reachable: no Fleet, and no walk.
pub fn start(start: Start, cx: &mut App) -> Task<Result<Started, String>> {
    match shells::of(cx) {
        Some(shells) => shells.start(start, cx),
        None => Task::ready(Err("Fleet is not connected".into())),
    }
}
