//! The one jump control: a pill over the box that says a plan or a sketch waits below the words in
//! view, and takes the person to it (`CallChip`).

use gpui::{div, prelude::*, AnyElement, Context, ListOffset, Role, SharedString};
use ipc::PlanCardState;

use super::rows::{glyph, tip, Size};
use super::{Extra, JobView};
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

impl JobView {
    /// The room the thread keeps clear at its foot while the chip stands over it, so the chip never covers a card's acts: the
    /// chip's height and the gap round it. Nothing where it is not drawn.
    pub(super) fn chip_room(&self, id: &SharedString, cx: &Context<Self>) -> gpui::Pixels {
        let look = cx.look();
        if self.render_callchip(id, cx).is_some() { look.length(token::H_CONTROL_SM) + look.length(token::SPACE_2) * 2. } else { gpui::px(0.) }
    }

    pub(super) fn render_callchip(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.pages.get(id)?;
        let tail = page.entry(page.items);
        let waiting = page.extras.iter().any(|extra| matches!(extra, Extra::Plan(_) | Extra::Sketch(_)));
        // The tail is below what is drawn: it is past the foot of the list's view, or not reached yet.
        let (viewport, top) = (page.list.viewport_bounds(), page.list.logical_scroll_top());
        let below = top.item_ix < tail && page.list.bounds_for_item(tail).is_none_or(|bounds| bounds.top() >= viewport.bottom());
        if !waiting || !below {
            return None;
        }
        let planned = page.extras.iter().any(|extra| matches!(extra, Extra::Plan(PlanCardState::Waiting)));
        let look = cx.look();
        let job = id.clone();
        let chip = labelled(
            div()
                .id("call-chip")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Go to what waits"))
                .children(planned.then(|| div().child("Plan")))
                .child(glyph(look, "arrow-down", Size::Small, look.color(token::STATUS_AWAITING_REVIEW)))
                // The press is the chip's alone: a row under it would take the keys, then scroll out of view.
                .block_mouse_except_scroll()
                .on_click(cx.listener(move |this, _, window, cx| {
                    if let Some(page) = this.pages.get(&job) {
                        page.list.scroll_to(ListOffset { item_ix: page.entry(page.items), offset_in_item: gpui::px(0.) });
                    }
                    window.focus(&this.focus, cx);
                    cx.notify();
                })),
            Role::Button,
            if planned { "Plan" } else { "Go to what waits" },
        );
        Some(div().flex().justify_center().child(chip).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thread at its tail with a plan set aside and a sketch pinned after it, on a window tall enough for the card.
    #[gpui::test]
    fn the_height_a_pinned_sketch_is_cut_to_settles_while_the_chip_may_come_and_go(cx: &mut gpui::TestAppContext) {
        let (view, cx) = crate::job_view::parts_fold::rooted_job(cx, "thread-jump", "tool-edit", gpui::size(gpui::px(1624.), gpui::px(1061.)), |_| {});
        let mut heights = Vec::new();
        let mut chips = Vec::new();
        for _ in 0..24 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
            heights.push(view.read_with(cx, |view, _| view.pane_height));
            chips.push(crate::job_view::parts_fold::drawn(cx, Role::Button, "Go to what waits"));
        }
        let last = heights[23];
        assert!(last.is_some(), "the thread was never measured");
        assert!(heights[12..].iter().all(|height| *height == last), "the room the pin is cut to changed from frame to frame: {heights:?}");
        assert!(chips[12..].iter().all(|chip| *chip == chips[23]), "the chip came and went from frame to frame: {chips:?}");
    }
}
