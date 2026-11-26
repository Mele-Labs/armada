//! The talk under the card: the Operator session's own thread, his words and its replies, in a list that follows its tail.
//! A send with the Discuss tag puts a row "About: <card title>" in it, and the replies follow in the same talk. The cards
//! the Operator raised in a talk are drawn whole at its foot, inset under its line. While the talk overflows its space
//! the card in front folds to one line; a card that would take the front then waits in the new-card strip above the talk.
//!
//! The rows are the session's messages as Fleet serves them in its thread (`SessionRow::Message`); Fleet's notices to
//! it are left out. A message he has sent that the thread has not echoed yet is drawn as his, so a send shows at once.

use gpui::{canvas, div, list, prelude::*, Animation, AnimationExt as _, AnyElement, App, BoxShadow, Context, FontWeight, Role, SharedString, Window};
use gpui_component::text::TextView;
use ipc::{SessionRow, SessionVoice, ThreadItem};

use super::page::OperatorPageView;
use super::wire::Wire;
use super::view::{press, pressed, Weight};
use crate::labels::labelled;
use crate::look::{token, ActiveLook as _, Look};
use crate::titlebar::parts::{glyph, tip};

/// The card a message is about, as Fleet tags his words with it (`[About card turn:7 "Title"]` on a line of its own),
/// and his words after the tag.
pub(super) fn tag_of(text: &str) -> Option<(&str, &str, &str)> {
    let rest = text.strip_prefix("[About card ")?;
    let (line, words) = rest.split_once('\n').unwrap_or((rest, ""));
    let line = line.trim_end().trim_end_matches(']');
    let (id, title) = line.split_once(' ').unwrap_or((line, ""));
    Some((id, title.trim().trim_matches('"'), words.trim_start()))
}

/// One row of the talk.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Said {
    /// Where his words turn to a card: "About: <its title>".
    About { key: String, title: SharedString },
    /// His words. A send the thread has not echoed yet has no time.
    You { id: String, at: Option<String>, words: SharedString },
    /// The Operator's reply.
    Operator { id: String, at: String, text: SharedString },
    /// A card the Operator raised in the talk, by id.
    Raised(String),
}

/// What the talk draws: the thread's messages, his send not echoed yet, and the cards raised in the talk. An About row stands
/// before his words whenever the card they are about is not the one his last words were about; words about no card have none.
pub(super) fn said_of(items: &[ThreadItem], sent: Option<(&str, Option<(&str, &str)>)>, raised: Vec<String>) -> Vec<Said> {
    let mut said = Vec::new();
    let mut about: Option<String> = None;
    let mut turned = |said: &mut Vec<Said>, card: Option<(&str, &str)>| match card {
        Some((id, title)) if about.as_deref() != Some(id) => {
            said.push(Said::About { key: format!("about:{}", said.len()), title: title.to_string().into() });
            about = Some(id.to_string());
        }
        Some(_) => {}
        None => about = None,
    };
    let mut last_words: Option<&str> = None;
    for item in items {
        let ThreadItem::Row { row: SessionRow::Message { id, at, from, text, notice, .. } } = item else { continue };
        match from {
            SessionVoice::You if notice.is_none() => {
                let (card, words) = match tag_of(text) {
                    Some((card, title, words)) => (Some((card, title)), words),
                    None => (None, text.as_str()),
                };
                turned(&mut said, card);
                said.push(Said::You { id: id.clone(), at: Some(at.as_str().to_string()), words: words.to_string().into() });
                last_words = Some(words);
            }
            SessionVoice::Agent if !text.trim().is_empty() => said.push(Said::Operator { id: id.clone(), at: at.as_str().to_string(), text: text.clone().into() }),
            _ => {}
        }
    }
    if let Some((words, card)) = sent.filter(|(words, _)| last_words.map(str::trim) != Some(words.trim())) {
        turned(&mut said, card);
        said.push(Said::You { id: "sent".into(), at: None, words: words.to_string().into() });
    }
    said.extend(raised.into_iter().map(Said::Raised));
    said
}

