//! The menu a right press on a tab opens, and the pane acts a menu and a key share. Every tab's menu
//! reads Zoom or Restore, Split right and Split down, then Move to with each other pane and Next pane,
//! and a Close; a Terminal tab's own acts stand before them and its Close is the one that ends the
//! shell. The thread tab's is its Job menu (`head.rs`) with the same entries after the Job's acts.
//! Classic has no Move to; native has it so a tab moves without a drag (14 Nov).

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

/// Move to's entries: a pane each, by its place in the tree, then the next pane.
const MOVE_IDS: [&str; 12] = ["move:0", "move:1", "move:2", "move:3", "move:4", "move:5", "move:6", "move:7", "move:8", "move:9", "move:10", "move:11"];
const MOVE_NEXT: &str = "move:next";

/// Whether a menu entry is one of Move to's.
pub(super) fn is_move(id: &str) -> bool {
    id.starts_with("move:")
}

impl JobView {
    /// The name a pane reads by in Move to: the word of its tab in front, or Empty.
    fn pane_word(&self, job: &SharedString, pane: &tree::Pane) -> String {
        super::panes_place::front_of(pane).map_or_else(|| "Empty".to_string(), |tab| self.tab_word(job, tab).to_string())
    }

    /// Move to, under a label: every pane but the one the tab is in, by the word of its tab in front, then Next pane with its
    /// key (take A of the sketch he approved on 14 Nov). Nothing where the tab's pane is the only one.
    pub(super) fn move_entries(&self, tab_id: &str) -> Vec<Entry> {
        let Some(job) = self.shown.clone() else { return Vec::new() };
        let arrangement = self.arrangement();
        let Some(from) = tree::pane_of_tab(&arrangement, tab_id).map(|pane| pane.id.clone()) else { return Vec::new() };
        let panes = tree::panes_of(&arrangement.root);
        if panes.len() < 2 {
            return Vec::new();
        }
        let mut entries = vec![Entry::Label("Move to")];
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (pane, id) in panes.iter().zip(MOVE_IDS) {
            let word = self.pane_word(&job, pane);
            let count = seen.entry(word.clone()).or_default();
            *count += 1;
            if pane.id == from {
                continue;
            }
            let label = if *count > 1 { format!("{word} pane {count}") } else { format!("{word} pane") };
            entries.push(Entry::Item { id, label: label.into(), icon: None, danger: false, key: None, off: None });
        }
        entries.push(Entry::Item { id: MOVE_NEXT, label: "Next pane".into(), icon: None, danger: false, key: shortcut("move_tab_next_pane"), off: None });
        entries
    }

    /// A Move to entry picked for a tab: the tab goes to the end of that pane, in front, and the keys go with it.
    pub(super) fn move_tab_picked(&mut self, act: &str, tab_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let arrangement = self.arrangement().into_owned();
        let to = if act == MOVE_NEXT {
            tree::pane_of_tab(&arrangement, tab_id).and_then(|pane| super::panes_place::next_pane_of(&arrangement, &pane.id))
        } else {
            MOVE_IDS.iter().position(|id| *id == act).and_then(|at| tree::panes_of(&arrangement.root).get(at).map(|pane| pane.id.clone()))
        };
        let Some(to) = to else { return };
        let tab = tab_id.to_string();
        if self.rearrange(None, false, move |arrangement| tree::move_tab_to_pane(arrangement, &tab, &to, None), cx) {
            self.want_pane_reads(window, cx);
            let focus = self.arrangement().focus.clone();
            self.focus_in_pane(&focus, window, cx);
        }
    }

    /// The Job's thread tab, where a pane holds it.
    pub(super) fn thread_tab_id(&self) -> Option<String> {
        tree::holder_of(&self.arrangement(), "thread").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "thread")).map(|tab| tab.id.clone())
    }

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
        let moves = self.move_entries(tab_id);
        if !moves.is_empty() {
            entries.push(Entry::Rule);
            entries.extend(moves);
        }
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
            } else if is_move(id) {
                this.move_tab_picked(id, &tab_id, window, cx);
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
