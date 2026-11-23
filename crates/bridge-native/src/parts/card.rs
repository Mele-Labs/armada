//! A card: a raised ground inside a hairline, with an optional head band over its body.
//!
//! The frame is the Operator card's (`operator/card.rs`, `card_frame`), the raised ground, the
//! hairline and the clipped corners, with the edge in the hue a caller gives and the neutral
//! hairline where it gives none. The head band is that file's `card_head`: the title in large
//! heading type over a band with its own rule.

use gpui::{div, prelude::*, Div, ElementId, FontWeight, Hsla, SharedString, Stateful};

use crate::look::{token, Look};

pub fn card(look: &Look, id: impl Into<ElementId>, edge: Option<Hsla>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge.unwrap_or_else(|| look.color(token::BORDER_DEFAULT)))
        .bg(look.color(token::BG_RAISED))
        .overflow_hidden()
}

/// The band at the card's head: its title, with a rule under it. `hue` tints the band.
pub fn head(look: &Look, id: impl Into<ElementId>, hue: Option<Hsla>, title: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_3))
        .bg(hue.map_or_else(|| look.color(token::BG_SUNKEN), |hue| hue.opacity(0.1)))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(hue.map_or_else(|| look.color(token::BORDER_SUBTLE), |hue| hue.opacity(0.4)))
        .text_size(look.length(token::TEXT_LG))
        .line_height(look.length(token::LEADING_LG))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_DEFAULT))
        .child(title.into())
}

/// The card's body: the padding and gap its children sit in.
pub fn body(look: &Look, id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::PAD_CARD))
}
