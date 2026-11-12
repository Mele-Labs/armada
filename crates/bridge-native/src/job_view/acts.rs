//! What a card remembers while it is on screen, and how a press on it reaches Fleet.
//!
//! **A press answers at once.** The card shows the answer in the frame the press lands, the act goes
//! to Fleet off the main thread, and a refusal takes the answer back and says Fleet's reason on the
//! card. Fleet's own row replaces the answer when it arrives, so an answer kept here is never read
//! once the data says the same.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Local};
use gpui::{
    div, prelude::*, Context, ElementId, Entity, FontWeight, Role, SharedString, Stateful, Window,
};
use gpui_component::input::{Textarea, TextareaState};

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
    /// Fleet's reason for the last act on a card it refused.
    pub said: HashMap<String, SharedString>,
    pub modes: HashMap<String, Mode>,
    pub boxes: HashMap<String, Entity<TextareaState>>,
    pub folded: HashMap<String, bool>,
    /// The revision of a sketch the person chose, counting from the oldest; absent is the newest.
    pub revision: HashMap<String, usize>,
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

/// What Fleet's reason for a refused act reads as, drawn under the card's buttons.
pub(super) fn refusal(look: &Look, key: &str, why: SharedString) -> gpui::AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("refused:{key}")))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::STATUS_COMPLETED_FAILED))
            .child(why),
        Role::Status,
        "Refused",
    )
    .into_any_element()
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
        let existing = self.cards_mut().and_then(|cards| cards.boxes.get(&format!("{key}:{mode:?}")).cloned());
        let area = match existing {
            Some(area) => area,
            None => {
                let rows = if mode == Mode::Edit { (4, 12) } else { (3, 8) };
                let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(rows.0, rows.1));
                area.update(cx, |area, cx| area.set_value(text.to_string(), window, cx));
                area
            }
        };
        let box_key = format!("{key}:{mode:?}");
        self.aim(key);
        if let Some(cards) = self.cards_mut() {
            cards.modes.insert(key.to_string(), mode);
            cards.boxes.insert(box_key, area.clone());
        }
        area.update(cx, |area, cx| area.focus(window, cx));
        // The box comes into view with the caret, as a focused box does in Classic.
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)).filter(|_| ix != SHEET) {
            page.list.scroll_to_reveal_item(page.entry(ix));
        }
        self.card_changed(ix, cx);
    }

    /// Shut a card's box and forget what was typed in it.
    pub(super) fn close_box(&mut self, ix: usize, key: &str, cx: &mut Context<Self>) {
        if let Some(cards) = self.cards_mut() {
            cards.modes.remove(key);
            cards.boxes.retain(|name, _| !name.starts_with(&format!("{key}:")));
        }
        self.card_changed(ix, cx);
    }

    /// The words in a card's open box.
    pub(super) fn typed_in(&self, key: &str, mode: Mode, cx: &Context<Self>) -> String {
        self.shown
            .as_ref()
            .and_then(|id| self.pages.get(id))
            .and_then(|page| page.cards.boxes.get(&format!("{key}:{mode:?}")))
            .map(|area| area.read(cx).value().to_string())
            .unwrap_or_default()
    }

    /// The box as a card draws it.
    pub(super) fn box_of(&self, key: &str, mode: Mode, name: &'static str, look: &Look, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let area = self.shown.as_ref().and_then(|id| self.pages.get(id))?.cards.boxes.get(&format!("{key}:{mode:?}"))?;
        let _ = cx;
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("box:{key}:{name}")))
                    .key_context(crate::keymap::TEXT_BOX)
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(Textarea::new(area).appearance(false).aria_label(name)),
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
        if let Some(cards) = self.cards_mut() {
            cards.answers.insert(key.clone(), answer);
            cards.said.remove(&key);
            cards.modes.remove(&key);
            cards.boxes.retain(|name, _| !name.starts_with(&format!("{key}:")));
        }
        self.card_changed(ix, cx);
        self.post_card(job, ix, key, route, body, window, cx);
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
        let _ = job;
        let Some(fleet) = self.fleet.clone() else { return };
        let Some(port) = fleet.read(cx).held().port() else {
            failed(self, "Fleet is not connected".into(), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, route, body))?;
                    if (200..300).contains(&status) {
                        return Ok(());
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            if let Err(why) = answer {
                let _ = this.update(cx, |this, cx| failed(this, why.into(), cx));
            }
        })
        .detach();
    }

    /// Fleet refused: the answer is taken back and its reason stands on the card.
    pub(super) fn undo(&mut self, job: &SharedString, ix: usize, key: &str, why: SharedString, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            page.cards.answers.remove(key);
            page.cards.said.insert(key.to_string(), why);
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
