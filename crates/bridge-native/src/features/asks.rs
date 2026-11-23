//! The rows a feature page's cards hold: a Job, an ask, a design. A row's state is a dot in its hue with its words in the
//! tooltip; an ask's row says its title over where it stands and the day, and in a fixed column at the right is its Story link.
//! The older asks of a kind fold under one line.

use gpui::{div, prelude::*, px, App, ClickEvent, Div, ElementId, FontWeight, Hsla, Role, SharedString, Stateful, Window};

use super::dossier::{dot, pill_quiet};
use super::page_model::{AskLine, Hue, Row};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::settings::view::tip;
use crate::titlebar::parts::glyph;

/// Where a row stands in its card.
#[derive(Clone, Copy, Default)]
pub struct Spot {
    /// The keys are on it.
    pub picked: bool,
    /// It is the first of its card or lane, and has no hairline above.
    pub first: bool,
    /// It is in the Bug lane: an open one reads red, the way the sketch draws it.
    pub bug: bool,
    /// The hairline above it, where the card draws its own.
    pub divider: Option<Hsla>,
}

/// The token a state hue is.
pub fn hue_token(hue: Hue) -> Token {
    match hue {
        Hue::Running => token::STATUS_RUNNING,
        Hue::Done => token::STATUS_COMPLETED_SUCCESS,
        Hue::Handed => token::STATUS_HANDED_IN,
        Hue::Bad => token::STATUS_COMPLETED_FAILED,
        Hue::Warn => token::STATUS_AWAITING_REVIEW,
        Hue::NotStarted => token::STATUS_NOT_STARTED,
        Hue::Muted => token::FG_SUBTLE,
    }
}

/// The dot of a row, in its state's hue, with its tooltip. What runs has the soft ring round it.
fn mark(look: &Look, row: &Row, bug: bool) -> impl IntoElement {
    let open_bug = bug && matches!(row.hue, Hue::Running | Hue::Handed);
    let hue = if open_bug { token::STATUS_COMPLETED_FAILED } else { hue_token(row.hue) };
    let running = row.hue == Hue::Running && !open_bug;
    let mark = div()
        .id(SharedString::from(format!("feature-row-state-{}", row.key)))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .w(look.length(token::SPACE_3))
        .h(look.length(token::LEADING_XS))
        .tooltip(tip(row.said.clone()));
    labelled(mark.child(dot(look, hue, running)), Role::Image, row.said.clone())
}

/// The frame every row shares: its hairline above, and the accent at 12% where the keys are on it.
pub(super) fn frame(look: &Look, id: impl Into<ElementId>, spot: Spot) -> Stateful<Div> {
    let edge = spot.divider.unwrap_or_else(|| look.color(token::BORDER_SUBTLE));
    div()
        .id(id.into())
        .w_full()
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_2) + px(2.))
        .px(look.length(token::SPACE_3) + px(2.))
        .py(look.length(token::SPACE_2) + px(1.))
        .when(!spot.first, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(edge))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .when(spot.picked, |row| row.bg(look.color(token::ACCENT).opacity(0.12)))
        .when(!spot.picked, |row| row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
}

fn sub(look: &Look, words: String) -> Div {
    div().min_w_0().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(words))
}

/// The title over the line under it, which is the middle of every row.
fn text(look: &Look, title: &str, weight: Token, under: Option<String>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(div().font_weight(FontWeight(look.number(weight))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.to_string())))
        .children(under.filter(|under| !under.is_empty()).map(|under| sub(look, under)))
}

/// A Job's row, or a bug a Job filed: its dot, its title over its state and day.
pub fn job(look: &Look, id: impl Into<ElementId>, row: &Row, spot: Spot) -> Stateful<Div> {
    let bug = spot.bug;
    frame(look, id, spot).child(mark(look, row, bug)).child(text(look, &row.title, token::WEIGHT_HEADING, Some(row.meta.clone())))
}

/// A bug a Job filed, which is the same row as an ask's with no Story to open.
pub fn filed(look: &Look, id: impl Into<ElementId>, row: &Row, spot: Spot) -> Stateful<Div> {
    frame(look, id, spot).child(mark(look, row, true)).child(text(look, &row.title, token::WEIGHT_MEDIUM, Some(row.meta.clone())))
}

/// A design: what kind it is as a quiet pill, its title, and at the right the day.
pub fn design(look: &Look, id: impl Into<ElementId>, row: &Row, spot: Spot) -> Stateful<Div> {
    frame(look, id, spot)
        .items_center()
        .child(pill_quiet(look, row.said.clone()))
        .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.title.clone())))
        .when(!row.meta.is_empty(), |line| line.child(div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(row.meta.clone()))))
}

/// An ask's row, in fixed columns: the state dot, its title over where it stands and the day, and at the right `Story`, which is
/// absent for an ask with no Job. `story` is what a press on it does.
pub fn ask(look: &Look, id: impl Into<ElementId>, row: &Row, line: &AskLine, spot: Spot, story: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Stateful<Div> {
    let id: ElementId = id.into();
    let ink = look.color(if spot.picked { token::ACCENT } else { token::FG_SUBTLE });
    let go = div().flex_none().w(look.length(token::SPACE_12)).flex().items_center().justify_end().h(look.length(token::LEADING_XS)).when(line.story.is_some(), |cell| {
        cell.child(labelled(
            div()
                .id(SharedString::from(format!("{id}-story")))
                .flex()
                .items_center()
                .gap(px(2.))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(ink)
                .cursor_pointer()
                .hover(|style| style.text_color(look.color(token::ACCENT)))
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    story(event, window, cx);
                })
                .child("Story")
                .child(glyph("chevron-right", look.length(token::SPACE_3) + px(2.), ink)),
            Role::Link,
            format!("Story: {}", row.title),
        ))
    });
    let bug = spot.bug;
    frame(look, id, spot).child(mark(look, row, bug)).child(text(look, &row.title, token::WEIGHT_MEDIUM, Some(format!("{} · {}", line.state_said, line.date)))).child(go)
}

/// The line under the fifth row of a kind that folds the older ones. Pressing it opens them, and pressing it again folds them.
pub fn fold(look: &Look, id: impl Into<ElementId>, open: bool, picked: bool, press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> impl IntoElement {
    let words = if open { "Earlier" } else { "Earlier, folded" };
    labelled(
        div()
            .id(id.into())
            .w_full()
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .when(picked, |row| row.bg(look.color(token::ACCENT).opacity(0.12)))
            .when(!picked, |row| row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
            .aria_expanded(open)
            .on_click(press)
            .child(words),
        Role::Button,
        words,
    )
}
