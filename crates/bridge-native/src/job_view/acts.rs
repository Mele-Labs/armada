//! What a card remembers while it is on screen, and how a press on it reaches Fleet.
//!
//! **A press answers at once.** The card shows the answer in the frame the press lands, the act goes
//! to Fleet off the main thread, and a refusal takes the answer back and says Fleet's reason on the
//! card. Fleet's own row replaces the answer when it arrives, so an answer kept here is never read
//! once the data says the same.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Local};
use gpui::{
    div, prelude::*, App, Context, ElementId, Entity, FontWeight, Role, SharedString, Stateful, Window,
};
use gpui_component::input::{Enter, InputEvent, Textarea, TextareaState};

use super::rich_note::{kept_of, Resized, RichNote};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The place of a card that is drawn on the review sheet and in no list.
pub(super) const SHEET: usize = usize::MAX;

/// How the person answered a card, as the card reads it before Fleet's row says so.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Answer {
    Approved,
    Fixed(String),
    SentBack(String),
    Withdrawn,
    /// A proposed workflow put away.
    Dismissed,
    /// A rule sent.
    Sent,
    /// The plan was put away to keep talking.
    SetAside,
    /// A plan approval in flight.
    Going,
    /// A Judge asked for and not yet reading; it holds the id of the reading that stood when it was pressed.
    Judging(Option<String>),
    /// A pick on a handoff card.
    Handoff(ipc::HandoffPick),
    /// A Ready to try card put off: the time it comes back at, as it was sent.
    Later(String),
}

impl CardState {
    /// A card's boxes shut: they and what watches them are let go.
    pub fn drop_boxes(&mut self, key: &str) {
        let prefix = format!("{key}:");
        self.modes.remove(key);
        self.boxes.retain(|name, _| !name.starts_with(&prefix));
        self.notes.retain(|name, _| !name.starts_with(&prefix));
        self.typing.retain(|name, _| !name.starts_with(&prefix));
    }

    /// What the person pressed on a sketch card, while it is the drawing he pressed it on.
    pub fn pressed(&self, key: &str, card: &ipc::ThreadSketch) -> Option<&Answer> {
        let answer = self.answers.get(key)?;
        match self.answered_on.get(key) {
            Some(on) if *on != super::cards::newest_of(card) => None,
            _ => Some(answer),
        }
    }
}

/// Which of a card's boxes is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Read,
    Edit,
    Note,
}

/// Everything the cards of one Job's page remember, by the card's key.
#[derive(Default)]
pub(super) struct CardState {
    pub answers: HashMap<String, Answer>,
    /// What a sketch was drawn as, by `newest_of`, when its answer was pressed. A Job that draws it again, as a pinned
    /// card's id stays the same under, asks again: the answer was to the drawing before.
    pub answered_on: HashMap<String, String>,
    /// Fleet's reason for the last act on a card it refused.
    pub said: HashMap<String, SharedString>,
    pub modes: HashMap<String, Mode>,
    pub boxes: HashMap<String, Entity<TextareaState>>,
    /// The boxes that carry pictures, tags and formatting, by box: each wraps the text area of the same name in `boxes`.
    pub notes: HashMap<String, Entity<RichNote>>,
    /// Each open box keeping its words as they are typed, by box.
    pub typing: HashMap<String, gpui::Subscription>,
    /// Cards drawn with words kept for them: an open one gets its box back, a settled one lets them go.
    /// A draw writes here and the next frame acts on it. The last is whether the card's Note box is a rich one.
    pub kept_seen: std::cell::RefCell<Vec<(String, usize, bool, bool)>>,
    pub folded: HashMap<String, bool>,
    /// The revision of a sketch the person chose, counting from the oldest, and how many there were when he chose it;
    /// absent is the newest. A newer one arriving ends the choice (`picked_revision`).
    pub revision: HashMap<String, (usize, usize)>,
    /// Folds a card opened: its earlier text, its technical groups, its unchanged groups.
    pub opened: HashSet<String>,
    /// The card the thread keys act on: the one last pressed, else the newest.
    pub target: Option<String>,
    /// How the person settled a Judge's note, before Fleet's reading says so, by `reading/note`.
    pub judged: HashMap<String, ipc::JudgeNoteHow>,
    /// The sketch card under the pointer, and the one holding the keys: either plays its steps.
    pub pointed: Option<String>,
    pub focused: Option<String>,
    /// The sketch card playing its steps, and since when.
    pub playing: Option<(String, std::time::Instant)>,
    /// Note boxes just opened, to be scrolled clear of the composer once they are measured.
    pub reveal: HashMap<String, std::rc::Rc<std::cell::Cell<bool>>>,
    /// A handoff card's sizes as last drawn, by card and part: the brief's whole height (over four lines it folds
    /// behind Show all) and the head's width (a narrow head puts the question under the figure).
    pub measured: std::cell::RefCell<HashMap<String, std::rc::Rc<std::cell::Cell<gpui::Pixels>>>>,
    /// What the cards of artifacts that hold parts remember (`parts_card`).
    pub parts: super::parts_card::PartsState,
    /// The build opened under the Ready to try card, by card id: the one drawn whole in place.
    pub ready_pick: Option<String>,
    /// The cards that carry a Judge, by the card's own key: the list place, the press's key and what it reads. A draw writes
    /// here and the Judge's key reads it.
    pub judgeable: std::cell::RefCell<HashMap<String, (usize, String, ipc::JudgeArtifact)>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Primary,
    Secondary,
    Ghost,
}

