//! A menu over the window, as Electron's `DropdownMenu` draws it: a row's menu at the pointer, or
//! the Filter's under its button. ↑ and ↓ move between its entries, Enter picks, Esc and a press
//! outside it shut it.

use gpui::{
    anchored, deferred, div, prelude::*, svg, App, Context, Anchor, DismissEvent, EventEmitter,
    FocusHandle, Focusable, KeyDownEvent, Pixels, Point, Role, SharedString, Window, px,
};

use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// One line of a menu.
#[derive(Clone)]
pub enum Entry {
    /// An act: picking it shuts the menu.
    Act { id: &'static str, label: SharedString, icon: Option<&'static str>, danger: bool },
    /// A switch, ticked while on: picking it leaves the menu open, as Electron's View group does.
    Toggle { id: &'static str, label: SharedString, on: bool },
    /// One of several, ticked while chosen: picking it shuts the menu.
    Choice { id: &'static str, label: SharedString, on: bool },
    /// An act that may show its key at the right edge, or be off with the reason it is, as
    /// Electron's `DropdownMenu` items with a `shortcut` or a `hint` do.
    Item { id: &'static str, label: SharedString, icon: Option<&'static str>, danger: bool, key: Option<SharedString>, off: Option<SharedString> },
    Rule,
    Label(&'static str),
    /// A line that cannot be picked now, with the reason in its tooltip.
    Off { label: SharedString, hint: &'static str },
    /// A row with the parts the title bar's picker and menu give it: a glyph, a detail, a key, a gear.
    Row(Row),
    /// A filter field. What a person types narrows the entries after it, to the next rule when
    /// `to_rule`, else to the end, as Electron's `filterEntries` does.
    Search { label: &'static str, to_rule: bool },
}

/// One row of a menu that carries more than a label.
#[derive(Clone)]
pub struct Row {
    pub id: &'static str,
    pub label: SharedString,
    pub icon: Option<&'static str>,
    /// Quiet mono words after the label, such as `Not set up`.
    pub detail: Option<SharedString>,
    /// The act's key, drawn as a keycap.
    pub key: Option<SharedString>,
    /// `Some` makes it a choice, ticked while true.
    pub ticked: Option<bool>,
    /// Under a heading row: one glyph's width in.
    pub member: bool,
    /// Drawn in the accent colour, as the acts below the picker's rule are.
    pub accent: bool,
    /// A dot that says something wants the person, in the awaiting colour.
    pub attention: bool,
    /// The reason the row cannot be picked, said in its tooltip; `None` while it can.
    pub off: Option<SharedString>,
    /// A gear at the row's end that opens what the row names, saying `Open <name>`.
    pub gear: Option<SharedString>,
    /// Picking it flips its tick and leaves the menu open, as Electron's View group does.
    pub keep: bool,
    /// A glyph button at the row's end that opens a layer about the row (the picker's place glyph); → on the row does the same.
    pub trail: Option<Trail>,
}

/// The mark a row ends in, and what it says.
#[derive(Clone)]
pub struct Trail {
    pub icon: &'static str,
    /// The button's name.
    pub said: SharedString,
    /// Its tooltip.
    pub tip: SharedString,
}

impl Row {
    pub fn new(id: &'static str, label: impl Into<SharedString>) -> Row {
        Row { id, label: label.into(), icon: None, detail: None, key: None, ticked: None, member: false, accent: false, attention: false, off: None, gear: None, keep: false, trail: None }
    }

    pub fn icon(mut self, icon: &'static str) -> Row {
        self.icon = Some(icon);
        self
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Row {
        self.detail = Some(detail.into());
        self
    }

    pub fn key(mut self, key: impl Into<SharedString>) -> Row {
        let key: SharedString = key.into();
        self.key = (!key.is_empty()).then_some(key);
        self
    }

    pub fn ticked(mut self, on: bool) -> Row {
        self.ticked = Some(on);
        self
    }

    pub fn member(mut self) -> Row {
        self.member = true;
        self
    }

    pub fn accent(mut self) -> Row {
        self.accent = true;
        self
    }

    pub fn attention(mut self) -> Row {
        self.attention = true;
        self
    }

    /// Off while `off` holds, with `why` in its tooltip.
    pub fn off_when(self, off: bool, why: &'static str) -> Row {
        if off { self.off(why) } else { self }
    }

    /// Off: it cannot be picked, and `why` is said in its tooltip.
    pub fn off(mut self, why: impl Into<SharedString>) -> Row {
        self.off = Some(why.into());
        self
    }

    pub fn keeps_open(mut self) -> Row {
        self.keep = true;
        self
    }

    pub fn trail(mut self, icon: &'static str, said: impl Into<SharedString>, tip: impl Into<SharedString>) -> Row {
        self.trail = Some(Trail { icon, said: said.into(), tip: tip.into() });
        self
    }

    pub fn gear(mut self, said: impl Into<SharedString>) -> Row {
        self.gear = Some(said.into());
        self
    }
}

impl Entry {
    fn label(&self) -> Option<&str> {
        match self {
            Entry::Act { label, .. } | Entry::Toggle { label, .. } | Entry::Choice { label, .. } | Entry::Item { label, .. } | Entry::Off { label, .. } => Some(label),
            Entry::Row(row) => Some(&row.label),
            Entry::Rule | Entry::Label(_) | Entry::Search { .. } => None,
        }
    }

    /// Whether the keys can rest on it: an item that is off is read, not chosen.
    fn pickable(&self) -> bool {
        !matches!(self, Entry::Search { .. } | Entry::Rule | Entry::Label(_) | Entry::Off { .. } | Entry::Item { off: Some(_), .. } | Entry::Row(Row { off: Some(_), .. }))
    }
}

pub enum Picked {
    /// An entry, by its id; `keep` while the menu stays open.
    Entry { id: &'static str, keep: bool },
}

/// A gear or trail at a row's end was pressed, or → on the row: open what the row names, and leave the menu.
pub struct Gear(pub &'static str);

pub struct Dropdown {
    pub entries: Vec<Entry>,
    /// Where the menu's corner sits, in the window.
    at: Point<Pixels>,
    /// Which of its corners sits there: the top left for a row's menu, the top right under Filter.
    corner: Anchor,
    cursor: Option<usize>,
    name: SharedString,
    focus: FocusHandle,
    /// What has been typed into the menu's filter field, if it has one.
    query: String,
}

impl EventEmitter<Picked> for Dropdown {}
impl EventEmitter<Gear> for Dropdown {}
impl EventEmitter<DismissEvent> for Dropdown {}

impl Focusable for Dropdown {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Dropdown {
    /// A menu at `at` named `name`; `cursor_first` puts the keys on its first entry, as a menu
    /// opened from the keyboard does.
    pub fn new(name: impl Into<SharedString>, entries: Vec<Entry>, at: Point<Pixels>, corner: Anchor, cursor_first: bool, cx: &mut Context<Self>) -> Self {
        let mut this = Dropdown { entries, at, corner, cursor: None, name: name.into(), focus: cx.focus_handle(), query: String::new() };
        if cursor_first {
            this.cursor = this.first();
        }
        this
    }

    fn first(&self) -> Option<usize> {
        self.visible().0.into_iter().find(|&at| self.entries[at].pickable())
    }

    /// The entries drawn, by place, and whether the query left nothing in its reach. With no field or
    /// no query that is every entry. A heading with nothing under it goes, and a heading row that
    /// matches keeps the rows under it.
    fn visible(&self) -> (Vec<usize>, bool) {
        let len = self.entries.len();
        let all = || (0..len).collect::<Vec<_>>();
        let needle = self.query.trim().to_lowercase();
        let Some((field, to_rule)) = self.entries.iter().enumerate().find_map(|(at, entry)| match entry {
            Entry::Search { to_rule, .. } => Some((at, *to_rule)),
            _ => None,
        }) else {
            return (all(), false);
        };
        if needle.is_empty() {
            return (all(), false);
        }
        let end = if to_rule { (field + 1..len).find(|&at| matches!(self.entries[at], Entry::Rule)).unwrap_or(len) } else { len };
        let holds = |at: usize| self.entries[at].label().is_some_and(|label| label.to_lowercase().contains(&needle));
        let under = |at: usize| matches!(&self.entries[at], Entry::Row(row) if row.member);
        let mut kept: Vec<usize> = (0..=field).collect();
        let mut heading: Option<usize> = None;
        let mut at = field + 1;
        let mut any = false;
        while at < end {
            match &self.entries[at] {
                Entry::Label(_) => {
                    heading = Some(at);
                    at += 1;
                }
                Entry::Rule | Entry::Search { .. } => {
                    heading = None;
                    at += 1;
                }
                _ => {
                    let mut next = at + 1;
                    while next < end && under(next) {
                        next += 1;
                    }
                    let own = holds(at);
                    let members: Vec<usize> = (at + 1..next).filter(|&member| own || holds(member)).collect();
                    if own || !members.is_empty() {
                        kept.extend(heading.take());
                        kept.push(at);
                        kept.extend(members);
                        any = true;
                    }
                    at = next;
                }
            }
        }
        kept.extend(end..len);
        (kept, !any)
    }

    fn searching(&self) -> bool {
        self.entries.iter().any(|entry| matches!(entry, Entry::Search { .. }))
    }

    fn step(&mut self, by: isize) {
        let ids: Vec<usize> = self.visible().0.into_iter().filter(|&at| self.entries[at].pickable()).collect();
        if ids.is_empty() {
            return;
        }
        let now = self.cursor.and_then(|at| ids.iter().position(|&id| id == at));
        let next = match now {
            None if by > 0 => 0,
            None => ids.len() - 1,
            Some(at) => (at as isize + by).clamp(0, ids.len() as isize - 1) as usize,
        };
        self.cursor = Some(ids[next]);
    }

    fn pick(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.get_mut(at) else { return };
        match entry {
            Entry::Act { id, .. } | Entry::Choice { id, .. } => {
                let id = *id;
                cx.emit(Picked::Entry { id, keep: false });
                cx.emit(DismissEvent);
            }
            Entry::Toggle { id, on, .. } => {
                *on = !*on;
                let id = *id;
                cx.emit(Picked::Entry { id, keep: true });
                cx.notify();
            }
            Entry::Item { id, off: None, .. } => {
                let id = *id;
                cx.emit(Picked::Entry { id, keep: false });
                cx.emit(DismissEvent);
            }
            Entry::Row(row) => {
                if row.off.is_some() {
                    return;
                }
                let id = row.id;
                if row.keep {
                    row.ticked = row.ticked.map(|on| !on);
                    cx.emit(Picked::Entry { id, keep: true });
                    cx.notify();
                    return;
                }
                cx.emit(Picked::Entry { id, keep: false });
                cx.emit(DismissEvent);
            }
            Entry::Item { off: Some(_), .. } | Entry::Rule | Entry::Label(_) | Entry::Off { .. } | Entry::Search { .. } => {}
        }
    }

    /// The text a key adds to the filter, if it is one that types.
    fn typed(event: &KeyDownEvent) -> Option<String> {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.control || keystroke.modifiers.alt {
            return None;
        }
        if keystroke.key == "space" {
            return Some(" ".into());
        }
        let text = keystroke.key_char.clone().filter(|text| !text.chars().any(char::is_control)).or_else(|| (keystroke.key.chars().count() == 1).then(|| keystroke.key.clone()))?;
        (keystroke.key.chars().count() == 1).then_some(text)
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.searching() {
            let typed = if event.keystroke.key == "backspace" { self.query.pop().map(|_| ()) } else { Self::typed(event).map(|text| self.query.push_str(&text)) };
            if typed.is_some() || event.keystroke.key == "backspace" {
                self.cursor = None;
                cx.stop_propagation();
                cx.notify();
                return;
            }
        }
        match event.keystroke.key.as_str() {
            "down" => {
                self.step(1);
                window.focus(&self.focus, cx);
            }
            "up" => self.step(-1),
            "enter" => match self.cursor.or_else(|| if self.searching() { self.first() } else { None }) {
                Some(at) => self.pick(at, cx),
                None => return,
            },
            "escape" => cx.emit(DismissEvent),
            "right" => match self.cursor.and_then(|at| match self.entries.get(at) {
                Some(Entry::Row(row)) if row.gear.is_some() || row.trail.is_some() => Some(row.id),
                _ => None,
            }) {
                Some(id) => {
                    cx.emit(Gear(id));
                    cx.emit(DismissEvent);
                }
                None => return,
            },
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_entry(&self, at: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let has_icons = self.entries.iter().any(|entry| matches!(entry, Entry::Act { icon: Some(_), .. } | Entry::Item { icon: Some(_), .. } | Entry::Row(Row { icon: Some(_), .. })));
        let has_checks = self.entries.iter().any(|entry| matches!(entry, Entry::Toggle { .. } | Entry::Choice { .. } | Entry::Row(Row { ticked: Some(_), .. })));
        let glyph = look.length(token::SPACE_4);
        match &self.entries[at] {
            Entry::Rule => div()
                .h(look.length(token::BORDER_WIDTH))
                .mx(look.length(token::SPACE_2))
                .my(look.length(token::SPACE_1))
                .bg(look.color(token::BORDER_SUBTLE))
                .into_any_element(),
            Entry::Label(text) => div()
                .h(look.length(token::H_MENU_ITEM))
                .flex()
                .items_center()
                .px(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(*text)
                .into_any_element(),
            Entry::Off { label, hint } => div()
                .id(SharedString::from(format!("off:{label}")))
                .h(look.length(token::H_MENU_ITEM))
                .flex()
                .items_center()
                .px(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_SUBTLE))
                .opacity(0.5)
                .tooltip({
                    let hint = *hint;
                    move |window, cx| gpui_component::tooltip::Tooltip::new(hint).build(window, cx)
                })
                .child(label.clone())
                .into_any_element(),
            Entry::Search { label, .. } => {
                let ink = look.color(token::FG_DEFAULT);
                let query = self.query.clone();
                let typed = !query.is_empty();
                labelled(
                    div()
                        .id("menu-search")
                        .flex_none()
                        .h(look.length(token::H_CONTROL))
                        .mx(look.length(token::SPACE_1))
                        .mb(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_3))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::ACCENT))
                        .bg(look.color(token::BG_SUNKEN))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(ink)
                        .child(svg().path(icon_path("search")).size(glyph).flex_none().text_color(look.color(token::FG_SUBTLE)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .child(div().truncate().child(query))
                                // Where the next letter goes: a bar of the hairline's width in the accent.
                                .child(div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_4)).bg(look.color(token::ACCENT))),
                        )
                        .when(typed, |field| {
                            field.child(labelled(
                                div()
                                    .id("clear-search")
                                    .flex_none()
                                    .size(glyph)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Clear search").build(window, cx))
                                    .child(svg().path(icon_path("x")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.query.clear();
                                        this.cursor = None;
                                        cx.notify();
                                    })),
                                Role::Button,
                                "Clear search",
                            ))
                        }),
                    Role::SearchInput,
                    *label,
                )
                .into_any_element()
            }
            Entry::Row(row) => {
                let ink = if row.off.is_some() {
                    look.color(token::FG_SUBTLE)
                } else if row.accent {
                    look.color(token::ACCENT)
                } else if row.ticked == Some(true) {
                    look.color(token::FG_DEFAULT)
                } else {
                    look.color(token::FG_MUTED)
                };
                let role = if row.ticked.is_some() { Role::MenuItemCheckBox } else { Role::MenuItem };
                let (id, label) = (row.id, row.label.clone());
                let live = row.off.is_none();
                let gear = row.gear.clone().map(|said| {
                    let tip = said.clone();
                    labelled(
                        div()
                            .id(SharedString::from(format!("gear-{id}")))
                            .flex_none()
                            .size(glyph)
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
                            .child(svg().path(icon_path("settings")).size(glyph).text_color(look.color(token::FG_MUTED)))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(Gear(id));
                                cx.emit(DismissEvent);
                            })),
                        Role::Button,
                        said,
                    )
                });
                let trail = row.trail.clone().map(|trail| {
                    let tip = trail.tip.clone();
                    labelled(
                        div()
                            .id(SharedString::from(format!("trail-{id}")))
                            .flex_none()
                            .size(look.length(token::SPACE_6))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(look.length(token::RADIUS_MD))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
                            .child(svg().path(icon_path(trail.icon)).size(look.length(token::SPACE_3)).text_color(look.color(token::STATUS_COMPLETED_SUCCESS)))
                            .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                                cx.stop_propagation();
                                window.prevent_default();
                            })
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(Gear(id));
                                cx.emit(DismissEvent);
                            })),
                        Role::Button,
                        trail.said,
                    )
                });
                labelled(
                    div()
                        .id(id)
                        .h(look.length(token::H_MENU_ITEM))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_2))
                        .when(row.member, |line| line.pl(look.length(token::SPACE_2) + glyph + look.length(token::SPACE_3)))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(ink)
                        .when(live && self.cursor == Some(at), |line| line.bg(look.color(token::BG_HOVER)))
                        .when(live, |line| line.hover(|style| style.bg(look.color(token::BG_HOVER))).cursor_pointer())
                        .when(has_icons, |line| {
                            line.child(div().flex_none().size(glyph).children(row.icon.map(|icon| svg().path(icon_path(icon)).size(glyph).text_color(ink))))
                        })
                        .child(div().flex_1().min_w_0().truncate().child(label.clone()))
                        .children(row.detail.clone().map(|detail| {
                            div()
                                .flex_none()
                                .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                                .text_size(look.length(token::TEXT_2XS))
                                .text_color(look.color(token::FG_SUBTLE))
                                .child(detail)
                        }))
                        .when(row.attention, |line| {
                            line.child(div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::STATUS_AWAITING_REVIEW)))
                        })
                        .children(row.key.clone().map(|key| {
                            div()
                                .flex_none()
                                .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                                .text_size(look.length(token::TEXT_2XS))
                                .text_color(look.color(token::FG_SUBTLE))
                                .child(key)
                        }))
                        .children(trail)
                        .children(gear)
                        .when(has_checks, |line| {
                            line.child(div().flex_none().size(glyph).when(row.ticked == Some(true), |mark| {
                                mark.child(svg().path(icon_path("check")).size(glyph).text_color(look.color(token::ACCENT)))
                            }))
                        })
                        .when_some(row.off.clone(), |line, why| line.tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(why.clone()).build(window, cx)))
                        .when(live, |line| line.on_click(cx.listener(move |this, _, _, cx| this.pick(at, cx)))),
                    role,
                    label,
                )
                .into_any_element()
            }
            entry => {
                let (id, label, icon, danger, ticked, key, off) = match entry {
                    Entry::Act { id, label, icon, danger } => (*id, label.clone(), *icon, *danger, None, None, None),
                    Entry::Item { id, label, icon, danger, key, off } => (*id, label.clone(), *icon, *danger, None, key.clone(), off.clone()),
                    Entry::Toggle { id, label, on } | Entry::Choice { id, label, on } => (*id, label.clone(), None, false, Some(*on), None, None),
                    Entry::Rule | Entry::Label(_) | Entry::Off { .. } | Entry::Row(_) | Entry::Search { .. } => unreachable!("matched above"),
                };
                let ink = if off.is_some() {
                    look.color(token::FG_SUBTLE)
                } else if danger {
                    look.color(token::STATUS_COMPLETED_FAILED)
                } else if ticked == Some(true) {
                    look.color(token::FG_DEFAULT)
                } else {
                    look.color(token::FG_MUTED)
                };
                let role = if ticked.is_some() { Role::MenuItemCheckBox } else { Role::MenuItem };
                labelled(
                    div()
                        .id(id)
                        .h(look.length(token::H_MENU_ITEM))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(ink)
                        .when(self.cursor == Some(at), |row| row.bg(look.color(token::BG_HOVER)))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .cursor_pointer()
                        .when(has_icons, |row| {
                            row.child(div().flex_none().size(glyph).children(icon.map(|icon| svg().path(icon_path(icon)).size(glyph).text_color(ink))))
                        })
                        .child(div().flex_1().min_w_0().truncate().child(label.clone()))
                        .children(key.map(|key| {
                            div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(key)
                        }))
                        .when(has_checks, |row| {
                            row.child(div().flex_none().size(glyph).when(ticked == Some(true), |mark| {
                                mark.child(svg().path(icon_path("check")).size(glyph).text_color(look.color(token::ACCENT)))
                            }))
                        })
                        .when_some(off, |row, why| {
                            row.tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(why.clone()).build(window, cx))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| this.pick(at, cx))),
                    role,
                    label,
                )
                .into_any_element()
            }
        }
    }
}

