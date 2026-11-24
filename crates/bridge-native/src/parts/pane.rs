//! A pane: a title band over a body.
//!
//! The band's type is a section's heading (`job_view/system.rs`, `section`): mono 2xs in the subtle
//! ink at heading weight. That heading floats over the page; a pane sets it in a band with a rule
//! under it, on the sunken ground.

use gpui::{div, prelude::*, Div, ElementId, SharedString, Stateful};

use super::text::{text, TextStyle};
use crate::look::{token, Look};

/// The pane: the band is in, and the caller adds the body's children after it.
pub fn pane(look: &Look, id: impl Into<ElementId>, title: impl Into<SharedString>) -> Stateful<Div> {
    let band = div()
        .flex_none()
        .flex()
        .items_center()
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .bg(look.color(token::BG_SUNKEN))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .child(text(look, "pane-title", TextStyle::Mono, title));
    div().id(id).flex().flex_col().child(band)
}