/// A key chip, drawn on the card the thread keys act on (`ThreadKbd`).
pub(super) fn kbd(look: &Look, text: &str) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_KBD))
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_b(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .text_color(look.color(token::FG_MUTED))
        .font_family(super::rows::mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .child(SharedString::from(text.to_string()))
}

/// A button as a card draws it. The caller adds `on_click` and wraps it with `labelled`.
#[allow(clippy::too_many_arguments)]
pub(super) fn button(
    look: &Look,
    id: impl Into<ElementId>,
    said: &str,
    tone: Tone,
    big: bool,
    chip: Option<&str>,
    on: bool,
) -> Stateful<gpui::Div> {
    let base = div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(if big { token::H_CONTROL_LG } else { token::H_CONTROL }))
        .px(look.length(if big { token::SPACE_4 } else { token::SPACE_3 }))
        .rounded(look.length(if big { token::RADIUS_LG } else { token::RADIUS_MD }))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)));
    let styled = match tone {
        Tone::Primary => base
            .bg(look.color(token::ACCENT))
            .border_color(look.color(token::ACCENT))
            .text_color(look.color(token::TEXT_ON_ACCENT))
            .when(on, |button| button.hover(|style| style.bg(look.color(token::ACCENT_HOVER)))),
        Tone::Secondary if big => base
            .bg(look.color(token::FG_DEFAULT).opacity(0.08))
            .text_color(look.color(token::FG_DEFAULT))
            .when(on, |button| button.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.14)))),
        Tone::Secondary => base
            .bg(look.color(token::BG_SUNKEN))
            .border_color(look.color(token::BORDER_STRONG))
            .text_color(look.color(token::FG_DEFAULT))
            .when(on, |button| button.hover(|style| style.bg(look.color(token::BG_HOVER)))),
        Tone::Ghost => base
            .text_color(look.color(token::FG_MUTED))
            .when(on, |button| button.hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))),
    };
    styled
        .when(on, |button| button.cursor_pointer())
        .when(!on, |button| button.opacity(0.4))
        .child(SharedString::from(said.to_string()))
        .children(chip.map(|text| kbd(look, text)))
}

/// A row of answers, or one press, that Fleet does not carry to the Mac a Job runs on: dimmed, and its tooltip says
/// where it runs. A press on one still goes through the act's own post, which refuses it with the same sentence.
pub(super) fn gated(row: Stateful<gpui::Div>, off: Option<SharedString>) -> Stateful<gpui::Div> {
    row.when_some(off, |row, why| row.opacity(0.4).tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(why.clone()).build(window, cx)))
}

/// What Fleet's reason for a refused act reads as, drawn under the card's buttons.
pub(super) fn refusal(look: &Look, key: &str, why: SharedString) -> gpui::AnyElement {
    crate::judge_chips::refusal(look, key, why)
}

/// The time a card shows for its own stamp.
pub(super) fn at_of(at: &ipc::Instant) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok().map(|then| then.with_timezone(&Local))
}