impl OperatorPageView {
    /// The talk read again from the Operator's thread, and its list moved to it: the rows that stayed are kept, and a talk that
    /// grew folds the card he had opened beside it.
    pub(super) fn sync_talk(&mut self, cx: &App) {
        let session = self.page.as_ref().and_then(|page| page.session_id.clone());
        // His last send not echoed yet: words about no card, else words about the card the talk is about.
        let sent = match &self.loose {
            Some(words) => Some((words.to_string(), None)),
            None => self.talk.as_ref().and_then(|talk| talk.sent.as_ref().map(|sent| (sent.to_string(), Some((talk.about.clone(), talk.title.to_string()))))),
        };
        let raised = self.talk_cards().into_iter().map(|turn| turn.id.clone()).collect();
        let said = {
            let items: &[ThreadItem] = match (&self.wire, &session) {
                (Wire::Live(fleet), Some(session)) => fleet.read(cx).held().threads.get(session.as_str()).map_or(&[], |held| held.items.as_slice()),
                (Wire::Live(_), None) => &[],
                (Wire::Mock(mock), _) => mock.thread.items.as_slice(),
            };
            said_of(items, sent.as_ref().map(|(words, card)| (words.as_str(), card.as_ref().map(|(about, title)| (about.as_str(), title.as_str())))), raised)
        };
        if said == self.talk_said {
            return;
        }
        let kept = self.talk_said.iter().zip(&said).take_while(|(old, new)| old == new).count();
        self.talk_list.splice(kept..self.talk_said.len(), said.len() - kept);
        self.talk_said = said;
    }

