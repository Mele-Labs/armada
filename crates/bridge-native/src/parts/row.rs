//! A row: its words at the left, its control at the right, a hairline above it unless it is first.
//!
//! The settings row's look (`settings/rows.rs`, `split_row`): 24 and 16 padding, the title in
//! heading type with an optional glyph and a muted line under it, the control in its own column.
//! That row's accent stripe for a changed setting is left out: a stripe on one side of a box is not
//! in the screens' rules.

use gpui::{div, prelude::*, AnyElement, Div, ElementId, Hsla, SharedString, Stateful};

use super::text::{text, TextStyle};
use crate::look::{token, Look};

pub fn row(
    look: &Look,
    id: impl Into<ElementId>,
    first: bool,
    glyph: Option<(&str, Hsla)>,
    title: Option<SharedString>,
    says: Option<SharedString>,
    controls: Vec<AnyElement>,
) -> Stateful<Div> {
    let edge = look.length(token::BORDER_WIDTH);
    let words = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .children(glyph.map(|(name, ink)| super::glyph(name, look.length(token::SPACE_4), ink)))
                .children(title.map(|title| text(look, "row-title", TextStyle::Heading, title))),
        )
        .children(says.map(|says| text(look, "row-says", TextStyle::Muted, says)));
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_8))
        .px(look.length(token::SPACE_6))
        .py(look.length(token::SPACE_4))
        .when(!first, |line| line.border_t(edge).border_color(look.color(token::BORDER_SUBTLE)))
        .child(words)
        .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).children(controls))
}
