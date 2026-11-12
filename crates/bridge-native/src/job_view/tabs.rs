//! Each pane tab's × and each strip's + (Add tab), as Electron's `PaneView` draws them. A tab keeps
//! its ×'s room whether it shows or not, so a tab is as wide either way; it shows on the tab in front
//! and on hover. A closed tab stays closed for that Job until + brings it back.
//!
//! **+ offers the tabs built here, each in its own pane, in Electron's order of kinds.** Electron's
//! menu lists every kind for every pane; the native Job draws its thread and Now in fixed places, so
//! their +s offer their own kind, and the top pane's offers the rest.

use std::collections::HashSet;

use gpui::{div, prelude::*, Anchor, Context, DismissEvent, Entity, Focusable, Pixels, Point, Role, SharedString, Stateful, Subscription, Window};

use super::head::{drawn, below_left, tip};
use super::side::Zone;
use super::JobView;
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, Look};

/// A Job's three panes, by what each holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Pane {
    Thread,
    Top,
    Now,
}

impl Pane {
    /// The tabs a pane holds and + offers: id, word and glyph.
    fn kinds(self) -> &'static [(&'static str, &'static str, &'static str)] {
        match self {
            Pane::Thread => &[("thread", "Thread", "message-square")],
            // In Electron's order of kinds (`JOB_TAB_KINDS`), the ones built here alone.
            Pane::Top => &[
                ("done", "Done", "circle-check"),
                ("open", "Open", "circle"),
                ("deferred", "Deferred", "circle-dashed"),
                ("plan", "Plan", "list-checks"),
                ("changes", "Changes", "diff"),
                ("pulls", "Pull Requests", "git-pull-request"),
                ("issues", "Issues", "ticket"),
                ("jobs", "Jobs", "link"),
                ("ledger", "Ledger", "files"),
                ("files", "Files", "folder-tree"),
                ("subagents", "Subagents", "zap"),
                ("checks", "Checks", "shield-check"),
                ("system", "System", "cpu"),
                ("bays", "Bays", "layout-grid"),
                ("spine", "Spine", "waypoints"),
                ("judge", "Judge", "scale"),
            ],
            Pane::Now => &[("now", "Now", "activity")],
        }
    }

    fn named(self) -> &'static str {
        match self {
            Pane::Thread => "tabs-add-thread",
            Pane::Top => "tabs-add-top",
            Pane::Now => "tabs-add-now",
        }
    }
}

pub(super) struct AddMenu {
    pub menu: Entity<Dropdown>,
    _subs: Vec<Subscription>,
}

/// The tabs closed on each Job, and those + opened that start shut, by Job id.
#[derive(Default)]
pub(super) struct Closed {
    shut: std::collections::HashMap<SharedString, HashSet<&'static str>>,
    opened: std::collections::HashMap<SharedString, HashSet<&'static str>>,
}

impl JobView {
    pub(super) fn tab_closed(&self, tab: &str) -> bool {
        if Zone::ALL.into_iter().any(|zone| zone.id() == tab && zone.opened_by_hand()) {
            return !self.shown.as_ref().and_then(|id| self.tabs_closed.opened.get(id)).is_some_and(|opened| opened.contains(tab));
        }
        self.shown.as_ref().and_then(|id| self.tabs_closed.shut.get(id)).is_some_and(|closed| closed.contains(tab))
    }

    /// Top's tab in front: the zone, or the first of its tabs still open.
    pub(super) fn zone_shown(&self) -> Option<Zone> {
        if !self.tab_closed(self.zone.id()) {
            return Some(self.zone);
        }
        Zone::ALL.into_iter().find(|zone| !self.tab_closed(zone.id()))
    }

    /// A closed thread comes back, as a walk arriving brings it (`reopenThread`).
    pub(super) fn reopen_thread(&mut self, id: &SharedString) {
        if let Some(closed) = self.tabs_closed.shut.get_mut(id) {
            closed.remove("thread");
        }
    }

    pub(super) fn close_tab(&mut self, tab: &'static str, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if let Some(opened) = self.tabs_closed.opened.get_mut(&id) {
            opened.remove(tab);
        }
        self.tabs_closed.shut.entry(id).or_default().insert(tab);
        if tab == "judge" {
            self.close_judge_tab();
        }
        if tab == "system" {
            self.panes.systems.stop();
        }
        if let Some(zone) = self.zone_shown() {
            self.zone = zone;
        }
        cx.notify();
    }

    pub(super) fn add_tab(&mut self, tab: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.shown.clone() {
            if let Some(closed) = self.tabs_closed.shut.get_mut(&id) {
                closed.remove(tab);
            }
            self.tabs_closed.opened.entry(id).or_default().insert(tab);
        }
        if let Some(zone) = Zone::ALL.into_iter().find(|zone| zone.id() == tab) {
            self.zone = zone;
            if let Some(at) = Zone::ALL.into_iter().filter(|one| !self.tab_closed(one.id())).position(|one| one == zone) {
                self.panes.top_tabs.scroll_to_item(at);
            }
        }
        if tab == "thread" {
            window.focus(&self.focus, cx);
        }
        self.want_pane_reads(window, cx);
        cx.notify();
    }

    fn open_add_menu(&mut self, pane: Pane, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let entries = pane.kinds().iter().map(|&(id, word, glyph)| Entry::Item { id, label: word.into(), icon: Some(glyph), danger: false, key: None, off: None }).collect();
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Add tab", entries, at, Anchor::TopLeft, false, cx).at_point(window, cx));
        let picked = cx.subscribe_in(&menu, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.add_tab(id, window, cx);
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.tabs_menu = None;
            if let Some(back) = back.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.tabs_menu = Some(AddMenu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }

    /// A strip's +, at its end.
    pub(super) fn add_button(&self, pane: Pane, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        labelled(
            frame(look, pane.named(), "plus").ml(look.length(token::SPACE_1)).tooltip(tip("Add tab".into())).on_click(cx.listener(move |this, _, window, cx| {
                let at = drawn(window, cx, Role::Button, &format!("Add tab: {}", pane.kinds()[0].1)).map_or_else(|| window.mouse_position(), below_left);
                this.open_add_menu(pane, at, window, cx);
            })),
            Role::Button,
            format!("Add tab: {}", pane.kinds()[0].1),
        )
    }

    /// A tab's ×. `group` is the tab's own hover group.
    pub(super) fn close_button(&self, tab: &'static str, label: &str, group: SharedString, front: bool, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        labelled(
            frame(look, SharedString::from(format!("tab-close:{tab}")), "x")
                .mr(look.length(token::SPACE_1))
                .when(!front, |close| close.opacity(0.).group_hover(group, |style| style.opacity(1.)))
                .tooltip(tip("Close".into()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.close_tab(tab, cx);
                })),
            Role::Button,
            format!("Close {label}"),
        )
    }
}

/// A small ghost icon button, `--h-control-sm` square, as Electron's `size="sm" iconOnly`.
pub(super) fn frame(look: &Look, id: impl Into<gpui::ElementId>, icon: &'static str) -> Stateful<gpui::Div> {
    let ink = look.color(token::FG_SUBTLE);
    let (ground, hover) = (look.color(token::BG_HOVER), look.color(token::FG_DEFAULT));
    div()
        .id(id)
        .flex_none()
        .self_center()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::H_CONTROL_SM))
        .rounded(look.length(token::RADIUS_MD))
        .text_color(ink)
        .cursor_pointer()
        .hover(move |style| style.bg(ground).text_color(hover))
        .child(gpui::svg().path(icon_path(icon)).size(look.length(token::SPACE_4)).text_color(ink))
}
