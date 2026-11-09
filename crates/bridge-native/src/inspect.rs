//! The inspect card: the one surface of every rich hover (`primitives/Inspect/Inspect.css`). The overlay ground, a
//! hairline, the deepest shadow and one radius; a thin top edge in the item's tone; a head naming what it is, a
//! title, a state band that is the one place colour carries meaning, rows of facts, and the keys that work in it.

use gpui::{div, prelude::*, FontWeight, Hsla, SharedString};

use crate::look::{token, Look};
use crate::titlebar::parts::{card, glyph, kbd, mono, small};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Narrow,
    Wide,
}

/// One row of facts: a term and its value, the value in mono where it is a machine's.
pub struct Fact {
    pub term: SharedString,
    pub value: SharedString,
    pub mono: bool,
    pub tone: Option<Hsla>,
}

impl Fact {
    pub fn new(term: impl Into<SharedString>, value: impl Into<SharedString>) -> Fact {
        Fact { term: term.into(), value: value.into(), mono: false, tone: None }
    }

    pub fn mono(mut self) -> Fact {
        self.mono = true;
        self
    }

    pub fn tone(mut self, tone: Hsla) -> Fact {
        self.tone = Some(tone);
        self
    }
}

/// The state band: a glyph and the words, over a 14% wash of their colour.
pub struct State {
    pub glyph: Option<&'static str>,
    pub said: SharedString,
    pub ink: Hsla,
}

pub struct Spec {
    pub kind_glyph: &'static str,
    pub kind: SharedString,
    pub place: Option<SharedString>,
    pub title: SharedString,
    pub state: Option<State>,
    /// The top edge's colour; transparent where the item has no tone.
    pub edge: Option<Hsla>,
    pub width: Width,
    pub keys: Vec<(&'static str, &'static str)>,
}

/// Rows of facts, the term over a mono or plain value, as `.armada-inspect-card__facts`.
pub fn facts(rows: Vec<Fact>, look: &Look) -> gpui::Div {
    div().flex().flex_col().gap(look.length(token::BORDER_WIDTH) * 2.).children(rows.into_iter().map(|row| {
        small(look)
            .flex()
            .justify_between()
            .gap(look.length(token::SPACE_3))
            .child(div().flex_none().max_w(gpui::relative(0.6)).text_color(look.color(token::FG_MUTED)).child(row.term))
            .child(
                div()
                    .min_w_0()
                    .text_right()
                    .text_color(row.tone.unwrap_or_else(|| look.color(token::FG_DEFAULT)))
                    .when(row.mono, |value| value.font_family(mono(look)))
                    .child(row.value),
            )
    }))
}

/// A heading over a group of rows (`InspectSection`).
pub fn heading(said: impl Into<SharedString>, look: &Look) -> gpui::Div {
    small(look).mt(look.length(token::SPACE_1)).text_color(look.color(token::FG_MUTED)).child(said.into())
}

impl Spec {
    /// The card with `body` between the state band and the keys.
    pub fn draw(self, body: Vec<gpui::AnyElement>, look: &Look) -> gpui::Div {
        let edge = self.edge.unwrap_or_else(gpui::transparent_black);
        let head = small(look)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .text_color(look.color(token::FG_MUTED))
            .child(glyph(self.kind_glyph, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(self.kind))
            .children(self.place.map(|place| {
                div().flex_initial().min_w_0().ml_auto().truncate().font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).child(place)
            }));
        let title = div()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .child(self.title);
        let state = self.state.map(|state| {
            small(look)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .bg(state.ink.opacity(0.14))
                .text_color(state.ink)
                .children(state.glyph.map(|name| glyph(name, look.length(token::SPACE_4), state.ink)))
                .child(state.said)
        });
        let keys = (!self.keys.is_empty()).then(|| {
            small(look)
                .flex()
                .flex_wrap()
                .gap_x(look.length(token::SPACE_3))
                .gap_y(look.length(token::SPACE_1))
                .pt(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .text_color(look.color(token::FG_MUTED))
                .children(self.keys.into_iter().map(|(key, verb)| {
                    div().flex().items_center().gap(look.length(token::SPACE_1)).child(kbd(key, look)).child(verb)
                }))
        });
        card(look)
            .when(self.width == Width::Wide, |card| card.w(look.length(token::W_TOOLTIP)))
            .when(self.width == Width::Narrow, |card| card.w(look.length(token::SPACE_12) * 5.))
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_4))
            .relative()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(look.length(token::BORDER_WIDTH) * 2.)
                    .rounded_t(look.length(token::RADIUS_LG))
                    .bg(edge),
            )
            .child(head)
            .child(title)
            .children(state)
            .children(body)
            .children(keys)
    }
}
