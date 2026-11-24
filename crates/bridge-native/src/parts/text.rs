//! A line of words in one of the app's six type styles.
//!
//! Each style is read off the helper that used it most: `Body` and `Heading` from a settings row's
//! title (`settings/rows.rs`, `split_row`), `Title` from a card's head (`operator/card.rs`,
//! `card_head`), `Muted` from a row's description (`settings/rows.rs`, `says`), `Small` from the
//! chips' size, `Mono` from a section's heading (`job_view/system.rs`, `section`).

use gpui::{div, prelude::*, Div, ElementId, FontWeight, SharedString, Stateful};

use crate::look::{token, Look};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyle {
    Body,
    Heading,
    Title,
    Muted,
    Small,
    Mono,
}

impl TextStyle {
    pub fn named(word: &str) -> Option<TextStyle> {
        Some(match word {
            "body" => TextStyle::Body,
            "heading" => TextStyle::Heading,
            "title" => TextStyle::Title,
            "muted" => TextStyle::Muted,
            "small" => TextStyle::Small,
            "mono" => TextStyle::Mono,
            _ => return None,
        })
    }
}

/// The type of `style`, with no words in it: the caller adds its own.
pub fn styled(look: &Look, id: impl Into<ElementId>, style: TextStyle) -> Stateful<Div> {
    let line = div().id(id).min_w_0();
    match style {
        TextStyle::Body => line.text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)),
        TextStyle::Heading => line
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT)),
        TextStyle::Title => line
            .text_size(look.length(token::TEXT_LG))
            .line_height(look.length(token::LEADING_LG))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT)),
        TextStyle::Muted => line.text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)),
        TextStyle::Small => line.text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)),
        TextStyle::Mono => line
            .font_family(super::mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_SUBTLE)),
    }
}

/// `said` in `style`.
pub fn text(look: &Look, id: impl Into<ElementId>, style: TextStyle, said: impl Into<SharedString>) -> Stateful<Div> {
    styled(look, id, style).child(said.into())
}
