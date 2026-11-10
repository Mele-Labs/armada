//! The window's own title row, drawn where macOS's title bar was (`TitleBar.css`, `.armada-title-bar`).
//!
//! The window is frameless, so this row is what moves it: a press and drag moves the window and a
//! double press zooms it, as the title bar did. The traffic lights sit over its start, so what it
//! draws begins after Electron's gutter. What it holds is the [`titlebar`](crate::titlebar) entity,
//! which a screen mounts once and finds again on every frame.

use gpui::{AnyElement, App, IntoElement, Window};

use crate::titlebar::TitleBar;

/// The title row with what stands under it: the Keep and Revert bar and the Fleet notice. The end
/// region keeps an empty Job slot for the open Job's Layout control and view icons, which the Jobs
/// screens fill with `TitleBar::fill_job_slot`.
pub fn title_row(window: &mut Window, cx: &mut App) -> AnyElement {
    window.use_keyed_state("title-bar", cx, |window, cx| TitleBar::new(window, cx)).into_any_element()
}
