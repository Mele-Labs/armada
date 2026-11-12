//! The question deck (`job-view.md`, B4.4): one question at a time, its options numbered, Other
//! last, and Answer, Next and Skip under them.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use gpui_component::input::TextareaState;
use ipc::{
    AnswerSessionAsk, AnswerWaiting, AskedQuestion, CallAnswer, DismissWaiting, QuestionAnswer, SendSessionMessage, SendSessionMessageBody,
    SessionId,
};

use super::acts::{button, kbd, refusal, Mode, Tone};
use std::rc::Rc;

use super::dock::Thing;
use super::keys::{Card, Run};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

fn encoded<T: serde::Serialize>(body: &T) -> Result<String, String> {
    ipc::encode(body).map_err(|why| format!("{why:?}"))
}

/// Where a question deck stands: the front question, what is picked for each, and whether Other is on.
#[derive(Default, Clone)]
pub(super) struct Deck {
    pub front: usize,
    pub picked: Vec<Vec<usize>>,
    pub other: Vec<bool>,
    pub sending: bool,
}

/// What a deck answers.
#[derive(Clone)]
pub(super) enum Source {
    Call(String),
    Lapsed(String),
    Waiting(String),
}

fn name_of(question: &AskedQuestion) -> &str {
    if question.header.trim().is_empty() { &question.question } else { &question.header }
}

fn question_title(question: &AskedQuestion) -> String {
    let header = question.header.trim();
    if header.is_empty() { question.question.clone() } else { format!("{header}: {}", question.question) }
}

impl JobView {
    fn deck_of(&self, id: &str, key: &str, count: usize) -> Deck {
        let mut deck = self.dock_of(id).and_then(|dock| dock.decks.get(key).cloned()).unwrap_or_default();
        deck.picked.resize(count, Vec::new());
        deck.other.resize(count, false);
        deck
    }

    /// What is chosen for question `at`: the options picked, and the words typed when Other is on.
    fn chosen_for(&self, key: &str, at: usize, question: &AskedQuestion, deck: &Deck, cx: &Context<Self>) -> Vec<String> {
        let mut chosen: Vec<String> = deck.picked[at].iter().filter_map(|n| question.options.get(*n)).map(|option| option.label.clone()).collect();
        if deck.other[at] {
            let words = self.typed_in(&format!("dock:{key}:{at}"), Mode::Note, cx);
            if !words.trim().is_empty() {
                chosen.push(words.trim().to_string());
            }
        }
        chosen
    }

