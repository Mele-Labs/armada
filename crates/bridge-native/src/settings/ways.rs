//! Settings › Ways of working: how he takes a Job's work in (read, look or listen) and how he answers it (type,
//! talk or press), as two rows of cards that save when pressed. Under the first row sit the chosen way's options,
//! and the ways not chosen fold into an Also set line. The cards are drawn here for the first-run screen too
//! (`first_run.rs`), larger.
//!
//! The wire is `take_in`, `answer_with`, `read_results`, `look_results`, `look_questions`, the `listen_*` switches,
//! `listen_voice` and `buttons_words` (`ipc::Preferences`). The engine Talk hears with is the `voice` key of the
//! `thread.look` setting, which stays where it is set: this page says which is in force and does not draw a second editor.

use std::time::Duration;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, Entity, FontWeight, KeyDownEvent, Role, SharedString, Stateful, Subscription, Task, Toggled, Window};
use ipc::{AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, TakeIn};

use super::overlay::SettingsOverlay;
use super::view::{bone, bounds_of, card, ghost_with, ringed, small, switch_track, tip};
use super::wire::{Put, Wire};
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::voice::{self, Access, Spoke, Voice};
use crate::whisper::State as Whisper;

/// The sentence the Listen card speaks.
pub const SAMPLE: &str = "The Settings window Job is done. A walk is waiting.";

// ---- the ways -------------------------------------------------------------------------------------------

/// Which row a way is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Take,
    Answer,
}

impl Row {
    pub fn title(self) -> &'static str {
        match self {
            Row::Take => "Take Jobs in",
            Row::Answer => "Answer Jobs",
        }
    }
}

/// One card: its word on the wire, its name and line, its glyph and the hue of its tile.
#[derive(Clone, Copy)]
pub struct Way {
    pub row: Row,
    pub word: &'static str,
    pub label: &'static str,
    pub line: &'static str,
    pub glyph: &'static str,
    pub ink: Token,
}

/// Read blue, Look teal, Listen amber, Type grey, Talk orange, Buttons green.
pub const WAYS: [Way; 6] = [
    Way { row: Row::Take, word: "read", label: "Read", line: "A short note, and a page for big work", glyph: "notebook-text", ink: token::ACCENT },
    Way { row: Row::Take, word: "look", label: "Look", line: "A picture leads every result and question", glyph: "image", ink: token::STATUS_HANDED_IN },
    Way { row: Row::Take, word: "listen", label: "Listen", line: "A Mac voice says when a Job ends or needs you", glyph: "bell", ink: token::STATUS_AWAITING_REVIEW },
    Way { row: Row::Answer, word: "type", label: "Type", line: "Write in the box under a Job", glyph: "text-cursor", ink: token::STATUS_NOT_STARTED },
    Way { row: Row::Answer, word: "talk", label: "Talk", line: "Hold ⌥Space anywhere and speak", glyph: "mic", ink: token::STATUS_ESCALATED },
    Way { row: Row::Answer, word: "buttons", label: "Buttons", line: "Every question comes with options to press", glyph: "message-circle-question-mark", ink: token::STATUS_COMPLETED_SUCCESS },
];

pub fn ways_of(row: Row) -> &'static [Way] {
    match row {
        Row::Take => &WAYS[..3],
        Row::Answer => &WAYS[3..],
    }
}

pub fn way(word: &str) -> Option<&'static Way> {
    WAYS.iter().find(|one| one.word == word)
}

/// A closed set's word as it rides on the wire.
pub fn word_of<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value).ok().and_then(|word| word.as_str().map(str::to_string)).unwrap_or_default()
}

/// What each choice of a set says, by its word: the wire word first.
pub const READ_RESULTS: [(&str, &str); 3] = [("note_then_page", "Short note, page for big work"), ("always_page", "Always a page"), ("thread_only", "Thread only")];
pub const LOOK_RESULTS: [(&str, &str); 3] = [("picture_every", "A picture for every result"), ("picture_screens", "Pictures only for screen work"), ("walk_visible", "A walk for anything visible")];
pub const LOOK_QUESTIONS: [(&str, &str); 2] = [("pictures_mix", "Pictures side by side, parts can mix"), ("pictures_whole", "Pictures side by side, whole only")];

/// The three switches Listen shows, by name on the wire. `listen_operator` is left off the page.
pub const LISTEN_SWITCHES: [(&str, &str, &str); 3] =
    [("listen_end_summary", "Summary when a Job ends", "Summary"), ("listen_needs_you", "Needs you, read aloud", "Needs you"), ("listen_progress", "Progress at each step", "Progress")];

/// The set a choice belongs to, with its heading.
struct Choice {
    name: &'static str,
    heading: &'static str,
    options: &'static [(&'static str, &'static str)],
}

const READ_CHOICES: [Choice; 1] = [Choice { name: "read_results", heading: "Results", options: &READ_RESULTS }];
const LOOK_CHOICES: [Choice; 2] = [Choice { name: "look_results", heading: "Results open on", options: &LOOK_RESULTS }, Choice { name: "look_questions", heading: "Questions", options: &LOOK_QUESTIONS }];

fn choices_of(word: &str) -> &'static [Choice] {
    match word {
        "read" => &READ_CHOICES,
        "look" => &LOOK_CHOICES,
        _ => &[],
    }
}

/// The word a choice holds now.
pub fn held_word(prefs: &Preferences, name: &str) -> String {
    match name {
        "take_in" => word_of(&prefs.take_in),
        "answer_with" => word_of(&prefs.answer_with),
        "read_results" => word_of(&prefs.read_results),
        "look_results" => word_of(&prefs.look_results),
        "look_questions" => word_of(&prefs.look_questions),
        _ => String::new(),
    }
}

/// A switch's state now.
pub fn held_switch(prefs: &Preferences, name: &str) -> bool {
    match name {
        "listen_end_summary" => prefs.listen_end_summary,
        "listen_needs_you" => prefs.listen_needs_you,
        "listen_progress" => prefs.listen_progress,
        "listen_operator" => prefs.listen_operator,
        "buttons_words" => prefs.buttons_words,
        "first_run_answered" => prefs.first_run_answered,
        _ => false,
    }
}

