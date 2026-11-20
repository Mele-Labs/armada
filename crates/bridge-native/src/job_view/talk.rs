//! Talk: a press on the mic, or ⌥Space held in the box, listens, and each word heard joins the
//! box. Nothing is sent by it. What a hearing is lives in `crate::voice`; this file is the button,
//! its held key, and the card that says why a hearing stopped (`SessionComposer.tsx`, `Talk`).
//! A Job's box and the New Job request both hear through it, each as a [`Hears`].

use gpui::{div, prelude::*, svg, AnyElement, ClickEvent, Context, Div, Entity, Stateful, FontWeight, KeyDownEvent, KeyUpEvent, ModifiersChangedEvent, Role, SharedString, Task, Window};
use gpui_component::input::TextareaState;

use super::composer::Kept;
use super::head::icon_button;
use super::JobView;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::voice::{self, Engine, Heard, Hearing, Source, Why};

#[derive(Default)]
pub(crate) struct Talk {
    /// The button is pressed: the person is being heard.
    pub listening: bool,
    /// The hearing was started by ⌥Space, so its release stops it.
    held: bool,
    hearing: Option<Hearing>,
    /// macOS Dictation is typing into the box, and the release is what stops it.
    dictating: bool,
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

/// A box Talk joins words to.
pub(crate) trait Hears: Sized + 'static {
    fn talk(&mut self) -> &mut Talk;
    fn talk_now(&self) -> &Talk;
    /// The text box the words join, which takes the keys while it hears.
    fn talk_box(&self) -> Entity<TextareaState>;
    /// The engine and where it hears from, or none where the box has nothing to hear for.
    fn talk_source(&mut self, cx: &mut Context<Self>) -> Option<(Engine, Source)>;
    /// Words heard, in order, to join the box's text.
    fn heard_words(&mut self, words: Vec<String>, window: &mut Window, cx: &mut Context<Self>);
}

/// The engine and source for a box: a mock box hears `script`, a live one the helper.
pub(crate) fn source_for(engine: Engine, mock: bool, script: impl FnOnce() -> voice::Script) -> Source {
    let whisper = crate::whisper::paths();
    if engine == Engine::Whisper && whisper.is_none() && !mock {
        // Pressing Talk on Whisper before it is here fetches it, and Apple's voice is heard meanwhile.
        crate::whisper::install();
    }
    Source { helper: if mock { None } else { voice::helper() }, whisper: if mock { None } else { whisper }, script: mock.then(script) }
}

impl Talk {
    /// Leaving the box stops what was heard for it.
    pub(crate) fn leave(&mut self) {
        if let Some(hearing) = self.hearing.take() {
            hearing.release();
        }
        if std::mem::take(&mut self.dictating) {
            crate::dictation::dictate(false);
        }
        self.tasks.clear();
        self.listening = false;
        self.held = false;
        self.refused = None;
    }
}

/// ⌥Space in the box starts a hearing that its release stops.
pub(crate) fn key_down<T: Hears>(this: &mut T, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<T>) -> bool {
    let keys = &event.keystroke;
    if keys.modifiers.alt && !keys.modifiers.platform && !keys.modifiers.control && keys.key == "space" {
        if !event.is_held {
            start(this, true, window, cx);
        }
        return true;
    }
    false
}

pub(crate) fn key_up<T: Hears>(this: &mut T, event: &KeyUpEvent, cx: &mut Context<T>) {
    if this.talk_now().held && event.keystroke.key == "space" {
        end(this, cx);
    }
}

pub(crate) fn modifiers<T: Hears>(this: &mut T, event: &ModifiersChangedEvent, cx: &mut Context<T>) {
    if this.talk_now().held && !event.modifiers.alt {
        end(this, cx);
    }
}

/// The card goes: Esc, a press elsewhere, coming back from Settings, or Talk pressed again.
pub(crate) fn put_away<T: Hears>(this: &mut T, cx: &mut Context<T>) -> bool {
    let shown = this.talk().refused.take().is_some();
    if shown {
        cx.notify();
    }
    shown
}

/// Listening shows at once; the voice starts behind it.
pub(crate) fn start<T: Hears>(this: &mut T, held: bool, window: &mut Window, cx: &mut Context<T>) {
    if this.talk_now().listening {
        return;
    }
    let Some((engine, source)) = this.talk_source(cx) else { return };
    // The box takes focus first, with the caret at its end: a press on Talk leaves it on the button.
    this.talk_box().update(cx, |input, cx| {
        let end = input.value().len();
        input.set_selected_range(end..end, cx);
        input.focus(window, cx);
    });
    let talk = this.talk();
    talk.listening = true;
    talk.held = held;
    talk.refused = None;
    talk.count += 1;
    let mine = talk.count;
    let hearing = voice::hear(engine, source);
    let heard = hearing.heard.clone();
    talk.hearing = Some(hearing);
    let task = cx.spawn_in(window, async move |this, cx| {
        while let Ok(first) = heard.recv().await {
            let mut batch = vec![first];
            while let Ok(more) = heard.try_recv() {
                batch.push(more);
            }
            let over = batch.iter().any(|one| matches!(one, Heard::End(_)));
            if this.update_in(cx, |this, window, cx| heard_batch(this, mine, batch, window, cx)).is_err() || over {
                break;
            }
        }
    });
    this.talk().tasks.push(task);
    cx.notify();
}

