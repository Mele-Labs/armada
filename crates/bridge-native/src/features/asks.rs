//! A row of the open section: a glyph in its state's hue, a line and a meta line, and in a fixed column at the right
//! the chip of a Job that is on the feature now, or an ask's Story link. An ask's row says his words, where he said
//! them and where the ask stands; the older rows fold under one line.

use gpui::{div, prelude::*, App, ClickEvent, Div, ElementId, FontWeight, HighlightStyle, Role, SharedString, Stateful, StyledText, Window};

use super::page_model::{AskLine, Hue, Row};
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
        Hue::NotStarted => token::STATUS_NOT_STARTED,
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

/// The 16px glyph of a row, in its state's hue, with its tooltip.
fn mark(look: &Look, row: &Row) -> impl IntoElement {
    let mark = div()
        .id(SharedString::from(format!("feature-row-state-{}", row.key)))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::SPACE_4))
        .mt(gpui::px(2.))
        .tooltip(tip(row.said.clone()));
    labelled(mark.child(glyph(row.glyph, look.length(token::SPACE_4), look.color(hue_token(row.hue)))), Role::Image, row.said.clone())
}

/// The frame every row shares: its hairline above, and the accent at 12% where the keys are on it.
fn frame(look: &Look, id: impl Into<ElementId>, picked: bool) -> Stateful<Div> {
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
}

/// One row. The one the keys are on wears the accent at 12%, and its top line goes.
pub fn row(look: &Look, id: impl Into<ElementId>, row: &Row, picked: bool) -> Stateful<Div> {
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
    frame(look, id, picked).child(mark(look, row)).child(text).child(on)
}

/// The second line of an ask: where he said it, with the Job's title as a link-looking word in the accent, and where it stands.
fn meta(look: &Look, line: &AskLine) -> impl IntoElement {
    let mut said = line.before.clone();
    let job = line.job.as_ref().map(|job| {
        let from = said.len();
        said.push_str(job);
        from..said.len()
    });
    said.push_str(&line.after);
    let mut styled = StyledText::new(SharedString::from(said));
    if let Some(range) = job {
        styled = styled.with_highlights([(range, HighlightStyle { color: Some(look.color(token::ACCENT)), ..Default::default() })]);
    }
    div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(styled)
}

/// An ask's row, in fixed columns: the state glyph, his words over where they were said and where they stand, and at the
/// right `Story >`, which is absent for an ask with no Job. `story` is what a press on it does.
pub fn ask(look: &Look, id: impl Into<ElementId>, row: &Row, line: &AskLine, picked: bool, story: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Stateful<Div> {
    let id: ElementId = id.into();
    let text = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(gpui::px(2.))
        .child(div().min_w_0().truncate().whitespace_nowrap().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(line.words.clone())))
        .child(meta(look, line));
    // The link stands in a column of its own at the right, so the links line up down the list.
    let ink = look.color(if picked { token::ACCENT } else { token::FG_SUBTLE });
    let go = div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_2)).self_center().flex().items_center().justify_end().when(line.story.is_some(), |cell| {
        cell.child(labelled(
            div()
                .id(SharedString::from(format!("{id}-story")))
                .flex()
                .items_center()
                .gap(gpui::px(2.))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(ink)
                .cursor_pointer()
                .hover(|style| style.text_color(look.color(token::ACCENT)))
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    story(event, window, cx);
                })
                .child("Story")
                .child(glyph("chevron-right", look.length(token::SPACE_3) + gpui::px(2.), ink)),
            Role::Link,
            format!("Story: {}", row.title),
        ))
    });
    frame(look, id, picked).child(mark(look, row)).child(text).child(go)
}

/// The line under the fifth row that folds the older ones. Pressing it opens them, and pressing it again folds them.
pub fn fold(look: &Look, id: impl Into<ElementId>, open: bool, picked: bool, press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> impl IntoElement {
    let words = if open { "Earlier" } else { "Earlier, folded" };
    labelled(
        div()
            .id(id.into())
            .w_full()
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(if picked { gpui::transparent_black() } else { look.color(token::BORDER_SUBTLE) })
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_SUBTLE))
            .when(picked, |row| row.bg(look.color(token::ACCENT).opacity(0.12)).rounded(look.length(token::RADIUS_MD)))
            .when(!picked, |row| row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
            .aria_expanded(open)
            .on_click(press)
            .child(words),
        Role::Button,
        words,
    )
}
