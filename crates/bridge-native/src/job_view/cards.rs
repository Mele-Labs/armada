//! The cards that put something to the person in the thread: a sketch, a document and a draft, and
//! the quiet file, window and judged rows (`job-view.md`, B1 and B3).
//!
//! What Fleet serves is drawn; what the person answers goes back through `answer_sketch`,
//! `answer_document`, `answer_waiting` and `dismiss_waiting`. The scene a sketch holds draws in a
//! scene renderer this crate does not have yet, so a sketch card draws its band, its frame and its
//! answers, and a sunken pane where the scene goes.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, MouseButton, Role, SharedString};
use ipc::{
    AnswerDocument, AnswerSketch, AnswerWaiting, CardSettled, DismissWaiting, DocumentHow, DraftHow, DraftSettled, Instant,
    SessionId, SketchHow, ThreadDocument, ThreadSketch,
};

use std::rc::Rc;

use super::acts::{button, refusal, Answer, Mode, Tone};
use super::menus::teach_entries;
use super::pop::NUMBERED;
use super::rows::{glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

/// How the thread reads a card that was answered.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Said {
    Approved,
    SentBack,
}

fn encoded<T: serde::Serialize>(body: &T) -> Result<String, String> {
    ipc::encode(body).map_err(|why| format!("{why:?}"))
}

/// The card's frame in the spacious take: a quiet fill and a hairline edge, reaching a little past
/// the words (`--spacious-bleed`).
fn frame(look: &Look, bleed: bool) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .overflow_hidden()
        .rounded(look.length(token::RADIUS_LG))
        .bg(look.color(token::FG_DEFAULT).opacity(0.04))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::FG_DEFAULT).opacity(0.07))
        .when(bleed, |frame| frame.mx(-(look.length(token::SPACE_4) + look.length(token::SPACE_1))))
}

