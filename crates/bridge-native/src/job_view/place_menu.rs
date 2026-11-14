//! The menu a right press on a tab opens, and the pane acts a menu and a key share. Every tab's menu
//! reads Zoom or Restore, Split right and Split down, and a Close; a Terminal tab's own acts stand
//! before them and its Close is the one that ends the shell. The thread tab's is its Job menu
//! (`head.rs`) with the same three entries after the Job's acts.

use gpui::{prelude::*, Context, DismissEvent, Focusable, Pixels, Point, SharedString, Window};

use super::pane_tree::{self as tree, SplitInto};
use super::view_tabs::ACT_IDS;
use super::JobView;
use crate::dropdown::{Dropdown, Entry, Picked};

/// A registry act's key as a menu draws it.
fn shortcut(act: &str) -> Option<SharedString> {
    crate::generated::actions::act(act).map(|row| row.shortcut.replace("Enter", "\u{21a9}").into())
}

/// Zoom or Restore, Split right and Split down, as Electron's `tabEntries` has them.
pub(super) fn pane_entries(zoomed: bool) -> Vec<Entry> {
    vec![
        Entry::Item {
            id: "zoom",
            label: if zoomed { "Restore" } else { "Zoom" }.into(),
            icon: Some(if zoomed { "minimize-2" } else { "maximize-2" }),
            danger: false,
            key: shortcut("pane_zoom"),
            off: None,
        },
        Entry::Item { id: "split-right", label: "Split right".into(), icon: None, danger: false, key: shortcut("pane_split_right"), off: None },
        Entry::Item { id: "split-down", label: "Split down".into(), icon: None, danger: false, key: shortcut("pane_split_down"), off: None },
    ]
}

impl JobView {
    /// The pane that holds the Job's thread tab.
    pub(super) fn thread_pane(&self) -> Option<String> {
        tree::holder_of(&self.arrangement(), "thread").map(|pane| pane.id.clone())
    }

    /// A menu entry or a key that acts on a pane: zoom it, or split it.
    pub(super) fn pane_act_on(&mut self, act: &str, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        match act {
            "zoom" => self.toggle_zoom(pane, cx),
            "split-right" => self.split(pane, SplitInto::Right, window, cx),
            "split-down" => self.split(pane, SplitInto::Down, window, cx),
            _ => {}
        }
    }

    /// A tab's menu at `at`, from a right press or the keys. The keys go back to where they were when it shuts.
    pub(super) fn open_tab_menu(&mut self, tab_id: &str, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(tab) = self.tab_of(tab_id) else { return };
        if tab.kind == "thread" {
            self.open_head_menu(at, from_keys, window, cx);
            return;
        }
        let Some(pane) = tree::pane_of_tab(&self.arrangement(), tab_id).map(|pane| pane.id.clone()) else { return };
        let acts = self.view_id_of(&job, &tab).and_then(|view| self.view_tabs.by_job.get(&job)?.iter().find(|one| one.id == view).map(|one| one.menu.clone())).unwrap_or_default();
        let own_close = !acts.is_empty();
        let zoomed = self.zoomed().as_deref() == Some(pane.as_str());
        let mut entries: Vec<Entry> = acts.iter().zip(ACT_IDS).map(|(act, id)| Entry::Item { id, label: act.label.clone(), icon: None, danger: act.danger, key: None, off: None }).collect();
        if own_close {
            entries.push(Entry::Rule);
        }
        entries.extend(pane_entries(zoomed));
        if !own_close {
            entries.push(Entry::Rule);
            entries.push(Entry::Item { id: "close", label: "Close".into(), icon: Some("x"), danger: false, key: None, off: None });
        }
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Tab", entries, at, gpui::Anchor::TopLeft, from_keys, cx).at_point(window, cx));
        let tab_id = tab_id.to_string();
        let restore = back.clone();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.tabs_menu = None;
            if let Some(act) = ACT_IDS.iter().position(|one| one == id).and_then(|at| acts.get(at)).map(|one| one.act.clone()) {
                window.defer(cx, move |window, cx| act(window, cx));
            } else if *id == "close" {
                this.close_tab_id(&tab_id, window, cx);
            } else {
                this.pane_act_on(id, &pane, window, cx);
            }
            if let Some(back) = restore.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
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
        self.tabs_menu = Some(super::tabs::AddMenu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }
}
