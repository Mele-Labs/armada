//! Pane zoom, as Electron's `job-panes/zoom.ts` has it. ⌥⌘Enter, a double press on a pane's strip, or a
//! tab's Zoom covers the Job's own area, to the right of the Jobs list, with one pane; Restore or the same
//! key puts it back. A zoom is where he is looking, not how the Job is laid out: it is not kept and is not
//! a step back, and another Job on screen ends it (`place.rs`). Close pane is a change of the arrangement
//! and lives there too.

use gpui::{Context, KeyDownEvent, Window};

use super::JobView;

impl JobView {
    /// ⌥⌘Enter, outside a text box as Electron's `usePaneKeys` takes it: the pane the keys are in.
    pub(super) fn zoom_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let keys = &event.keystroke;
        let chord = keys.modifiers.alt && keys.modifiers.platform && !keys.modifiers.shift && !keys.modifiers.control;
        if !chord || keys.key != "enter" || self.shown.is_none() || self.sheet_over() {
            return false;
        }
        if window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
            return false;
        }
        let pane = self.zoomed().unwrap_or_else(|| self.here(window, cx));
        self.toggle_zoom(&pane, cx);
        true
    }
}
