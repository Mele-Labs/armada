//! Talk: a press on the mic, or ⌥Space held in the box, listens, and each word heard joins the
//! box. Nothing is sent by it. What a hearing is lives in `crate::voice`; this file is the button,
//! its held key, and the card that says why a hearing stopped (`SessionComposer.tsx`, `Talk`).

use gpui::{div, prelude::*, svg, AnyElement, ClickEvent, Context, FontWeight, KeyDownEvent, KeyUpEvent, ModifiersChangedEvent, Role, SharedString, Task, Window};

use super::composer::Kept;
use super::head::icon_button;
use super::JobView;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::voice::{self, Engine, Heard, Hearing, Source, Why};

#[derive(Default)]
pub(super) struct Talk {
    /// The button is pressed: the person is being heard.
    pub listening: bool,
    /// The hearing was started by ⌥Space, so its release stops it.
    held: bool,
    hearing: Option<Hearing>,
    /// One per hearing: a Whisper hearing keeps giving words after it is released.
    tasks: Vec<Task<()>>,
    count: u64,
    /// Why the last hearing ended early, drawn as a card over the button.
    pub refused: Option<Why>,
}

/// What the card says for each reason: a title, the steps in order, and the Settings pane that
/// holds the fix, as `(microphone, button)`.
fn guide(why: Why) -> (&'static str, &'static [&'static str], Option<(bool, &'static str)>) {
    match why {
        Why::MicrophoneDenied => (
            "The microphone is off for Armada Voice",
            &["Open **Privacy & Security**, then **Microphone**.", "Turn on **Armada Voice**.", "Press Talk again."],
            Some((true, "Open Microphone settings")),
        ),
        Why::SpeechDenied => (
            "Speech recognition is off for Armada Voice",
            &["Open **Privacy & Security**, then **Speech Recognition**.", "Turn on **Armada Voice**.", "Press Talk again."],
            Some((false, "Open Speech Recognition settings")),
        ),
        Why::MicrophoneUnavailable => ("No microphone is available", &[], None),
        _ => ("Talk stopped", &[], None),
    }
}