/// What a settled card says, and its mark.
fn settled_line(look: &Look, key: &str, said: Said, words: Option<&str>) -> AnyElement {
    let (word, icon, ink) = match said {
        Said::Approved => ("Approved", "badge-check", token::STEP_ADVANCED),
        Said::SentBack => ("Sent back", "reply", token::FG_MUTED),
    };
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .children((said == Said::SentBack).then_some(words).flatten().map(|words| quote(look, words)))
        .child(labelled(
            div()
                .id(SharedString::from(format!("settled:{key}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().id(SharedString::from(format!("settled-mark:{key}"))).child(glyph(look, icon, Size::Mark, look.color(ink))).tooltip(tip(word)))
                .child(word),
            Role::Status,
            word,
        ))
        .into_any_element()
}

/// The note a card was sent back with: a quote with the person's colour at its edge.
fn quote(look: &Look, words: &str) -> AnyElement {
    div()
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .border_l(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::YOU_EDGE))
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .child(SharedString::from(words.to_string()))
        .into_any_element()
}

/// The sketch or document an approval is for.
pub(super) enum Subject {
    Sketch { sketch: String, drawn: Option<String> },
    Document { page: String, revision: u32 },
}

impl JobView {
    /// Approve, Send back and Withdraw, or how it was answered (`ApprovalAnswers`, B1.3).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn approval(
        &self,
        ix: usize,
        key: &str,
        subject: Subject,
        open: bool,
        withdraw: bool,
        settled: Option<(Said, Option<String>)>,
        inset: bool,
        page: &Page,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let noting = open && page.cards.modes.get(key) == Some(&Mode::Note);
        let note_blank = self.typed_in(key, Mode::Note, cx).trim().is_empty();
        let subject = std::rc::Rc::new(subject);
        let job = self.shown.clone().unwrap_or_default();
        let session_id = SessionId::carried(job.to_string());
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2)).when(inset, |c| c.px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_4)).pt(look.length(token::SPACE_2)));
        if noting {
            column = column.children(self.box_of(key, Mode::Note, "Note", look, cx));
        }
        if let Some((said, words)) = &settled {
            column = column.child(settled_line(look, key, *said, words.as_deref()));
        }
        if let Some(why) = page.cards.said.get(key) {
            column = column.child(refusal(look, key, why.clone()));
        }
        if open {
            let answer = {
                let (key, subject, session_id) = (key.to_string(), subject.clone(), session_id.clone());
                move |how: SketchHow, words: Option<String>| -> (&'static str, Result<String, String>) {
                    match &*subject {
                        Subject::Sketch { sketch, drawn } => (
                            "/sessions/sketch/answer",
                            encoded(&AnswerSketch { session_id: session_id.clone(), sketch: sketch.clone(), drawn: drawn.clone().unwrap_or_default(), how, words }),
                        ),
                        Subject::Document { page, revision } => {
                            let _ = &key;
                            (
                                "/sessions/document/answer",
                                encoded(&AnswerDocument {
                                    session_id: session_id.clone(),
                                    document: format!("document:{page}"),
                                    revision: revision.to_string(),
                                    how: if how == SketchHow::Approved { DocumentHow::Approved } else { DocumentHow::SentBack },
                                    words,
                                }),
                            )
                        }
                    }
                }
            };
            let footer = div().flex().items_center().gap(look.length(token::SPACE_2));
            let footer = if noting {
                let (send_key, cancel_key) = (key.to_string(), key.to_string());
                let answer = answer.clone();
                footer
                    .child(labelled(
                        button(look, SharedString::from(format!("send-back:{key}")), "Send back", Tone::Primary, true, None, !note_blank).when(!note_blank, |b| {
                            b.on_click(cx.listener(move |this, _, window, cx| {
                                let note = this.typed_in(&send_key, Mode::Note, cx).trim().to_string();
                                let (route, body) = answer(SketchHow::SentBack, Some(note.clone()));
                                this.settle(ix, send_key.clone(), Answer::SentBack(note), route, body, window, cx);
                            }))
                        }),
                        Role::Button,
                        "Send back",
                    ))
                    .child(labelled(
                        button(look, SharedString::from(format!("cancel:{key}")), "Cancel", Tone::Ghost, true, None, true)
                            .on_click(cx.listener(move |this, _, _, cx| this.close_box(ix, &cancel_key, cx))),
                        Role::Button,
                        "Cancel",
                    ))
            } else {
                let (approve_key, note_key, withdraw_key) = (key.to_string(), key.to_string(), key.to_string());
                let title_id = match &*subject {
                    Subject::Sketch { sketch, .. } => sketch.clone(),
                    Subject::Document { page, .. } => format!("document:{page}"),
                };
                let answer = answer.clone();
                let on_target = self.is_target(key);
                footer
                    .child(labelled(
                        button(look, SharedString::from(format!("approve:{key}")), "Approve", Tone::Primary, true, None, true).on_click(cx.listener(
                            move |this, _, window, cx| {
                                let (route, body) = answer(SketchHow::Approved, None);
                                this.settle(ix, approve_key.clone(), Answer::Approved, route, body, window, cx);
                            },
                        )),
                        Role::Button,
                        "Approve",
                    ))
                    .child(labelled(
                        button(look, SharedString::from(format!("note:{key}")), "Send back", Tone::Secondary, true, None, true)
                            .on_click(cx.listener(move |this, _, window, cx| this.open_box(ix, &note_key, Mode::Note, "", window, cx))),
                        Role::Button,
                        "Send back",
                    ))
                    .when(withdraw, |footer| {
                        footer.child(labelled(
                            button(look, SharedString::from(format!("withdraw:{key}")), "Withdraw", Tone::Ghost, true, None, true).on_click(cx.listener(
                                move |this, _, window, cx| {
                                    let session_id = SessionId::carried(this.shown.clone().unwrap_or_default().to_string());
                                    let body = encoded(&DismissWaiting { session_id, item_id: title_id.clone(), reason: None, misread: false });
                                    this.settle(ix, withdraw_key.clone(), Answer::Withdrawn, "/sessions/waiting/dismiss", body, window, cx);
                                },
                            )),
                            Role::Button,
                            "Withdraw",
                        ))
                    })
                    .when(on_target, |footer| footer)
            };
            column = column.child(labelled(footer.id(SharedString::from(format!("answers:{key}"))), Role::Toolbar, "Answers"));
        }
        column.into_any_element()
    }

    /// The Job's sketch put to the person, or shown (B1.2, B1.3).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sketch(&self, ix: usize, card: &ThreadSketch, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let key = card.id.clone();
        let optimistic = page.cards.answers.get(&key);
        let framed = (card.asked && !card.withdrawn) || card.settled.is_some() || optimistic.is_some();
        let settled = settled_of(card.settled.as_ref(), optimistic);
        let open = card.asked && !card.withdrawn && settled.is_none() && !card.replaced;
        let revisions = card.revisions.len();
        let picked = page.cards.revision.get(&key).copied().filter(|at| *at < revisions);
        let shown_earlier = picked.is_some_and(|at| at + 1 < revisions);
        let drawn = self.drawn_of(&card.sketch, cx);
        let band = self.sketch_band(ix, &key, &card.title, card.replaced, card.against.as_deref().filter(|_| !shown_earlier), Some((&card.at, revisions, picked)), look, now, cx);
        let glass = glass(look, card.replaced);
        let body = frame(look, true)
            .child(band)
            .child(glass)
            .when(framed, |frame| {
                frame.child(self.approval(ix, &key, Subject::Sketch { sketch: card.sketch.clone(), drawn }, open && !shown_earlier, true, settled, true, page, look, cx))
            });
        labelled(
            body.id(SharedString::from(format!("sketch:{key}"))).when(card.replaced, |frame| frame.border_dashed()),
            Role::Article,
            format!("Sketch: {}", card.title),
        )
        .into_any_element()
    }

    /// The newest sketch put to the person, pinned after the plan (`SketchPin`, B1.4).
    pub(super) fn sketch_pin(&self, ix: usize, card: &ThreadSketch, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = format!("pin:{}", card.id);
        let optimistic = page.cards.answers.get(&key);
        let settled = settled_of(card.settled.as_ref(), optimistic);
        let folded = self.folded(&key, false);
        let revisions = card.revisions.len();
        let picked = page.cards.revision.get(&key).copied().filter(|at| *at < revisions);
        let shown_earlier = picked.is_some_and(|at| at + 1 < revisions);
        let drawn = self.drawn_of(&card.sketch, cx);
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_color(look.color(token::FG_MUTED))
            .child(glyph(look, "pencil-ruler", Size::Small, look.color(token::FG_MUTED)))
            .child(super::rows::eyebrow(look, &card.title).text_color(look.color(token::FG_MUTED)))
            .child(div().id(SharedString::from(format!("pin-mark:{key}"))).child(glyph(look, "bell-ring", Size::Mark, look.color(token::STEP_WAITING))).tooltip(tip("Sketch waiting for approval")))
            .child(div().flex_1())
            .child(self.revision_button(ix, &key, &card.title, revisions, picked, look, cx))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("fold:{key}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip(if folded { "Unfold" } else { "Fold" }))
                    .child(glyph(look, if folded { "chevron-right" } else { "chevron-down" }, Size::Small, look.color(token::FG_MUTED)))
                    .on_click({
                        let fold_key = key.clone();
                        cx.listener(move |this, _, _, cx| this.fold_card(ix, &fold_key, false, cx))
                    }),
                Role::Button,
                if folded { "Unfold" } else { "Fold" },
            ));
        let open = settled.is_none() && !shown_earlier;
        let aim = key.clone();
        labelled(
            div()
                .id(SharedString::from(format!("sketch-pin:{key}")))
                .mx(-(look.length(token::SPACE_4) + look.length(token::SPACE_1)))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    this.aim(&aim);
                    cx.notify();
                }))
                .child(band)
                .when(!folded, |card| card.child(div().h(look.length(token::SPACE_12) * 6.).rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).overflow_hidden().flex().items_center().justify_center().child(glyph(look, "pencil-ruler", Size::Mark, look.color(token::FG_SUBTLE)))))
                .child(self.approval(ix, &key, Subject::Sketch { sketch: card.sketch.clone(), drawn }, open, true, settled, false, page, look, cx)),
            Role::Article,
            format!("Sketch to approve: {}", card.title),
        )
        .into_any_element()
    }

    /// `detail.drawn` of the ledger sketch, which an answer names back.
    pub(super) fn drawn_of(&self, sketch: &str, cx: &gpui::App) -> Option<String> {
        let job = self.shown.as_ref()?;
        self.record(job, cx)?.attachments.iter().find(|one| one.kind == "sketch" && one.target == sketch)?.detail.get("drawn").cloned()
    }

    /// The band over a sketch: its glyph, title, tags, revision menu and time.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sketch_band(
        &self,
        ix: usize,
        key: &str,
        title: &str,
        replaced: bool,
        against: Option<&str>,
        more: Option<(&Instant, usize, Option<usize>)>,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let tag = |text: String| div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(text);
        let mut band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_1))
            .child(glyph(look, "pencil-ruler", Size::Small, look.color(token::FG_MUTED)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(title.to_string())),
            )
            .when(replaced, |band| band.child(tag("Replaced".into())))
            .children(against.map(|against| tag(format!("Against {against}"))));
        if let Some((at, revisions, picked)) = more {
            if revisions > 0 {
                band = band.child(self.revision_button(ix, key, title, revisions, picked, look, cx));
            }
            band = band.child(self.stamp(at, look, now, true, cx));
        }
        band.into_any_element()
    }

    /// The button that names the revision shown, and opens the menu of them.
    #[allow(clippy::too_many_arguments)]
    fn revision_button(&self, ix: usize, key: &str, title: &str, revisions: usize, picked: Option<usize>, look: &Look, cx: &Context<Self>) -> AnyElement {
                let shown = picked.map_or(revisions, |at| at + 1);
                let (menu_key, menu_title) = (key.to_string(), title.to_string());
                labelled(
                    div()
                        .id(SharedString::from(format!("revision:{key}")))
                        .flex_none()
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                        .tooltip(tip("Revision"))
                        .child(format!("Revision {shown}"))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                let entries: Vec<Entry> = (1..=revisions.min(NUMBERED.len()))
                                    .rev()
                                    .map(|n| Entry::Choice { id: NUMBERED[n - 1], label: format!("Revision {n}").into(), on: n == shown })
                                    .collect();
                                let key = menu_key.clone();
                                this.open_pop(
                                    format!("Revision of {menu_title}"),
                                    entries,
                                    event.position,
                                    false,
                                    None,
                                    move |this, id, _, cx| {
                                        let n = NUMBERED.iter().position(|one| *one == id).map_or(revisions, |at| at + 1);
                                        if let Some(page) = this.shown.clone().and_then(|job| this.pages.get_mut(&job)) {
                                            if n >= revisions {
                                                page.cards.revision.remove(&key);
                                            } else {
                                                page.cards.revision.insert(key.clone(), n - 1);
                                            }
                                        }
                                        this.card_changed(ix, cx);
                                    },
                                    window,
                                    cx,
                                );
                            }),
                        ),
                    Role::Button,
                    format!("Revision {shown}"),
                )