    /// An option or the Other row: a box that is on or off, in the deck's hue.
    #[allow(clippy::too_many_arguments)]
    fn choice_row(&self, id: String, number: usize, label: &str, description: &str, on: bool, multi: bool, chip: Option<&str>, look: &Look) -> gpui::Stateful<gpui::Div> {
        let hue = look.color(token::STEP_WAITING);
        let _ = multi;
        div()
            .id(SharedString::from(id))
            .focusable()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .cursor_pointer()
            .bg(if on { hue.opacity(0.14) } else { look.color(token::FG_DEFAULT).opacity(0.04) })
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { hue } else { look.color(token::FG_DEFAULT).opacity(0.07) })
            .hover(|style| style.border_color(hue.opacity(0.6)))
            .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_SUBTLE)).child(format!("{number}")))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(div().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(label.to_string())))
                    .children((!description.is_empty()).then(|| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(description.to_string())))),
            )
            .children(chip.map(|text| kbd(look, text)))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn deck(&self, id: &SharedString, key: &str, questions: &[AskedQuestion], can_skip: bool, no_other: bool, source: Source, look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STEP_WAITING);
        let deck = self.deck_of(id, key, questions.len());
        let front = deck.front.min(questions.len().saturating_sub(1));
        let Some(question) = questions.get(front) else { return div().into_any_element() };
        let total = questions.len();
        let last = front + 1 == total;
        let target = self.is_target(&format!("question:{key}"));
        let answers: Vec<(String, Vec<String>)> = questions.iter().enumerate().map(|(at, q)| (q.question.clone(), self.chosen_for(key, at, q, &deck, cx))).collect();
        let all_chosen = answers.iter().all(|(_, chosen)| !chosen.is_empty());
        let here_chosen = !answers[front].1.is_empty();
        let multi = question.multi_select;
        let role = if multi { Role::CheckBox } else { Role::RadioButton };
        let options = question.options.iter().enumerate().map(|(n, option)| {
            let deck_key = key.to_string();
            labelled(
                self.choice_row(format!("answer:{key}:{front}:{n}"), n + 1, &option.label, &option.description, deck.picked[front].contains(&n), multi, None, look)
                    .on_click(cx.listener(move |this, _, window, cx| this.deck_pick(&deck_key, front, total, n, multi, window, cx))),
                role,
                option.label.clone(),
            )
            .into_any_element()
        });
        let other_on = deck.other[front];
        let other = (!no_other).then(|| {
            let deck_key = key.to_string();
            labelled(
                self.choice_row(format!("answer:{key}:{front}:other"), question.options.len() + 1, "Other", "", other_on, multi, target.then_some("\u{2325}E"), look)
                    .on_click(cx.listener(move |this, _, window, cx| this.deck_other(&deck_key, front, total, multi, window, cx))),
                role,
                "Other",
            )
            .into_any_element()
        });
        let steps = (total > 1).then(|| {
            div().flex().items_center().gap(look.length(token::SPACE_1)).ml_auto().children(questions.iter().enumerate().map(|(at, q)| {
                let done = !answers[at].1.is_empty();
                let deck_key = key.to_string();
                let name = if done { format!("{}: {}", name_of(q), answers[at].1.join(", ")) } else { name_of(q).to_string() };
                labelled(
                    div()
                        .id(SharedString::from(format!("step:{key}:{at}")))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::SPACE_6))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if at == front { hue } else { look.color(token::BORDER_DEFAULT) })
                        .bg(if done { hue } else { gpui::transparent_black() })
                        .text_color(look.color(if done { token::BG_SUNKEN } else { token::FG_MUTED }))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .cursor_pointer()
                        .tooltip(tip(name.clone()))
                        .child(if done { glyph(look, "check", Size::Small, look.color(token::BG_SUNKEN)).into_any_element() } else { div().child(format!("{}", at + 1)).into_any_element() })
                        .on_click(cx.listener(move |this, _, _, cx| this.deck_front(&deck_key, total, at, cx))),
                    Role::Button,
                    name,
                )
            }))
        });
        let go = {
            let (go_id, go_key, go_source, go_answers) = (id.clone(), key.to_string(), source.clone(), answers.clone());
            let blocked = deck.sending || !all_chosen;
            labelled(
                button(look, SharedString::from(format!("answer-go:{key}")), "Answer", Tone::Primary, false, target.then_some("\u{2325}\u{21b5}"), !blocked).when(!blocked, |b| {
                    b.on_click(cx.listener(move |this, _, window, cx| this.deck_answer(&go_id, &go_key, &go_source, go_answers.clone(), window, cx)))
                }),
                Role::Button,
                "Answer",
            )
        };
        let next = {
            let deck_key = key.to_string();
            labelled(
                button(look, SharedString::from(format!("answer-next:{key}")), "Next", Tone::Secondary, false, None, here_chosen)
                    .when(here_chosen, |b| b.on_click(cx.listener(move |this, _, _, cx| this.deck_front(&deck_key, total, front + 1, cx)))),
                Role::Button,
                "Next",
            )
        };
        let skip = can_skip.then(|| {
            let (skip_id, skip_source) = (id.clone(), source.clone());
            labelled(
                button(look, SharedString::from(format!("answer-skip:{key}")), "Skip", Tone::Ghost, false, target.then_some("\u{2325}X"), true)
                    .on_click(cx.listener(move |this, _, window, cx| this.deck_skip(&skip_id, &skip_source, window, cx))),
                Role::Button,
                "Skip",
            )
        });
        if let Some(page) = self.pages.get(id) {
            let (answer_id, answer_key, answer_source, answer_answers) = (id.clone(), key.to_string(), source.clone(), answers.clone());
            let (other_key, skip_id, skip_source) = (key.to_string(), id.clone(), source.clone());
            let sending = deck.sending;
            page.keys.set(
                &format!("question:{key}"),
                Some(Card {
                    at: usize::MAX / 2,
                    kind: "Question".into(),
                    title: question_title(question),
                    yes: (last && all_chosen && !sending).then(|| -> (String, Run) {
                        ("Answer".into(), Rc::new(move |this, window, cx| this.deck_answer(&answer_id, &answer_key, &answer_source, answer_answers.clone(), window, cx)))
                    }),
                    change: (!no_other).then(|| -> (String, Run) { ("Other".into(), Rc::new(move |this, window, cx| this.deck_other(&other_key, front, total, multi, window, cx))) }),
                    send_back: None,
                    no: can_skip.then(|| -> (String, Run) { ("Skip".into(), Rc::new(move |this, window, cx| this.deck_skip(&skip_id, &skip_source, window, cx))) }),
                }),
            );
        }
        let footer = div().flex().items_center().gap(look.length(token::SPACE_2)).child(if last { go } else { next }).children(skip).children(steps);
        let typed_box = other_on.then(|| self.box_of(&format!("dock:{key}:{front}"), Mode::Note, "Other", look, cx)).flatten();
        let said = self.dock_of(id).and_then(|dock| dock.said.get(&format!("question:{key}")).cloned());
        labelled(
            div()
                .id(SharedString::from(format!("deck:{key}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .child(div().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(question.question.clone())))
                .child(labelled(
                    div().id(SharedString::from(format!("options:{key}"))).flex().flex_col().gap(look.length(token::SPACE_2)).children(options).children(other),
                    Role::Group,
                    "Options",
                ))
                .children(typed_box)
                .children(said.map(|why| refusal(look, key, why)))
                .child(labelled(footer.id(SharedString::from(format!("answers:{key}"))), Role::Toolbar, "Answers")),
            Role::Group,
            question.question.clone(),
        )
        .into_any_element()
    }

    pub(super) fn deck_pick(&mut self, key: &str, front: usize, total: usize, n: usize, multi: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let mut deck = self.deck_of(&id, key, total);
        if multi {
            match deck.picked[front].iter().position(|one| *one == n) {
                Some(at) => {
                    deck.picked[front].remove(at);
                }
                None => deck.picked[front].push(n),
            }
        } else {
            deck.picked[front] = vec![n];
            deck.other[front] = false;
            if front + 1 < total {
                deck.front = front + 1;
            }
        }
        if let Some(dock) = self.dock_mut() {
            dock.decks.insert(key.to_string(), deck);
        }
        self.aim(&format!("question:{key}"));
        window.focus(&self.dock_focus, cx);
        cx.notify();
    }

    pub(super) fn deck_other(&mut self, key: &str, front: usize, total: usize, multi: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let mut deck = self.deck_of(&id, key, total);
        let on = if multi { !deck.other[front] } else { true };
        deck.other[front] = on;
        if !multi {
            deck.picked[front].clear();
        }
        deck.front = front;
        if let Some(dock) = self.dock_mut() {
            dock.decks.insert(key.to_string(), deck);
        }
        self.aim(&format!("question:{key}"));
        if on {
            let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 4));
            area.update(cx, |area, cx| area.focus(window, cx));
            if let Some(page) = self.pages.get_mut(&id) {
                page.cards.boxes.entry(format!("dock:{key}:{front}:{:?}", Mode::Note)).or_insert(area);
            }
        }
        cx.notify();
    }

    fn deck_front(&mut self, key: &str, total: usize, to: usize, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let mut deck = self.deck_of(&id, key, total);
        deck.front = to.min(total.saturating_sub(1));
        if let Some(dock) = self.dock_mut() {
            dock.decks.insert(key.to_string(), deck);
        }
        cx.notify();
    }

    /// `1` to `9` pick that option of the front question, or Other one past the last.
    pub(super) fn pick_digit(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let found = {
            let (things, _) = self.things(&id, cx);
            let detail = self.dock_of(&id).and_then(|dock| dock.detail.clone());
            things.iter().find_map(|thing| match thing {
                Thing::Ask(ask) if !ask.call.questions.is_empty() => Some((ask.call.call.clone(), ask.call.questions.clone(), false)),
                Thing::Lapsed(lapsed) => Some((lapsed.id.clone(), lapsed.questions.clone(), false)),
                Thing::Item(item) if !item.options.is_empty() && detail.as_deref() == Some(&format!("item:{}", item.id)) => Some((
                    item.id.clone(),
                    vec![AskedQuestion {
                        question: item.text.clone(),
                        header: String::new(),
                        multi_select: false,
                        options: item.options.iter().map(|o| ipc::AskedChoice { label: o.label.clone(), description: String::new(), preview: String::new() }).collect(),
                    }],
                    true,
                )),
                _ => None,
            })
        };
        let Some((key, questions, no_other)) = found else { return };
        let deck = self.deck_of(&id, &key, questions.len());
        let front = deck.front.min(questions.len().saturating_sub(1));
        let Some(question) = questions.get(front) else { return };
        if n <= question.options.len() {
            self.deck_pick(&key, front, questions.len(), n - 1, question.multi_select, window, cx);
        } else if n == question.options.len() + 1 && !no_other {
            self.deck_other(&key, front, questions.len(), question.multi_select, window, cx);
        }
    }

    fn waiting_choice(&self, id: &str, item: &str, label: &str, cx: &Context<Self>) -> Option<u32> {
        let item = self.record(id, cx)?.waiting_for.iter().find(|one| one.id == item)?;
        item.options.iter().position(|option| option.label == label).map(|at| at as u32)
    }

    pub(super) fn deck_answer(&mut self, id: &SharedString, key: &str, source: &Source, answers: Vec<(String, Vec<String>)>, window: &mut Window, cx: &mut Context<Self>) {
        let session_id = SessionId::carried(id.to_string());
        if let Some(dock) = self.dock_mut() {
            dock.decks.entry(key.to_string()).or_default().sending = true;
        }
        let (route, body): (&'static str, Result<String, String>) = match source {
            Source::Call(call) => (
                "/sessions/ask/answer",
                encoded(&AnswerSessionAsk {
                    session_id,
                    call: call.clone(),
                    answer: CallAnswer::AllowOnce,
                    note: None,
                    answers: answers.iter().map(|(question, chosen)| QuestionAnswer { question: question.clone(), chosen: chosen.clone() }).collect(),
                    folder: None,
                    kind: None,
                }),
            ),
            Source::Lapsed(_) => {
                let text = answers.iter().map(|(question, chosen)| format!("Answering your earlier question \"{question}\": {}", chosen.join(", "))).collect::<Vec<_>>().join("\n");
                ("/sessions/message", encoded(&SendSessionMessageBody { message: SendSessionMessage { session_id, text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() }, aim: None }))
            }
            Source::Waiting(item) => {
                let choice = answers.first().and_then(|(_, chosen)| chosen.first()).and_then(|label| self.waiting_choice(id, item, label, cx));
                ("/sessions/waiting/answer", encoded(&AnswerWaiting { session_id, item_id: item.clone(), choice, text: None, mode: None, fixed: None }))
            }
        };
        if let Source::Lapsed(lapse) = source {
            if let Some(dock) = self.dock_mut() {
                dock.hidden.insert(lapse.clone());
                dock.detail = None;
            }
        }
        let said_key = format!("question:{key}");
        let release = key.to_string();
        self.post_with(
            id.clone(),
            route,
            body,
            move |this, why, cx| {
                if let Some(dock) = this.dock_mut() {
                    if let Some(deck) = dock.decks.get_mut(&release) {
                        deck.sending = false;
                    }
                    dock.said.insert(said_key, why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    pub(super) fn deck_skip(&mut self, id: &SharedString, source: &Source, window: &mut Window, cx: &mut Context<Self>) {
        match source {
            Source::Call(call) => {
                let body = encoded(&AnswerSessionAsk { session_id: SessionId::carried(id.to_string()), call: call.clone(), answer: CallAnswer::Refuse, note: None, answers: Vec::new(), folder: None, kind: None });
                self.post_with(id.clone(), "/sessions/ask/answer", body, |_, _, _| {}, window, cx);
            }
            Source::Lapsed(lapse) => {
                if let Some(dock) = self.dock_mut() {
                    dock.hidden.insert(lapse.clone());
                    dock.detail = None;
                }
            }
            Source::Waiting(item) => {
                let body = encoded(&DismissWaiting { session_id: SessionId::carried(id.to_string()), item_id: item.clone(), reason: None, misread: false });
                self.post_with(id.clone(), "/sessions/waiting/dismiss", body, |_, _, _| {}, window, cx);
            }
        }
        cx.notify();
    }
}
