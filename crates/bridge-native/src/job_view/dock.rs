//! What waits on the person, over the message box: the Needs you band and the panel that opens from
//! it (`job-view.md`, B7). The question deck is `deck.rs` and the permission card `perm.rs`.
//!
//! **Electron draws a question or a permission in this panel, not in the thread.** The thread's tail
//! holds the plan and the pinned sketch; a question, a permission and the lapsed questions are here,
//! where `1` to `9` and the four `⌥` keys reach them with the caret still in the box.

use std::collections::{HashMap, HashSet};

use gpui::{div, prelude::*, AnyElement, ClipboardItem, Context, Focusable as _, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{Lapsed, ReachesTo, ThreadAsk, WaitingActKind, WaitingItem, WaitingSource};

use super::deck::Deck;
use super::perm::Perm;
use super::rows::{eyebrow, glyph, tip, Size};
use super::JobView;
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
    pub said: HashMap<String, SharedString>,
}

/// One thing the panel lists.
pub(super) enum Thing<'a> {
    Ask(&'a ThreadAsk),
    Lapsed(&'a Lapsed),
    Item(&'a WaitingItem),
}

pub(super) fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// What a waiting item stands for in the list: its glyph and the words that name it.
fn mark_of(item: &WaitingItem) -> (&'static str, &'static str) {
    if !item.options.is_empty() {
        return ("message-circle-question-mark", "Question");
    }
    match item.act.as_ref().map(|act| act.kind) {
        Some(WaitingActKind::Walk) => ("app-window", "Opens the walk"),
        Some(WaitingActKind::Answer) => ("message-square", "Goes to the question"),
        Some(WaitingActKind::ApprovePr) => ("git-pull-request-arrow", "Opens the pull request"),
        Some(WaitingActKind::Run) => ("terminal", "Copies the command"),
        Some(WaitingActKind::Message) => ("square-pen", "Goes to the message"),
        None if !item.steps.is_empty() => ("list-checks", "Steps"),
        None => ("bell-ring", "Waiting on you"),
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

    /// The things the panel lists, in the order Electron lists them, and the pages waiting for review.
    pub(super) fn things<'a>(&'a self, id: &str, cx: &'a gpui::App) -> (Vec<Thing<'a>>, Vec<&'a WaitingItem>) {
        let mut things = Vec::new();
        let mut review = Vec::new();
        if let Some(held) = self.thread(id, cx) {
            if let Some(ask) = &held.head.asked {
                things.push(Thing::Ask(ask));
            } else if let Some(lapsed) = held.head.lapsed.as_ref().filter(|lapsed| self.dock_of(id).is_none_or(|dock| !dock.hidden.contains(&lapsed.id))) {
                things.push(Thing::Lapsed(lapsed));
            }
        }
        if let Some(record) = self.record(id, cx) {
            for item in &record.waiting_for {
                match item.source {
                    WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document => review.push(item),
                    WaitingSource::Agent | WaitingSource::Walk | WaitingSource::Replied => things.push(Thing::Item(item)),
                    _ => {}
                }
            }
        }
        (things, review)
    }

    pub(super) fn thing_key(thing: &Thing) -> String {
        match thing {
            Thing::Ask(ask) => format!("question:{}", ask.call.call),
            Thing::Lapsed(lapsed) => format!("question:{}", lapsed.id),
            Thing::Item(item) => format!("item:{}", item.id),
        }
    }

    /// A question that has not opened the panel yet opens it, once.
    pub(super) fn arrive(&mut self, id: &SharedString, cx: &gpui::App) {
        let key = {
            let (things, _) = self.things(id, cx);
            things.iter().find_map(|thing| match thing {
                Thing::Ask(ask) => Some(ask.call.call.clone()),
                Thing::Lapsed(lapsed) => Some(lapsed.id.clone()),
                Thing::Item(_) => None,
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
            self.composer.update(cx, |composer, cx| composer.focus(window, cx));
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// Whether the panel is up over the thread now.
    pub(super) fn needs_up(&self, id: &str, cx: &gpui::App) -> bool {
        let (things, review) = self.things(id, cx);
        (!things.is_empty() || !review.is_empty()) && self.dock_of(id).and_then(|dock| dock.open).unwrap_or(false)
    }

    /// The thread dims while the panel is up; a press on it folds the panel.
    pub(super) fn render_scrim(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.needs_up(id, cx) {
            return None;
        }
        let look = cx.look();
        Some(
            div()
                .id("needs-scrim")
                .absolute()
                .inset_0()
                .bg(look.color(token::BG_BASE).opacity(0.5))
                .on_click(cx.listener(|this, _, window, cx| this.set_needs(false, None, window, cx)))
                .into_any_element(),
        )
    }

    /// The band, and over it the panel when it is open.
    pub(super) fn render_needs(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let (things, review) = self.things(id, cx);
        if things.is_empty() && review.is_empty() {
            if let Some(page) = self.pages.get(id) {
                page.keys.mark("question:");
                page.keys.sweep();
            }
            return None;
        }
        let open = self.needs_up(id, cx);
        let hue = look.color(token::STEP_WAITING);
        let glyphs: Vec<AnyElement> = things
            .iter()
            .map(|thing| match thing {
                Thing::Ask(ask) => ("message-circle-question-mark", ask_label(ask)),
                Thing::Lapsed(_) => ("message-circle-question-mark", "Unanswered".to_string()),
                Thing::Item(item) => (mark_of(item).0, mark_of(item).1.to_string()),
            })
            .chain(review.iter().map(|item| {
                let icon = match item.source {
                    WaitingSource::Plan => "list-checks",
                    WaitingSource::Sketch => "pencil-ruler",
                    _ => "file-text",
                };
                (icon, item.text.clone())
            }))
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
                .child(glyph(look, if open { "chevron-down" } else { "chevron-up" }, Size::Small, look.color(token::FG_INVERSE)))
                .on_click(cx.listener(|this, _, window, cx| {
                    let up = this.shown.clone().is_some_and(|id| this.needs_up(&id, cx));
                    this.set_needs(!up, None, window, cx);
                })),
            Role::Button,
            "Needs you",
        );
        let band = div()
            .flex_none()
            .h(look.length(token::SPACE_6) + look.length(token::SPACE_1))
            .flex()
            .items_center()
            .rounded_t(look.length(token::RADIUS_LG))
            .bg(hue)
            .text_color(look.color(token::FG_INVERSE))
            .overflow_hidden()
            .child(trigger);
        let keys = self.pages.get(id).map(|page| page.keys.clone());
        if let Some(keys) = &keys {
            keys.mark("question:");
        }
        let panel = open.then(|| self.render_panel(id, &things, &review, look, cx));
        if let Some(keys) = &keys {
            keys.sweep();
        }
        Some(
            div()
                .relative()
                .flex_none()
                .w_full()
                .children(panel.map(|panel| div().absolute().bottom_full().left_0().right_0().child(panel)))
                .child(band)
                .into_any_element(),
        )
    }

    fn render_panel(&self, id: &SharedString, things: &[Thing], review: &[&WaitingItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STEP_WAITING);
        let detail = self.dock_of(id).and_then(|dock| dock.detail.clone());
        let only = things.len() == 1 && review.is_empty() && matches!(things[0], Thing::Ask(_) | Thing::Lapsed(_));
        let detail = detail.or_else(|| only.then(|| Self::thing_key(&things[0])));
        let content: AnyElement = match detail.as_ref().and_then(|key| things.iter().find(|thing| &Self::thing_key(thing) == key)) {
            Some(thing) => self.render_detail(id, thing, only, look, cx),
            None => self.render_list(things, review, look, cx),
        };
        div()
            .id("needs-panel")
            .track_focus(&self.dock_focus)
            .occlude()
            .on_key_down(cx.listener(Self::on_panel_key))
            .max_h(look.length(token::SPACE_12) * 10.)
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

    fn render_list(&self, things: &[Thing], review: &[&WaitingItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        type Press = Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>;
        let row = |at: String, icon: &'static str, said: String, text: String, press: Press| {
            let name = text.clone();
            labelled(
                div()
                    .id(SharedString::from(format!("needs-row:{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(div().id(SharedString::from(format!("needs-mark:{at}"))).flex_none().child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED))).tooltip(tip(said)))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(text)))
                    .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))),
                Role::Button,
                format!("Open {name}"),
            )
            .into_any_element()
        };
        let mut groups: Vec<(&'static str, Vec<AnyElement>)> =
            vec![("Questions", vec![]), ("To review", vec![]), ("Walks", vec![]), ("Commands", vec![]), ("PRs", vec![])];
        for thing in things {
            let key = Self::thing_key(thing);
            match thing {
                Thing::Ask(ask) => {
                    let key2 = key.clone();
                    groups[0].1.push(row(key, "message-circle-question-mark", ask_label(ask), ask_text(ask), Box::new(move |this, window, cx| this.set_needs(true, Some(key2.clone()), window, cx))));
                }
                Thing::Lapsed(lapsed) => {
                    let key2 = key.clone();
                    let text = lapsed.questions.first().map(|q| q.question.clone()).unwrap_or_default();
                    groups[0].1.push(row(key, "message-circle-question-mark", "Unanswered".into(), text, Box::new(move |this, window, cx| this.set_needs(true, Some(key2.clone()), window, cx))));
                }
                Thing::Item(item) => {
                    let (icon, said) = mark_of(item);
                    let kind = item.act.as_ref().map(|act| act.kind);
                    let group = match kind {
                        Some(WaitingActKind::Walk) => 2,
                        Some(WaitingActKind::Run) => 3,
                        None if !item.steps.is_empty() => 3,
                        Some(WaitingActKind::ApprovePr) => 4,
                        _ => 0,
                    };
                    let key2 = key.clone();
                    let target = item.act.as_ref().map(|act| act.target.clone());
                    let opens_here = !item.options.is_empty();
                    groups[group].1.push(row(
                        key,
                        icon,
                        said.into(),
                        item.text.clone(),
                        Box::new(move |this, window, cx| {
                            if opens_here {
                                this.set_needs(true, Some(key2.clone()), window, cx);
                            } else if kind == Some(WaitingActKind::Run) {
                                if let Some(target) = &target {
                                    cx.write_to_clipboard(ClipboardItem::new_string(target.clone()));
                                }
                            } else if kind == Some(WaitingActKind::Message) {
                                this.set_needs(false, None, window, cx);
                                this.to_composer(window, cx);
                            } else {
                                this.set_needs(false, None, window, cx);
                            }
                        }),
                    ));
                }
            }
        }
        if !review.is_empty() {
            let said: Vec<String> = review.iter().map(|item| item.text.clone()).collect();
            groups[1].1.push(row(
                "review".into(),
                "list-checks",
                "Goes to the card".into(),
                said.join(", "),
                Box::new(|this, window, cx| {
                    this.set_needs(false, None, window, cx);
                    this.pin_shown();
                    cx.notify();
                }),
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children(groups.into_iter().filter(|(_, rows)| !rows.is_empty()).map(|(name, rows)| {
                labelled(
                    div()
                        .id(SharedString::from(format!("needs-group:{name}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .child(eyebrow(look, name).text_color(look.color(token::FG_MUTED)).px(look.length(token::SPACE_3)))
                        .children(rows),
                    Role::Region,
                    name,
                )
            }))
            .into_any_element()
    }

    /// The thread goes to its end.
    pub(super) fn pin_shown(&mut self) {
        if let Some(id) = self.shown.clone() {
            self.pin(&id);
        }
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
            Thing::Item(item) => {
                let questions = vec![ipc::AskedQuestion {
                    question: item.text.clone(),
                    header: String::new(),
                    multi_select: false,
                    options: item
                        .options
                        .iter()
                        .map(|option| ipc::AskedChoice { label: option.label.clone(), description: option.description.clone().unwrap_or_default(), preview: String::new() })
                        .collect(),
                }];
                self.deck(id, &item.id, &questions, false, true, super::deck::Source::Waiting(item.id.clone()), look, cx)
            }
        };
        let said = self.dock_of(id).and_then(|dock| dock.said.get(&Self::thing_key(thing)).cloned());
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(back)
            .child(div().w_full().min_w_0().child(body))
            .children(said.map(|why| super::acts::refusal(look, "dock", why)))
            .into_any_element()
    }
}
