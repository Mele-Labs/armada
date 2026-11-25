//! A question's answers on the thread's first card (`first_card`): an option to press for each choice, then Answer in words
//! and Dismiss. A press on an option answers it; Answer in words opens a box that takes formatting, pictures and tags. The
//! sheet answers the same item, and either place's answer takes it out of the other.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{AnswerWaiting, DismissWaiting, SessionId, WaitingItem};

use super::acts::{refusal, Mode};
use super::keys::{Card, Run};
use super::rows::tip;
use super::sketch_card::{act, Kind};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

const ANSWER: &str = "/sessions/waiting/answer";
const DISMISS: &str = "/sessions/waiting/dismiss";

/// The key a question's card takes, for its box, its keys and a refusal: `asked:<item>`.
pub(super) fn card_key(item: &str) -> String {
    format!("asked:{item}")
}

impl JobView {
    /// The answers to question `waiting` titled `title`, on the first card at list place `ix`: its options, then Answer in
    /// words and Dismiss. Answer in words puts the box and Cancel and Send answer in their place.
    pub(super) fn question_body(&self, ix: usize, waiting: &WaitingItem, title: &str, page: &Page, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let item = waiting.id.clone();
        let key = card_key(&item);
        let noting = page.cards.modes.get(&key) == Some(&Mode::Note);
        let options: Vec<AnyElement> = waiting
            .options
            .iter()
            .enumerate()
            .map(|(at, option)| {
                let press = item.clone();
                let row = div()
                    .id(SharedString::from(format!("asked-option:{item}:{at}")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1) / 2.)
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .cursor_pointer()
                    .hover(|style| style.border_color(look.color(token::BORDER_STRONG)).bg(look.color(token::BG_HOVER)))
                    .tab_index(0)
                    .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(option.label.clone())),
                    )
                    .children(option.description.clone().filter(|said| !said.trim().is_empty()).map(|said| {
                        div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said))
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.answer_asked(ix, &press, Some(at), window, cx);
                    }));
                labelled(row, Role::Button, option.label.clone()).into_any_element()
            })
            .collect();
        let options = (!options.is_empty() && !noting).then(|| {
            labelled(div().id(SharedString::from(format!("asked-options:{item}"))).flex().flex_col().gap(look.length(token::SPACE_2)).children(options), Role::Group, "Options")
        });
        let words: Run = {
            let item = item.clone();
            Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.open_note(ix, &card_key(&item), "Answer in words", window, cx))
        };
        let send: Run = {
            let item = item.clone();
            Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.answer_asked(ix, &item, None, window, cx))
        };
        let away: Run = {
            let item = item.clone();
            Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.dismiss_asked(ix, &item, window, cx))
        };
        let cancel: Run = {
            let key = key.clone();
            Rc::new(move |this: &mut JobView, _: &mut Window, cx: &mut Context<JobView>| this.close_box_kept(ix, &key, cx))
        };
        page.keys.set(
            &key,
            Some(Card { at: ix, kind: "Question".into(), title: title.to_string(), yes: None, change: Some(("Answer in words".into(), words.clone())), send_back: None, no: Some(("Dismiss".into(), away.clone())) }),
        );
        let foot = if noting {
            // `⌘Enter` in the box sends the answer, as Send answer does.
            page.keys.commit(&key, Some(send.clone()));
            let blank = self.typed_in(&key, Mode::Note, cx).trim().is_empty();
            let esc = cancel.clone();
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .capture_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    // A list open over the words is closed by Esc first; the box itself shuts on the next.
                    if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() && !this.note_offering(cx) {
                        cx.stop_propagation();
                        esc(this, window, cx);
                    }
                }))
                .children(self.box_of(&key, Mode::Note, "Answer in words", look, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(look.length(token::SPACE_2))
                        .child(labelled(
                            act(look, format!("asked-cancel:{item}"), "Cancel", Kind::Ghost, "Esc", true).on_click(cx.listener(move |this, _, window, cx| cancel(this, window, cx))),
                            Role::Button,
                            "Cancel",
                        ))
                        .child(labelled(
                            act(look, format!("asked-send:{item}"), "Send answer", Kind::Primary, "\u{2318}\u{21b5}", !blank).when(!blank, |button| button.on_click(cx.listener(move |this, _, window, cx| send(this, window, cx)))),
                            Role::Button,
                            "Send answer",
                        )),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    act(look, format!("asked-words:{item}"), "Answer in words", Kind::Secondary, "", true)
                        .tooltip(tip("Answer with your own words"))
                        .on_click(cx.listener(move |this, _, window, cx| words(this, window, cx))),
                    Role::Button,
                    "Answer in words",
                ))
                .child(div().flex_1())
                .child(labelled(
                    act(look, format!("asked-dismiss:{item}"), "Dismiss", Kind::Ghost, "", true)
                        .tooltip(tip("Put it away with no answer"))
                        .on_click(cx.listener(move |this, _, window, cx| away(this, window, cx))),
                    Role::Button,
                    "Dismiss",
                ))
                .into_any_element()
        };
        let said = page.cards.said.get(&key).map(|why| refusal(look, &key, why.clone()));
        options
            .map(IntoElement::into_any_element)
            .into_iter()
            .chain(said)
            .chain([super::acts::gated(div().id(SharedString::from(format!("answers:{key}"))).child(foot), self.off_shown_to(ANSWER, cx)).into_any_element()])
            .collect()
    }

    /// Question `item` answered on its thread card at list place `ix`: by its option `choice`, else in the words of its box.
    pub(super) fn answer_asked(&mut self, ix: usize, item: &str, choice: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let key = card_key(item);
        let words = choice.is_none().then(|| self.typed_in(&key, Mode::Note, cx).trim().to_string()).filter(|words| !words.is_empty());
        if choice.is_none() && words.is_none() {
            return;
        }
        let extras = if words.is_some() { self.extras_of(&key, Mode::Note, cx) } else { Default::default() };
        let body = ipc::encode(&AnswerWaiting {
            session_id: SessionId::carried(job.to_string()),
            item_id: item.to_string(),
            choice: choice.and_then(|at| u32::try_from(at).ok()),
            text: words,
            mode: None,
            fixed: None,
            extras,
        });
        self.asked_settles(ix, item, ANSWER, body.map_err(|why| format!("{why:?}")), window, cx);
    }

    /// Question `item` put away from its thread card with no answer.
    pub(super) fn dismiss_asked(&mut self, ix: usize, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let body = ipc::encode(&DismissWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string(), reason: None, misread: false });
        self.asked_settles(ix, item, DISMISS, body.map_err(|why| format!("{why:?}")), window, cx);
    }

    /// The question goes from the thread, Needs you and the sheet in the frame of the press; a refusal brings it back with
    /// Fleet's sentence on its card.
    fn asked_settles(&mut self, ix: usize, item: &str, route: &'static str, body: Result<String, String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let key = card_key(item);
        self.close_box(ix, &key, cx);
        if let Some(page) = self.pages.get_mut(&job) {
            page.dock.hidden.insert(item.to_string());
            page.dock.said.remove(&format!("item:{item}"));
            page.cards.said.remove(&key);
        }
        self.want_needs(&job, cx);
        self.sync(&job, cx);
        // Read from inside the card, the questions left stand at the top rather than scrolled past.
        if let Some(page) = self.pages.get_mut(&job) {
            let (entry, top) = (page.entry(ix), page.list.logical_scroll_top());
            if !page.list.is_following_tail() && top.item_ix == entry {
                page.list.scroll_to(gpui::ListOffset { item_ix: entry, offset_in_item: gpui::px(0.) });
            }
        }
        let (back_job, back_item) = (job.clone(), item.to_string());
        self.post_with(
            job,
            route,
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back_job) {
                    page.dock.hidden.remove(&back_item);
                    page.cards.said.insert(card_key(&back_item), why);
                }
                this.want_needs(&back_job, cx);
                this.sync(&back_job, cx);
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }
}
