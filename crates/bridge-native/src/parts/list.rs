//! A list: its entries one under another, a hairline between them.
//!
//! No helper in the app draws a bare list; the look is the rule the rows already keep, a hairline
//! above every row but the first (`settings/rows.rs`, `split_row`), inside a rounded hairline frame
//! as the card draws it.

use gpui::{div, prelude::*, AnyElement, Div, ElementId, Stateful};

use crate::look::{token, Look};

pub fn list(look: &Look, id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .overflow_hidden()
}

/// One entry of the list: `child`, with the hairline above it unless it is first.
pub fn entry(look: &Look, first: bool, child: AnyElement) -> Div {
    div().when(!first, |entry| entry.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))).child(child)
}