impl JobView {
    /// A card changed its height: that list element measures again.
    pub(super) fn card_changed(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) {
            // A card drawn on the review sheet has no place of its own: the thread behind measures again.
            if ix == SHEET {
                page.list.remeasure();
            } else {
                page.remeasure(ix);
            }
        }
        cx.notify();
    }

    pub(super) fn cards_mut(&mut self) -> Option<&mut CardState> {
        let id = self.shown.clone()?;
        self.pages.get_mut(&id).map(|page| &mut page.cards)
    }

    /// The keys act on this card from now.
    pub(super) fn aim(&mut self, key: &str) {
        if let Some(cards) = self.cards_mut() {
            cards.target = Some(key.to_string());
        }
    }

    /// Open one of a card's boxes, with `text` in it, and put the caret there.
    pub(super) fn open_box(&mut self, ix: usize, key: &str, mode: Mode, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.put_box(ix, key, mode, text, true, window, cx);
    }

    /// A box with its words, the ones kept for it before `text`; `focus` puts the caret in it and brings it into view.
    #[allow(clippy::too_many_arguments)]
    fn put_box(&mut self, ix: usize, key: &str, mode: Mode, text: &str, focus: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.put_box_as(ix, key, mode, text, focus, None, window, cx);
    }

    /// A Note box that carries pictures, tags and formatting, as the Job's own box does: where a person writes back to an agent.
    /// `name` is what the box is called to VoiceOver.
    pub(super) fn open_note(&mut self, ix: usize, key: &str, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.put_box_as(ix, key, Mode::Note, "", true, Some(name), window, cx);
    }

    /// `rich` is the name of a box that is a `RichNote` over its text area.
    #[allow(clippy::too_many_arguments)]
    fn put_box_as(&mut self, ix: usize, key: &str, mode: Mode, text: &str, focus: bool, rich: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let box_key = format!("{key}:{mode:?}");
        let existing = self.cards_mut().and_then(|cards| cards.boxes.get(&box_key).cloned());
        let area = match existing {
            Some(area) => area,
            None => {
                let rows = if mode == Mode::Edit { (4, 12) } else { (3, 8) };
                let kept = self.kept_key(key, mode).map(|kept| self.draft_words(&kept)).filter(|words| !words.is_empty());
                let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(rows.0, rows.1));
                match (rich, self.kept_key(key, mode).map(|kept| self.draft_of(&kept)).filter(|draft| !draft.text.is_empty())) {
                    // A rich box takes its chips back with its words.
                    (Some(_), Some(draft)) => area.update(cx, |area, cx| area.set_value(draft.content(), window, cx)),
                    _ => area.update(cx, |area, cx| area.set_value(kept.unwrap_or_else(|| text.to_string()), window, cx)),
                }
                if let Some(name) = rich {
                    // Made before the box's own watcher, so what a keystroke draws is drawn before the draft is read.
                    let scope = self.note_scope(cx);
                    let name = name.to_string();
                    let note = cx.new(|cx| RichNote::new(area.clone(), name, scope, window, cx));
                    // A tray or a list coming or going changes the card's height.
                    let sized = cx.subscribe(&note, move |this, _, _: &Resized, cx| this.card_changed(ix, cx));
                    if let Some(cards) = self.cards_mut() {
                        cards.notes.insert(box_key.clone(), note);
                        cards.typing.insert(format!("{box_key}#size"), sized);
                    }
                }
                if let Some(kept) = self.kept_key(key, mode) {
                    let is_rich = rich.is_some();
                    let typing = cx.subscribe_in(&area, window, move |this, area, event: &InputEvent, _, cx| {
                        if let InputEvent::Change = event {
                            if is_rich {
                                this.keep_draft(&kept, kept_of(area, cx), cx);
                            } else {
                                let words = area.read(cx).value().to_string();
                                this.keep_words(&kept, &words, cx);
                            }
                        }
                    });
                    if let Some(cards) = self.cards_mut() {
                        cards.typing.insert(box_key.clone(), typing);
                    }
                }
                area
            }
        };
        if focus {
            self.aim(key);
        }
        if let Some(cards) = self.cards_mut() {
            cards.modes.insert(key.to_string(), mode);
            cards.boxes.insert(box_key, area.clone());
        }
        if focus {
            area.update(cx, |area, cx| area.focus(window, cx));
            // The box comes into view with the caret, as a focused box does in Classic.
            if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)).filter(|_| ix != SHEET) {
                page.list.scroll_to_reveal_item(page.entry(ix));
            }
        }
        self.card_changed(ix, cx);
    }

    /// Shut a card's box and forget what was typed in it.
    /// The box shut and its words kept, as `close_box` but for the words: Esc leaves them for the next time it opens.
    pub(super) fn close_box_kept(&mut self, ix: usize, key: &str, cx: &mut Context<Self>) {
        if let Some(cards) = self.cards_mut() {
            cards.drop_boxes(key);
        }
        self.card_changed(ix, cx);
    }

    pub(super) fn close_box(&mut self, ix: usize, key: &str, cx: &mut Context<Self>) {
        if let Some(cards) = self.cards_mut() {
            cards.drop_boxes(key);
        }
        self.forget_box_words(key, cx);
        self.card_changed(ix, cx);
    }

    /// Where a card's box keeps its words: Classic's `keep`, `<kind>:<job>:<item>`, then `:fixed` or
    /// `:note` (`SessionThread.tsx:2130`, `DraftRow.tsx:159`).
    fn kept_key(&self, key: &str, mode: Mode) -> Option<String> {
        let job = self.shown.as_ref()?;
        let end = match mode {
            Mode::Edit => "fixed",
            Mode::Note => "note",
            Mode::Read => return None,
        };
        let (kind, item) = key.split_once(':').unwrap_or(("card", key));
        Some(format!("{kind}:{job}:{item}:{end}"))
    }

    /// Whether words are kept for the card's Edit or Note box.
    pub(super) fn holds_kept(&self, key: &str) -> bool {
        [Mode::Edit, Mode::Note].into_iter().filter_map(|mode| self.kept_key(key, mode)).any(|kept| !self.draft_words(&kept).is_empty())
    }

    /// The words of a card's boxes go: it was cancelled or settled.
    fn forget_box_words(&mut self, key: &str, cx: &mut Context<Self>) {
        for mode in [Mode::Edit, Mode::Note] {
            if let Some(kept) = self.kept_key(key, mode) {
                self.keep_words(&kept, "", cx);
            }
        }
    }

    /// Cards drawn last frame with words kept: an open card's box comes back as it was left, Edit
    /// before Note, without the caret; a settled card's words go (`DraftRow.tsx:161-185`).
    pub(super) fn tend_kept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(seen) = self.shown.as_ref().and_then(|id| self.pages.get(id)).map(|page| page.cards.kept_seen.take()) else { return };
        for (key, ix, open, rich) in seen {
            if !open {
                self.forget_box_words(&key, cx);
                continue;
            }
            if self.cards_mut().is_some_and(|cards| cards.modes.contains_key(&key)) {
                continue;
            }
            let kept = |mode| self.kept_key(&key, mode).is_some_and(|kept| !self.draft_words(&kept).is_empty());
            let (fixed, noted) = (kept(Mode::Edit), kept(Mode::Note));
            if fixed {
                self.put_box(ix, &key, Mode::Edit, "", false, window, cx);
            } else if noted {
                self.put_box_as(ix, &key, Mode::Note, "", false, rich.then_some("What should change?"), window, cx);
            }
        }
    }

    /// `⌘Enter` in a card's open box: the act the box is for, as its primary press does.
    pub(super) fn commit_box(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let run = self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| page.keys.commit_of(key));
        if let Some(run) = run {
            run(self, window, cx);
        }
    }

    /// The words in a card's open box.
    /// A rich box gives them as a message sends them: trimmed, with a blank line where a quote or a list ends in words.
    pub(super) fn typed_in(&self, key: &str, mode: Mode, cx: &Context<Self>) -> String {
        let cards = self.shown.as_ref().and_then(|id| self.pages.get(id)).map(|page| &page.cards);
        let box_key = format!("{key}:{mode:?}");
        if let Some(note) = cards.and_then(|cards| cards.notes.get(&box_key)) {
            return note.read(cx).sent(cx);
        }
        cards.and_then(|cards| cards.boxes.get(&box_key)).map(|area| area.read(cx).value().to_string()).unwrap_or_default()
    }

    /// The files and tags a card's open box holds, for the answer it sends; none where the box is not a rich one.
    pub(super) fn extras_of(&self, key: &str, mode: Mode, cx: &App) -> ipc::AnswerExtras {
        self.shown
            .as_ref()
            .and_then(|id| self.pages.get(id))
            .and_then(|page| page.cards.notes.get(&format!("{key}:{mode:?}")))
            .map(|note| note.read(cx).extras(cx))
            .unwrap_or_default()
    }

    /// Put away the lists open over the words of the shown Job's boxes. True where there was one: Esc has done its work.
    pub(super) fn close_note_lists(&mut self, cx: &mut Context<Self>) -> bool {
        let open: Vec<Entity<RichNote>> = self.shown.as_ref().and_then(|id| self.pages.get(id)).map(|page| page.cards.notes.values().filter(|note| note.read(cx).offering()).cloned().collect()).unwrap_or_default();
        for note in &open {
            note.update(cx, |note, cx| note.close_offer(cx));
        }
        !open.is_empty()
    }

    /// A list is open over the words of one of the shown Job's boxes, so Esc is the list's.
    pub(super) fn note_offering(&self, cx: &App) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).is_some_and(|page| page.cards.notes.values().any(|note| note.read(cx).offering()))
    }

    /// The box as a card draws it.
    pub(super) fn box_of(&self, key: &str, mode: Mode, name: impl Into<SharedString>, look: &Look, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let name: SharedString = name.into();
        let cards = &self.shown.as_ref().and_then(|id| self.pages.get(id))?.cards;
        let area = cards.boxes.get(&format!("{key}:{mode:?}"))?;
        let note = cards.notes.get(&format!("{key}:{mode:?}")).cloned();
        let commit_key = key.to_string();
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("box:{key}:{name}")))
                    .key_context(crate::keymap::TEXT_BOX)
                    // ⌘↩ sends, taken before the text area puts a line break in (`DraftRow.tsx:309`).
                    .capture_action(cx.listener(move |this, action: &Enter, window, cx| {
                        if action.secondary {
                            cx.stop_propagation();
                            this.commit_box(&commit_key, window, cx);
                        }
                    }))
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .map(|frame| match note {
                        Some(note) => frame.child(note),
                        None => frame.child(Textarea::new(area).appearance(false).aria_label(name.clone())),
                    }),
                Role::TextInput,
                name,
            )
            .into_any_element(),
        )
    }

    /// Answer a card: it reads the answer in this frame, and Fleet is told. A refusal takes the
    /// answer back and the card says why.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn settle(
        &mut self,
        ix: usize,
        key: String,
        answer: Answer,
        route: &'static str,
        body: Result<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.shown.clone() else { return };
        self.aim(&key);
        let on = self.drawing_of_key(&job, &key, cx);
        if let Some(cards) = self.cards_mut() {
            match on {
                Some(on) => cards.answered_on.insert(key.clone(), on),
                None => cards.answered_on.remove(&key),
            };
            cards.answers.insert(key.clone(), answer);
            cards.said.remove(&key);
            cards.drop_boxes(&key);
        }
        self.forget_box_words(&key, cx);
        // An answer pressed in the sketch read over the window shuts the box the keys were in: they go back to the read. Its
        // foot and the card behind it draw the same acts under one key, so the key says which it was.
        if let Some(focus) = self.sketch_reading.as_ref().filter(|reading| reading.answer == key).map(|reading| reading.focus.clone()) {
            window.focus(&focus, cx);
        }
        self.card_changed(ix, cx);
        self.post_card(job, ix, key, route, body, window, cx);
    }

    /// What the sketch under a card's key is drawn as now: the pinned card is `pin:<id>`, a card in the thread its own id.
    fn drawing_of_key(&self, job: &SharedString, key: &str, cx: &Context<Self>) -> Option<String> {
        let held = self.thread(job, cx)?;
        let id = key.strip_prefix("pin:").unwrap_or(key);
        super::sketch_read::card_of(&held.head, &held.items, id).map(super::cards::newest_of)
    }

    /// An act with no answer to show: it goes, and a refusal is said on the card.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn post_card(
        &mut self,
        job: SharedString,
        ix: usize,
        key: String,
        route: &'static str,
        body: Result<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.post_with(job.clone(), route, body, move |this, why, cx| this.undo(&job, ix, &key, why, cx), window, cx);
    }

    /// An act sent off the main thread; `failed` runs with Fleet's reason when it was refused.
    pub(super) fn post_with(
        &mut self,
        job: SharedString,
        route: &'static str,
        body: Result<String, String>,
        failed: impl FnOnce(&mut JobView, SharedString, &mut Context<JobView>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(why) = self.off_to(&job, route, cx) {
            failed(self, why, cx);
            return;
        }
        let Some(fleet) = self.fleet.clone() else {
            if let Ok(body) = &body {
                if self.mock_keep(&job, route, body, cx) {
                    return;
                }
            }
            match crate::mock::act_refused(route) {
                Some(why) => failed(self, why.into(), cx),
                None if route == "/sessions/sketch/answer" => self.mock_sketch_answered(&job, body, cx),
                None if route == super::parts_card::ANSWER_PART => {
                    if let Err(why) = self.mock_part_answered(&job, body, cx) {
                        failed(self, why.into(), cx);
                    }
                }
                None if route == super::parts_card::PRESS => self.mock_part_pressed(&job, body, cx),
                None if route == super::ready_card::ANSWER_READY => {
                    if let Err(why) = self.mock_ready_answered(&job, body, cx) {
                        failed(self, why.into(), cx);
                    }
                }
                None => {}
            }
            return;
        };
        let Some(port) = fleet.read(cx).held().port() else {
            failed(self, "Fleet is not connected".into(), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let sent = body.len();
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, route, body))?;
                    crate::refusal::answered("POST", route, Some(sent), status, bytes).map(|_| ()).map_err(|refusal| refusal.to_string())
                })
                .await;
            if let Err(why) = answer {
                let _ = this.update(cx, |this, cx| failed(this, why.into(), cx));
            }
        })
        .detach();
    }

    /// Mock data: Fleet takes a sketch's answer as it does, settling only that sketch. It stops waiting
    /// on him, and the thread's pin is read again, so it moves to the next sketch still open.
    fn mock_sketch_answered(&mut self, job: &SharedString, body: Result<String, String>, cx: &mut Context<Self>) {
        let Ok(answer) = body.map_err(|_| ()).and_then(|body| ipc::decode::<ipc::AnswerSketch>("AnswerSketch", body.as_bytes()).map_err(|_| ())) else { return };
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let Some(held) = self.mock.get_mut(job.as_str()) else { return };
        if let Some(at) = crate::mock::sketch_answered(&mut held.head, &mut held.items, &answer.sketch, answer.how, &now, answer.words) {
            held.version += 1;
            held.mark_dirty(at);
        }
        if let Some(page) = self.pages.get_mut(job) {
            page.dock.hidden.insert(answer.sketch);
        }
        self.sync(job, cx);
        cx.notify();
    }

    /// Fleet refused: the answer is taken back and its reason stands on the card.
    pub(super) fn undo(&mut self, job: &SharedString, ix: usize, key: &str, why: SharedString, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            page.cards.answers.remove(key);
            page.cards.said.insert(key.to_string(), why);
            // A build to try whose answer was refused is opened again, so its sentence stands where he looks.
            if super::ready_card::is_card(key) {
                page.cards.ready_pick = Some(key.to_string());
            }
            if ix == SHEET {
                page.list.remeasure();
            } else {
                page.remeasure(ix);
            }
        }
        cx.notify();
    }

    /// The fold state of a card, with the default the card starts in.
    pub(super) fn folded(&self, key: &str, start: bool) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| page.cards.folded.get(key).copied()).unwrap_or(start)
    }

    pub(super) fn fold_card(&mut self, ix: usize, key: &str, start: bool, cx: &mut Context<Self>) {
        let now = !self.folded(key, start);
        if let Some(cards) = self.cards_mut() {
            cards.folded.insert(key.to_string(), now);
        }
        self.card_changed(ix, cx);
    }

    pub(super) fn open_in_card(&self, key: &str) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).is_some_and(|page| page.cards.opened.contains(key))
    }

    pub(super) fn flip_in_card(&mut self, ix: usize, key: &str, cx: &mut Context<Self>) {
        if let Some(cards) = self.cards_mut() {
            if !cards.opened.remove(key) {
                cards.opened.insert(key.to_string());
            }
        }
        self.card_changed(ix, cx);
    }

    /// Whether the thread keys act on this card now.
    pub(super) fn is_target(&self, key: &str) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).is_some_and(|page| page.keys.target_of(page.cards.target.as_deref()).as_deref() == Some(key))
    }
}
