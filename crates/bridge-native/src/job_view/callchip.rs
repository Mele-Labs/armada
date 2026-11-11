//! The one jump control: a pill over the box that says a plan or a sketch waits below the words in
//! view, and takes the person to it (`CallChip`).

use gpui::{div, prelude::*, AnyElement, Context, ListOffset, Role, SharedString};
use ipc::PlanCardState;

use super::rows::{glyph, tip, Size};
use super::{Extra, JobView};
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

impl JobView {
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
