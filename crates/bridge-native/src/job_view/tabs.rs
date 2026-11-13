//! Each pane's + (Add tab), and the small ghost button a strip's × and + are made of, as Electron's
//! `PaneView` draws them.
//!
//! **+ offers one list in every pane**, Electron's order of kinds, the ones built here: the thread
//! where none is open, Done, Now, Open and Deferred, Plan and the rest the right column's top pane has
//! held, then the kinds other parts of the app put there (`view_tabs.rs`), the Terminal first. The tab
//! opens in the pane whose + was pressed; where that pane already holds a tab of the kind, it comes to
//! the front instead of a second opening.

use gpui::{div, prelude::*, Anchor, Context, DismissEvent, Entity, Focusable, Pixels, Point, Role, SharedString, Stateful, Subscription, Window};

use super::head::{below_left, drawn, tip};
use super::pane_tree::{self as tree, Extras};
use super::JobView;
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, Look};

pub(super) struct AddMenu {
    pub menu: Entity<Dropdown>,
    pub _subs: Vec<Subscription>,
}

/// The kinds + offers: id, word and glyph, in Electron's order of kinds (`JOB_TAB_KINDS`).
const KINDS: [(&str, &str, &str); 19] = [
    ("thread", "Thread", "message-square"),
    ("done", "Done", "circle-check"),
    ("now", "Now", "activity"),
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
    ("workflow", "Workflow", "workflow"),
    ("system", "System", "cpu"),
    ("bays", "Bays", "layout-grid"),
    ("spine", "Spine", "waypoints"),
    ("judge", "Judge", "scale"),
];

/// Done or Open switched off in Settings › Layout: neither drawn nor offered by +. Open is the registry's `next`.
pub(super) fn laid_off(tab: &str) -> bool {
    let panel = match tab {
        "done" => "done",
        "open" => "next",
        _ => return false,
    };
    crate::settings::layout::hidden("job.panels", panel)
}

impl JobView {
    /// A kind of tab a Job has, opened by an act of the Job's own (Plan from Now, Judge from the plan): in front where it
    /// is open already, else in the pane that keeps such tabs.
    pub(super) fn add_tab(&mut self, tab: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_kind(tab, cx);
        if tab == "thread" {
            self.front_thread();
            window.focus(&self.focus, cx);
        }
        self.want_pane_reads(window, cx);
        cx.notify();
    }

    /// + in a pane picked a built kind: a tab of it in that pane, in front, or the one the pane has brought forward.
    fn add_to_pane(&mut self, pane: &str, kind: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let held = tree::pane_by_id(&self.arrangement(), pane).and_then(|one| one.tabs.iter().find(|tab| tab.kind == kind)).map(|tab| tab.id.clone());
        match held {
            Some(tab) => self.activate_tab(&tab, window, cx),
            None if kind == "thread" => {
                self.rearrange(None, true, tree::reopen_thread, cx);
                self.front_thread();
                window.focus(&self.focus, cx);
            }
            None => {
                let pane = pane.to_string();
                self.rearrange(None, false, move |arrangement| tree::add_tab(arrangement, &pane, kind, Extras::default()), cx);
            }
        }
        self.want_pane_reads(window, cx);
    }

    /// The menu + opens in a pane, at `at`.
    pub(super) fn open_add_menu(&mut self, pane: &str, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        // Every pane's + offers the kinds other parts of the app put there after its own.
        let offers = cx.try_global::<super::view_tabs::TabOffers>().map(|offers| offers.0.clone()).unwrap_or_default();
        let arrangement = self.arrangement().into_owned();
        let into = super::place::view_pane_of(&arrangement, pane);
        let entries: Vec<Entry> = KINDS
            .iter()
            .filter(|&&(id, _, _)| !laid_off(id) && !(matches!(id, "thread" | "now") && tree::has_kind(&arrangement, id)))
            .map(|&(id, word, glyph)| Entry::Item { id, label: word.into(), icon: Some(glyph), danger: false, key: None, off: None })
            .chain(offers.iter().map(|offer| Entry::Item { id: offer.id, label: offer.label.clone(), icon: Some(offer.glyph), danger: false, key: None, off: None }))
            .collect();
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Add tab", entries, at, Anchor::TopLeft, false, cx).at_point(window, cx));
        let target = pane.to_string();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.tabs_menu = None;
            if let Some(offer) = offers.iter().find(|offer| offer.id == *id) {
                let (open, job) = (offer.open.clone(), this.shown.as_ref().map(|job| job.to_string()));
                if let Some(job) = job {
                    // The tab the source opens, whenever it does, goes to this pane.
                    this.aim_at(&job, &target);
                    window.defer(cx, move |window, cx| open(job, into, window, cx));
                }
                return;
            }
            if let Some(kind) = KINDS.iter().map(|one| one.0).find(|kind| kind == id) {
                this.add_to_pane(&target, kind, window, cx);
            }
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

    /// A pane's +, at its strip's end. It reads as it always has where the default arrangement gave the pane a name.
    pub(super) fn add_button(&mut self, pane: &str, look: &Look, cx: &mut Context<Self>) -> impl IntoElement {
        let name = match super::place::slot_of(pane) {
            super::place::Slot::Thread => "Add tab: Thread",
            super::place::Slot::Top => "Add tab: Done",
            super::place::Slot::Now => "Add tab: Now",
            super::place::Slot::Other => "Add tab",
        };
        let target = pane.to_string();
        let handle = self.shown.clone().map(|job| self.add_handle(&job, pane, cx));
        labelled(
            frame(look, SharedString::from(format!("tabs-add:{pane}")), "plus")
                .ml(look.length(token::SPACE_1))
                .when_some(handle, |button, handle| button.track_focus(&handle).tab_stop(true))
                .tooltip(tip("Add tab".into()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    let at = drawn(window, cx, Role::Button, name).map_or_else(|| window.mouse_position(), below_left);
                    this.open_add_menu(&target, at, window, cx);
                })),
            Role::Button,
            name,
        )
    }

    /// A tab's ×. `group` is the tab's own hover group.
    pub(super) fn close_button(&self, tab: &str, label: &str, group: SharedString, front: bool, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let tab_id = tab.to_string();
        labelled(
            frame(look, SharedString::from(format!("tab-close:{tab}")), "x")
                .mr(look.length(token::SPACE_1))
                .when(!front, |close| close.opacity(0.).group_hover(group, |style| style.opacity(1.)))
                .tooltip(tip("Close".into()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.close_tab_id(&tab_id, window, cx);
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