/// A save put into what is held, as it is drawn before Fleet answers. `false` where the name or word is not one.
pub fn apply(prefs: &mut Preferences, name: &str, put: &Put) -> bool {
    match (name, put) {
        ("layout_choices", Put::Text(text)) => prefs.layout_choices = text.clone(),
        ("time_format", Put::Text(text)) => prefs.time_format = text.clone(),
        ("take_in", Put::Text(text)) => match TakeIn::from_word(text) {
            Some(word) => prefs.take_in = word,
            None => return false,
        },
        ("answer_with", Put::Text(text)) => match AnswerWith::from_word(text) {
            Some(word) => prefs.answer_with = word,
            None => return false,
        },
        ("read_results", Put::Text(text)) => match ReadResults::from_word(text) {
            Some(word) => prefs.read_results = word,
            None => return false,
        },
        ("look_results", Put::Text(text)) => match LookResults::from_word(text) {
            Some(word) => prefs.look_results = word,
            None => return false,
        },
        ("look_questions", Put::Text(text)) => match LookQuestions::from_word(text) {
            Some(word) => prefs.look_questions = word,
            None => return false,
        },
        ("listen_voice", Put::Text(text)) => prefs.listen_voice = text.trim().to_string(),
        ("first_run_answered", Put::Switch(on)) => prefs.first_run_answered = *on,
        ("listen_end_summary", Put::Switch(on)) => prefs.listen_end_summary = *on,
        ("listen_needs_you", Put::Switch(on)) => prefs.listen_needs_you = *on,
        ("listen_progress", Put::Switch(on)) => prefs.listen_progress = *on,
        ("listen_operator", Put::Switch(on)) => prefs.listen_operator = *on,
        ("buttons_words", Put::Switch(on)) => prefs.buttons_words = *on,
        _ => return false,
    }
    true
}

fn said_by(options: &[(&str, &'static str)], word: &str) -> &'static str {
    options.iter().find(|(wire, _)| *wire == word).map_or("", |(_, said)| said)
}

/// A way's options in one line, for the chip it folds into: what Read, Look or Listen is set to now. `voice` is the
/// name of the voice Listen reads with, empty for the Mac's own.
pub fn also_set(prefs: &Preferences, word: &str, voice: &str) -> String {
    match word {
        "read" => said_by(&READ_RESULTS, &held_word(prefs, "read_results")).to_string(),
        "look" => {
            let results = match held_word(prefs, "look_results").as_str() {
                "picture_screens" => "Pictures for screen work",
                "walk_visible" => "Walk for anything visible",
                _ => "Picture for every result",
            };
            let questions = if held_word(prefs, "look_questions") == "pictures_whole" { "whole only" } else { "parts can mix" };
            format!("{results}, {questions}")
        }
        "listen" => {
            let mut parts: Vec<&str> = LISTEN_SWITCHES.iter().filter(|(name, ..)| held_switch(prefs, name)).map(|(.., short)| *short).collect();
            if parts.is_empty() {
                parts.push("Nothing read aloud");
            }
            parts.push(if voice.is_empty() { "Mac voice" } else { voice });
            parts.join(", ")
        }
        _ => String::new(),
    }
}

// ---- the cards, as the first run draws them too ---------------------------------------------------------

/// How large a card is drawn: on the Settings page, or on the first-run screen.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Page,
    Large,
}

/// One card, not yet pressable: the caller gives it its press and its label (`Role::RadioButton`, `way.label`).
/// A chosen card has the accent edge, the accent fill and the check in its corner; `ring` is the row's focus resting on it.
pub fn way_card(look: &Look, way: &Way, chosen: bool, ring: bool, size: Size, id: impl Into<gpui::ElementId>) -> Stateful<gpui::Div> {
    let large = size == Size::Large;
    let ink = look.color(way.ink);
    let tile = look.length(if large { token::SPACE_12 } else { token::H_CONTROL_LG });
    let glyph = look.length(if large { token::SPACE_6 } else { token::SPACE_4 });
    let edge = if ring { look.length(token::FOCUS_RING_WIDTH) } else { look.length(token::BORDER_WIDTH) };
    let rim = if ring {
        super::view::ring_colour(look)
    } else if chosen {
        look.color(token::ACCENT)
    } else {
        look.color(token::BORDER_SUBTLE)
    };
    div()
        .id(id.into())
        .relative()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(look.length(if large { token::SPACE_3 } else { token::SPACE_2 }))
        .p(look.length(if large { token::PAD_CARD } else { token::SPACE_3 }))
        .rounded(look.length(token::RADIUS_LG))
        .border(edge)
        .border_color(rim)
        .bg(if chosen { look.color(token::ACCENT_FAINT) } else { look.color(token::BG_RAISED) })
        .cursor_pointer()
        .when(!chosen, |card| card.hover(|style| style.bg(look.color(token::BG_HOVER))))
        .child(
            div()
                .flex_none()
                .size(tile)
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .bg(ink.opacity(0.22))
                .child(svg().path(icon_path(way.glyph)).size(glyph).text_color(ink)),
        )
        .child(
            div()
                .text_size(look.length(if large { token::TEXT_BASE } else { token::TEXT_SM }))
                .line_height(look.length(if large { token::LEADING_BASE } else { token::LEADING_SM }))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(way.label),
        )
        .child(
            div()
                .text_size(look.length(if large { token::TEXT_SM } else { token::TEXT_XS }))
                .line_height(look.length(if large { token::LEADING_SM } else { token::LEADING_XS }))
                .text_color(look.color(token::FG_MUTED))
                .child(way.line),
        )
        .when(chosen, |card| {
            card.child(
                div()
                    .absolute()
                    .top(look.length(token::SPACE_2))
                    .right(look.length(token::SPACE_2))
                    .size(look.length(token::SPACE_4))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(look.color(token::ACCENT))
                    .child(svg().path(icon_path("check")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_INVERSE))),
            )
        })
        .aria_toggled(if chosen { Toggled::True } else { Toggled::False })
}

/// A row of three cards as one radio group. The group holds the keys: ← and → choose within it.
pub fn way_row(look: &Look, id: &'static str, row: Row, cards: Vec<AnyElement>, focus: &gpui::FocusHandle, size: Size) -> AnyElement {
    labelled(
        div().id(id).track_focus(focus).flex().gap(look.length(if size == Size::Large { token::SPACE_4 } else { token::SPACE_3 })).children(cards),
        Role::RadioGroup,
        row.title(),
    )
    .into_any_element()
}

/// The way a press of ← or → on a row moves to: one along, stopping at the ends.
pub fn moved(row: Row, from: &str, by: isize) -> Option<&'static Way> {
    let ways = ways_of(row);
    let at = ways.iter().position(|one| one.word == from)? as isize;
    let to = (at + by).clamp(0, ways.len() as isize - 1) as usize;
    (to as isize != at).then(|| &ways[to])
}

