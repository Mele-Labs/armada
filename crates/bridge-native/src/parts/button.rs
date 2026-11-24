//! A button, primary, secondary or ghost.
//!
//! Its look is the title bar's `Btn` (`titlebar/parts.rs`), which a hundred call sites across a
//! dozen screens draw and the next most used button (the settings ghost, 22 sites) is a smaller
//! variant of. Primary is the accent fill, secondary a raised fill with a hairline, ghost no ground
//! until the pointer is on it. The press is the caller's: a button off carries none.

use gpui::{div, prelude::*, Div, ElementId, FontWeight, SharedString, Stateful};

use crate::look::{token, Look};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
}

impl Kind {
    pub fn named(word: &str) -> Option<Kind> {
        Some(match word {
            "primary" => Kind::Primary,
            "secondary" => Kind::Secondary,
            "ghost" => Kind::Ghost,
            _ => return None,
        })
    }
}

pub fn button(look: &Look, id: impl Into<ElementId>, kind: Kind, glyph: Option<&str>, said: impl Into<SharedString>, off: bool) -> Stateful<Div> {
    let (ground, hover, ink, edge) = match kind {
        Kind::Primary => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT)),
        Kind::Secondary => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_DEFAULT)),
        Kind::Ghost => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), gpui::transparent_black()),
    };
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let body = div()
        .id(id)
        .flex_none()
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_3))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .bg(ground)
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(ink)
        .focusable()
        .tab_stop(!off)
        .focus_visible(move |style| style.border_color(ring))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
        .children(glyph.map(|name| super::glyph(name, look.length(token::SPACE_4), ink)))
        .child(said.into());
    if off {
        body.opacity(0.5)
    } else {
        body.cursor_pointer().hover(move |style| style.bg(hover))
    }
}
