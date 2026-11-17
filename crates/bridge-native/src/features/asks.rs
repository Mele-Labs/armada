//! A row of the open section as a basic line: a glyph in its state's hue, the title and a meta line, and the
//! chip of a Job that is on the feature now in a fixed column at the right. The full row of an ask, with its
//! Job and its Story link, replaces [`row`].

use gpui::{div, prelude::*, Div, ElementId, FontWeight, Role, SharedString, Stateful};

use super::page_model::{Hue, Row};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::settings::view::tip;
use crate::titlebar::parts::glyph;

/// The token a state hue is.
pub fn hue_token(hue: Hue) -> Token {
    match hue {
        Hue::Running => token::STATUS_RUNNING,
        Hue::Done => token::STATUS_COMPLETED_SUCCESS,
        Hue::Handed => token::STATUS_HANDED_IN,
        Hue::Bad => token::STATUS_COMPLETED_FAILED,
        Hue::Warn => token::STATUS_AWAITING_REVIEW,
        Hue::Muted => token::FG_SUBTLE,
    }
}

/// A filled chip: its words in a hue on a tint of that hue.
pub fn tinted(look: &Look, hue: Token, words: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(hue).opacity(0.14))
        .text_color(look.color(hue))
        .text_size(look.length(token::TEXT_2XS))
        .whitespace_nowrap()
        .child(words.into())
}

/// One row. The one the keys are on wears the accent at 12%, and its top line goes.
pub fn row(look: &Look, id: impl Into<ElementId>, row: &Row, picked: bool) -> Stateful<Div> {
    let mark = div()
        .id(SharedString::from(format!("feature-row-state-{}", row.key)))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::SPACE_4))
        .mt(gpui::px(2.))
        .tooltip(tip(row.said.clone()));
    let mark = labelled(mark.child(glyph(row.glyph, look.length(token::SPACE_4), look.color(hue_token(row.hue)))), Role::Image, row.said.clone());
    let text = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(gpui::px(2.))
        .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.title.clone())))
        .when(!row.meta.is_empty(), |text| {
            text.child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(row.meta.clone())))
        });
    // The chip of a Job on it now stands in a column of its own, so it lines up down the list.
    let on = div().flex_none().w(look.length(token::SPACE_12) * 2.).flex().items_center().justify_end().when(row.on, |cell| cell.child(tinted(look, token::STATUS_RUNNING, "On it now")));
    div()
        .id(id.into())
        .w_full()
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_2))
        .py(look.length(token::SPACE_3))
        .border_t(look.length(token::BORDER_WIDTH))
        .border_color(if picked { gpui::transparent_black() } else { look.color(token::BORDER_SUBTLE) })
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
        .when(picked, |row| row.bg(look.color(token::ACCENT).opacity(0.12)).rounded(look.length(token::RADIUS_MD)))
        .when(!picked, |row| row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
        .child(mark)
        .child(text)
        .child(on)
}
