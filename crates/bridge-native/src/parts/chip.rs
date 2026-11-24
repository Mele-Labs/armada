//! A chip: a small word in a colour, on a tint of it, ringed by a hairline of it.
//!
//! Its look is the state chip's (`job_view/band_card.rs`, `state_chip`): a round-ended pill, the
//! colour at 16 percent behind the word, 2xs medium type, a glyph before the word. The Mac pill
//! (`pairing.rs`, `pill`) draws the same tint with square corners; the state chip is the one on
//! most screens. The hairline in the hue is the screens' rule for a coloured thing.

use gpui::{div, prelude::*, Div, ElementId, FontWeight, Hsla, SharedString, Stateful};

use crate::look::{token, Look};

pub fn chip(look: &Look, id: impl Into<ElementId>, ink: Hsla, glyph: Option<&str>, said: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .border(look.length(token::BORDER_WIDTH))
        .border_color(ink.opacity(0.4))
        .bg(ink.opacity(0.16))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(ink)
        .children(glyph.map(|name| super::glyph(name, look.length(token::TEXT_2XS), ink)))
        .child(said.into())
}