.into_any_element()
    }

    /// A document revision the Job kept (B1.5).
    pub(super) fn document(&self, ix: usize, card: &ThreadDocument, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let key = card.id.clone();
        let optimistic = page.cards.answers.get(&key);
        let settled = settled_of(card.settled.as_ref(), optimistic);
        let open = card.asked && settled.is_none();
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_1))
            .child(glyph(look, "file-text", Size::Small, look.color(token::FG_MUTED)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(card.title.clone())),
            )
            .child(
                div()
                    .id(SharedString::from(format!("revision-tag:{key}")))
                    .flex_none()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .tooltip(tip(format!("Revision {}", card.revision)))
                    .child(format!("r{}", card.revision)),
            )
            .child(self.stamp(&card.at, look, now, true, cx));
        let body = frame(look, true).child(band).when(open || settled.is_some() || page.cards.said.contains_key(&key), |frame| {
            frame.child(self.approval(ix, &key, Subject::Document { page: card.page.clone(), revision: card.revision }, open, false, settled, true, page, look, cx))
        });
        labelled(body.id(SharedString::from(format!("document:{key}"))), Role::Article, format!("Document: {}, Revision {}", card.title, card.revision)).into_any_element()
    }

    /// A file, a window or a judged reading the Job showed: a quiet card with a glyph and a name.
    fn quiet_base(&self, id: &str, icon: &'static str, parts: Vec<AnyElement>, look: &Look) -> gpui::Stateful<gpui::Div> {
        div()
            .id(SharedString::from(format!("quiet:{id}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .w_full()
            .min_w_0()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .bg(look.color(token::FG_DEFAULT).opacity(0.04))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::FG_DEFAULT).opacity(0.07))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED)))
            .children(parts)
    }

    pub(super) fn quiet(&self, id: &str, icon: &'static str, label: String, parts: Vec<AnyElement>, look: &Look) -> AnyElement {
        labelled(self.quiet_base(id, icon, parts, look), Role::Row, label).into_any_element()
    }

    pub(super) fn file_row(&self, id: &str, name: &str, look: &Look) -> AnyElement {
        let parts = vec![
            div().flex_none().child("Opened").into_any_element(),
            div().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(name.to_string())).into_any_element(),
        ];
        self.quiet(id, "file", format!("Opened {name}"), parts, look)
    }

    /// A reading that finished. A press opens the plan at its notes; the other artifacts open in panes the Job panes step draws.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn judged_row(&self, id: &str, artifact: &ipc::JudgeArtifact, at: &Instant, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let title = &artifact.title;
        let parts = vec![
            div().flex_none().child("Judged").into_any_element(),
            div().min_w_0().truncate().font_family(mono(look)).child(SharedString::from(title.to_string())).into_any_element(),
            div().flex_1().into_any_element(),
            self.stamp(at, look, now, true, cx),
        ];
        let spot = page.keys.spot(&format!("judged:{id}"), cx);
        let plan = artifact.kind == ipc::JudgeKind::Plan;
        let row = self
            .quiet_base(id, "scale", parts, look)
            .track_focus(&spot.focus)
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.08)))
            .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if plan && matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    this.show_notes(window, cx);
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                if plan {
                    this.show_notes(window, cx);
                }
            }));
        labelled(row, Role::Button, format!("Judged {title}")).into_any_element()
    }

    pub(super) fn window_row(&self, id: &str, title: &str, look: &Look) -> AnyElement {
        let parts = vec![div().min_w_0().truncate().child(SharedString::from(title.to_string())).into_any_element()];
        self.quiet(id, "app-window", format!("Open window {title}"), parts, look)
    }
}

