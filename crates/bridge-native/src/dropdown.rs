//! A menu over the window, as Electron's `DropdownMenu` draws it: a row's menu at the pointer, or
//! the Filter's under its button. ↑ and ↓ move between its entries, Enter picks, Esc and a press
//! outside it shut it.

use gpui::{
    anchored, deferred, div, prelude::*, svg, App, Context, Anchor, DismissEvent, EventEmitter,
    FocusHandle, Focusable, KeyDownEvent, Pixels, Point, Role, SharedString, Window,
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
    Rule,
    Label(&'static str),
}

impl Entry {
    fn id(&self) -> Option<&'static str> {
        match self {
            Entry::Act { id, .. } | Entry::Toggle { id, .. } | Entry::Choice { id, .. } => Some(id),
            Entry::Rule | Entry::Label(_) => None,
        }
    }
}

pub enum Picked {
    /// An entry, by its id; `keep` while the menu stays open.
    Entry { id: &'static str, keep: bool },
}

pub struct Dropdown {
    pub entries: Vec<Entry>,
    /// Where the menu's corner sits, in the window.
    at: Point<Pixels>,
    /// Which of its corners sits there: the top left for a row's menu, the top right under Filter.
    corner: Anchor,
    cursor: Option<usize>,
    name: SharedString,
    focus: FocusHandle,
}

impl EventEmitter<Picked> for Dropdown {}
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
        let mut this = Dropdown { entries, at, corner, cursor: None, name: name.into(), focus: cx.focus_handle() };
        if cursor_first {
            this.cursor = this.first();
        }
        this
    }

    fn first(&self) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id().is_some())
    }

    fn step(&mut self, by: isize) {
        let ids: Vec<usize> = (0..self.entries.len()).filter(|&at| self.entries[at].id().is_some()).collect();
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
            Entry::Rule | Entry::Label(_) => {}
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "down" => {
                self.step(1);
                window.focus(&self.focus, cx);
            }
            "up" => self.step(-1),
            "enter" => match self.cursor {
                Some(at) => self.pick(at, cx),
                None => return,
            },
            "escape" => cx.emit(DismissEvent),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_entry(&self, at: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let has_icons = self.entries.iter().any(|entry| matches!(entry, Entry::Act { icon: Some(_), .. }));
        let has_checks = self.entries.iter().any(|entry| matches!(entry, Entry::Toggle { .. } | Entry::Choice { .. }));
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
            entry => {
                let (id, label, icon, danger, ticked) = match entry {
                    Entry::Act { id, label, icon, danger } => (*id, label.clone(), *icon, *danger, None),
                    Entry::Toggle { id, label, on } | Entry::Choice { id, label, on } => (*id, label.clone(), None, false, Some(*on)),
                    Entry::Rule | Entry::Label(_) => unreachable!("matched above"),
                };
                let ink = if danger {
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
                        .when(has_checks, |row| {
                            row.child(div().flex_none().size(glyph).when(ticked == Some(true), |mark| {
                                mark.child(svg().path(icon_path("check")).size(glyph).text_color(look.color(token::ACCENT)))
                            }))
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
        let entries: Vec<_> = (0..self.entries.len()).map(|at| self.render_entry(at, cx)).collect();
        let look = cx.look();
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
                        .w(look.length(token::W_MENU))
                        .max_h(look.length(token::PALETTE_MAX_HEIGHT))
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
