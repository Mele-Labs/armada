//! The talk about a page, drawn from Fleet's margin threads (`margin.rs` holds them): the margin beside the page, one
//! thread a passage, and the rounds of it in the Job's thread. An answer changes nothing on the page; a suggested edit
//! stands struck and added, in the page and in its thread, until he presses Accept; a settled thread leaves the Open tab
//! for the page's Decisions.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Hsla, KeyDownEvent, Role, SharedString};
use gpui_component::input::{Enter, Textarea};
use ipc::{MarginAct, MarginRoundKind, MarginState, MarginThread, MarginVoice, SuggestionHow, ThreadMargin};

use super::acts::{at_of, button, kbd, refusal, Tone};
use super::reader::Reader;
use super::rows::tip;
use super::sketch_card::mixed;
use super::JobView;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// The keys under a comment and on the held bar, as the chips on their buttons show them.
const SEND_NOW: &str = "\u{2318}\u{21b5}";
const SEND_HELD: &str = "\u{21e7}\u{2318}\u{21b5}";

fn glyph(look: &Look, name: &str, ink: Hsla) -> gpui::Svg {
    gpui::svg().path(icon_path(name)).flex_none().size(look.length(token::TEXT_SM)).text_color(ink)
}

/// Where a thread stands, as a glyph and the words its tooltip says.
fn state_of(state: MarginState) -> (&'static str, &'static str, Token) {
    match state {
        MarginState::Open => ("message-square", "Sent, the agent has not answered", token::FG_MUTED),
        MarginState::Answered => ("message-square-text", "Answered, the page is unchanged", token::ACCENT),
        MarginState::Suggested => ("diff", "A change waits for your Accept", token::YOU),
        MarginState::Held => ("clock", "Held until you send held comments", token::YOU),
        MarginState::Settled => ("check", "Settled", token::STATUS_COMPLETED_SUCCESS),
    }
}

/// A round of talk, as a glyph and the words its tooltip says.
fn round_of(kind: MarginRoundKind) -> (&'static str, &'static str) {
    match kind {
        MarginRoundKind::Comments => ("messages-square", "Comments in the margin"),
        MarginRoundKind::Change => ("diff", "A change to the page"),
        MarginRoundKind::Accepted => ("check", "A change accepted"),
        MarginRoundKind::Rejected => ("x", "A change rejected"),
        MarginRoundKind::Settled => ("circle-check", "Settled"),
        MarginRoundKind::SentBack => ("reply", "Sent back from the page"),
    }
}

fn small(look: &Look, ink: Token, words: impl Into<SharedString>) -> gpui::Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(ink)).child(words.into())
}

/// One line of a thread: who said it, when, and the words.
fn said(look: &Look, by: MarginVoice, at: &ipc::Instant, words: &str, held: bool) -> AnyElement {
    let (name, icon, ink) = match by {
        MarginVoice::You => ("You", "user", look.color(token::YOU)),
        MarginVoice::Agent => ("Agent", "bot", look.color(token::ACCENT)),
    };
    let when = at_of(at).map(|at| at.format("%H:%M").to_string()).unwrap_or_default();
    div()
        .flex()
        .gap(look.length(token::SPACE_2))
        .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(glyph(look, icon, ink)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .gap(look.length(token::SPACE_2))
                        .child(small(look, token::FG_DEFAULT, name).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                        .child(small(look, token::FG_SUBTLE, when))
                        .when(held, |line| line.child(small(look, token::YOU, "Held"))),
                )
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(words.to_string()))),
        )
        .into_any_element()
}

/// The passage a thread is on, in his colour.
fn quote(look: &Look, words: &str, gone: bool) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_SM))
        .bg(if gone { look.color(token::BG_SUNKEN) } else { mixed(look, token::YOU, 0.12) })
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_MUTED))
        .when(gone, |line| line.line_through())
        .child(SharedString::from(format!("\u{201c}{words}\u{201d}")))
}

