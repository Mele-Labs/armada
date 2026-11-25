//! An artifact of parts read whole in a tab of its own, as a page and a draft are read in theirs. While it asks it is read
//! only, under one line that opens the review sheet, the one place it is answered (one ask, one place, 19 Nov). The drawing
//! is large; the card there never folds, so what he answered stays to read.

use gpui::{div, prelude::*, AnyElement, Context, KeyDownEvent, Window};

use super::acts::SHEET;
use super::row_tabs::RowTab;
use super::JobView;
use crate::look::{token, Look};

impl JobView {
    /// The tab's body: the artifact's card, whole, centred in the column the thread reads in.
    pub(super) fn artifact_body(&self, id: &gpui::SharedString, tab: &RowTab, look: &Look, cx: &Context<Self>) -> AnyElement {
        let empty = || div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element();
        let Some((page, one)) = self.pages.get(id).zip(self.artifact_of(id, &tab.key)) else { return empty() };
        let Some(newest) = one.revisions.last().map(|last| last.number) else { return empty() };
        let card = self.parts_card(SHEET, &tab.key, newest, true, false, page, look, cx);
        let artifact = tab.key.clone();
        // The scroller holds one block and no flex row: a row stretches its child to the pane's height, so a card taller than the
        // pane never overflowed it and nothing scrolled.
        div()
            .id("row-tab-body")
            .track_focus(&self.row_focus)
            .track_scroll(&self.body_scroll)
            .size_full()
            .overflow_y_scroll()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.artifact_key(&artifact, event, window, cx)))
            .child(div().w_full().flex().justify_center().child(div().w_full().min_w_0().max_w(self.column(look)).children(card)))
            .into_any_element()
    }

    /// The keys over the open artifact: paging the column. The rest are the card's own.
    fn artifact_key(&mut self, artifact: &str, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform {
            return;
        }
        let mods = event.keystroke.modifiers;
        if !mods.modified() && !self.parts_typing(artifact, window, cx) {
            let page = self.body_scroll.bounds().size.height * 0.9;
            let by = match event.keystroke.key.as_str() {
                "pagedown" | "space" => Some(-page),
                "pageup" => Some(page),
                _ => None,
            };
            if let Some(by) = by {
                let top = (self.body_scroll.offset().y + by).clamp(-self.body_scroll.max_offset().y, gpui::px(0.));
                self.body_scroll.set_offset(gpui::point(gpui::px(0.), top));
                cx.stop_propagation();
                cx.notify();
                return;
            }
        }
        // One that asks is answered in the sheet: Enter opens it there, and no key answers it here.
        if self.reads_only(artifact, cx) {
            if !mods.modified() && event.keystroke.key == "enter" {
                cx.stop_propagation();
                self.open_artifact_review(artifact, window, cx);
            }
            return;
        }
        self.parts_key(artifact, event, window, cx);
    }
}
