//! What waits on the person, over the message box: the Needs you band and the panel that opens from
//! it (`job-view.md`, B7). The question deck is `deck.rs` and the permission card `perm.rs`.
//!
//! **What the band and the panel draw is Fleet's own grouping** (`GET /sessions/needs_you`, `ipc::NeedsYou`): its
//! sections in its order under its labels, each row with the mark, the press and the put-away it names. This file
//! maps those to pictures and handlers and sorts, filters and places nothing.
//!
//! **Electron draws a question or a permission in this panel, not in the thread.** The thread's tail
//! holds the plan and the pinned sketch; a question, a permission and the lapsed questions are here,
//! where `1` to `9` and the four `⌥` keys reach them with the caret still in the box.

use std::collections::{HashMap, HashSet};

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FocusHandle, Focusable as _, FontWeight, KeyDownEvent, Pixels, Role, SharedString, Subscription, Window};
use gpui_component::input::InputEvent;
use ipc::{
    BuildSource, ConfirmWaiting, DismissWaiting, DoneWaiting, Lapsed, NeedsYou, NeedsYouAct, NeedsYouDismiss, NeedsYouGlyphKind, NeedsYouRow, NeedsYouSectionKind,
    ReachesTo, SessionId, ThreadAsk,
};

use super::acts::Mode;
use super::deck::Deck;
use super::perm::Perm;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What the panel remembers of one Job.
#[derive(Default)]
pub(super) struct DockState {
    /// The person's say on whether the panel is open; absent, it is shut.
    pub open: Option<bool>,
    /// The thing the panel is showing whole.
    pub detail: Option<String>,
    /// Questions that already opened the panel, so one opens it once.
    pub seen: HashSet<String>,
    pub decks: HashMap<String, Deck>,
    pub perms: HashMap<String, Perm>,
    /// Lapsed questions answered or skipped.
    pub hidden: HashSet<String>,
    /// Sentences read from a reply that the person said are questions, before Fleet's row says so.
    pub confirmed: HashSet<String>,
    pub said: HashMap<String, SharedString>,
    /// Where each command pressed here stands, by the command: a restart's key and a run item's row.
    pub runs: HashMap<String, RunMark>,
    /// The one-line box open under a row: a reason for putting it away, or a note for its Done.
    pub row_box: Option<RowBox>,
}

/// What a row's open box is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RowBoxFor {
    /// A reason that goes to the agent with the removal.
    Reason,
    /// A note that goes with Done.
    Note,
}

/// The box under a row: Enter sends it, Esc puts it away.
pub(super) struct RowBox {
    pub row: String,
    pub what: RowBoxFor,
    _enter: Subscription,
}

/// Where a command pressed in the panel stands, as Electron's `RestartKey` marks it.
#[derive(Clone)]
pub(super) enum RunMark {
    Running,
    Passed,
    Failed(SharedString),
}

impl RunMark {
    /// Its glyph and its words.
    fn look(&self) -> (&'static str, SharedString) {
        match self {
            RunMark::Running => ("loader-circle", "Running".into()),
            RunMark::Passed => ("circle-check", "Passed".into()),
            RunMark::Failed(why) => ("circle-x", why.clone()),
        }
    }
}

/// One thing the panel lists.
pub(super) enum Thing<'a> {
    Ask(&'a ThreadAsk),
    Lapsed(&'a Lapsed),
    /// A row of Fleet's Needs you.
    Row(&'a NeedsYouRow),
}

/// A section of the list as Fleet sent it, with its rows as things.
pub(super) struct Group<'a> {
    pub kind: NeedsYouSectionKind,
    pub label: &'a str,
    pub things: Vec<Thing<'a>>,
}

/// The choices a row asks, where its act is an answer.
pub(super) fn row_options(row: &NeedsYouRow) -> &[ipc::WaitingOption] {
    match &row.act {
        NeedsYouAct::Answer { options, .. } => options,
        _ => &[],
    }
}

pub(super) fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// The menu of a sentence Fleet read from a reply: A question, unless it was said already, and Not a question.
fn misread_entries(confirmed: bool) -> Vec<Entry> {
    let mut entries = Vec::new();
    if !confirmed {
        entries.push(Entry::Act { id: "yes", label: "A question".into(), icon: Some("thumbs-up"), danger: false });
    }
    entries.push(Entry::Act { id: "no", label: "Not a question".into(), icon: Some("thumbs-down"), danger: false });
    entries
}

/// A row of the list: a filled, rounded press the panel's width, edged in the band's hue on hover.
fn filled(look: &Look, row: gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div> {
    row.flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_HOVER))
        .cursor_pointer()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .hover(|style| style.border_color(look.color(token::STEP_WAITING)))
}

/// The picture of each mark Fleet names, by the registry's glyph.
pub(super) fn icon_of(kind: NeedsYouGlyphKind) -> &'static str {
    match kind {
        NeedsYouGlyphKind::Question => "message-circle-question-mark",
        NeedsYouGlyphKind::Reply => "reply",
        NeedsYouGlyphKind::ReadFromReply => "scan-text",
        NeedsYouGlyphKind::Walk => "app-window",
        NeedsYouGlyphKind::Answer => "message-square",
        NeedsYouGlyphKind::PullRequest => "git-pull-request-arrow",
        NeedsYouGlyphKind::Run => "terminal",
        NeedsYouGlyphKind::Message => "square-pen",
        NeedsYouGlyphKind::Steps => "list-checks",
        NeedsYouGlyphKind::Waiting => "bell-ring",
        NeedsYouGlyphKind::LinkedJob => "link",
        NeedsYouGlyphKind::Operator => "bot",
        NeedsYouGlyphKind::Todo => "square-check-big",
        NeedsYouGlyphKind::Plan => "list-checks",
        NeedsYouGlyphKind::Sketch => "pencil-ruler",
        NeedsYouGlyphKind::Draft | NeedsYouGlyphKind::Document => "file-text",
        NeedsYouGlyphKind::Restart => "rotate-cw",
        NeedsYouGlyphKind::Done => "circle-check",
    }
}

/// What the thread's ask says it asks: the first question, or the command's first line.
pub(super) fn ask_text(ask: &ThreadAsk) -> String {
    ask.call.questions.first().map(|question| question.question.clone()).unwrap_or_else(|| first_line(&ask.command).to_string())
}

