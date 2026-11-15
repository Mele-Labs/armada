//! An artifact of parts read whole in a tab of its own, as a page and a draft are read in theirs. Every part is answered
//! there as it is on the card (`parts_card`): takes with 1 to 4 and Send pick, words with Approve and Ask for changes, buttons
//! pressed. The drawing is large, with ⌘+, ⌘−, ⌘0 and the pinch; the card there never folds, so what he answered stays to read.

use gpui::{div, prelude::*, AnyElement, Context, KeyDownEvent, Window};

use super::acts::SHEET;
use super::row_tabs::RowTab;
use super::sketch_keys::lens_key;
use super::JobView;
use crate::look::{token, Look};

impl JobView {
    /// The tab's body: the artifact's card, whole, centred in the column the thread reads in.
    pub(super) fn artifact_body(&self, id: &gpui::SharedString, tab: &RowTab, look: &Look, cx: &Context<Self>) -> AnyElement {
        let empty = || div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element();
        let Some((page, one)) = self.pages.get(id).zip(self.artifact_of(id, &tab.key)) else { return empty() };
        let Some(newest) = one.revisions.last().map(|last| last.number) else { return empty() };
        let card = self.parts_card(SHEET, &tab.key, newest, true, page, look, cx);
        let artifact = tab.key.clone();
        div()
            .id("row-tab-body")
            .track_focus(&self.row_focus)
            .size_full()
            .overflow_y_scroll()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .flex()
            .justify_center()
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.artifact_key(&artifact, event, window, cx)))
            .child(div().w_full().min_w_0().max_w(self.column(look)).children(card))
            .into_any_element()
    }

    /// The keys over the open artifact: ⌘+ and ⌘− zoom the drawing and ⌘0 fits it. The rest are the card's own.
    fn artifact_key(&mut self, artifact: &str, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if event.keystroke.modifiers.platform {
            let drawing = self.artifact_of(&job, artifact).and_then(|one| one.parts.iter().find(|part| matches!(part.content, ipc::PartContent::Drawing { .. })).map(|part| part.id.clone()));
            if let (Some(by), Some(part)) = (lens_key(&event.keystroke), drawing) {
                self.zoom_part(&super::parts_card::part_key(artifact, &part), by, cx);
                cx.stop_propagation();
            }
            return;
        }
        self.parts_key(artifact, event, window, cx);
    }
}