/// One line of a suggested edit: the words taken out or put in, in the diff's colours.
fn change_line(look: &Look, added: bool, words: &str) -> AnyElement {
    let (bg, fg, sign) = if added { (token::DIFF_ADD_BG, token::DIFF_ADD_FG, "+") } else { (token::DIFF_DEL_BG, token::DIFF_DEL_FG, "\u{2212}") };
    div()
        .flex()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_2))
        .py(look.length(token::SPACE_1))
        .bg(look.color(bg))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(fg))
        .child(div().flex_none().child(sign))
        .child(div().flex_1().min_w_0().when(!added, |line| line.line_through()).child(SharedString::from(words.to_string())))
        .into_any_element()
}

/// A box he types in, framed as the composer frames one: in the accent while it is the comment being begun.
fn typing_box(look: &Look, focused: bool) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(if focused { token::ACCENT } else { token::BORDER_DEFAULT }))
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_DEFAULT))
}

impl JobView {
    /// The margin: Open and Settled at its head, the comments he holds, the comment he is writing, then a thread for
    /// each passage. `pins` are notes he left on the page before it had a margin.
    pub(super) fn margin_column(&self, id: &SharedString, name: &str, held: &Reader, pins: Vec<AnyElement>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let margin = &held.margin;
        let settled_tab = margin.settled;
        let tab = |which: bool, said: &'static str| {
            let (job, reader) = (id.clone(), name.to_string());
            let on = settled_tab == which;
            labelled(
                div()
                    .id(SharedString::from(format!("margin-tab:{said}:{name}")))
                    .flex()
                    .items_center()
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if on { look.color(token::BORDER_DEFAULT) } else { gpui::transparent_black() })
                    .when(on, |tab| tab.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)))
                    .when(!on, |tab| tab.cursor_pointer().text_color(look.color(token::FG_MUTED)).hover(|style| style.text_color(look.color(token::FG_DEFAULT))))
                    .on_click(cx.listener(move |this, _, _, cx| this.margin_tab(&job, &reader, which, cx)))
                    .child(said),
                Role::Tab,
                said,
            )
        };
        let (job, reader) = (id.clone(), name.to_string());
        let begin = labelled(
            div()
                .id(SharedString::from(format!("margin-begin:{name}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .cursor_pointer()
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .tooltip(tip("Comment on the words selected in the page"))
                .child(kbd(look, "N"))
                .child("comment on a passage")
                .on_click(cx.listener(move |this, _, window, cx| this.reader_page_note(&job, &reader, false, window, cx))),
            Role::Button,
            "Comment on a passage",
        );
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                labelled(
                    div().id(SharedString::from(format!("margin-tabs:{name}"))).flex().gap(look.length(token::SPACE_1)).p(look.length(token::BORDER_WIDTH) * 2.).rounded(look.length(token::RADIUS_MD)).bg(look.color(token::BG_SUNKEN)).child(tab(false, "Open")).child(tab(true, "Settled")),
                    Role::TabList,
                    "Margin",
                ),
            )
            .child(begin);

        let mut body: Vec<AnyElement> = Vec::new();
        if let Some(why) = margin.said.clone() {
            body.push(refusal(look, &format!("margin:{name}"), why));
        }
        if !margin.loaded {
            body.extend((0..3).map(|at| {
                div()
                    .id(SharedString::from(format!("margin-shape:{name}:{at}")))
                    .h(look.length(token::SPACE_12) * 2.)
                    .rounded(look.length(token::RADIUS_LG))
                    .bg(look.color(token::BG_SUNKEN))
                    .into_any_element()
            }));
        } else if settled_tab {
            let done: Vec<&MarginThread> = margin.threads.iter().filter(|one| one.state == MarginState::Settled).collect();
            if done.is_empty() {
                body.push(small(look, token::FG_SUBTLE, "No thread on this page is settled").into_any_element());
            }
            body.extend(done.into_iter().map(|one| self.margin_settled(name, one, look)));
        } else {
            if margin.holds() {
                body.push(self.margin_held_bar(id, name, margin.busy, look, cx));
            }
            if let Some(new) = &margin.new {
                body.push(self.margin_new_card(id, name, new, margin.busy, look, cx));
            }
            let open: Vec<&MarginThread> = margin.open().collect();
            if open.is_empty() && margin.new.is_none() {
                body.push(small(look, token::FG_SUBTLE, "Select words in the page and press N to comment on them").into_any_element());
            }
            body.extend(open.into_iter().map(|one| self.margin_thread(id, name, one, margin, look, cx)));
        }
        body.extend(pins);

        labelled(
            div()
                .id(SharedString::from(format!("margin:{name}")))
                .flex_none()
                .w(look.length(token::W_DOCK))
                .min_h_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_4))
                .border_l(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_BASE))
                .overflow_y_scroll()
                .child(head)
                .children(body),
            Role::Group,
            "Margin",
        )
        .into_any_element()
    }

    /// The comments he holds wait here until he sends them together.
    fn margin_held_bar(&self, id: &SharedString, name: &str, busy: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, reader) = (id.clone(), name.to_string());
        let send = labelled(
            button(look, SharedString::from(format!("margin-send-held:{name}")), "Send held", Tone::Secondary, false, Some(SEND_HELD), !busy)
                .tooltip(tip("Send every held comment on this page to the agent"))
                .on_click(cx.listener(move |this, _, window, cx| this.margin_send_held(&job, &reader, window, cx))),
            Role::Button,
            "Send held",
        );
        labelled(
            div()
                .id(SharedString::from(format!("margin-held:{name}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .bg(mixed(look, token::YOU, 0.12))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(mixed(look, token::YOU, 0.4))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(glyph(look, "clock", look.color(token::YOU)))
                .child(div().flex_1().min_w_0().truncate().child("Held comments"))
                .child(send),
            Role::Group,
            "Held comments",
        )
        .into_any_element()
    }

    /// Not sent, Hold and Send now, under a comment or a reply that has words in it.
    #[allow(clippy::too_many_arguments)]
    fn margin_send_row(&self, id: &SharedString, name: &str, key: &str, thread: Option<&str>, busy: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let send = |hold: bool| {
            let (job, reader, thread) = (id.clone(), name.to_string(), thread.map(str::to_string));
            let (said, tone, chip) = if hold { ("Hold", Tone::Secondary, None) } else { ("Send now", Tone::Primary, Some(SEND_NOW)) };
            labelled(
                button(look, SharedString::from(format!("margin-{}:{key}", if hold { "hold" } else { "send" })), said, tone, false, chip, !busy)
                    .tooltip(tip(if hold { "Keep it until you send held comments" } else { "Send it to the agent now" }))
                    .on_click(cx.listener(move |this, _, window, cx| match &thread {
                        Some(thread) => this.margin_send_reply(&job, &reader, thread, hold, window, cx),
                        None => this.margin_send_new(&job, &reader, hold, window, cx),
                    })),
                Role::Button,
                said,
            )
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(small(look, token::FG_SUBTLE, "Not sent"))
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(send(true)).child(send(false)))
            .into_any_element()
    }

    /// The comment he is writing on words of the page, with the caret in it.
    #[allow(clippy::too_many_arguments)]
    fn margin_new_card(&self, id: &SharedString, name: &str, new: &super::margin::NewComment, busy: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let area = new.area.clone();
        let empty = area.read(cx).value().trim().is_empty();
        let (job, reader, job2, reader2) = (id.clone(), name.to_string(), id.clone(), name.to_string());
        let (job3, reader3) = (id.clone(), name.to_string());
        let close = labelled(
            super::head::icon_button(look, "margin-new-close", "x", "Drop this comment".into(), false, false)
                .id(SharedString::from(format!("margin-new-close:{name}")))
                .on_click(cx.listener(move |this, _, window, cx| this.margin_drop_new(&job3, &reader3, window, cx))),
            Role::Button,
            "Drop this comment",
        );
        let edge = area.clone();
        labelled(
            div()
                .id(SharedString::from(format!("margin-new:{name}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(mixed(look, token::YOU, 0.55))
                .bg(look.color(token::BG_RAISED))
                .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key != "escape" || event.keystroke.modifiers.modified() {
                        return;
                    }
                    // Esc drops an empty comment, and leaves one with words in it unsent, kept.
                    cx.stop_propagation();
                    if edge.read(cx).value().trim().is_empty() {
                        this.margin_drop_new(&job, &reader, window, cx);
                    } else {
                        window.focus(&this.row_focus, cx);
                    }
                }))
                .capture_action(cx.listener(move |this, action: &Enter, window, cx| {
                    if action.secondary {
                        cx.stop_propagation();
                        this.margin_send_new(&job2, &reader2, false, window, cx);
                    }
                }))
                .child(div().flex().items_start().gap(look.length(token::SPACE_2)).child(quote(look, &new.anchor.quote, false)).child(close))
                .child(
                    typing_box(look, true).child(labelled(
                        div().id(SharedString::from(format!("margin-new-box:{name}"))).key_context(crate::keymap::TEXT_BOX).child(Textarea::new(&area).appearance(false).aria_label("Comment")),
                        Role::TextInput,
                        "Comment",
                    )),
                )
                .when(!empty, |card| card.child(self.margin_send_row(id, name, &format!("new:{name}"), None, busy, look, cx))),
            Role::Group,
            "New comment",
        )
        .into_any_element()
    }

    /// One thread: the passage and where it stands, what was said, a suggested edit, his reply, and its acts.
    #[allow(clippy::too_many_arguments)]
    fn margin_thread(&self, id: &SharedString, name: &str, one: &MarginThread, margin: &super::margin::Margin, look: &Look, cx: &Context<Self>) -> AnyElement {
        let on = margin.on.as_deref() == Some(one.id.as_str());
        let busy = margin.busy;
        let (icon, what, ink) = state_of(one.state);
        let key = format!("{name}:{}", one.id);
        let state = div()
            .id(SharedString::from(format!("margin-state:{key}")))
            .flex_none()
            .h(look.length(token::LEADING_XS))
            .flex()
            .items_center()
            .tooltip(tip(what))
            .child(glyph(look, icon, look.color(ink)));
        let head = div().flex().items_start().gap(look.length(token::SPACE_2)).child(quote(look, &one.anchor.quote, one.gone)).child(labelled(state, Role::Image, what));
        let gone = one.gone.then(|| small(look, token::FG_SUBTLE, "The passage is gone from the page"));
        let lines: Vec<AnyElement> = one.messages.iter().map(|said_| said(look, said_.by, &said_.at, &said_.text, said_.held)).collect();

        let pending = one.suggestion.as_ref().filter(|edit| edit.decided.is_none());
        let suggestion = one.suggestion.as_ref().map(|edit| {
            let decided = edit.decided.as_ref().map(|done| match (done.how, done.revision) {
                (SuggestionHow::Accepted, Some(revision)) => format!("Accepted, in revision {revision}"),
                (SuggestionHow::Accepted, None) => "Accepted".to_string(),
                (SuggestionHow::Rejected, _) => "Rejected, the page is unchanged".to_string(),
            });
            let answer = |accept: bool| {
                let (job, reader, thread) = (id.clone(), name.to_string(), one.id.clone());
                let (said, tone) = if accept { ("Accept", Tone::Primary) } else { ("Reject", Tone::Secondary) };
                labelled(
                    button(look, SharedString::from(format!("margin-{}:{key}", said.to_lowercase())), said, tone, false, None, !busy)
                        .tooltip(tip(if accept { "Write this change into the page" } else { "Leave the page as it is" }))
                        .on_click(cx.listener(move |this, _, window, cx| this.margin_do(&job, &reader, &thread, if accept { MarginAct::Accept } else { MarginAct::Reject }, window, cx))),
                    Role::Button,
                    said,
                )
            };
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(div().flex().flex_col().rounded(look.length(token::RADIUS_MD)).overflow_hidden().child(change_line(look, false, &edit.struck)).child(change_line(look, true, &edit.added)))
                .when_some(decided, |block, said| block.child(small(look, token::FG_SUBTLE, said)))
                .when(pending.is_some(), |block| block.child(div().flex().gap(look.length(token::SPACE_2)).child(answer(true)).child(answer(false))))
        });

        let reply = margin.replies.get(&one.id).map(|(area, _)| {
            let empty = area.read(cx).value().trim().is_empty();
            let (job, thread) = (id.clone(), one.id.clone());
            let reader = name.to_string();
            let row_focus = self.row_focus.clone();
            typing_box(look, false)
                .id(SharedString::from(format!("margin-reply-frame:{key}")))
                .capture_key_down(cx.listener(move |_, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        window.focus(&row_focus, cx);
                    }
                }))
                .capture_action(cx.listener(move |this, action: &Enter, window, cx| {
                    if action.secondary {
                        cx.stop_propagation();
                        this.margin_send_reply(&job, &reader, &thread, false, window, cx);
                    }
                }))
                .child(labelled(
                    div().id(SharedString::from(format!("margin-reply:{key}"))).key_context(crate::keymap::TEXT_BOX).child(Textarea::new(area).appearance(false).aria_label("Reply")),
                    Role::TextInput,
                    "Reply",
                ))
                .when(!empty, |frame| frame.child(self.margin_send_row(id, name, &key, Some(&one.id), busy, look, cx)))
        });

        // Make this change and Settle stand in the same place on every thread.
        let act = |change: bool| {
            let (job, reader, thread) = (id.clone(), name.to_string(), one.id.clone());
            let (said, tone, tooltip) = if change { ("Make this change", Tone::Secondary, "Ask the agent to change the page") } else { ("Settle", Tone::Ghost, "Close this thread and keep what came of it in Decisions") };
            let live = !busy && !(change && pending.is_some());
            labelled(
                button(look, SharedString::from(format!("margin-{}:{key}", if change { "change" } else { "settle" })), said, tone, false, None, live)
                    .tooltip(tip(tooltip))
                    .when(live, |button| {
                        button.on_click(cx.listener(move |this, _, window, cx| this.margin_do(&job, &reader, &thread, if change { MarginAct::MakeChange } else { MarginAct::Settle { outcome: None } }, window, cx)))
                    }),
                Role::Button,
                said,
            )
        };
        let acts = div().flex().gap(look.length(token::SPACE_2)).child(act(true)).child(act(false));

        let (job, reader, thread) = (id.clone(), name.to_string(), one.id.clone());
        let label = format!("Thread on \u{201c}{}\u{201d}", one.anchor.quote);
        labelled(
            div()
                .id(SharedString::from(format!("margin-thread:{key}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on { mixed(look, token::YOU, 0.55) } else { look.color(token::BORDER_DEFAULT) })
                .bg(look.color(token::BG_RAISED))
                .on_click(cx.listener(move |this, _, _, cx| this.margin_on(&job, &reader, &thread, cx)))
                .child(head)
                .children(gone)
                .children(lines)
                .children(suggestion)
                .children(reply)
                .child(acts),
            Role::Group,
            label,
        )
        .into_any_element()
    }

    /// A settled thread, in the Settled tab: what came of it, kept in the page's Decisions.
    fn margin_settled(&self, name: &str, one: &MarginThread, look: &Look) -> AnyElement {
        let words = one.outcome.clone().unwrap_or_else(|| one.anchor.quote.clone());
        labelled(
            div()
                .id(SharedString::from(format!("margin-settled:{name}:{}", one.id)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph(look, "check", look.color(token::STATUS_COMPLETED_SUCCESS)))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(words.clone())))
                .child(small(look, token::ACCENT, "In Decisions").flex_none()),
            Role::Group,
            words,
        )
        .into_any_element()
    }

    /// One round of the talk on a page, in the Job's thread: one line, with Open to the margin. The round that waits on
    /// him is tinted in his colour. A page sent back shows each note whole, with Open to its passage.
    pub(super) fn margin_round(&self, card: &ThreadMargin, look: &Look, cx: &Context<Self>) -> AnyElement {
        if card.kind == MarginRoundKind::SentBack {
            return self.margin_sent_card(card, look, cx);
        }
        let waits = card.waits_on_you;
        let (icon, what) = round_of(card.kind);
        let (page, thread) = (card.page.clone(), card.threads.first().cloned());
        let open = labelled(
            button(look, SharedString::from(format!("round-open:{}", card.id)), "Open", if waits { Tone::Secondary } else { Tone::Ghost }, false, None, true)
                .tooltip(tip("Open the page with its margin"))
                .on_click(cx.listener(move |this, _, window, cx| this.margin_open(&page, thread.as_deref(), None, window, cx))),
            Role::Button,
            "Open",
        );
        let glyph_ = labelled(
            div().id(SharedString::from(format!("round-kind:{}", card.id))).flex_none().flex().items_center().tooltip(tip(what)).child(glyph(look, icon, look.color(if waits { token::YOU } else { token::FG_MUTED }))),
            Role::Image,
            what,
        );
        labelled(
            div()
                .id(SharedString::from(format!("round:{}", card.id)))
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if waits { mixed(look, token::YOU, 0.45) } else { look.color(token::BORDER_SUBTLE) })
                .bg(if waits { mixed(look, token::YOU, 0.10) } else { look.color(token::BG_RAISED) })
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .child(glyph_)
                .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(card.title.clone())))
                .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(card.summary.clone())))
                .child(open),
            Role::Article,
            format!("{}: {}", card.title, card.summary),
        )
        .into_any_element()
    }

    /// What he sent back from a page: the page, and for each note the section, the passage and the whole note, with
    /// Open to that spot. Fleet's line for the agent and the page's path are never shown.
    fn margin_sent_card(&self, card: &ThreadMargin, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (page, thread) = (card.page.clone(), card.threads.first().cloned());
        let open = labelled(
            button(look, SharedString::from(format!("sent-open:{}", card.id)), "Open", Tone::Secondary, false, None, true)
                .tooltip(tip("Open the page"))
                .on_click(cx.listener(move |this, _, window, cx| this.margin_open(&page, thread.as_deref(), None, window, cx))),
            Role::Button,
            "Open",
        );
        let chip = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::H_BADGE))
            .px(look.length(token::SPACE_2))
            .rounded_full()
            .bg(mixed(look, token::YOU, 0.16))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(mixed(look, token::YOU, 0.4))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::YOU))
            .child(gpui::svg().path(icon_path("reply")).flex_none().size(look.length(token::TEXT_2XS)).text_color(look.color(token::YOU)))
            .child("Sent back");
        let notes: Vec<AnyElement> = card
            .notes
            .iter()
            .enumerate()
            .map(|(at, note)| {
                let (page, thread, passage) = (card.page.clone(), note.thread.clone(), note.passage.clone());
                let open = labelled(
                    button(look, SharedString::from(format!("sent-open:{}:{at}", card.id)), "Open", Tone::Ghost, false, None, true)
                        .tooltip(tip("Open the page at this passage"))
                        .on_click(cx.listener(move |this, _, window, cx| this.margin_open(&page, thread.as_deref(), passage.as_deref(), window, cx))),
                    Role::Button,
                    "Open at the passage",
                );
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .when(at > 0, |note| note.pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(small(look, token::FG_MUTED, note.section.clone().unwrap_or_default()).flex_1().min_w_0().truncate())
                            .child(open),
                    )
                    .when_some(note.passage.clone(), |block, passage| block.child(div().flex().child(quote(look, &passage, false))))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(note.note.clone())))
                    .into_any_element()
            })
            .collect();
        let card_ = labelled(
            div()
                .id(SharedString::from(format!("sent-card:{}", card.id)))
                .w_full()
                .max_w(look.length(token::W_DIALOG_WIDE))
                .flex()
                .flex_col()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(mixed(look, token::YOU, 0.45))
                .bg(look.color(token::BG_RAISED))
                .overflow_hidden()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .bg(mixed(look, token::YOU, 0.14))
                        .child(glyph(look, "file-text", look.color(token::YOU)))
                        .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(card.title.clone())))
                        .child(chip)
                        .child(open),
                )
                .child(div().flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::SPACE_3)).children(notes)),
            Role::Article,
            format!("Sent back: {}", card.title),
        );
        div().w_full().flex().justify_end().child(card_).into_any_element()
    }
}
