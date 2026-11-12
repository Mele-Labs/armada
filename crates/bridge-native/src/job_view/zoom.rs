//! Pane zoom and Close pane, as Electron's `job-panes/zoom.ts` and `PaneView` have them. ⌥⌘Enter, a
//! double press on a pane's strip, or the thread tab's Zoom covers the Job's own area, to the right of
//! the Jobs list, with one pane; Restore or the same key puts it back. A zoom is where he is looking,
//! not how the Job is laid out: it is not kept, and another Job on screen ends it. Close pane takes
//! away a pane whose tabs are all closed.

use std::collections::{HashMap, HashSet};

use gpui::{prelude::*, Context, KeyDownEvent, Role, SharedString, Window};

use super::head::tip;
use super::tabs::{frame, Pane};
use super::JobView;
use crate::labels::labelled;
use crate::look::Look;

#[derive(Default)]
pub(super) struct Zoom {
    /// The pane covering the Job's area, if one is.
    on: Option<Pane>,
    /// The pane last pressed in, which the key zooms where the keys are not in the thread.
    here: Option<Pane>,
    /// The panes closed on each Job.
    closed: HashMap<SharedString, HashSet<Pane>>,
}

impl JobView {
    /// Whether the thread's place and the right column are drawn: one of them alone while a pane of
    /// it is zoomed, and neither where every pane in it is closed.
    pub fn split_shows(&self) -> (bool, bool) {
        match self.zoom.on {
            Some(Pane::Thread) => (true, false),
            Some(Pane::Top | Pane::Now) => (false, true),
            None => (!self.pane_closed(Pane::Thread), !(self.pane_closed(Pane::Top) && self.pane_closed(Pane::Now))),
        }
    }

    /// Whether a pane is drawn: not closed, and no other pane zoomed over it.
    pub(super) fn pane_drawn(&self, pane: Pane) -> bool {
        !self.pane_closed(pane) && self.zoom.on.is_none_or(|on| on == pane)
    }

    pub(super) fn pane_zoomed(&self, pane: Pane) -> bool {
        self.zoom.on == Some(pane)
    }

    fn pane_closed(&self, pane: Pane) -> bool {
        self.shown.as_ref().and_then(|id| self.zoom.closed.get(id)).is_some_and(|closed| closed.contains(&pane))
    }

    /// A pane with no tab left in it, which offers Close pane.
    pub(super) fn pane_empty(&self, pane: Pane) -> bool {
        match pane {
            Pane::Thread => self.tab_closed("thread") && self.shown.as_ref().is_none_or(|id| self.row_tabs_empty(id)) && !self.view_tabs_in(super::ViewPane::Thread),
            Pane::Top => self.zone_shown().is_none() && !self.view_tabs_in(super::ViewPane::Top),
            Pane::Now => self.tab_closed("now") && !self.view_tabs_in(super::ViewPane::Bottom),
        }
    }

    /// A press anywhere in a pane makes it the one the key zooms (`focusIn`).
    pub(super) fn pane_pressed(&mut self, pane: Pane) {
        self.zoom.here = Some(pane);
    }

    /// The pane zoomed or put back (`toggleZoom`).
    pub(super) fn toggle_zoom(&mut self, pane: Pane, cx: &mut Context<Self>) {
        self.zoom.on = if self.zoom.on == Some(pane) { None } else { Some(pane) };
        self.zoom.here = Some(pane);
        cx.notify();
    }

    /// Another Job on screen ends a zoom.
    pub(super) fn end_zoom(&mut self) {
        self.zoom.on = None;
        self.zoom.here = None;
    }

    /// ⌥⌘Enter, outside a text box as Electron's `usePaneKeys` takes it: the pane the keys are in, or
    /// the one last pressed in.
    pub(super) fn zoom_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let keys = &event.keystroke;
        let chord = keys.modifiers.alt && keys.modifiers.platform && !keys.modifiers.shift && !keys.modifiers.control;
        if !chord || keys.key != "enter" || self.shown.is_none() {
            return false;
        }
        if window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
            return false;
        }
        let in_thread = self.focus.contains_focused(window, cx) || self.head_tab_focused(window, cx);
        let pane = self.zoom.on.or(if in_thread { Some(Pane::Thread) } else { self.zoom.here }).unwrap_or(Pane::Thread);
        self.toggle_zoom(pane, cx);
        true
    }

    /// Close pane: the pane goes, and the panes beside it take its room (`closePane`).
    pub(super) fn close_pane(&mut self, pane: Pane, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.zoom.on == Some(pane) {
            self.zoom.on = None;
        }
        self.zoom.closed.entry(id).or_default().insert(pane);
        cx.notify();
    }

    /// The end of a pane's strip: Restore while it is zoomed, and Close pane while it holds no tab.
    pub(super) fn strip_end(&self, pane: Pane, look: &Look, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let name = |what: &str| match pane {
            Pane::Thread => format!("{what}: Thread"),
            Pane::Top => format!("{what}: Open, Done and Deferred"),
            Pane::Now => format!("{what}: Now"),
        };
        let mut end = Vec::new();
        if self.pane_zoomed(pane) {
            end.push(
                labelled(
                    frame(look, SharedString::from(name("restore")), "minimize-2").tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Restore").key_binding(gpui::Keystroke::parse("alt-cmd-enter").ok().map(gpui_component::kbd::Kbd::new)).build(window, cx)).on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_zoom(pane, cx);
                    })),
                    Role::Button,
                    "Restore",
                )
                .into_any_element(),
            );
        }
        if self.pane_empty(pane) {
            end.push(
                labelled(
                    frame(look, SharedString::from(name("close-pane")), "x").tooltip(tip("Close pane".into())).on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.close_pane(pane, cx);
                    })),
                    Role::Button,
                    "Close pane",
                )
                .into_any_element(),
            );
        }
        end
    }
}