/// How a card reads as answered: Fleet's word, else what the person just pressed.
pub(super) fn settled_of(data: Option<&CardSettled>, pressed: Option<&Answer>) -> Option<(Said, Option<String>)> {
    match pressed {
        Some(Answer::Approved) => return Some((Said::Approved, None)),
        Some(Answer::SentBack(words)) => return Some((Said::SentBack, Some(words.clone()))),
        _ => {}
    }
    data.map(|settled| (if settled.how == SketchHow::Approved { Said::Approved } else { Said::SentBack }, settled.words.clone()))
}

/// The pane a sketch's scene is drawn in.
pub(super) fn glass(look: &Look, replaced: bool) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .mx(look.length(token::SPACE_2))
        .h(look.length(token::H_ROW_UNIT) * 8.)
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(token::BG_SUNKEN))
        .when(replaced, |glass| glass.opacity(0.5))
        .child(glyph(look, "pencil-ruler", Size::Mark, look.color(token::FG_SUBTLE)))
        .into_any_element()
}

// ---- the draft card (B3) ---------------------------------------------------------------------

/// What the draft is, from its row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DraftKind {
    Draft,
    Workflow,
}

fn settled_word(kind: DraftKind, how: DraftHow) -> (&'static str, &'static str, bool) {
    match (kind, how) {
        (DraftKind::Workflow, DraftHow::Withdrawn) => ("Dismissed", "ban", false),
        (_, DraftHow::Approved) => ("Approved", "badge-check", true),
        (_, DraftHow::Fixed) => ("Approved with your fixes", "badge-check", true),
        (_, DraftHow::SentBack) => ("Sent back", "reply", false),
        (_, DraftHow::Withdrawn) => ("Withdrawn", "ban", false),
    }
}