impl JobView {
    /// ⌥Space in the box starts a hearing that its release stops.
    pub(super) fn talk_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let keys = &event.keystroke;
        if keys.modifiers.alt && !keys.modifiers.platform && !keys.modifiers.control && keys.key == "space" {
            if !event.is_held {
                self.talk_start(true, window, cx);
            }
            return true;
        }
        false
    }

    pub(super) fn talk_key_up(&mut self, event: &KeyUpEvent, cx: &mut Context<Self>) {
        if self.composer.talk.held && event.keystroke.key == "space" {
            self.talk_end(cx);
        }
    }

    pub(super) fn talk_modifiers(&mut self, event: &ModifiersChangedEvent, cx: &mut Context<Self>) {
        if self.composer.talk.held && !event.modifiers.alt {
            self.talk_end(cx);
        }
    }

    /// The card goes: Esc, a press elsewhere, coming back from Settings, or Talk pressed again.
    pub(super) fn talk_put_away(&mut self, cx: &mut Context<Self>) -> bool {
        let shown = self.composer.talk.refused.take().is_some();
        if shown {
            cx.notify();
        }
        shown
    }

    /// Listening shows at once; the voice starts behind it.
    pub(super) fn talk_start(&mut self, held: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.composer.talk.listening {
            return;
        }
        let Some(id) = self.shown.clone() else { return };
        let engine = self.composer.quick.voice();
        let mock = self.fleet.is_none();
        let whisper = crate::whisper::paths();
        if engine == Engine::Whisper && whisper.is_none() && !mock {
            // Pressing Talk on Whisper before it is here fetches it, and Apple's voice is heard meanwhile.
            crate::whisper::install();
        }
        let source = Source {
            helper: if mock { None } else { voice::helper() },
            whisper: if mock { None } else { whisper },
            script: mock.then(|| self.composer.catalog.scripts.get(id.as_ref()).cloned().unwrap_or_default()),
        };
        // The box takes focus first, with the caret at its end: a press on Talk leaves it on the button.
        self.composer.input.update(cx, |input, cx| {
            let end = input.value().len();
            input.set_selected_range(end..end, cx);
            input.focus(window, cx);
        });
        let talk = &mut self.composer.talk;
        talk.listening = true;
        talk.held = held;
        talk.refused = None;
        talk.count += 1;
        let mine = talk.count;
        let hearing = voice::hear(engine, source);
        let heard = hearing.heard.clone();
        talk.hearing = Some(hearing);
        talk.tasks.push(cx.spawn_in(window, async move |this, cx| {
            while let Ok(first) = heard.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = heard.try_recv() {
                    batch.push(more);
                }
                let over = batch.iter().any(|one| matches!(one, Heard::End(_)));
                if this.update_in(cx, |this, window, cx| this.heard(mine, batch, window, cx)).is_err() || over {
                    break;
                }
            }
        }));
        cx.notify();
    }

    /// The release: the voice finishes what it heard and ends.
    pub(super) fn talk_end(&mut self, cx: &mut Context<Self>) {
        let talk = &mut self.composer.talk;
        if let Some(hearing) = &talk.hearing {
            hearing.release();
        }
        talk.listening = false;
        talk.held = false;
        cx.notify();
    }

    /// Leaving the Job stops what was heard for it.
    pub(super) fn talk_leave(&mut self) {
        let talk = &mut self.composer.talk;
        if let Some(hearing) = talk.hearing.take() {
            hearing.release();
        }
        talk.tasks.clear();
        talk.listening = false;
        talk.held = false;
        talk.refused = None;
    }

    /// A batch from a hearing: its words join the box, folded into one redraw.
    fn heard(&mut self, mine: u64, batch: Vec<Heard>, window: &mut Window, cx: &mut Context<Self>) {
        let mut text = self.kept(cx).text;
        let held = self.kept(cx);
        let mut words = false;
        for one in &batch {
            match one {
                Heard::Word(word) => {
                    text = voice::joined(&text, word);
                    words = true;
                }
                // A hearing that ended on its own leaves the button; a stop he pressed already did, and a newer hearing is not this one's.
                Heard::End(why) if mine == self.composer.talk.count => {
                    let talk = &mut self.composer.talk;
                    talk.hearing = None;
                    talk.listening = false;
                    talk.held = false;
                    talk.refused = *why;
                }
                Heard::End(_) => {}
            }
        }
        if words {
            self.set_words(Kept { text, tags: held.tags }, window, cx);
        }
        cx.notify();
    }

    fn render_guide(&self, look: &Look, why: Why, cx: &Context<Self>) -> AnyElement {
        let (title, steps, pane) = guide(why);
        let card = labelled(
            div()
                .id("talk-guide")
                .occlude()
                .absolute()
                .bottom(look.length(token::H_CONTROL) + look.length(token::SPACE_2))
                .right_0()
                .w(look.length(token::SPACE_12) * 5.5)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.talk_put_away(cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(look.length(token::H_CONTROL))
                                .rounded(look.length(token::RADIUS_MD))
                                .bg(look.color(token::BG_SUNKEN))
                                .child(svg().path(icon_path("mic-off")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))),
                        )
                        .child(div().flex_1().min_w_0().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title)),
                )
                .children((!steps.is_empty()).then(|| {
                    div().flex().flex_col().gap(look.length(token::SPACE_2)).children(steps.iter().enumerate().map(|(at, step)| {
                        div()
                            .flex()
                            .gap(look.length(token::SPACE_2))
                            .child(div().flex_none().text_color(look.color(token::FG_MUTED)).child(format!("{}.", at + 1)))
                            .child(marked(look, step))
                    }))
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .children(pane.map(|(microphone, said)| {
                            labelled(
                                div()
                                    .id("talk-guide-open")
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .h(look.length(token::H_CONTROL))
                                    .px(look.length(token::SPACE_3))
                                    .rounded(look.length(token::RADIUS_MD))
                                    .bg(look.color(token::ACCENT))
                                    .text_color(look.color(token::TEXT_ON_ACCENT))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                                    .child(said)
                                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| cx.open_url(voice::settings_address(microphone)))),
                                Role::Button,
                                said,
                            )
                        }))
                        .child(
                            div()
                                .ml_auto()
                                .px(look.length(token::SPACE_1))
                                .rounded(look.length(token::RADIUS_SM))
                                .border(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_DEFAULT))
                                .text_size(look.length(token::TEXT_XS))
                                .text_color(look.color(token::FG_MUTED))
                                .child("Esc"),
                        ),
                ),
            Role::Status,
            "Talk could not hear",
        );
        // Painted after the box around it, whose edge would cross it otherwise.
        gpui::deferred(card).with_priority(1).into_any_element()
    }

    /// The mic, and over it the card where a hearing stopped for a reason.
    pub(super) fn render_talk(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let listening = self.composer.talk.listening;
        let button = labelled(
            icon_button(look, "talk", "mic", format!("Talk  {}", if cfg!(target_os = "macos") { "\u{2325}Space" } else { "Alt+Space" }).into(), listening, false)
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    if this.composer.talk.listening {
                        this.talk_end(cx);
                    } else {
                        this.talk_start(false, window, cx);
                    }
                })),
            Role::Button,
            "Talk",
        );
        div().relative().flex_none().child(button).children(self.composer.talk.refused.map(|why| self.render_guide(look, why, cx))).into_any_element()
    }
}

/// `**bold**` in a step, drawn as what it marks.
fn marked(look: &Look, text: &'static str) -> impl IntoElement {
    let mut line = div().flex().flex_wrap().flex_1().min_w_0();
    for (at, part) in text.split("**").enumerate() {
        if part.is_empty() {
            continue;
        }
        // A flex item drops the space at its edge, so the space is made one it keeps.
        let edge = |one: char| if one == ' ' { '\u{a0}' } else { one };
        let mut kept: Vec<char> = part.chars().collect();
        if let Some(first) = kept.first_mut() {
            *first = edge(*first);
        }
        if let Some(last) = kept.last_mut() {
            *last = edge(*last);
        }
        let part: SharedString = kept.into_iter().collect::<String>().into();
        line = line.child(if at % 2 == 1 { div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(part) } else { div().child(part) });
    }
    line
}