// ---- a sample spoken ------------------------------------------------------------------------------------

/// What the sample is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sample {
    Idle,
    Speaking,
    /// The voice asked for is not on this Mac.
    VoiceMissing,
    /// The helper is not here, or did not answer.
    NoHelper,
}

impl Sample {
    /// The sentence that says why nothing is being spoken, where there is a reason.
    pub fn problem(self) -> Option<&'static str> {
        match self {
            Sample::VoiceMissing => Some("That voice is not on this Mac."),
            Sample::NoHelper => Some("Armada Voice did not answer."),
            Sample::Idle | Sample::Speaking => None,
        }
    }
}

/// The sample's one speaker, kept by whoever draws the press: a Settings page or the first-run screen.
pub struct Sampler {
    pub state: Sample,
    speech: Option<voice::Speech>,
    _task: Option<Task<()>>,
}

impl Sampler {
    pub fn new() -> Sampler {
        Sampler { state: Sample::Idle, speech: None, _task: None }
    }

    /// The press: speak the sentence in `voice`, or stop it where it is being said. On mock data and in walks nothing
    /// is spawned: the speaking state is shown for a moment.
    pub fn press(&mut self, voice: &str, mock: bool, cx: &mut Context<Sampler>) {
        if self.state == Sample::Speaking {
            if let Some(speech) = &self.speech {
                speech.stop();
            }
            if mock {
                self._task = None;
                self.state = Sample::Idle;
            }
            cx.notify();
            return;
        }
        self.speech = None;
        self.state = Sample::Speaking;
        if mock {
            self._task = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(4000)).await;
                let _ = this.update(cx, |this, cx| {
                    this.state = Sample::Idle;
                    cx.notify();
                });
            }));
            cx.notify();
            return;
        }
        let Some(helper) = voice::helper() else {
            self.state = Sample::NoHelper;
            cx.notify();
            return;
        };
        let speech = voice::speak(helper, voice, SAMPLE);
        let said = speech.said.clone();
        self.speech = Some(speech);
        self._task = Some(cx.spawn(async move |this, cx| {
            while let Ok(spoke) = said.recv().await {
                let over = !matches!(spoke, Spoke::Speaking);
                let _ = this.update(cx, |this, cx| {
                    this.state = match spoke {
                        Spoke::Speaking => Sample::Speaking,
                        Spoke::Spoken | Spoke::Stopped => Sample::Idle,
                        Spoke::Missing => Sample::VoiceMissing,
                        Spoke::Failed => Sample::NoHelper,
                    };
                    if over {
                        this.speech = None;
                    }
                    cx.notify();
                });
                if over {
                    break;
                }
            }
        }));
        cx.notify();
    }
}

/// "Hear a sample" and what it is doing beside it: a speaking mark that says so, or the reason there is no sound.
pub fn sample_press(look: &Look, state: Sample, off: bool, focus: Option<&gpui::FocusHandle>, window: &Window) -> (Stateful<gpui::Div>, Option<AnyElement>) {
    let button = ghost_with(look, "ways-sample", Some("play"), "Hear a sample", off).border_color(look.color(token::BORDER_DEFAULT));
    let button = match focus {
        Some(focus) => ringed(look, button.track_focus(focus), focus, window),
        None => button,
    };
    let beside: Option<AnyElement> = match state {
        Sample::Speaking => Some(
            labelled(
                div()
                    .id("ways-speaking")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .tooltip(tip("The sample is being spoken"))
                    .child(svg().path(icon_path("radio")).size(look.length(token::SPACE_4)).text_color(look.color(token::STATUS_RUNNING)))
                    .child(small(look, "Speaking", look.color(token::STATUS_RUNNING))),
                Role::Status,
                "Speaking",
            )
            .into_any_element(),
        ),
        other => other.problem().map(|said| labelled(small(look, said, look.color(token::STATUS_COMPLETED_FAILED)).id("ways-sample-said"), Role::Alert, said).into_any_element()),
    };
    (button, beside)
}

// ---- the page's state -----------------------------------------------------------------------------------

/// What the page holds besides the preferences: the options opened by a chip, and what the helper and Whisper said.
pub struct WaysPage {
    /// The way whose options are drawn, where a chip opened one that is not the chosen. `None` is the chosen.
    pub opened: Option<&'static str>,
    pub voices: Option<Result<Vec<Voice>, String>>,
    /// What the microphone may do: `None` until read, `Some(None)` where the helper could not say.
    pub microphone: Option<Option<Access>>,
    pub whisper: Option<Whisper>,
    asked: bool,
    pub sample: Entity<Sampler>,
    _watch: Subscription,
    _read: Option<Task<()>>,
    _install: Option<Task<()>>,
}

impl WaysPage {
    pub fn new(cx: &mut Context<SettingsOverlay>) -> WaysPage {
        let sample = cx.new(|_| Sampler::new());
        let watch = cx.observe(&sample, |_, _, cx| cx.notify());
        WaysPage { opened: None, voices: None, microphone: None, whisper: None, asked: false, sample, _watch: watch, _read: None, _install: None }
    }

