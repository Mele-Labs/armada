//! The line pinned above the composer while the Job works: what it is doing now, in the words its row in the Jobs list uses
//! (`jobs::now_of`), with the ring that turns. It stands for no Job that is idle or waits on him, whose row says something else.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};
use ipc::JobLamp;

use super::rows::tip;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

impl JobView {
    /// What the Job is doing now, where its turn runs and it asks nothing of him.
    pub(super) fn live_line_of(&self, id: &str, cx: &gpui::App) -> Option<String> {
        if !self.working(id, cx) {
            return None;
        }
        let job = self.record(id, cx)?;
        if job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs) {
            return None;
        }
        crate::jobs::now_of(job)
    }

    /// The line at the thread's foot, one line cut with an ellipsis, its whole words in the tooltip.
    pub(super) fn render_live_line(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let words = self.live_line_of(id, cx)?;
        let look = cx.look();
        let said = SharedString::from(words);
        let ink = look.color(token::STATUS_RUNNING);
        Some(
            div()
                .w_full()
                .flex_none()
                .flex()
                .justify_center()
                .px(look.length(token::SPACE_4) + super::bleed(look))
                .pb(look.length(token::SPACE_2))
                .child(labelled(
                    div()
                        .id("live-line")
                        .w_full()
                        .min_w_0()
                        .max_w(self.column(look))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(self.working_mark(look, ink))
                        .child(div().id("live-words").flex_1().min_w_0().truncate().tooltip(tip(said.clone())).child(said.clone())),
                    Role::Status,
                    SharedString::from(format!("Now: {said}")),
                ))
                .into_any_element(),
        )
    }
}