impl JobView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draft(
        &self,
        ix: usize,
        id: &str,
        at: &Instant,
        title: &str,
        text: &str,
        before: Option<&str>,
        settled: Option<&DraftSettled>,
        change: Option<i64>,
        headed: bool,
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let key = format!("draft:{id}");
        let kind = if change.is_some() { DraftKind::Workflow } else { DraftKind::Draft };
        let mode = page.cards.modes.get(&key).copied().unwrap_or(Mode::Read);
        let pressed = page.cards.answers.get(&key);
        let how: Option<(DraftHow, Option<String>)> = match pressed {
            Some(Answer::Approved) => Some((DraftHow::Approved, None)),
            Some(Answer::Fixed(words)) => Some((DraftHow::Fixed, Some(words.clone()))),
            Some(Answer::SentBack(words)) => Some((DraftHow::SentBack, Some(words.clone()))),
            Some(Answer::Withdrawn) | Some(Answer::Dismissed) => Some((DraftHow::Withdrawn, None)),
            _ => settled.map(|settled| (settled.how, settled.words.clone())),
        };
        let open = how.is_none();
        let editing = open && mode == Mode::Edit;
        let noting = open && mode == Mode::Note;
        let target = self.is_target(&key);
        let eyebrow_word = if kind == DraftKind::Workflow { "Workflow" } else { "Draft" };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_color(look.color(token::FG_MUTED))
            .child(super::rows::eyebrow(look, eyebrow_word).text_color(look.color(token::FG_MUTED)))
            .child(div().flex_1())
            .child(self.stamp(at, look, now, false, cx));
        let shown_text = match &how {
            Some((DraftHow::Fixed, Some(words))) => words.clone(),
            _ => text.to_string(),
        };
        let more_before = before.map(|before| {
            let opened = self.open_in_card(&format!("{key}:before"));
            let key_before = format!("{key}:before");
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    button(look, SharedString::from(format!("before:{key}")), "Text it replaces", Tone::Ghost, false, None, true)
                        .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &key_before, cx))),
                    Role::Button,
                    "Text it replaces",
                ))
                .children(opened.then(|| {
                    div()
                        .w_full()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::BG_SUNKEN))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(SharedString::from(before.to_string()))
                }))
        });
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(head).child(
            div()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(title.to_string())),
        );
        body = if editing {
            body.children(self.box_of(&key, Mode::Edit, "Draft text", look, cx))
        } else {
            let words = div().min_w_0().child(self.prose(format!("{key}:text"), &shown_text, look)).into_any_element();
            // Words selected in an open draft take their own menu, Edit and Add note, in place of the selection bar.
            body.child(if open { self.block(page, &key, words, cx) } else { words })
        };
        body = body.children(more_before);
        if noting {
            body = body.children(self.box_of(&key, Mode::Note, "Note", look, cx));
        }
        if let Some((how, words)) = &how {
            let (word, icon, good) = settled_word(kind, *how);
            body = body
                .children(matches!(how, DraftHow::SentBack | DraftHow::Withdrawn).then_some(words.as_deref()).flatten().map(|words| quote(look, words)))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("settled:{key}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(
                            div()
                                .id(SharedString::from(format!("settled-mark:{key}")))
                                .child(glyph(look, icon, Size::Mark, look.color(if good { token::STEP_ADVANCED } else { token::FG_MUTED })))
                                .tooltip(tip(word)),
                        )
                        .child(word),
                    Role::Status,
                    word,
                ));
        }
        if let Some(why) = page.cards.said.get(&key) {
            body = body.child(refusal(look, &key, why.clone()));
        }
        if open {
            body = body.child(self.draft_footer(ix, &key, id, kind, mode, text, &title, target, headed, page, look, cx));
        } else if !headed {
            page.keys.set(&key, None);
        }
        let name = format!("{}: {title}", eyebrow_word);
        let aim = key.clone();
        let spot = self.spot(page, &format!("card:{key}"), cx);
        let card = div()
            .id(SharedString::from(format!("card:{key}")))
            .track_focus(&spot.focus)
            .w_full()
            .p(look.length(token::PAD_CARD))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if target && open { token::ACCENT } else { token::BORDER_DEFAULT }))
            .bg(look.color(token::BG_RAISED))
            .on_mouse_down(MouseButton::Left, cx.listener({
                let focus = spot.focus.clone();
                move |this, _, window, cx| {
                    this.aim(&aim);
                    // A box inside the card takes the press first; the card takes it when nothing in it did.
                    if !focus.contains_focused(window, cx) {
                        window.focus(&focus, cx);
                    }
                    cx.notify();
                }
            }))
            .child(body);
        let card = self.menu_on(card, &spot, "Draft", spot.focus.clone(), Rc::new(teach_entries), self.teach_pick("words", text.to_string(), Some(at.clone())), cx);
        labelled(card, Role::Article, name).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn draft_footer(
        &self,
        ix: usize,
        key: &str,
        id: &str,
        kind: DraftKind,
        mode: Mode,
        text: &str,
        title: &str,
        target: bool,
        headed: bool,
        page: &Page,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let job = self.shown.clone().unwrap_or_default();
        let item = format!("draft:{id}");
        let (yes, change, back, no) = (target.then_some("\u{2325}\u{21b5}"), target.then_some("\u{2325}E"), target.then_some("\u{2325}R"), target.then_some("\u{2325}X"));
        let fixed = self.typed_in(key, Mode::Edit, cx);
        let note = self.typed_in(key, Mode::Note, cx);
        let word_withdraw = if kind == DraftKind::Workflow { "Dismiss" } else { "Withdraw" };
        let answer_body = {
            let (job, item) = (job.clone(), item.clone());
            move |choice: Option<u32>, words: Option<String>, fixed: Option<String>| {
                encoded(&AnswerWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.clone(), choice, text: words, mode: None, fixed })
            }
        };
        let withdraw = {
            let (job, item) = (job.clone(), item.clone());
            move |reason: Option<String>| encoded(&DismissWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.clone(), reason, misread: false })
        };
        {
            let editing = mode == Mode::Edit;
            let noting = mode == Mode::Note;
            let (approve_word, fix_word) = ("Approve", "Approve with fixes");
            let word_out = word_withdraw;
            let (k1, k2, k3, k4, k5) = (key.to_string(), key.to_string(), key.to_string(), key.to_string(), key.to_string());
            let (answer1, answer2, out1, out2) = (answer_body.clone(), answer_body.clone(), withdraw.clone(), withdraw.clone());
            let (edit_text, original) = (text.to_string(), text.to_string());
            let yes: Option<(String, super::keys::Run)> = if editing {
                (!fixed.trim().is_empty()).then(|| {
                    (
                        fix_word.to_string(),
                        Rc::new(move |this: &mut JobView, window: &mut gpui::Window, cx: &mut Context<JobView>| {
                            let words = this.typed_in(&k1, Mode::Edit, cx);
                            if words == original {
                                this.settle(ix, k1.clone(), Answer::Approved, "/sessions/waiting/answer", answer1(Some(0), None, None), window, cx);
                            } else {
                                this.settle(ix, k1.clone(), Answer::Fixed(words.clone()), "/sessions/waiting/answer", answer1(None, None, Some(words)), window, cx);
                            }
                        }) as super::keys::Run,
                    )
                })
            } else if noting {
                None
            } else {
                Some((
                    approve_word.to_string(),
                    Rc::new(move |this: &mut JobView, window: &mut gpui::Window, cx: &mut Context<JobView>| {
                        this.settle(ix, k2.clone(), Answer::Approved, "/sessions/waiting/answer", answer2(Some(0), None, None), window, cx)
                    }) as super::keys::Run,
                ))
            };
            let change: Option<(String, super::keys::Run)> = (!noting && kind != DraftKind::Workflow && !editing).then(|| {
                ("Edit".to_string(), Rc::new(move |this: &mut JobView, window: &mut gpui::Window, cx: &mut Context<JobView>| this.open_box(ix, &k3, Mode::Edit, &edit_text, window, cx)) as super::keys::Run)
            });
            let send_back: Option<(String, super::keys::Run)> = (!editing && !noting && kind != DraftKind::Workflow).then(|| {
                ("Note".to_string(), Rc::new(move |this: &mut JobView, window: &mut gpui::Window, cx: &mut Context<JobView>| this.open_box(ix, &k4, Mode::Note, "", window, cx)) as super::keys::Run)
            });
            let no: Option<(String, super::keys::Run)> = (!editing).then(|| {
                let _ = &out2;
                (
                    word_out.to_string(),
                    Rc::new(move |this: &mut JobView, window: &mut gpui::Window, cx: &mut Context<JobView>| {
                        let reason = if noting { Some(this.typed_in(&k5, Mode::Note, cx).trim().to_string()).filter(|text| !text.is_empty()) } else { None };
                        let answer = if kind == DraftKind::Workflow { Answer::Dismissed } else { Answer::Withdrawn };
                        this.settle(ix, k5.clone(), answer, "/sessions/waiting/dismiss", out1(reason), window, cx)
                    }) as super::keys::Run,
                )
            });
            let kind_word = match kind {
                DraftKind::Workflow => "Workflow",
                _ => "Draft",
            };
            // A draft drawn on the review sheet is that sheet's, and the card in the thread keeps its keys.
            if !headed {
                page.keys.set(key, Some(super::keys::Card { at: ix, kind: kind_word.into(), title: title.to_string(), yes, change, send_back, no }));
            }
        }
        let key_s = key.to_string();
        let row = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        let button_at = |name: &'static str, tone: Tone, chip: Option<&'static str>, on: bool| {
            button(look, SharedString::from(format!("{name}:{key}")), name, tone, false, chip, on)
        };
        let row = match mode {
            Mode::Read => {
                let (approve_key, edit_key, note_key, out_key) = (key_s.clone(), key_s.clone(), key_s.clone(), key_s.clone());
                let (approve_body, out_body) = (answer_body.clone(), withdraw.clone());
                let edit_text = text.to_string();
                // Headed, the sheet's head holds Approve and Send back; the card keeps Edit and Withdraw.
                row.children((!headed).then(|| {
                    labelled(
                        button_at("Approve", Tone::Primary, yes, true).on_click(cx.listener(move |this, _, window, cx| {
                            this.settle(ix, approve_key.clone(), Answer::Approved, "/sessions/waiting/answer", approve_body(Some(0), None, None), window, cx)
                        })),
                        Role::Button,
                        "Approve",
                    )
                }))
                .when(kind != DraftKind::Workflow, |row| {
                    row.child(labelled(
                        button_at("Edit", Tone::Secondary, change, true)
                            .on_click(cx.listener(move |this, _, window, cx| this.open_box(ix, &edit_key, Mode::Edit, &edit_text, window, cx))),
                        Role::Button,
                        "Edit",
                    ))
                    .children((!headed).then(|| {
                        labelled(
                            button_at("Note", Tone::Secondary, back, true)
                                .on_click(cx.listener(move |this, _, window, cx| this.open_box(ix, &note_key, Mode::Note, "", window, cx))),
                            Role::Button,
                            "Note",
                        )
                    }))
                })
                .child(div().flex_1())
                .child(labelled(
                    button(look, SharedString::from(format!("withdraw:{key}")), word_withdraw, Tone::Secondary, false, no, true).on_click(cx.listener(
                        move |this, _, window, cx| {
                            let answer = if kind == DraftKind::Workflow { Answer::Dismissed } else { Answer::Withdrawn };
                            this.settle(ix, out_key.clone(), answer, "/sessions/waiting/dismiss", out_body(None), window, cx)
                        },
                    )),
                    Role::Button,
                    word_withdraw,
                ))
            }
            Mode::Edit => {
                let blank = fixed.trim().is_empty();
                let (go_key, cancel_key) = (key_s.clone(), key_s.clone());
                let original = text.to_string();
                row.child(labelled(
                    button_at("Approve with fixes", Tone::Primary, yes, !blank).when(!blank, |b| {
                        b.on_click(cx.listener(move |this, _, window, cx| {
                            let words = this.typed_in(&go_key, Mode::Edit, cx);
                            if words == original {
                                this.settle(ix, go_key.clone(), Answer::Approved, "/sessions/waiting/answer", answer_body(Some(0), None, None), window, cx);
                            } else {
                                this.settle(ix, go_key.clone(), Answer::Fixed(words.clone()), "/sessions/waiting/answer", answer_body(None, None, Some(words)), window, cx);
                            }
                        }))
                    }),
                    Role::Button,
                    "Approve with fixes",
                ))
                .child(labelled(
                    button_at("Cancel", Tone::Ghost, None, true).on_click(cx.listener(move |this, _, _, cx| this.close_box(ix, &cancel_key, cx))),
                    Role::Button,
                    "Cancel",
                ))
            }
            Mode::Note => {
                let blank = note.trim().is_empty();
                let (go_key, cancel_key, out_key) = (key_s.clone(), key_s.clone(), key_s.clone());
                row.child(labelled(
                    button_at("Send back", Tone::Primary, None, !blank).when(!blank, |b| {
                        b.on_click(cx.listener(move |this, _, window, cx| {
                            let words = this.typed_in(&go_key, Mode::Note, cx).trim().to_string();
                            this.settle(ix, go_key.clone(), Answer::SentBack(words.clone()), "/sessions/waiting/answer", answer_body(None, Some(words), None), window, cx);
                        }))
                    }),
                    Role::Button,
                    "Send back",
                ))
                .child(labelled(
                    button_at("Cancel", Tone::Ghost, None, true).on_click(cx.listener(move |this, _, _, cx| this.close_box(ix, &cancel_key, cx))),
                    Role::Button,
                    "Cancel",
                ))
                .child(div().flex_1())
                .child(labelled(
                    button(look, SharedString::from(format!("withdraw:{key}")), word_withdraw, Tone::Secondary, false, None, true).on_click(cx.listener(
                        move |this, _, window, cx| {
                            let reason = this.typed_in(&out_key, Mode::Note, cx).trim().to_string();
                            let reason = (!reason.is_empty()).then_some(reason);
                            this.settle(ix, out_key.clone(), Answer::Withdrawn, "/sessions/waiting/dismiss", withdraw(reason), window, cx)
                        },
                    )),
                    Role::Button,
                    word_withdraw,
                ))
            }
        };
        labelled(row.id(SharedString::from(format!("answers:{key}"))), Role::Toolbar, "Answers").into_any_element()
    }
}