    /// Settings opened again: read the helper afresh, and show the chosen way's options.
    pub fn reopen(&mut self) {
        self.asked = false;
        self.opened = None;
    }

    fn voice_name(&self, id: &str) -> String {
        match &self.voices {
            Some(Ok(voices)) => voices.iter().find(|one| one.id == id).map_or(id.to_string(), |one| one.name.clone()),
            _ => id.to_string(),
        }
    }
}

/// Every focus key the page can draw, made before it is.
pub fn key_names() -> Vec<String> {
    let mut names = vec!["ways:take".to_string(), "ways:answer".into(), "ways:voice".into(), "ways:sample".into(), "ways:whisper".into(), "ways:microphone".into(), "ways:sw:buttons_words".into()];
    names.extend(WAYS.iter().filter(|one| one.row == Row::Take).map(|one| format!("ways:chip:{}", one.word)));
    names.extend(LISTEN_SWITCHES.iter().map(|(name, ..)| format!("ways:sw:{name}")));
    for word in ["read", "look"] {
        for choice in choices_of(word) {
            names.extend(choice.options.iter().map(|(option, _)| format!("ways:opt:{}:{option}", choice.name)));
        }
    }
    names
}

fn mic_words(access: Option<Option<Access>>) -> (&'static str, &'static str, &'static str, Token) {
    match access {
        None => ("circle-dashed", "Reading", "Reading what this Mac allows", token::FG_SUBTLE),
        Some(None) => ("circle-alert", "Not known", "Armada Voice did not say what this Mac allows", token::STATUS_AWAITING_REVIEW),
        Some(Some(Access::Granted)) => ("check", "On", "The microphone is on for Armada Voice", token::STATUS_COMPLETED_SUCCESS),
        Some(Some(Access::Denied)) => ("x", "Off", "The microphone is off for Armada Voice", token::STATUS_COMPLETED_FAILED),
        Some(Some(Access::Restricted)) => ("ban", "Restricted", "This Mac restricts the microphone", token::STATUS_COMPLETED_FAILED),
        Some(Some(Access::Unasked)) => ("circle-dashed", "Not asked yet", "The microphone is asked for at the first press of Talk", token::FG_MUTED),
        Some(Some(Access::Unknown)) => ("circle-alert", "Not known", "This Mac did not say what the microphone allows", token::STATUS_AWAITING_REVIEW),
    }
}

/// The engine Talk hears with, as `thread.look` holds it.
pub fn engine_name(engine: voice::Engine) -> &'static str {
    match engine {
        voice::Engine::Apple => "Apple on-device",
        voice::Engine::Whisper => "Whisper",
        voice::Engine::Dictation => "macOS Dictation",
    }
}

fn voice_ids() -> &'static [&'static str] {
    static IDS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| (0..1024).map(|n| &*Box::leak(format!("v{n}").into_boxed_str())).collect())
}

/// The entries of the voice list: the Mac's own first, then Premium, Enhanced and the rest, each under its heading.
fn voice_entries(voices: &[Voice], now: &str) -> Vec<Entry> {
    let ids = voice_ids();
    let mut entries = vec![Entry::Search { label: "Find a voice", to_rule: false }, Entry::Choice { id: ids[0], label: "Mac default".into(), on: now.is_empty() }];
    let mut heading = None;
    for (n, one) in voices.iter().enumerate().take(ids.len() - 1) {
        let part = match one.quality {
            voice::Quality::Premium => "Premium",
            voice::Quality::Enhanced => "Enhanced",
            voice::Quality::Default => "Standard",
        };
        if heading != Some(part) {
            entries.push(Entry::Label(part));
            heading = Some(part);
        }
        let label: SharedString = if one.language.is_empty() { one.name.clone().into() } else { format!("{} ({})", one.name, one.language).into() };
        entries.push(Entry::Choice { id: ids[n + 1], label, on: one.id == now });
    }
    entries
}

// ---- the page -------------------------------------------------------------------------------------------

impl SettingsOverlay {
    fn mock_wire(&self) -> bool {
        matches!(self.wire, Wire::Mock(_))
    }

    fn chosen(&self, name: &str) -> String {
        self.prefs.held().map(|held| held_word(held, name)).unwrap_or_default()
    }

