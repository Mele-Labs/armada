//! A block whose view is another Job's: its title and one line saying when it arrives.

use gpui::{div, prelude::*, Context, Render, Role, SharedString, Window};

use super::HomeBlockKind;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::titlebar::parts::{glyph, sans};

pub struct PlaceholderBlock {
    kind: HomeBlockKind,
}

impl PlaceholderBlock {
    pub fn new(kind: HomeBlockKind) -> Self {
        PlaceholderBlock { kind }
    }

    /// What the card says under its title.
    pub fn line(&self) -> &'static str {
        match self.kind {
            HomeBlockKind::SystemOverview => "Arrives with the System overview",
            HomeBlockKind::SystemStats => "Arrives with System stats",
            HomeBlockKind::NeedsCards | HomeBlockKind::FloorPlan => "Arrives later",
        }
    }
}

impl Render for PlaceholderBlock {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let kind = self.kind;
        let title = labelled(
            div()
                .id(SharedString::from(format!("{}-title", kind.slug())))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .text_size(look.length(token::TEXT_SM))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(glyph(kind.mark(), look.length(token::SPACE_4), look.color(kind.hue())))
                .child(div().min_w_0().truncate().child(kind.title())),
            Role::Heading,
            kind.title(),
        );
        let line = self.line();
        let said = labelled(
            div().id(SharedString::from(format!("{}-line", kind.slug()))).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(line),
            Role::Status,
            line,
        );
        div()
            .id(SharedString::from(format!("home-{}", kind.slug())))
            .size_full()
            .flex()
            .flex_col()
            .justify_center()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .overflow_hidden()
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(title)
            .child(said)
    }
}
