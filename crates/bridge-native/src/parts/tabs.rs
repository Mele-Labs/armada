//! A row of tabs in a sunken track, the picked one raised in an accent wash and ring.
//!
//! The settings segment control's look (`settings/rows.rs`, `segment_track` and `segment`): the
//! track a hairline and a sunken ground, the picked segment `ACCENT_MUTED` inside an `ACCENT` ring
//! with heading weight, the others plain muted words that lift to the default ink under the pointer.

use gpui::{div, prelude::*, Div, ElementId, FontWeight, SharedString, Stateful};

use crate::look::{token, Look};

pub fn track(look: &Look, id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1) / 2.)
        .p(look.length(token::SPACE_1) / 2. + look.length(token::BORDER_WIDTH))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
}

pub fn tab(look: &Look, id: impl Into<ElementId>, said: impl Into<SharedString>, picked: bool) -> Stateful<Div> {
    let edge = look.length(token::BORDER_WIDTH);
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let hover = look.color(token::FG_DEFAULT);
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL) - look.length(token::SPACE_1) - edge * 2.)
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(edge)
        .border_color(gpui::transparent_black())
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .focusable()
        .tab_stop(true)
        .focus_visible(move |style| style.border_color(ring))
        .map(|tab| {
            if picked {
                tab.bg(look.color(token::ACCENT_MUTED)).border_color(look.color(token::ACCENT)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT))
            } else {
                tab.text_color(look.color(token::FG_MUTED)).cursor_pointer().hover(move |style| style.text_color(hover))
            }
        })
        .child(said.into())
}
