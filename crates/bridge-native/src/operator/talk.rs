//! The talk about a card: the card folded to one line, then the Operator session's thread from his words on, and the cards
//! the Operator raised in the talk, whole and indented under its line. The box stays at the foot with the keys in it.
//!
//! The rows are the session's own messages as Fleet serves them in its thread (`SessionRow::Message`), his and the Operator's.
//! A message he has sent that the thread has not echoed yet is drawn as his, so a send shows at once.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use gpui_component::text::TextView;
use ipc::{SessionRow, SessionVoice, ThreadItem};

use super::page::OperatorPageView;
use super::view::round;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::glyph;

/// His words as he typed them: Fleet delivers them with a first line that tags the card, which the folded card above says already.
fn without_tag(text: &str) -> &str {
    match text.strip_prefix("[About card ") {
        Some(rest) => rest.split_once('\n').map_or("", |(_, words)| words.trim_start()),
        None => text,
    }
}

impl OperatorPageView {
    /// The main column while he talks: the folded card and the talk scroll, the box fixed at the foot.
    pub(super) fn talk_column(&self, window: &gpui::Window, look: &Look, cx: &Context<Self>) -> gpui::Stateful<gpui::Div> {
        let Some(talk) = self.talk.clone() else { return div().id("operator-main") };
        let card = self.items().iter().find(|turn| turn.id == talk.about).cloned();
        let mut parts: Vec<AnyElement> = Vec::new();
        // The folded card stays at the head of the column, out of the scroll, so the line it is about is always in sight.
        let folded = card.as_ref().map(|card| self.folded(card, talk.unfolded, look, cx));
        if let Some(card) = card.as_ref().filter(|_| talk.unfolded) {
            parts.push(self.card(card, false, window, look, cx));
        }
        parts.extend(self.talk_rows(cx).iter().enumerate().filter_map(|(at, item)| self.talk_row(at, item, look, cx)));
        // What he sent, until the thread echoes it.
        if let Some(sent) = talk.sent.as_ref().filter(|sent| !self.talk_rows(cx).iter().any(|item| matches!(item, ThreadItem::Row { row: SessionRow::Message { text, .. } } if without_tag(text) == sent.trim()))) {
            parts.push(self.his_words("talk-sent", sent.as_ref(), None, look, cx));
        }
        // The cards the Operator raised in the talk, whole, under its line.
        let raised: Vec<AnyElement> = self.talk_cards().into_iter().map(|turn| self.card(turn, true, window, look, cx)).collect();
        parts.extend(raised);
        let scroll = div()
            .id("operator-talk")
            .track_scroll(&self.talk_scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .pt(look.length(token::SPACE_4) + look.length(token::SPACE_1))
            .pb(look.length(token::SPACE_4))
            .children(parts.into_iter().map(|part| div().flex_none().child(part)));
        div()
            .id("operator-main")
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .children(folded.map(|folded| div().flex_none().px(look.length(token::SPACE_6)).pt(look.length(token::SPACE_4) + look.length(token::SPACE_1)).pb(look.length(token::SPACE_2)).child(folded)))
            .child(labelled(scroll, Role::Log, "Talk with the Operator"))
            .child(div().flex_none().px(look.length(token::SPACE_6)).child(self.composer.clone()))
    }

    fn his_words(&self, id: &'static str, text: &str, at: Option<&str>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said: SharedString = text.to_string().into();
        labelled(
            div()
                .id(SharedString::from(format!("{id}-{}", at.unwrap_or("now"))))
                .flex()
                .justify_end()
                .child(
                    div()
                        .max_w(gpui::relative(0.8))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_4))
                        .py(look.length(token::SPACE_3))
                        .rounded(round(look))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_RAISED))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .child(said.clone())
                        .children(at.map(|at| div().flex().justify_end().child(self.stamp(at, None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)))),
                ),
            Role::ListItem,
            format!("You: {said}"),
        )
        .into_any_element()
    }

    fn talk_row(&self, at: usize, item: &ThreadItem, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let ThreadItem::Row { row: SessionRow::Message { id, at: sent, from, text, .. } } = item else { return None };
        match from {
            SessionVoice::You => Some(self.his_words("talk-you", without_tag(text), Some(sent.as_str()), look, cx)),
            SessionVoice::Agent => {
                let said = text.clone();
                Some(
                    labelled(
                        div()
                            .id(SharedString::from(format!("talk-agent-{at}")))
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_2))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(look.length(token::SPACE_2))
                                    .text_size(look.length(token::TEXT_XS))
                                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                    .text_color(look.color(token::OPERATOR))
                                    .child(glyph("bot", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                                    .child("Operator")
                                    .child(self.stamp(sent.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)),
                            )
                            .child(
                                TextView::markdown(gpui::ElementId::Name(format!("talk:{id}").into()), said.clone())
                                    .w_full()
                                    .text_size(look.length(token::TEXT_SM))
                                    .line_height(gpui::relative(1.6))
                                    .text_color(look.color(token::FG_DEFAULT)),
                            ),
                        Role::ListItem,
                        format!("Operator: {said}"),
                    )
                    .into_any_element(),
                )
            }
            SessionVoice::Session { .. } => None,
        }
    }
}
