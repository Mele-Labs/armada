//! The window's own title row, drawn where macOS's title bar was (`TitleBar.css`, `.armada-title-bar`).
//!
//! The window is frameless, so this row is what moves it: a press and drag moves the window and a
//! double press zooms it, as the title bar did. The traffic lights sit over its start, so what it
//! draws begins after Electron's gutter.

use gpui::{div, prelude::*, svg, MouseButton, Role, Stateful, Div};

use crate::assets::MARK;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The title row, 44px, with Armada's mark after the traffic lights. Its places, search and Fleet
/// controls come with the title bar's own step.
pub fn title_row(look: &Look) -> Stateful<Div> {
    let mark = look.length(token::SPACE_4) + look.length(token::SPACE_1);
    div()
        .id("title-row")
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_TITLE_BAR))
        .px(look.length(token::SPACE_3))
        .on_mouse_down(MouseButton::Left, |event, window, _cx| {
            if event.click_count == 2 {
                window.titlebar_double_click();
            } else {
                window.start_window_move();
            }
        })
        .child(
            // `padding-inline-start: calc(var(--space-12) * 2)` clears the traffic lights.
            div().pl(look.length(token::SPACE_12) * 2.).child(labelled(
                div().id("mark").child(svg().path(MARK).size(mark).text_color(look.color(token::FG_DEFAULT))),
                Role::Image,
                "Armada",
            )),
        )
}