    /// The talk, between the card and the box: a list that follows its tail unless he scrolled up. Its overflow is measured
    /// as it is drawn, and folds the card in front from then on.
    pub(super) fn talk(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (long, tall, cut, state) = (self.talk_long.clone(), self.talk_tall.clone(), self.talk_cut.clone(), self.talk_list.clone());
        let watch = canvas(
            move |_, window, _| {
                // At its tail, the part of a row its top edge cuts, which stands blank so the talk starts on a whole row.
                let top = state.logical_scroll_top();
                let part = match state.is_scrolled_to_end() {
                    Some(true) if top.offset_in_item > gpui::px(0.) => state.bounds_for_item(top.item_ix).map_or(gpui::px(0.), |row| (row.bottom() - state.viewport_bounds().top()).max(gpui::px(0.))),
                    _ => gpui::px(0.),
                };
                if (part - cut.get()).abs() > gpui::px(0.5) {
                    cut.set(part);
                    window.refresh();
                }
                if !long.get() && state.max_offset_for_scrollbar().y > gpui::px(0.) {
                    long.set(true);
                    window.refresh();
                }
                // How tall the talk is whole, so it is given room for its rows up to its least room.
                let whole = state.viewport_bounds().size.height + state.max_offset_for_scrollbar().y;
                if (whole - tall.get()).abs() > gpui::px(0.5) {
                    tall.set(whole);
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_0();
        let room = if self.talk_said.is_empty() { gpui::px(0.) } else { self.talk_tall.get().min(look.length(token::SPACE_12) * 3.).max(look.length(token::SPACE_12)) };
        let rows = list(self.talk_list.clone(), cx.processor(|this, ix: usize, window, cx| this.talk_row(ix, window, cx))).size_full();
        labelled(
            div()
                .id("operator-talk")
                .relative()
                // The talk draws inside its own room, never over the card above it.
                .overflow_hidden()
                .flex_grow(1.)
                .flex_shrink(1.)
                // Room for its newest rows whole, up to three lines' worth, before the card above scrolls any: a short talk is
                // never cut at its top under the card. It gives that room up to the card above, which is drawn whole.
                .flex_basis(room)
                .min_h_0()
                .child(rows)
                .child(div().absolute().top_0().left_0().w_full().h(self.talk_cut.get()).bg(look.color(token::BG_RAISED)))
                .child(watch),
            Role::Log,
            "Talk with the Operator",
        )
        .into_any_element()
    }

    fn talk_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look().clone();
        let row = match self.talk_said.get(ix).cloned() {
            Some(Said::About { key, title }) => self.about_row(&key, &title, &look),
            Some(Said::You { id, at, words }) => self.his_words(&id, &words, at.as_deref(), &look, cx),
            Some(Said::Operator { id, at, text }) => self.its_words(&id, &at, &text, &look, cx),
            Some(Said::Raised(id)) => match self.items().iter().find(|turn| turn.id == id).cloned() {
                Some(turn) => self.card(&turn, true, window, &look, cx),
                None => gpui::Empty.into_any_element(),
            },
            None => gpui::Empty.into_any_element(),
        };
        div().pb(look.length(token::SPACE_3)).child(row).into_any_element()
    }

    /// "About: <card title>", where his words turn to a card: a hairline either side of it.
    fn about_row(&self, key: &str, title: &SharedString, look: &Look) -> AnyElement {
        let said = format!("About: {title}");
        let rule = || div().flex_1().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::OPERATOR_EDGE));
        labelled(
            div()
                .id(SharedString::from(key.to_string()))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::OPERATOR))
                .child(rule())
                .child(div().flex_none().max_w(gpui::relative(0.7)).truncate().child(said.clone()))
                .child(rule()),
            Role::Splitter,
            said,
        )
        .into_any_element()
    }

    /// His words, at the right in the bubble a Job's thread draws them in.
    fn his_words(&self, id: &str, words: &SharedString, at: Option<&str>, look: &Look, cx: &Context<Self>) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("talk-you:{id}")))
                .flex()
                .flex_col()
                .items_end()
                .gap(look.length(token::SPACE_1))
                .child(
                    div()
                        .max_w(gpui::relative(0.8))
                        .child(crate::job_view::rows::bubble(look).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(words.clone())),
                )
                .children(at.map(|at| div().px(look.length(token::SPACE_2)).child(self.stamp(at, None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)))),
            Role::ListItem,
            format!("You: {words}"),
        )
        .into_any_element()
    }

    /// The Operator's words: its name and the time over them, as prose with no bubble, as a Job's thread draws an agent's.
    fn its_words(&self, id: &str, at: &str, text: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("talk-agent:{id}")))
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
                        .child(self.stamp(at, None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)),
                )
                .child(
                    TextView::markdown(gpui::ElementId::Name(format!("talk:{id}").into()), text.clone())
                        .w_full()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_DEFAULT)),
                ),
            Role::ListItem,
            format!("Operator: {text}"),
        )
        .into_any_element()
    }

    /// The new-card strip: a card that would have taken the front while the card was folded or the talk in use waits here,
    /// with how many came, the newest's title, and Show. It glows once as the newest arrives, then holds.
    pub(super) fn strip(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let newest = self.fresh.last()?;
        let turn = self.items().iter().find(|turn| &turn.id == newest)?;
        let title = super::face::card_title(turn);
        let you = look.color(token::YOU);
        let count = self.fresh.len();
        let said = if count == 1 { "A new card".to_string() } else { format!("{count} new cards") };
        let show = press("strip-show", "Show".into(), None, Some(crate::keymap::shown("send_message").into()), Weight::Plain, look).tooltip(tip("Bring the newest card in front"));
        let band = div()
            .id("new-card-strip")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .pl(look.length(token::SPACE_4))
            .pr(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(you)
            .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
            .child(labelled(
                div().id("strip-bell").flex_none().flex().items_center().gap(look.length(token::SPACE_1)).tooltip(tip(said.clone())).child(glyph("bell", look.length(token::SPACE_4), you)).child(
                    div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(you).child(count.to_string()),
                ),
                Role::Status,
                said,
            ))
            .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title.clone()))
            .child(labelled(pressed(show, cx, |this, window, cx| this.show_fresh(window, cx)), Role::Button, "Show"));
        let glow = look.length(token::SPACE_3);
        let band = band.with_animation(SharedString::from(format!("strip-glow:{newest}")), Animation::new(look.duration(token::DURATION_PULSE)), move |band, at| {
            band.shadow(vec![BoxShadow { color: you.opacity(0.6 * (1. - at)), offset: gpui::point(gpui::px(0.), gpui::px(0.)), blur_radius: glow, spread_radius: glow * (0.25 * (1. - at)), inset: false }])
        });
        Some(labelled(div().id("new-card-strip-region").flex_none().child(band), Role::Region, format!("New card: {title}")).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, from: &str, text: &str) -> ThreadItem {
        let wire = serde_json::json!({ "kind": "row", "row": { "kind": "message", "id": id, "at": "2026-11-11T13:09:00-05:00", "from": { "kind": from }, "text": text } });
        ipc::decode::<ThreadItem>("ThreadItem", wire.to_string().as_bytes()).expect("a row decodes")
    }

    #[test]
    fn a_tagged_send_reads_as_its_card_and_his_words() {
        assert_eq!(tag_of("[About card turn:7 \"Main is red\"]\nWhy?"), Some(("turn:7", "Main is red", "Why?")));
        assert_eq!(tag_of("Why?"), None);
    }

    #[test]
    fn an_about_row_stands_where_his_words_turn_to_another_card_and_replies_follow_in_the_same_talk() {
        let items = vec![
            row("1", "you", "Is anything stuck?"),
            row("2", "agent", "No."),
            row("3", "you", "[About card turn:7 \"Main is red\"]\nWhy?"),
            row("4", "agent", "A flaky test."),
            row("5", "you", "[About card turn:7 \"Main is red\"]\nRetry it"),
            row("6", "you", "[About card turn:8 \"Rate limits\"]\nAnd this?"),
        ];
        let said = said_of(&items, None, Vec::new());
        let abouts: Vec<String> = said.iter().filter_map(|one| if let Said::About { title, .. } = one { Some(title.to_string()) } else { None }).collect();
        assert_eq!(abouts, ["Main is red", "Rate limits"], "one row where the talk turns to each card, none for more words on the same card");
        assert!(matches!(&said[3], Said::You { words, .. } if words.as_ref() == "Why?"), "his words are drawn without the tag: {said:?}");
        assert!(matches!(&said[4], Said::Operator { .. }));
    }

    #[test]
    fn a_send_not_echoed_yet_is_his_and_once_echoed_is_drawn_once() {
        let items = vec![row("1", "you", "[About card turn:7 \"Main is red\"]\nWhy?")];
        let said = said_of(&items, Some(("Why?", Some(("turn:7", "Main is red")))), Vec::new());
        assert_eq!(said.iter().filter(|one| matches!(one, Said::You { .. })).count(), 1);
        let said = said_of(&[], Some(("Why?", Some(("turn:7", "Main is red")))), vec!["turn:9".into()]);
        assert!(matches!(&said[..], [Said::About { .. }, Said::You { at: None, .. }, Said::Raised(_)]), "{said:?}");
    }

    #[test]
    fn words_about_no_card_follow_a_card_talk_with_no_about_row() {
        let items = vec![row("1", "you", "[About card turn:7 \"Main is red\"]\nWhy?"), row("2", "agent", "A flaky test.")];
        let said = said_of(&items, Some(("Which Jobs are running?", None)), Vec::new());
        assert!(matches!(&said[..], [Said::About { .. }, Said::You { .. }, Said::Operator { .. }, Said::You { at: None, .. }]), "{said:?}");
    }

    #[test]
    fn fleets_notices_to_the_operator_are_not_his_words() {
        let wire = serde_json::json!({ "kind": "row", "row": { "kind": "message", "id": "n", "at": "2026-11-11T13:09:00-05:00", "from": { "kind": "you" }, "text": "A Job stopped", "notice": "operator" } });
        let notice = ipc::decode::<ThreadItem>("ThreadItem", wire.to_string().as_bytes()).expect("a notice decodes");
        assert!(said_of(&[notice], None, Vec::new()).is_empty());
    }
}
