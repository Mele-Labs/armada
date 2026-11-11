//! The part of a window the app draws in. A walk window keeps its bar above the app, so an overlay
//! that covers "the whole window", such as a sheet or a dialog's scrim, covers this instead and the
//! walk's step stays readable.

use std::collections::HashMap;

use gpui::{point, px, App, Bounds, Global, Pixels, Window, WindowId};

/// Paint order for the walk's own chrome: its bar, the ring on a step's target and the note card.
/// Above every app layer, which use 0 to 3.
pub const WALK_CHROME: usize = 10;

#[derive(Default)]
struct Rooms(HashMap<WindowId, Bounds<Pixels>>);

impl Global for Rooms {}

/// Give the app in `window` only `room`, as the walk window does below its bar.
pub fn set(window: &Window, room: Bounds<Pixels>, cx: &mut App) {
    cx.default_global::<Rooms>().0.insert(window.window_handle().window_id(), room);
}

/// Where the app in `window` draws: all of the window, unless a walk keeps part of it.
pub fn of(window: &Window, cx: &App) -> Bounds<Pixels> {
    cx.try_global::<Rooms>()
        .and_then(|rooms| rooms.0.get(&window.window_handle().window_id()).copied())
        .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), window.viewport_size()))
}
