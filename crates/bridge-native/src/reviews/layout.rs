//! Named layouts of a pull request's panes, and the title bar's Layout menu that picks, saves,
//! renames, deletes and resets them. One picked layout applies to every pull request. They are kept
//! in memory until Fleet keeps named layouts.

use gpui::{div, prelude::*, Context, DismissEvent, Entity, FontWeight, Role, SharedString, Subscription, WeakEntity, Window};

use super::ReviewsScreen;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::job_view::pane_tree::Arrangement;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::titlebar::parts::{glyph, no_drag, tip};

/// Each entry id is a static string, so the menu's layouts draw from this table.
const IDS: [&str; 12] = ["layout-1", "layout-2", "layout-3", "layout-4", "layout-5", "layout-6", "layout-7", "layout-8", "layout-9", "layout-10", "layout-11", "layout-12"];

/// The layouts he has, the shipped one first, and which is picked.
pub struct Layouts {
    all: Vec<(String, Arrangement)>,
    picked: usize,
}

impl Layouts {
    pub fn new(default: Arrangement) -> Layouts {
        Layouts { all: vec![("Default".into(), default)], picked: 0 }
    }

    pub fn name(&self) -> &str {
        &self.all[self.picked].0
    }

    /// The picked layout as saved.
    pub fn arrangement(&self) -> Arrangement {
        self.all[self.picked].1.clone()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.all.iter().map(|(name, _)| name.as_str())
    }

    /// The shipped layout is never renamed or deleted, so Reset always has it to go back to.
    pub fn shipped(&self) -> bool {
        self.picked == 0
    }

    pub fn full(&self) -> bool {
        self.all.len() >= IDS.len()
    }

    pub fn pick(&mut self, at: usize) -> Option<Arrangement> {
        let (_, arrangement) = self.all.get(at)?;
        self.picked = at;
        Some(arrangement.clone())
    }

    /// Save as: a name already taken is saved over.
    pub fn save_as(&mut self, name: &str, arrangement: Arrangement) {
        if let Some(at) = self.all.iter().position(|(one, _)| one == name) {
            if at > 0 {
                self.all[at].1 = arrangement;
                self.picked = at;
            }
            return;
        }
        if self.full() {
            return;
        }
        self.all.push((name.to_string(), arrangement));
        self.picked = self.all.len() - 1;
    }

    pub fn rename(&mut self, name: &str) {
        if !self.shipped() && !self.all.iter().any(|(one, _)| one == name) {
            self.all[self.picked].0 = name.to_string();
        }
    }

    pub fn delete(&mut self) {
        if !self.shipped() {
            self.all.remove(self.picked);
            self.picked = 0;
        }
    }
}

/// The title bar's Layout control: the picked layout's name, and its menu.
pub struct LayoutControl {
    screen: WeakEntity<ReviewsScreen>,
    menu: Option<(Entity<Dropdown>, [Subscription; 2])>,
    _seen: Subscription,
}

impl LayoutControl {
    pub fn new(screen: &Entity<ReviewsScreen>, cx: &mut Context<Self>) -> Self {
        let seen = cx.observe(screen, |_, _, cx| cx.notify());
        LayoutControl { screen: screen.downgrade(), menu: None, _seen: seen }
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(screen) = self.screen.upgrade() else { return };
        let layouts = &screen.read(cx).layouts;
        let name = layouts.name().to_string();
        let mut entries: Vec<Entry> = layouts.names().zip(IDS).enumerate().map(|(at, (one, id))| Entry::Choice { id, label: one.to_string().into(), on: at == layouts.picked }).collect();
        entries.push(Entry::Rule);
        let off = |why: &str| layouts.shipped().then(|| SharedString::from(why.to_string()));
        entries.push(Entry::Item { id: "save-as", label: "Save as".into(), icon: Some("plus"), danger: false, key: None, off: layouts.full().then(|| "Twelve layouts at most".into()) });
        entries.push(Entry::Item { id: "rename", label: "Rename".into(), icon: Some("pencil"), danger: false, key: None, off: off("Default keeps its name") });
        entries.push(Entry::Item { id: "delete", label: "Delete".into(), icon: Some("x"), danger: true, key: None, off: off("Default is always there") });
        entries.push(Entry::Rule);
        entries.push(Entry::Act { id: "reset", label: "Reset".into(), icon: Some("refresh-cw"), danger: false });
        let at = super::under_label(Role::Button, &format!("Layout: {name}"), gpui::Point::default(), window, cx);
        let menu = cx.new(|cx| Dropdown::new("Layout", entries, at, gpui::Anchor::TopLeft, false, cx));
        let screen = self.screen.clone();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = *picked;
            this.menu = None;
            let Some(screen) = screen.upgrade() else { return };
            screen.update(cx, |screen, cx| match id {
                "save-as" => screen.ask(super::Ask::SaveAs, window, cx),
                "rename" => screen.ask(super::Ask::Rename, window, cx),
                "delete" => screen.delete_layout(cx),
                "reset" => screen.reset_layout(cx),
                other => {
                    if let Some(at) = IDS.iter().position(|one| *one == other) {
                        screen.pick_layout(at, cx);
                    }
                }
            });
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, _, cx| {
            this.menu = None;
            cx.notify();
        });
        let keys = gpui::Focusable::focus_handle(menu.read(cx), cx);
        window.focus(&keys, cx);
        self.menu = Some((menu, [picked, dismissed]));
        cx.notify();
    }
}

impl Render for LayoutControl {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let name = self.screen.upgrade().map(|screen| screen.read(cx).layouts.name().to_string()).unwrap_or_else(|| "Default".into());
        let said = format!("Layout: {name}");
        let hover = look.color(token::BG_HOVER);
        let ink = look.color(token::FG_MUTED);
        let button = labelled(
            no_drag(
                div()
                    .id("reviews-layout")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .hover(move |style| style.bg(hover)),
            )
            .tooltip(tip("Layout of every pull request's panes"))
            .on_click(cx.listener(|this, _, window, cx| this.open(window, cx)))
            .child(glyph("layout-dashboard", look.length(token::SPACE_4), ink))
            .child(div().text_color(ink).child("Layout:"))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(name)))
            .child(glyph("chevron-down", look.length(token::SPACE_3), ink)),
            Role::Button,
            said,
        );
        div().flex_none().child(button).children(self.menu.as_ref().map(|(menu, _)| menu.clone()))
    }
}