    /// The way whose options are drawn under the first row.
    fn opened_take(&self) -> &'static str {
        self.ways.opened.unwrap_or_else(|| way(&self.chosen("take_in")).map_or("read", |one| one.word))
    }

    /// Read what the helper and Whisper say, once each time Settings opens on this page. Off the main thread.
    pub(super) fn read_ways(&mut self, cx: &mut Context<Self>) {
        if self.ways.asked {
            return;
        }
        self.ways.asked = true;
        if self.mock_wire() {
            self.ways.voices = Some(Ok(voice::mock_voices()));
            self.ways.microphone = Some(Some(Access::Denied));
            self.ways.whisper.get_or_insert(Whisper::Absent);
            return;
        }
        self.ways._read = Some(cx.spawn(async move |this, cx| {
            let read = cx
                .background_executor()
                .spawn(async {
                    match voice::helper() {
                        None => (Err("Armada Voice is not installed.".to_string()), None, crate::whisper::state()),
                        Some(helper) => (voice::list_voices(&helper).map_err(|_| "Armada Voice did not list its voices.".to_string()), voice::microphone(&helper), crate::whisper::state()),
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.ways.voices = Some(read.0);
                this.ways.microphone = Some(read.1);
                this.ways.whisper = Some(read.2);
                cx.notify();
            });
        }));
    }

    /// The keys, in the order Tab reaches them: the first row, what sits under it, the second row and what sits under that.
    pub(super) fn ways_stops(&self, _cx: &App) -> Vec<gpui::FocusHandle> {
        self.ways_names().into_iter().filter_map(|name| self.prefs.keys.get(&name).cloned()).collect()
    }

    fn ways_names(&self) -> Vec<String> {
        let mut names = vec!["ways:take".to_string()];
        let opened = self.opened_take();
        for choice in choices_of(opened) {
            names.extend(choice.options.iter().map(|(option, _)| format!("ways:opt:{}:{option}", choice.name)));
        }
        if opened == "listen" {
            names.extend(LISTEN_SWITCHES.iter().map(|(name, ..)| format!("ways:sw:{name}")));
            names.push("ways:voice".into());
            names.push("ways:sample".into());
        }
        names.extend(ways_of(Row::Take).iter().filter(|one| one.word != opened).map(|one| format!("ways:chip:{}", one.word)));
        names.push("ways:answer".into());
        match self.chosen("answer_with").as_str() {
            "buttons" => names.push("ways:sw:buttons_words".into()),
            "talk" => {
                if self.ways.microphone == Some(Some(Access::Denied)) {
                    names.push("ways:microphone".into());
                }
                if matches!(self.ways.whisper, Some(Whisper::Absent)) {
                    names.push("ways:whisper".into());
                }
            }
            _ => {}
        }
        names
    }

    /// A chip pressed: that way's options are drawn, and the keys go to the first of them, since the chip is gone.
    fn open_way(&mut self, word: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.ways.opened = Some(word);
        let names = self.ways_names();
        let first = names.get(1).filter(|name| name.starts_with("ways:opt:") || name.starts_with("ways:sw:")).unwrap_or(&names[0]);
        if let Some(focus) = self.prefs.keys.get(first) {
            window.focus(focus, cx);
        }
        cx.notify();
    }

    /// A card pressed, or reached by ← and →: saved at once.
    fn choose_way(&mut self, row: Row, word: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let name = if row == Row::Take { "take_in" } else { "answer_with" };
        if row == Row::Take {
            self.ways.opened = None;
        }
        if self.chosen(name) != word {
            self.save_put_pref(name, Put::Text(word.to_string()), window, cx);
        }
        cx.notify();
    }

    /// The keys of a row of cards: ← and → choose along it. `true` where the row held the keys.
    pub(super) fn ways_key(&mut self, key: &gpui::Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt || !matches!(key.key.as_str(), "left" | "right") {
            return false;
        }
        for (row, name, focus) in [(Row::Take, "take_in", "ways:take"), (Row::Answer, "answer_with", "ways:answer")] {
            if self.prefs.keys.get(focus).is_some_and(|handle| handle.is_focused(window)) {
                if let Some(to) = moved(row, &self.chosen(name), if key.key == "left" { -1 } else { 1 }) {
                    self.choose_way(row, to.word, window, cx);
                }
                return true;
            }
        }
        false
    }

    pub(super) fn render_ways(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let body = match &self.prefs.read {
            None => div().flex().flex_col().gap(look.length(token::SPACE_3)).children((0..3).map(|n| bone(look, look.length(token::SPACE_6) * (7 - n % 2) as f32, look.length(token::LEADING_SM)))).into_any_element(),
            Some(Err(said)) => labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("ways-unread"), Role::Alert, said.clone()).into_any_element(),
            Some(Ok(_)) => {
                return vec![
                    labelled(card(look, "settings-card-take-in", "Take Jobs in", None, self.take_rows(look, window, cx)), Role::Region, "Take Jobs in").into_any_element(),
                    labelled(card(look, "settings-card-answer", "Answer Jobs", None, self.answer_rows(look, window, cx)), Role::Region, "Answer Jobs").into_any_element(),
                ];
            }
        };
        vec![labelled(card(look, "settings-card-ways", "Ways of working", None, vec![body]), Role::Region, "Ways of working").into_any_element()]
    }

    fn refusal(&self, look: &Look, name: &'static str) -> Option<AnyElement> {
        self.prefs.refused.get(name).map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id(SharedString::from(format!("ways-refused-{name}"))), Role::Alert, said.clone()).into_any_element())
    }

    fn cards_of(&self, row: Row, name: &'static str, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let focus = self.prefs.keys.get(if row == Row::Take { "ways:take" } else { "ways:answer" }).cloned();
        let ring = focus.as_ref().is_some_and(|focus| focus.is_focused(window));
        let held = self.chosen(name);
        let off = !self.live;
        let cards = ways_of(row)
            .iter()
            .map(|one| {
                let chosen = one.word == held;
                let word = one.word;
                let card = way_card(look, one, chosen, ring && chosen, Size::Page, SharedString::from(format!("way-{word}"))).when(!off, |card| {
                    card.on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(focus) = this.prefs.keys.get(if row == Row::Take { "ways:take" } else { "ways:answer" }) {
                            window.focus(focus, cx);
                        }
                        this.choose_way(row, word, window, cx);
                    }))
                });
                labelled(card, Role::RadioButton, one.label).into_any_element()
            })
            .collect();
        match focus {
            Some(focus) => way_row(look, if row == Row::Take { "ways-take" } else { "ways-answer" }, row, cards, &focus, Size::Page),
            None => div().into_any_element(),
        }
    }

    fn take_rows(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let opened = self.opened_take();
        let mut rows = vec![self.cards_of(Row::Take, "take_in", look, window, cx)];
        rows.extend(self.refusal(look, "take_in"));
        rows.push(self.way_options(opened, look, window, cx));
        let held = self.prefs.held().cloned().unwrap_or_default();
        let voice = self.ways.voice_name(&held.listen_voice);
        let others: Vec<AnyElement> = ways_of(Row::Take)
            .iter()
            .filter(|one| one.word != opened)
            .map(|one| {
                let word = one.word;
                let focus = self.prefs.keys.get(&format!("ways:chip:{word}")).cloned();
                let ink = look.color(one.ink);
                let said = format!("Open {} options", one.label);
                let chip = div()
                    .id(SharedString::from(format!("ways-chip-{word}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(ink.opacity(0.4))
                    .bg(ink.opacity(0.12))
                    .cursor_pointer()
                    .hover(|style| style.bg(ink.opacity(0.2)))
                    .child(svg().path(icon_path(one.glyph)).size(look.length(token::SPACE_4)).text_color(ink))
                    .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(one.label))
                    .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(also_set(&held, word, &voice))))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_way(word, window, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.open_way(word, window, cx);
                        }
                    }));
                let chip = match &focus {
                    Some(focus) => ringed(look, chip.track_focus(focus), focus, window),
                    None => chip,
                };
                labelled(chip, Role::Button, said).into_any_element()
            })
            .collect();
        rows.push(
            labelled(
                div()
                    .id("ways-also-set")
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(small(look, "Also set", look.color(token::FG_MUTED)))
                    .children(others),
                Role::Group,
                "Also set",
            )
            .into_any_element(),
        );
        rows
    }

    fn answer_rows(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut rows = vec![self.cards_of(Row::Answer, "answer_with", look, window, cx)];
        rows.extend(self.refusal(look, "answer_with"));
        match self.chosen("answer_with").as_str() {
            "buttons" => rows.push(self.panel("buttons", look, vec![self.switch_row("buttons_words", "Answer in words too", look, window, cx)])),
            "talk" => rows.push(self.talk_panel(look, window, cx)),
            _ => {}
        }
        rows
    }

    /// An options card: its left edge in the way's hue.
    fn panel(&self, word: &str, look: &Look, body: Vec<AnyElement>) -> AnyElement {
        let one = way(word);
        let ink = one.map_or(look.color(token::ACCENT), |one| look.color(one.ink));
        let name = format!("{} options", one.map_or("Ways", |one| one.label));
        labelled(
            div()
                .id(SharedString::from(format!("ways-options-{word}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .p(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_MD))
                .border_l(look.length(token::SPACE_1))
                .border_color(ink)
                .bg(look.color(token::BG_BASE))
                .children(body),
            Role::Group,
            name,
        )
        .into_any_element()
    }

    fn way_options(&self, word: &'static str, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let mut body: Vec<AnyElement> = choices_of(word).iter().map(|choice| self.choice(choice, look, window, cx)).collect();
        if word == "listen" {
            body.extend(LISTEN_SWITCHES.iter().map(|(name, said, _)| self.switch_row(name, said, look, window, cx)));
            body.push(self.voice_row(look, window, cx));
        }
        self.panel(word, look, body)
    }

    /// One closed set as radio rows under its heading: a press saves the word.
    fn choice(&self, choice: &'static Choice, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let held = self.chosen(choice.name);
        let off = !self.live;
        let rows = choice.options.iter().map(|(word, said)| {
            let on = *word == held;
            let focus = self.prefs.keys.get(&format!("ways:opt:{}:{word}", choice.name)).cloned();
            let (name, word): (&'static str, &'static str) = (choice.name, word);
            let dot = div()
                .flex_none()
                .size(look.length(token::SPACE_4))
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_STRONG) })
                .flex()
                .items_center()
                .justify_center()
                .children(on.then(|| div().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::ACCENT))));
            let row = div()
                .id(SharedString::from(format!("ways-opt-{name}-{word}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .mx(-look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .child(dot)
                .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(*said))
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .when_some(focus, |row, focus| ringed(look, row.track_focus(&focus), &focus, window))
                .when(!off, |row| {
                    row.cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| this.save_put_pref(name, Put::Text(word.to_string()), window, cx)))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.save_put_pref(name, Put::Text(word.to_string()), window, cx);
                            }
                        }))
                });
            labelled(row, Role::RadioButton, *said).into_any_element()
        });
        let refused = self.refusal(look, choice.name);
        labelled(
            div().id(SharedString::from(format!("ways-choice-{}", choice.name))).flex().flex_col().gap(look.length(token::SPACE_2)).child(small(look, choice.heading, look.color(token::FG_MUTED))).children(rows).children(refused),
            Role::RadioGroup,
            choice.heading,
        )
        .into_any_element()
    }

    /// A switch row as Time draws one: its words, and the switch at the right.
    fn switch_row(&self, name: &'static str, said: &'static str, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let on = self.prefs.held().is_some_and(|held| held_switch(held, name));
        let off = !self.live;
        let focus = self.prefs.keys.get(&format!("ways:sw:{name}")).cloned();
        let row = div()
            .id(SharedString::from(format!("ways-sw-{name}")))
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .mx(-look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(said))
            .child(switch_track(look, on, off))
            .when_some(focus, |row, focus| ringed(look, row.track_focus(&focus), &focus, window))
            .when(!off, |row| {
                row.cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| this.flip_way_switch(name, window, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.flip_way_switch(name, window, cx);
                        }
                    }))
            });
        let refused = self.refusal(look, name);
        div().flex().flex_col().gap(look.length(token::SPACE_1)).child(labelled(row, Role::Switch, said)).children(refused).into_any_element()
    }

    fn flip_way_switch(&mut self, name: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let on = self.prefs.held().is_some_and(|held| held_switch(held, name));
        self.save_put_pref(name, Put::Switch(!on), window, cx);
    }

    /// Listen's voice: a list from the helper, and the sample spoken in the one picked.
    fn voice_row(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let off = !self.live;
        let held = self.prefs.held().map(|held| held.listen_voice.clone()).unwrap_or_default();
        let said = if held.is_empty() { "Mac default".to_string() } else { self.ways.voice_name(&held) };
        let focus = self.prefs.keys.get("ways:voice").cloned();
        let trigger = div()
            .id("ways-voice")
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .min_w(look.length(token::SPACE_12) * 4.)
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_SM))
            .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
            .child(div().min_w_0().truncate().child(SharedString::from(said)))
            .child(svg().path(icon_path("chevron-down")).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
            .when_some(focus.clone(), |trigger, focus| ringed(look, trigger.track_focus(&focus), &focus, window))
            .when(!off, |trigger| {
                trigger
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.open_voices(false, window, cx)))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space" | "down") {
                            this.open_voices(true, window, cx);
                            cx.stop_propagation();
                        }
                    }))
            });
        let read_failed = match &self.ways.voices {
            Some(Err(said)) => Some(labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("ways-voices-said"), Role::Alert, said.clone()).into_any_element()),
            _ => None,
        };
        let state = self.ways.sample.read(cx).state;
        let (button, beside) = sample_press(look, state, off, self.prefs.keys.get("ways:sample"), window);
        let press = {
            let voice = held.clone();
            let mock = self.mock_wire();
            let button = button.when(!off, |button| {
                let key_voice = voice.clone();
                button
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if let Some(focus) = this.prefs.keys.get("ways:sample") {
                            window.focus(focus, cx);
                        }
                        let voice = voice.clone();
                        this.ways.sample.update(cx, |sampler, cx| sampler.press(&voice, mock, cx));
                    }))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            let voice = key_voice.clone();
                            this.ways.sample.update(cx, |sampler, cx| sampler.press(&voice, mock, cx));
                        }
                    }))
            });
            labelled(button, Role::Button, "Hear a sample")
        };
        let refused = self.refusal(look, "listen_voice");
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child("Voice"))
                    .child(labelled(trigger, Role::ComboBox, "Voice")),
            )
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).child(press).children(beside))
            .children(read_failed)
            .children(refused)
            .into_any_element()
    }

    /// The voice list, open under its control. A voice picked is saved at once.
    fn open_voices(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Ok(voices)) = self.ways.voices.clone() else { return };
        let now = self.prefs.held().map(|held| held.listen_voice.clone()).unwrap_or_default();
        let at = bounds_of(window, cx, Role::ComboBox, "Voice").map(|bounds| bounds.bottom_left()).unwrap_or_default();
        let back = window.focused(cx);
        let entries = voice_entries(&voices, &now);
        let menu = cx.new(|cx| Dropdown::new("Voice", entries, at, gpui::Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            let Some(n) = id.strip_prefix('v').and_then(|n| n.parse::<usize>().ok()) else { return };
            let word = if n == 0 { String::new() } else { voices.get(n - 1).map(|one| one.id.clone()).unwrap_or_default() };
            this.save_put_pref("listen_voice", Put::Text(word), window, cx);
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &gpui::DismissEvent, window, cx| {
            this.popup = None;
            if let Some(back) = &back {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = gpui::Focusable::focus_handle(menu.read(cx), cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some((menu, vec![picked, dismissed]));
        cx.notify();
    }

    /// Talk's options: the engine in force, what the microphone allows, and Whisper.
    fn talk_panel(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let engine = match self.setting("thread.look").map(|one| &one.value) {
            Some(ipc::SettingValue::Map(map)) => match map.get("voice") {
                Some(ipc::SettingValue::Text(name)) => voice::Engine::named(name),
                _ => voice::Engine::Apple,
            },
            _ => voice::Engine::Apple,
        };
        let row = |name: &'static str, value: AnyElement| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_CONTROL))
                .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(name))
                .child(value)
        };
        let muted = |said: SharedString| div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(said).into_any_element();
        let engine_row = labelled(
            row("Voice engine", muted(engine_name(engine).into())).id("ways-engine").tooltip(tip("Set in the Aa panel of a Job's thread")),
            Role::Group,
            "Voice engine",
        )
        .into_any_element();
        let (glyph, word, tooltip, ink) = mic_words(self.ways.microphone);
        let mark = labelled(
            div()
                .id("ways-microphone-state")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .tooltip(tip(tooltip))
                .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(ink)))
                .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(word)),
            Role::Status,
            format!("Microphone: {word}"),
        )
        .into_any_element();
        let fix = (self.ways.microphone == Some(Some(Access::Denied))).then(|| {
            let focus = self.prefs.keys.get("ways:microphone").cloned();
            let button = ghost_with(look, "ways-open-microphone", None, "Open Microphone settings", false).border_color(look.color(token::BORDER_DEFAULT));
            let button = match &focus {
                Some(focus) => ringed(look, button.track_focus(focus), focus, window),
                None => button,
            };
            let open = |this: &mut SettingsOverlay, cx: &mut Context<SettingsOverlay>| this.wire.open_link(voice::settings_address(true), cx);
            labelled(
                button
                    .on_click(cx.listener(move |this, _, _, cx| open(this, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            open(this, cx);
                        }
                    })),
                Role::Button,
                "Open Microphone settings",
            )
            .into_any_element()
        });
        let microphone = row("Microphone", div().flex().items_center().gap(look.length(token::SPACE_3)).child(mark).children(fix).into_any_element());
        let whisper = self.ways.whisper;
        let whisper_value: AnyElement = match whisper {
            None => bone(look, look.length(token::SPACE_12), look.length(token::LEADING_SM)).into_any_element(),
            Some(Whisper::Absent) => {
                let focus = self.prefs.keys.get("ways:whisper").cloned();
                let button = ghost_with(look, "ways-install-whisper", Some("download"), "Install Whisper", false).border_color(look.color(token::BORDER_DEFAULT));
                let button = match &focus {
                    Some(focus) => ringed(look, button.track_focus(focus), focus, window),
                    None => button,
                };
                labelled(
                    button
                        .on_click(cx.listener(|this, _, window, cx| this.install_whisper(window, cx)))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.install_whisper(window, cx);
                            }
                        })),
                    Role::Button,
                    "Install Whisper",
                )
                .into_any_element()
            }
            Some(Whisper::Installing(progress)) => {
                let percent = (progress.clamp(0., 1.) * 100.).round();
                labelled(
                    div()
                        .id("ways-whisper-progress")
                        .flex_none()
                        .w(look.length(token::SPACE_12) * 3.)
                        .h(look.length(token::SPACE_2))
                        .rounded_full()
                        .overflow_hidden()
                        .bg(look.color(token::FG_DEFAULT).opacity(0.12))
                        .tooltip(tip(format!("{percent}%")))
                        .aria_min_numeric_value(0.)
                        .aria_max_numeric_value(100.)
                        .aria_numeric_value(percent as f64)
                        .child(div().h_full().w(gpui::relative(percent as f32 / 100.)).rounded_full().bg(look.color(token::ACCENT))),
                    Role::ProgressIndicator,
                    "Installing Whisper",
                )
                .into_any_element()
            }
            Some(Whisper::Installed) => muted("Installed".into()),
        };
        self.panel("talk", look, vec![engine_row, microphone.into_any_element(), row("Whisper", whisper_value).into_any_element()])
    }

    /// Whisper comes down on the press and never before; the bar follows it until it is here.
    fn install_whisper(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The press is replaced by the bar, so the keys wait on the row above.
        if let Some(focus) = self.prefs.keys.get("ways:answer") {
            window.focus(focus, cx);
        }
        if self.mock_wire() {
            self.ways.whisper = Some(Whisper::Installing(0.4));
            cx.notify();
            return;
        }
        crate::whisper::install();
        self.ways._install = Some(cx.spawn(async move |this, cx| loop {
            let state = cx.background_executor().spawn(async { crate::whisper::state() }).await;
            let more = this.update(cx, |this, cx| {
                this.ways.whisper = Some(state);
                cx.notify();
                matches!(state, Whisper::Installing(_))
            });
            if !more.unwrap_or(false) {
                break;
            }
            cx.background_executor().timer(Duration::from_millis(300)).await;
        }));
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held() -> Preferences {
        Preferences::default()
    }

    #[test]
    fn a_chip_says_what_its_way_is_set_to_in_one_line() {
        let mut prefs = held();
        assert_eq!(also_set(&prefs, "read", ""), "Short note, page for big work");
        assert_eq!(also_set(&prefs, "look", ""), "Picture for every result, parts can mix");
        assert_eq!(also_set(&prefs, "listen", ""), "Summary, Needs you, Mac voice");
        prefs.read_results = ReadResults::ThreadOnly;
        prefs.look_results = LookResults::WalkVisible;
        prefs.look_questions = LookQuestions::PicturesWhole;
        prefs.listen_end_summary = false;
        prefs.listen_progress = true;
        assert_eq!(also_set(&prefs, "read", ""), "Thread only");
        assert_eq!(also_set(&prefs, "look", ""), "Walk for anything visible, whole only");
        assert_eq!(also_set(&prefs, "listen", "Zoe"), "Needs you, Progress, Zoe");
        prefs.listen_needs_you = false;
        prefs.listen_progress = false;
        assert_eq!(also_set(&prefs, "listen", ""), "Nothing read aloud, Mac voice");
    }

    #[test]
    fn a_save_is_put_into_what_is_held_and_a_word_outside_its_set_is_not() {
        let mut prefs = held();
        assert!(apply(&mut prefs, "take_in", &Put::Text("look".into())));
        assert!(apply(&mut prefs, "answer_with", &Put::Text("buttons".into())));
        assert!(apply(&mut prefs, "buttons_words", &Put::Switch(false)));
        assert!(apply(&mut prefs, "listen_voice", &Put::Text("  com.apple.voice.zoe ".into())));
        assert_eq!((word_of(&prefs.take_in), word_of(&prefs.answer_with), prefs.buttons_words, prefs.listen_voice.as_str()), ("look".to_string(), "buttons".to_string(), false, "com.apple.voice.zoe"));
        assert!(!apply(&mut prefs, "take_in", &Put::Text("shout".into())));
        assert!(!apply(&mut prefs, "listen_progress", &Put::Text("on".into())));
        assert!(!apply(&mut prefs, "volume", &Put::Switch(true)));
        assert_eq!(word_of(&prefs.take_in), "look");
    }

    #[test]
    fn the_arrows_move_along_a_row_and_stop_at_its_ends() {
        assert_eq!(moved(Row::Take, "read", 1).map(|one| one.word), Some("look"));
        assert_eq!(moved(Row::Take, "listen", 1).map(|one| one.word), None);
        assert_eq!(moved(Row::Take, "read", -1).map(|one| one.word), None);
        assert_eq!(moved(Row::Answer, "type", 1).map(|one| one.word), Some("talk"));
        assert_eq!(moved(Row::Answer, "buttons", -1).map(|one| one.word), Some("talk"));
    }

    #[test]
    fn each_word_a_card_or_choice_names_is_one_fleet_reads() {
        for one in ways_of(Row::Take) {
            assert!(TakeIn::from_word(one.word).is_some(), "{}", one.word);
        }
        for one in ways_of(Row::Answer) {
            assert!(AnswerWith::from_word(one.word).is_some(), "{}", one.word);
        }
        assert_eq!(READ_RESULTS.map(|(word, _)| word), ReadResults::WORDS);
        assert_eq!(LOOK_RESULTS.map(|(word, _)| word), LookResults::WORDS);
        assert_eq!(LOOK_QUESTIONS.map(|(word, _)| word), LookQuestions::WORDS);
    }

    #[test]
    fn the_voice_list_opens_on_the_macs_own_then_premium_enhanced_and_the_rest() {
        let entries = voice_entries(&voice::mock_voices(), "");
        let labels: Vec<String> = entries
            .iter()
            .map(|entry| match entry {
                Entry::Search { label, .. } => label.to_string(),
                Entry::Choice { label, .. } => label.to_string(),
                Entry::Label(heading) => format!("# {heading}"),
                _ => String::new(),
            })
            .collect();
        assert_eq!(
            labels,
            ["Find a voice", "Mac default", "# Premium", "Ava (en-US)", "Zoe (en-US)", "# Enhanced", "Allison (en-US)", "Daniel (en-GB)", "# Standard", "Alex (en-US)", "Karen (en-AU)", "Samantha (en-US)"]
        );
        assert!(matches!(&entries[1], Entry::Choice { on: true, .. }), "the Mac's own is ticked while no voice is chosen");
    }

    #[test]
    fn every_glyph_the_page_draws_is_one_the_app_ships() {
        let mut glyphs: Vec<&str> = WAYS.iter().map(|one| one.glyph).collect();
        glyphs.extend(["check", "radio", "play", "download", "chevron-down", "circle-dashed", "circle-alert", "x", "ban"]);
        for glyph in glyphs {
            assert!(gpui::AssetSource::load(&crate::assets::Assets, &crate::assets::icon_path(glyph)).ok().flatten().is_some(), "{glyph}");
        }
    }
}