/// The release: the voice finishes what it heard and ends.
pub(crate) fn end<T: Hears>(this: &mut T, cx: &mut Context<T>) {
    let talk = this.talk();
    if let Some(hearing) = &talk.hearing {
        hearing.release();
    }
    if std::mem::take(&mut talk.dictating) {
        crate::dictation::dictate(false);
        talk.hearing = None;
    }
    talk.listening = false;
    talk.held = false;
    cx.notify();
}

/// A batch from a hearing: its words join the box, folded into one redraw.
fn heard_batch<T: Hears>(this: &mut T, mine: u64, batch: Vec<Heard>, window: &mut Window, cx: &mut Context<T>) {
    let mut words = Vec::new();
    for one in batch {
        let talk = this.talk();
        match one {
            Heard::Word(word) => words.push(word),
            // A hearing that ended on its own leaves the button; a stop he pressed already did, and a newer hearing is not this one's.
            Heard::End(why) if mine == talk.count => {
                talk.hearing = None;
                talk.listening = false;
                talk.held = false;
                talk.refused = why;
            }
            Heard::End(_) => {}
            // Dictation types into the box itself, so nothing is joined here; a Mac that will not start it says so.
            Heard::Dictate if mine == talk.count => {
                if crate::dictation::dictate(true) {
                    talk.dictating = true;
                } else {
                    talk.hearing = None;
                    talk.listening = false;
                    talk.held = false;
                    talk.refused = Some(Why::HelperFailed);
                }
            }
            Heard::Dictate => {}
        }
    }
    if !words.is_empty() {
        this.heard_words(words, window, cx);
    }
    cx.notify();
}

/// The words heard joined to `text`, as a box shows them.
pub(crate) fn joined_all(text: String, words: &[String]) -> String {
    words.iter().fold(text, |text, word| voice::joined(&text, word))
}

impl JobView {
    /// The voice Talk hears with, as the person set it; the New Job sheet hears with the same one.
    pub(crate) fn voice(&self) -> Engine {
        self.composer.quick.voice()
    }
}

impl Hears for JobView {
    fn talk(&mut self) -> &mut Talk {
        &mut self.composer.talk
    }

    fn talk_now(&self) -> &Talk {
        &self.composer.talk
    }

    fn talk_box(&self) -> Entity<TextareaState> {
        self.composer.input.clone()
    }

    fn talk_source(&mut self, _cx: &mut Context<Self>) -> Option<(Engine, Source)> {
        let id = self.shown.clone()?;
        let engine = self.composer.quick.voice();
        let script = || self.composer.catalog.scripts.get(id.as_ref()).cloned().unwrap_or_default();
        Some((engine, source_for(engine, self.fleet.is_none(), script)))
    }

    fn heard_words(&mut self, words: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.kept(cx);
        self.set_words(Kept { text: joined_all(held.text, &words), tags: held.tags }, window, cx);
    }
}

impl JobView {
    /// ⌥Space in the box starts a hearing that its release stops.
    pub(super) fn talk_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        key_down(self, event, window, cx)
    }

    pub(super) fn talk_key_up(&mut self, event: &KeyUpEvent, cx: &mut Context<Self>) {
        key_up(self, event, cx);
    }

    pub(super) fn talk_modifiers(&mut self, event: &ModifiersChangedEvent, cx: &mut Context<Self>) {
        modifiers(self, event, cx);
    }

    pub(super) fn talk_put_away(&mut self, cx: &mut Context<Self>) -> bool {
        put_away(self, cx)
    }

    /// Leaving the Job stops what was heard for it.
    pub(super) fn talk_leave(&mut self) {
        self.composer.talk.leave();
    }

    /// The mic, and over it the card where a hearing stopped for a reason.
    pub(super) fn render_talk(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        render(self, look, cx)
    }
}

fn render_guide<T: Hears>(look: &Look, why: Why, cx: &Context<T>) -> AnyElement {
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
                put_away(this, cx);
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
pub(crate) fn render<T: Hears>(this: &T, look: &Look, cx: &Context<T>) -> AnyElement {
    render_with(this, look, |button| button, cx)
}

/// The mic as [`render`] draws it, `dress`ed by the box it sits in: its focus, and the ring that shows it.
pub(crate) fn render_with<T: Hears>(this: &T, look: &Look, dress: impl FnOnce(Stateful<Div>) -> Stateful<Div>, cx: &Context<T>) -> AnyElement {
    let listening = this.talk_now().listening;
    let button = labelled(
        dress(icon_button(look, "talk", "mic", format!("Talk  {}", if cfg!(target_os = "macos") { "\u{2325}Space" } else { "Alt+Space" }).into(), listening, false))
            .on_click(cx.listener(|this: &mut T, _: &ClickEvent, window, cx| {
                if this.talk_now().listening {
                    end(this, cx);
                } else {
                    start(this, false, window, cx);
                }
            })),
        Role::Button,
        "Talk",
    );
    div().relative().flex_none().child(button).children(this.talk_now().refused.map(|why| render_guide(look, why, cx))).into_any_element()
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