pub(super) fn ask_label(ask: &ThreadAsk) -> String {
    let call = &ask.call;
    if call.questions.is_empty() {
        return match call.reaches_to {
            Some(ReachesTo::Read) => "Permission to read".into(),
            Some(ReachesTo::Write) => "Permission to write".into(),
            Some(ReachesTo::Run) => "Permission to run".into(),
            None => "Permission".into(),
        };
    }
    let header = call.questions[0].header.trim();
    if header.is_empty() { "Question".into() } else { header.to_string() }
}

impl JobView {
    pub(super) fn dock_of(&self, id: &str) -> Option<&DockState> {
        self.pages.get(id).map(|page| &page.dock)
    }

    pub(super) fn dock_mut(&mut self) -> Option<&mut DockState> {
        let id = self.shown.clone()?;
        self.pages.get_mut(&id).map(|page| &mut page.dock)
    }

    /// Fleet's Needs you for the Job: what it last grouped, or on mock data the mock's own grouping.
    pub(super) fn needs_of<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Option<&'a NeedsYou> {
        match &self.fleet {
            Some(fleet) => fleet.read(cx).held().needs.get(id).map(|held| &held.view),
            None => self.mock_needs.get(id),
        }
    }

    /// Ask for the Job's Needs you again when its record's `waiting_for` is not the list the view was read
    /// for. Only the Job on screen is read; another is read when it comes on screen.
    pub(super) fn want_needs(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if self.shown.as_ref() != Some(id) {
            return;
        }
        let Some(record) = self.record(id, cx) else { return };
        let Some(fleet) = self.fleet.clone() else {
            let hidden = self.dock_of(id).map(|dock| dock.hidden.clone()).unwrap_or_default();
            let view = crate::mock::needs_you(id, &record.waiting_for, &|item| hidden.contains(item));
            self.mock_needs.insert(id.to_string(), view);
            return;
        };
        if fleet.read(cx).held().needs_current(id, &record.waiting_for) {
            // A row put away that Fleet's view no longer holds is Fleet's now: it may come back, as a walk shown again does.
            let lapsed = self.thread(id, cx).and_then(|held| held.head.lapsed.as_ref().map(|lapsed| lapsed.id.clone()));
            let held: Vec<String> = fleet.read(cx).held().needs.get(id.as_str()).into_iter().flat_map(|held| held.view.sections.iter().flat_map(|section| section.rows.iter().map(|row| row.id.clone()))).collect();
            if let Some(dock) = self.pages.get_mut(id).map(|page| &mut page.dock) {
                dock.hidden.retain(|away| held.contains(away) || lapsed.as_ref() == Some(away));
            }
            return;
        }
        let waiting = record.waiting_for.clone();
        fleet.update(cx, |fleet, cx| fleet.read_needs(id, waiting, cx));
    }

    /// The sections of the list, in Fleet's order and under Fleet's labels, each row a thing to press.
    /// **Nothing here is sorted or placed by this crate.** What the thread's head holds beyond Fleet's rows, the
    /// question open now and the lapsed ones, goes first in Questions; a row Fleet gives for the open question
    /// stands in that question's place.
    pub(super) fn groups<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Vec<Group<'a>> {
        use NeedsYouSectionKind as Kind;
        let hidden = self.dock_of(id).map(|dock| &dock.hidden);
        let held = self.thread(id, cx);
        let asked = held.and_then(|held| held.head.asked.as_ref());
        let carries = |row: &NeedsYouRow| asked.is_some_and(|ask| row.id == format!("ask:{}", ask.call.call) || row.id == format!("perm:{}", ask.call.call));
        let mut carried = false;
        let mut groups: Vec<Group<'a>> = Vec::new();
        for section in self.needs_of(id, cx).map(|view| view.sections.as_slice()).unwrap_or_default() {
            if matches!(section.kind, Kind::Restart | Kind::Done) {
                continue;
            }
            let things: Vec<Thing<'a>> = section
                .rows
                .iter()
                .filter(|row| !hidden.is_some_and(|hidden| hidden.contains(&row.id)))
                .filter_map(|row| match asked {
                    Some(ask) if carries(row) => (!std::mem::replace(&mut carried, true)).then_some(Thing::Ask(ask)),
                    _ => Some(Thing::Row(row)),
                })
                .collect();
            if !things.is_empty() {
                groups.push(Group { kind: section.kind, label: &section.label, things });
            }
        }
        let mut head: Vec<Thing<'a>> = Vec::new();
        if let Some(held) = held {
            match (&held.head.asked, &held.head.lapsed) {
                (Some(ask), _) if !carried => head.push(Thing::Ask(ask)),
                (None, Some(lapsed)) if self.dock_of(id).is_none_or(|dock| !dock.hidden.contains(&lapsed.id)) => head.push(Thing::Lapsed(lapsed)),
                _ => {}
            }
        }
        if !head.is_empty() {
            match groups.iter_mut().find(|group| group.kind == Kind::Questions) {
                Some(group) => {
                    let rows = std::mem::take(&mut group.things);
                    group.things = head.into_iter().chain(rows).collect();
                }
                None => groups.insert(0, Group { kind: Kind::Questions, label: "Questions", things: head }),
            }
        }
        groups
    }

    /// The things the panel lists, and the pages the band names for review: the To review rows, only where Fleet
    /// says the band offers the sheet.
    pub(super) fn things<'a>(&'a self, id: &str, cx: &'a gpui::App) -> (Vec<Thing<'a>>, Vec<&'a NeedsYouRow>) {
        let in_band = self.needs_of(id, cx).is_some_and(|view| view.review_in_band);
        let mut things = Vec::new();
        let mut review = Vec::new();
        for group in self.groups(id, cx) {
            if group.kind != NeedsYouSectionKind::ToReview {
                things.extend(group.things);
            } else if in_band {
                review.extend(group.things.into_iter().filter_map(|thing| match thing {
                    Thing::Row(row) => Some(row),
                    _ => None,
                }));
            }
        }
        (things, review)
    }

    /// Whether the band has a list to open: Fleet's `fold`, or a question the thread's head holds that Fleet's
    /// rows do not.
    pub(super) fn folds(&self, id: &str, cx: &gpui::App) -> bool {
        let (things, _) = self.things(id, cx);
        let fleet_folds = self.needs_of(id, cx).is_some_and(|view| view.fold);
        things.iter().any(|thing| !matches!(thing, Thing::Row(_))) || (fleet_folds && !things.is_empty())
    }

    /// The restarts the Job's items ask for, one key per command: Fleet's Restart section, whichever items ask it.
    pub(super) fn restarts(&self, id: &str, cx: &gpui::App) -> Vec<(String, BuildSource, bool, SharedString)> {
        let mut keys: Vec<(String, BuildSource, bool, SharedString)> = Vec::new();
        let Some(view) = self.needs_of(id, cx) else { return keys };
        let hidden = self.dock_of(id).map(|dock| &dock.hidden);
        for section in view.sections.iter().filter(|section| section.kind == NeedsYouSectionKind::Restart) {
            for row in section.rows.iter().filter(|row| !hidden.is_some_and(|hidden| hidden.contains(&row.id))) {
                if let NeedsYouAct::Restart { label, build, adopt, command } = &row.act {
                    if !keys.iter().any(|(held, ..)| held == command) {
                        keys.push((command.clone(), *build, *adopt, label.clone().into()));
                    }
                }
            }
        }
        keys
    }

    pub(super) fn thing_key(thing: &Thing) -> String {
        match thing {
            Thing::Ask(ask) => format!("question:{}", ask.call.call),
            Thing::Lapsed(lapsed) => format!("question:{}", lapsed.id),
            Thing::Row(row) => format!("item:{}", row.id),
        }
    }

    /// A question that has not opened the panel yet opens it, once.
    pub(super) fn arrive(&mut self, id: &SharedString, cx: &gpui::App) {
        // Nothing missing forgets that the tools band was put away.
        if self.record(id, cx).is_some_and(|record| record.tools_missing.is_none()) {
            if let Some(page) = self.pages.get_mut(id) {
                page.bands.tools_away = None;
            }
        }
        let key = {
            let (things, _) = self.things(id, cx);
            things.iter().find_map(|thing| match thing {
                Thing::Ask(ask) => Some(ask.call.call.clone()),
                Thing::Lapsed(lapsed) => Some(lapsed.id.clone()),
                Thing::Row(_) => None,
            })
        };
        let Some(key) = key else { return };
        if let Some(page) = self.pages.get_mut(id) {
            if page.dock.seen.insert(key) {
                page.dock.open = Some(true);
            }
        }
    }

    pub(super) fn set_needs(&mut self, open: bool, detail: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dock) = self.dock_mut() {
            dock.open = Some(open);
            dock.detail = detail;
        }
        if open {
            window.focus(&self.dock_focus, cx);
        } else if self.shown.as_ref().is_some_and(|id| !self.closed(id, cx)) {
            self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// ⌥⌘Y: the Needs you panel opens or folds from anywhere in the Job, where ⇧⌘Y would open the review sheet.
    pub(super) fn toggle_needs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.review_open() {
            return;
        }
        // Pages to review alone have no list to open: the band's press opens the sheet (`NeedsYouPanel`).
        if !self.folds(&id, cx) {
            return;
        }
        let up = self.needs_up(&id, cx);
        self.set_needs(!up, None, window, cx);
    }

    /// Whether the panel is up over the thread now. With only pages to review it has nothing to list and
    /// stays folded, as Electron's does, however it was left.
    pub(super) fn needs_up(&self, id: &str, cx: &gpui::App) -> bool {
        self.folds(id, cx) && self.dock_of(id).and_then(|dock| dock.open).unwrap_or(false)
    }

    /// The thread dims while the panel is up; a press on it, or a scroll of it, folds the panel so the
    /// thread can be read (his word, 10 Nov). The scroll goes on to the thread.
    pub(super) fn render_scrim(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.needs_up(id, cx) && !self.band_up(id, cx) {
            return None;
        }
        let look = cx.look();
        Some(
            div()
                .id("needs-scrim")
                .absolute()
                .inset_0()
                .bg(look.color(token::BG_BASE).opacity(0.5))
                .on_click(cx.listener(|this, _, window, cx| this.fold_over_thread(window, cx)))
                .on_scroll(cx.listener(|this, _, window, cx| this.fold_over_thread(window, cx)))
                .into_any_element(),
        )
    }

    fn fold_over_thread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.fold_bands(false, window, cx) {
            self.set_needs(false, None, window, cx);
        }
    }

    /// The band, and over it the panel when it is open.
    pub(super) fn render_needs(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let (things, review) = self.things(id, cx);
        let restarts = self.restarts(id, cx);
        // The marks on the band's head are Fleet's, in its order; the question the thread's head holds that Fleet's
        // rows do not carry comes first.
        let view = self.needs_of(id, cx);
        let on_a_row = |call: &str| view.is_some_and(|view| view.sections.iter().flat_map(|section| &section.rows).any(|row| row.id == format!("ask:{call}") || row.id == format!("perm:{call}")));
        let marks: Vec<(&'static str, String)> = things
            .iter()
            .filter_map(|thing| match thing {
                Thing::Ask(ask) if on_a_row(&ask.call.call) => None,
                Thing::Ask(ask) => Some(("message-circle-question-mark", ask_label(ask))),
                Thing::Lapsed(_) => Some(("message-circle-question-mark", "Unanswered".to_string())),
                Thing::Row(_) => None,
            })
            .chain(view.into_iter().flat_map(|view| view.glyphs.iter().map(|glyph| (icon_of(glyph.kind), glyph.said.clone()))))
            .collect();
        if marks.is_empty() && restarts.is_empty() {
            if let Some(page) = self.pages.get(id) {
                page.keys.mark("question:");
                page.keys.sweep();
            }
            return None;
        }
        let folds = self.folds(id, cx);
        // A band with nothing to list, a restart or pages to review alone, is never drawn open and has no fold.
        let open = self.needs_up(id, cx);
        let hue = look.color(token::STEP_WAITING);
        let glyphs: Vec<AnyElement> = marks
            .into_iter()
            .enumerate()
            .map(|(at, (icon, said))| {
                div()
                    .id(SharedString::from(format!("needs-glyph:{at}")))
                    .child(glyph(look, icon, Size::Small, look.color(token::FG_INVERSE)))
                    .tooltip(tip(said))
                    .into_any_element()
            })
            .collect();
        let trigger = labelled(
            div()
                .id("needs-trigger")
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .cursor_pointer()
                .child(glyph(look, "bell-ring", Size::Small, look.color(token::FG_INVERSE)))
                .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child("Needs you"))
                .child(div().flex().items_center().gap(look.length(token::SPACE_1)).children(glyphs))
                .child(div().flex_1())
                .when(folds, |trigger| {
                    trigger.child(glyph(look, if open { "chevron-down" } else { "chevron-up" }, Size::Small, look.color(token::FG_INVERSE)))
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    let Some(id) = this.shown.clone() else { return };
                    // With nothing else to answer, the band opens the sheet.
                    let alone = !this.folds(&id, cx) && this.review_applies(&id, cx);
                    if alone {
                        this.review_press(window, cx);
                    } else {
                        let up = this.needs_up(&id, cx);
                        this.set_needs(!up, None, window, cx);
                    }
                })),
            Role::Button,
            "Needs you",
        );
        let high = look.length(token::SPACE_6) + look.length(token::SPACE_1);
        let edge = look.length(token::BORDER_WIDTH);
        // Folded or open, the band lies over the frame's edge and draws that edge itself, over the key at
        // its end, so the frame's amber line runs unbroken round the corner. gpui clips a child to a square
        // box, so the key at the band's end takes the corner itself.
        let corner = look.length(token::RADIUS_LG);
        let last = restarts.len().checked_sub(1);
        let band = div()
            .flex_none()
            .h(high + edge)
            .border_t(edge)
            .border_l(edge)
            .border_r(edge)
            .border_color(hue)
            .flex()
            .items_center()
            .rounded_t(corner)
            .bg(hue)
            .text_color(look.color(token::FG_INVERSE))
            .overflow_hidden()
            .child(trigger)
            .children(restarts.into_iter().enumerate().map(|(at, (command, build, adopt, label))| {
                self.restart_key(id, command, build, adopt, label, (Some(at) == last).then_some(corner - edge), look, cx)
            }));
        let keys = self.pages.get(id).map(|page| page.keys.clone());
        if let Some(keys) = &keys {
            keys.mark("question:");
        }
        let panel = open.then(|| self.render_panel(id, &things, &review, look, cx));
        if let Some(keys) = &keys {
            keys.sweep();
        }
        // The band heads the composer's frame. Open, it rises over the thread with the panel under it,
        // and the box below stays where it was.
        let slot = div().relative().flex_none().w_full().h(high);
        Some(
            match panel {
                Some(panel) => slot.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left(-edge)
                        .right(-edge)
                        .flex()
                        .flex_col()
                        .child(band)
                        .child(panel),
                ),
                None => slot.child(div().absolute().top(-edge).left(-edge).right(-edge).child(band)),
            }
            .into_any_element(),
        )
    }

    /// A restart's key at the band's end, as Electron's `RestartKey`: its mark and its label, pressed
    /// folded or open. Nothing starts it again while it runs or once it passed; a failure leaves it
    /// to press again.
    #[allow(clippy::too_many_arguments)]
    fn restart_key(&self, id: &str, command: String, build: BuildSource, adopt: bool, label: SharedString, corner: Option<Pixels>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let mark = self.dock_of(id).and_then(|dock| dock.runs.get(&command).cloned());
        let held = matches!(mark, Some(RunMark::Running | RunMark::Passed));
        let (icon, said) = mark.as_ref().map_or(("rotate-cw", SharedString::from(command.clone())), RunMark::look);
        let accent = look.color(token::ACCENT);
        let hover = look.color(token::ACCENT_HOVER);
        labelled(
            div()
                .id(SharedString::from(format!("needs-restart:{command}")))
                .flex_none()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .bg(accent)
                .when_some(corner, |key, corner| key.rounded_tr(corner))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(!held, move |key| key.cursor_pointer().hover(move |style| style.bg(hover)))
                .child(glyph(look, icon, Size::Small, look.color(token::FG_INVERSE)))
                .child(label.clone())
                .tooltip(tip(said.to_string()))
                .when(!held, |key| {
                    key.on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.restart(command.clone(), build, adopt, window, cx);
                    }))
                }),
            Role::Button,
            label,
        )
        .into_any_element()
    }

    /// Mark a command pressed in the shown Job.
    fn mark_run(&mut self, job: &SharedString, command: &str, mark: Option<RunMark>, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            match mark {
                Some(mark) => page.dock.runs.insert(command.to_string(), mark),
                None => page.dock.runs.remove(command),
            };
        }
        cx.notify();
    }

    /// The restart key: Fleet's build switch, pressed as the title bar's Refresh preview presses it
    /// (`sessions-wired.tsx` `switching`). Fleet answering that it began is the pass.
    fn restart(&mut self, command: String, build: BuildSource, adopt: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let body = ipc::encode(&ipc::ChangeFleetBuild { build, adopt }).map_err(|why| format!("{why:?}"));
        self.mark_run(&job, &command, Some(RunMark::Running), cx);
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            self.mark_run(&job, &command, Some(RunMark::Failed("Fleet is not connected".into())), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/fleet/build/change", body))?;
                    if (200..300).contains(&status) {
                        return Ok(());
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            let mark = match answer {
                Ok(()) => RunMark::Passed,
                Err(why) => RunMark::Failed(why.into()),
            };
            let _ = this.update(cx, |this, cx| this.mark_run(&job, &command, Some(mark), cx));
        })
        .detach();
    }

    /// A run item pressed: its command runs where the Job works (`start_session_run`), and its row is
    /// marked until the run exits. A clean exit is settled by Fleet, which takes the item off the list.
    pub(super) fn run_item(&mut self, command: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.dock_of(&job).and_then(|dock| dock.runs.get(&command)).is_some_and(|mark| matches!(mark, RunMark::Running)) {
            return;
        }
        let start = ipc::StartSessionRun { session_id: SessionId::carried(job.to_string()), command: command.clone(), cols: None, rows: None };
        let body = ipc::encode(&start).map_err(|why| format!("{why:?}"));
        self.mark_run(&job, &command, Some(RunMark::Running), cx);
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            self.mark_run(&job, &command, Some(RunMark::Failed("Fleet is not connected".into())), cx);
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    runtime.block_on(async move {
                        let (status, bytes) = crate::transport::post(port, "/sessions/run/start", body).await?;
                        if !(200..300).contains(&status) {
                            return Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message));
                        }
                        let underway = ipc::decode::<ipc::SessionRunUnderway>("SessionRunUnderway", &bytes).map_err(|why| format!("{why:?}"))?;
                        let io = crate::transport::upgrade(port, &format!("/sessions/run/{}/observe", underway.run_id)).await?;
                        let mut messages = crate::frames::Messages::new(io);
                        while let Some(text) = messages.next().await {
                            if let Ok(ipc::SessionRunMessage::Exited { code, stopped }) = ipc::decode::<ipc::SessionRunMessage>("SessionRunMessage", text.as_bytes()) {
                                return match code {
                                    Some(0) => Ok(()),
                                    Some(code) => Err(format!("Exited {code}")),
                                    None if stopped => Err("Stopped".to_string()),
                                    None => Err("Ended by a signal".to_string()),
                                };
                            }
                        }
                        Err("The run closed before it exited".to_string())
                    })
                })
                .await;
            let mark = match answer {
                Ok(()) => RunMark::Passed,
                Err(why) => RunMark::Failed(why.into()),
            };
            let _ = this.update(cx, |this, cx| this.mark_run(&job, &command, Some(mark), cx));
        })
        .detach();
    }

    fn render_panel(&self, id: &SharedString, things: &[Thing], review: &[&NeedsYouRow], look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STEP_WAITING);
        let detail = self.dock_of(id).and_then(|dock| dock.detail.clone());
        let only = things.len() == 1 && review.is_empty() && matches!(things[0], Thing::Ask(_) | Thing::Lapsed(_));
        let detail = detail.or_else(|| only.then(|| Self::thing_key(&things[0])));
        let content: AnyElement = match detail.as_ref().and_then(|key| things.iter().find(|thing| &Self::thing_key(thing) == key)) {
            Some(thing) => self.render_detail(id, thing, only, look, cx),
            None => self.render_list(id, look, cx),
        };
        // Up to 95% of the thread, band included, so the Job's head always shows.
        let thread = self.pages.get(id).map(|page| page.list.viewport_bounds().size.height).unwrap_or_default();
        let band = look.length(token::SPACE_6) + look.length(token::SPACE_1);
        let most = if thread > band { thread * 0.95 - band } else { look.length(token::SPACE_12) * 10. };
        div()
            .id("needs-panel")
            .track_focus(&self.dock_focus)
            .occlude()
            .on_key_down(cx.listener(Self::on_panel_key))
            .max_h(most)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_4))
            .border_x(look.length(token::BORDER_WIDTH))
            .border_color(hue)
            .bg(look.color(token::BG_RAISED))
            .child(content)
            .into_any_element()
    }

    fn on_panel_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let typing = self.dock_typing(window, cx);
        match event.keystroke.key.as_str() {
            "escape" if self.dock_of(&id).is_some_and(|dock| dock.row_box.is_some()) => {
                self.close_row_box(window, cx);
                cx.stop_propagation();
            }
            "escape" => {
                let in_detail = self.dock_of(&id).is_some_and(|dock| dock.detail.is_some());
                self.set_needs(in_detail, None, window, cx);
                cx.stop_propagation();
            }
            "backspace" if !typing && self.dock_of(&id).is_some_and(|dock| dock.detail.is_some()) => {
                self.set_needs(true, None, window, cx);
                cx.stop_propagation();
            }
            key if !typing && key.len() == 1 && key.chars().all(|c| c.is_ascii_digit()) && !event.keystroke.modifiers.alt => {
                let n = key.parse::<usize>().unwrap_or(0);
                if n >= 1 {
                    self.pick_digit(n, window, cx);
                    cx.stop_propagation();
                }
            }
            _ => {}
        }
    }

    /// The caret is in a box of the panel.
    fn dock_typing(&self, window: &Window, cx: &gpui::App) -> bool {
        let Some(id) = self.shown.as_ref() else { return false };
        self.pages
            .get(id)
            .is_some_and(|page| page.cards.boxes.iter().any(|(name, area)| name.starts_with("dock:") && area.read(cx).focus_handle(cx).contains_focused(window, cx)))
    }

    /// The list: Fleet's sections in its order, each under its label, each row as Fleet gave it.
    fn render_list(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let in_band = self.needs_of(id, cx).is_some_and(|view| view.review_in_band);
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children(self.groups(id, cx).into_iter().map(|group| {
                let label = SharedString::from(group.label.to_string());
                labelled(
                    div()
                        .id(SharedString::from(format!("needs-group:{label}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        // A ring around the section stands off its heading and rows, as a row's does.
                        .p(look.length(token::SPACE_2))
                        .m(-look.length(token::SPACE_2))
                        .child(
                            div()
                                .text_size(look.length(token::TEXT_2XS))
                                .line_height(look.length(token::LEADING_2XS))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::FG_MUTED))
                                .child(label.clone()),
                        )
                        .children(if group.kind == NeedsYouSectionKind::ToReview && in_band {
                            vec![self.sheet_row(&group.things, look, cx)]
                        } else {
                            group.things.iter().map(|thing| self.thing_row(id, thing, look, cx)).collect()
                        }),
                    Role::Region,
                    label,
                )
            }))
            .into_any_element()
    }

    /// Where Fleet's band offers the review sheet, the pages to read are one row: a mark for each, and a press
    /// that opens the sheet (Electron's Review row).
    fn sheet_row(&self, pages: &[Thing], look: &Look, cx: &Context<Self>) -> AnyElement {
        let rows: Vec<&NeedsYouRow> = pages.iter().filter_map(|thing| if let Thing::Row(row) = thing { Some(*row) } else { None }).collect();
        let said = rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>().join(", ");
        let marks = rows.iter().map(|row| glyph(look, icon_of(row.glyph.kind), Size::Mark, look.color(token::FG_MUTED)));
        labelled(
            filled(look, div().id("needs-row:review"))
                .child(div().id("needs-mark:review").flex_none().flex().items_center().gap(look.length(token::SPACE_1)).children(marks).tooltip(tip("Opens the review")))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(said)))
                .tab_index(0)
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .on_click(cx.listener(|this, _, window, cx| this.review_press(window, cx))),
            Role::Button,
            "Review",
        )
        .into_any_element()
    }

    /// One thing of the list as a row.
    fn thing_row(&self, id: &SharedString, thing: &Thing, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = Self::thing_key(thing);
        match thing {
            Thing::Ask(ask) => {
                let open = key.clone();
                self.list_row(id, &key, "message-circle-question-mark", ask_label(ask).into(), ask_text(ask), Box::new(move |this, window, cx| this.set_needs(true, Some(open.clone()), window, cx)), None, look, cx)
            }
            Thing::Lapsed(lapsed) => {
                let open = key.clone();
                let text = lapsed.questions.first().map(|q| q.question.clone()).unwrap_or_default();
                self.list_row(id, &key, "message-circle-question-mark", "Unanswered".into(), text, Box::new(move |this, window, cx| this.set_needs(true, Some(open.clone()), window, cx)), None, look, cx)
            }
            Thing::Row(row) => {
                let command = match &row.act {
                    NeedsYouAct::Run { command } | NeedsYouAct::Restart { command, .. } | NeedsYouAct::Steps { command: Some(command), .. } => Some(command),
                    _ => None,
                };
                let ran = command.and_then(|command| self.dock_of(id)?.runs.get(command)).map(RunMark::look);
                let (icon, said) = ran.unwrap_or((icon_of(row.glyph.kind), row.glyph.said.clone().into()));
                let (act, item) = (row.act.clone(), row.id.clone());
                let open = key.clone();
                self.list_row(id, &key, icon, said, row.text.clone(), Box::new(move |this, window, cx| this.press_row(&open, &item, &act, window, cx)), Some(row), look, cx)
            }
        }
    }

    /// What a press on a row does, as Fleet's `act` says.
    fn press_row(&mut self, key: &str, item: &str, act: &NeedsYouAct, window: &mut Window, cx: &mut Context<Self>) {
        match act {
            NeedsYouAct::OpenPlan | NeedsYouAct::OpenSketch { .. } | NeedsYouAct::OpenDocument { .. } | NeedsYouAct::OpenDraft { .. } => {
                // Where the band offers the sheet it opens; else the page is on the thread's card.
                let sheet = self.shown.clone().and_then(|id| self.needs_of(&id, cx).map(|view| view.review_in_band)).unwrap_or(false);
                if sheet {
                    self.review_press(window, cx);
                } else if let NeedsYouAct::OpenDraft { row } = act {
                    // A draft opens its card as a tab of the Job (`openDraftCard`).
                    self.set_needs(false, None, window, cx);
                    self.open_draft_tab(row, window, cx);
                } else {
                    self.set_needs(false, None, window, cx);
                    self.pin_shown();
                    cx.notify();
                }
            }
            NeedsYouAct::Answer { options, .. } if !options.is_empty() => self.set_needs(true, Some(key.to_string()), window, cx),
            // A procedure with its command runs it where the Job works, as a command does.
            NeedsYouAct::Run { command } | NeedsYouAct::Steps { command: Some(command), .. } => self.run_item(command.clone(), window, cx),
            NeedsYouAct::Restart { command, build, adopt, .. } => self.restart(command.clone(), *build, *adopt, window, cx),
            NeedsYouAct::Reply | NeedsYouAct::Message => {
                self.set_needs(false, None, window, cx);
                self.to_composer(window, cx);
            }
            NeedsYouAct::OpenJob { id } => self.open_job(id.clone(), cx),
            NeedsYouAct::Done { .. } => self.open_row_box(item, RowBoxFor::Note, window, cx),
            NeedsYouAct::Walk { url } => {
                self.set_needs(false, None, window, cx);
                self.press_walk(url, window, cx);
            }
            NeedsYouAct::Answer { .. } | NeedsYouAct::Steps { command: None, .. } | NeedsYouAct::ApprovePr { .. } => self.set_needs(false, None, window, cx),
            NeedsYouAct::None => {}
        }
    }

    /// A row of the list: its mark, its words, and at its end what Fleet says can be done to it.
    #[allow(clippy::too_many_arguments)]
    fn list_row(
        &self,
        id: &SharedString,
        key: &str,
        icon: &'static str,
        said: SharedString,
        text: String,
        press: Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>,
        row: Option<&NeedsYouRow>,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let name = text.clone();
        let dock = self.dock_of(id);
        let page = self.pages.get(id);
        let misread = row.filter(|row| row.dismiss == NeedsYouDismiss::NotAQuestion);
        let spot = misread.zip(page).map(|(_, page)| page.keys.spot(&format!("needs-row:{key}"), cx));
        let refused = row.and(dock).and_then(|dock| dock.said.get(key).cloned());
        let confirmed = misread.is_some_and(|row| self.item_confirmed(id, &row.id, cx) || dock.is_some_and(|dock| dock.confirmed.contains(&row.id)));
        let thumbs = misread.zip(spot.as_ref()).map(|(row, spot)| self.thumbs(&row.id, confirmed, spot.focus.clone(), look, cx));
        let done = row.filter(|row| matches!(row.act, NeedsYouAct::Done { .. })).map(|row| self.done_button(row, look, cx));
        let away = row.and_then(|row| self.away_button(row, look, cx));
        let steps = match row.map(|row| &row.act) {
            Some(NeedsYouAct::Done { steps }) if !steps.is_empty() => Some(self.step_lines(key, steps, look)),
            _ => None,
        };
        let boxed = row.zip(dock).and_then(|(row, dock)| dock.row_box.as_ref().filter(|open| open.row == row.id)).and_then(|open| {
            let name = if open.what == RowBoxFor::Reason { "Reason" } else { "Note with Done" };
            let area = row.and_then(|row| self.box_of(&format!("dock:row:{}", row.id), Mode::Note, name, look, cx))?;
            // Enter sends it, as the cap beside the page's other boxes says.
            Some(div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().child(area)).child(super::acts::kbd(look, "\u{21b5}")).into_any_element())
        });
        let line = filled(look, div().id(SharedString::from(format!("needs-row:{key}"))))
            .child(div().id(SharedString::from(format!("needs-mark:{key}"))).flex_none().child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED))).tooltip(tip(said)))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(text)))
            .children(thumbs)
            .children(done)
            .children(away)
            .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)));
        let line = if spot.is_none() { line.tab_index(0).focus_visible(|style| style.border_color(look.color(token::ACCENT))) } else { line };
        // A sentence Fleet read from a reply: a right press offers A question and Not a question.
        let line = match (misread, spot.as_ref()) {
            (Some(row), Some(spot)) => {
                let item_id = row.id.clone();
                self.menu_on(
                    line.track_focus(&spot.focus),
                    spot,
                    "Question",
                    spot.focus.clone(),
                    Rc::new(move || misread_entries(confirmed)),
                    Rc::new(move |this, choice, window, cx| match choice {
                        "yes" => this.misread_yes(&item_id, window, cx),
                        _ => this.misread_no(&item_id, window, cx),
                    }),
                    cx,
                )
            }
            _ => line,
        };
        let line = labelled(line, Role::Button, format!("Open {name}")).into_any_element();
        if steps.is_none() && boxed.is_none() && refused.is_none() {
            return line;
        }
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(line)
            .children(steps)
            .children(boxed)
            .children(refused.map(|why| super::acts::refusal(look, key, why)))
            .into_any_element()
    }

    /// Whether Fleet's record says the sentence read from a reply was said to be a question.
    fn item_confirmed(&self, id: &str, item: &str, cx: &gpui::App) -> bool {
        self.record(id, cx).and_then(|record| record.waiting_for.iter().find(|one| one.id == item)).is_some_and(|one| one.confirmed)
    }

    /// The procedure a To do row carries, one numbered line each, under the row.
    fn step_lines(&self, key: &str, steps: &[String], look: &Look) -> AnyElement {
        div()
            .id(SharedString::from(format!("needs-steps:{key}")))
            .flex()
            .flex_col()
            .px(look.length(token::SPACE_3))
            .children(steps.iter().enumerate().map(|(at, step)| {
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(format!("{}. {step}", at + 1)))
            }))
            .into_any_element()
    }

    /// A small press at a row's end, a tab stop that Enter and Space press.
    fn row_control(&self, id: impl Into<gpui::ElementId>, primary: bool, look: &Look) -> gpui::Stateful<gpui::Div> {
        let control = div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .h(look.length(token::H_CONTROL_SM))
            .min_w(look.length(token::H_CONTROL_SM))
            .rounded(look.length(token::RADIUS_MD))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .tab_index(0)
            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)));
        if primary {
            control.bg(look.color(token::ACCENT)).text_color(look.color(token::TEXT_ON_ACCENT)).hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
        } else {
            control.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)))
        }
    }

    /// Done on a To do row: he did it. The row goes at once with whatever note is in its box.
    fn done_button(&self, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> AnyElement {
        let item = row.id.clone();
        labelled(
            self.row_control(SharedString::from(format!("needs-done:{}", row.id)), true, look)
                .px(look.length(token::SPACE_3))
                .child("Done")
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    let note = this.typed_in(&format!("dock:row:{item}"), Mode::Note, cx);
                    this.done_row(&item, Some(note), window, cx);
                })),
            Role::Button,
            "Done",
        )
        .into_any_element()
    }

    /// How a row is put away, as Fleet's `dismiss` says: nothing, a ×, Skip, or a × that takes a reason.
    /// A sentence read from a reply is put away by the thumbs, drawn beside it.
    fn away_button(&self, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let item = row.id.clone();
        let name = format!("Remove {}", row.text);
        match row.dismiss {
            NeedsYouDismiss::None | NeedsYouDismiss::NotAQuestion => None,
            NeedsYouDismiss::Plain => Some(
                labelled(
                    self.row_control(SharedString::from(format!("needs-remove:{}", row.id)), false, look)
                        .size(look.length(token::H_CONTROL_SM))
                        .tooltip(tip("Remove"))
                        .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.dismiss_row(&item, None, false, window, cx);
                        })),
                    Role::Button,
                    name,
                )
                .into_any_element(),
            ),
            NeedsYouDismiss::Skip => Some(
                labelled(
                    self.row_control(SharedString::from(format!("needs-skip:{}", row.id)), false, look)
                        .px(look.length(token::SPACE_3))
                        .child("Skip")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.dismiss_row(&item, None, false, window, cx);
                        })),
                    Role::Button,
                    "Skip",
                )
                .into_any_element(),
            ),
            NeedsYouDismiss::Reason => Some(
                labelled(
                    self.row_control(SharedString::from(format!("needs-remove:{}", row.id)), false, look)
                        .size(look.length(token::H_CONTROL_SM))
                        .tooltip(tip("Remove"))
                        .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.open_row_box(&item, RowBoxFor::Reason, window, cx);
                        })),
                    Role::Button,
                    name,
                )
                .into_any_element(),
            ),
        }
    }

    /// Open the one-line box under a row for its reason or its note, or put it away if it is open for that.
    fn open_row_box(&mut self, row: &str, what: RowBoxFor, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.dock_of(&id).and_then(|dock| dock.row_box.as_ref()).is_some_and(|open| open.row == row && open.what == what) {
            self.close_row_box(window, cx);
            return;
        }
        self.drop_row_box();
        let area = cx.new(|cx| gpui_component::input::TextareaState::new(window, cx).auto_grow(1, 3).submit_on_enter(true));
        let enter = cx.subscribe_in(&area, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { shift: false, .. }) {
                this.send_row_box(window, cx);
            }
        });
        area.update(cx, |area, cx| area.focus(window, cx));
        if let Some(page) = self.pages.get_mut(&id) {
            page.cards.boxes.insert(format!("dock:row:{row}:{:?}", Mode::Note), area);
            page.dock.row_box = Some(RowBox { row: row.to_string(), what, _enter: enter });
        }
        cx.notify();
    }

    /// Forget the open box and what was typed in it.
    fn drop_row_box(&mut self) {
        let Some(id) = self.shown.clone() else { return };
        if let Some(page) = self.pages.get_mut(&id) {
            if page.dock.row_box.take().is_some() {
                page.cards.boxes.retain(|name, _| !name.starts_with("dock:row:"));
            }
        }
    }

    /// Esc: the box goes, nothing is sent, and the keys are the panel's again.
    fn close_row_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.drop_row_box();
        window.focus(&self.dock_focus, cx);
        cx.notify();
    }

    /// Enter in the box: the reason removes the row, the note is done.
    fn send_row_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some((row, what)) = self.dock_of(&id).and_then(|dock| dock.row_box.as_ref()).map(|open| (open.row.clone(), open.what)) else { return };
        let words = self.typed_in(&format!("dock:row:{row}"), Mode::Note, cx);
        match what {
            RowBoxFor::Reason => self.dismiss_row(&row, Some(words), false, window, cx),
            RowBoxFor::Note => self.done_row(&row, Some(words), window, cx),
        }
        window.focus(&self.dock_focus, cx);
    }

    /// Put a row away. It leaves the list at once; Fleet's refusal puts it back with its words.
    pub(super) fn dismiss_row(&mut self, item: &str, reason: Option<String>, misread: bool, window: &mut Window, cx: &mut Context<Self>) {
        let reason = reason.map(|words| words.trim().to_string()).filter(|words| !words.is_empty());
        let body = ipc::encode(&DismissWaiting { session_id: SessionId::carried(self.shown.clone().unwrap_or_default().to_string()), item_id: item.to_string(), reason, misread });
        self.settle_row(item, "/sessions/waiting/dismiss", body.map_err(|why| format!("{why:?}")), window, cx);
    }

    /// He did what a To do row asked: `done_waiting`, with his note if he gave one.
    pub(super) fn done_row(&mut self, item: &str, note: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let note = note.map(|words| words.trim().to_string()).filter(|words| !words.is_empty());
        let body = ipc::encode(&DoneWaiting { session_id: SessionId::carried(self.shown.clone().unwrap_or_default().to_string()), item_id: item.to_string(), note });
        self.settle_row(item, "/sessions/waiting/done", body.map_err(|why| format!("{why:?}")), window, cx);
    }

    /// The row goes now; a refusal takes that back and says Fleet's reason under it.
    fn settle_row(&mut self, item: &str, route: &'static str, body: Result<String, String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.drop_row_box();
        if let Some(dock) = self.dock_mut() {
            dock.hidden.insert(item.to_string());
            if dock.detail.as_deref() == Some(&format!("item:{item}")) {
                dock.detail = None;
            }
            dock.said.remove(&format!("item:{item}"));
        }
        // On mock data nothing answers, so the view is grouped again without the row.
        self.want_needs(&job, cx);
        let (back, item) = (job.clone(), item.to_string());
        self.post_with(
            job,
            route,
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.dock.hidden.remove(&item);
                    page.dock.said.insert(format!("item:{item}"), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        window.focus(&self.dock_focus, cx);
        cx.notify();
    }

    /// The thread goes to its end.
    pub(super) fn pin_shown(&mut self) {
        if let Some(id) = self.shown.clone() {
            self.pin(&id);
        }
    }

    /// The two thumbs at the end of a sentence Fleet read from a reply: A question, and Not a question.
    /// Confirmed, Fleet has nothing left to ask, so there are none.
    fn thumbs(&self, item: &str, confirmed: bool, back: FocusHandle, look: &Look, cx: &Context<Self>) -> AnyElement {
        let thumb = |id: &'static str, said: &'static str, icon: &'static str, yes: bool| {
            let (item, back) = (item.to_string(), back.clone());
            labelled(
                div()
                    .id(SharedString::from(format!("{id}:{item}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip(said))
                    .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        if yes {
                            this.misread_yes(&item, window, cx);
                            window.focus(&back, cx);
                        } else {
                            this.misread_no(&item, window, cx);
                        }
                    })),
                Role::Button,
                said,
            )
        };
        div()
            .flex_none()
            .flex()
            .items_center()
            .children((!confirmed).then(|| thumb("misread-yes", "A question", "thumbs-up", true)))
            .child(thumb("misread-no", "Not a question", "thumbs-down", false))
            .into_any_element()
    }

    /// Thumbs up: the sentence is a question. It reads confirmed at once; a refusal takes that back.
    pub(super) fn misread_yes(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let Some(dock) = self.dock_mut() {
            dock.confirmed.insert(item.to_string());
            dock.said.remove(&format!("item:{item}"));
        }
        let body = ipc::encode(&ConfirmWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string() }).map_err(|why| format!("{why:?}"));
        let (back, item) = (job.clone(), item.to_string());
        self.post_with(
            job,
            "/sessions/waiting/confirm",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.dock.confirmed.remove(&item);
                    page.dock.said.insert(format!("item:{item}"), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    /// Thumbs down: the sentence asked nothing. It leaves the list at once; a refusal puts it back.
    pub(super) fn misread_no(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_row(item, None, true, window, cx);
    }

    fn render_detail(&self, id: &SharedString, thing: &Thing, only: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let back = labelled(
            div()
                .id("needs-back")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Back"))
                .child(glyph(look, "chevron-left", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, _, window, cx| this.set_needs(!only, None, window, cx))),
            Role::Button,
            "Back",
        );
        let body = match thing {
            Thing::Ask(ask) if ask.call.questions.is_empty() => self.permission(id, ask, look, cx),
            Thing::Ask(ask) => self.deck(id, &ask.call.call, &ask.call.questions, true, false, super::deck::Source::Call(ask.call.call.clone()), look, cx),
            Thing::Lapsed(lapsed) => self.deck(id, &lapsed.id, &lapsed.questions, true, false, super::deck::Source::Lapsed(lapsed.id.clone()), look, cx),
            Thing::Row(row) => {
                let questions = vec![ipc::AskedQuestion {
                    question: row.text.clone(),
                    header: String::new(),
                    multi_select: false,
                    options: row_options(row)
                        .iter()
                        .map(|option| ipc::AskedChoice { label: option.label.clone(), description: option.description.clone().unwrap_or_default(), preview: String::new() })
                        .collect(),
                }];
                self.deck(id, &row.id, &questions, false, true, super::deck::Source::Waiting(row.id.clone()), look, cx)
            }
        };
        let said = self.dock_of(id).and_then(|dock| dock.said.get(&Self::thing_key(thing)).cloned());
        // A sentence Fleet read from a reply carries its thumbs in the head too.
        let head_thumbs = match thing {
            Thing::Row(row) if row.dismiss == NeedsYouDismiss::NotAQuestion => {
                let confirmed = self.item_confirmed(id, &row.id, cx) || self.dock_of(id).is_some_and(|dock| dock.confirmed.contains(&row.id));
                Some(self.thumbs(&row.id, confirmed, self.focus.clone(), look, cx))
            }
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(back).children(head_thumbs))
            .child(div().w_full().min_w_0().child(body))
            .children(said.map(|why| super::acts::refusal(look, "dock", why)))
            .into_any_element()
    }
}