impl Render for Dropdown {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (shown, nothing) = self.visible();
        let mut entries: Vec<gpui::AnyElement> = Vec::new();
        for at in shown {
            entries.push(self.render_entry(at, cx));
            if nothing && matches!(self.entries[at], Entry::Search { .. }) {
                let look = cx.look();
                let said = SharedString::from(format!("Nothing matches “{}”.", self.query.trim()));
                entries.push(
                    labelled(
                        div()
                            .id("menu-empty")
                            .px(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_SM))
                            .text_color(look.color(token::FG_MUTED))
                            .child(said.clone()),
                        Role::Status,
                        said,
                    )
                    .into_any_element(),
                );
            }
        }
        let look = cx.look();
        let detailed = self.entries.iter().any(|entry| matches!(entry, Entry::Row(Row { detail: Some(_), .. })));
        deferred(
            anchored()
                .position(self.at)
                .anchor(self.corner)
                .snap_to_window_with_margin(look.length(token::SPACE_1))
                .child(labelled(
                    div()
                        .id("menu")
                        .track_focus(&self.focus)
                        .capture_key_down(cx.listener(Self::on_key))
                        .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
                        .occlude()
                        // Room for a detail after a name; and a menu with a filter field is a long one.
                        .w(look.length(token::W_MENU) + if detailed { look.length(token::SPACE_12) } else { px(0.) })
                        .max_h(look.length(token::PALETTE_MAX_HEIGHT) + if self.searching() { look.length(token::SPACE_12) } else { px(0.) })
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .p(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_OVERLAY))
                        .shadow(look.shadows(token::SHADOW_OVERLAY))
                        .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                        .children(entries),
                    Role::Menu,
                    self.name.clone(),
                )),
        )
        .with_priority(1)
    }
}

/// Where a menu opened from the keyboard sits under a row: its middle, at its bottom edge.
pub fn under(bounds: gpui::Bounds<Pixels>) -> Point<Pixels> {
    Point { x: bounds.origin.x + bounds.size.width / 2., y: bounds.origin.y + bounds.size.height }
}
