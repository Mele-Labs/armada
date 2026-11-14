//! The moving card: a Job on its way to his other Mac, pinned at the thread's end as the claims card is. Its head is an
//! amber dot and "Moving to <Mac>"; under it stand the move's three steps, the one it is at filled in amber. It has no
//! act, since Fleet's move cannot be cancelled (`crate::mac_move`).

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};

use super::rows::tip;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::mac_move::{moving_said, name_of, step_of, target_of, Step, STEPS};

impl JobView {
    /// The card, or `None` where the Job is not on its way to another Mac.
    pub(super) fn moving_card(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let record = self.record(id, cx)?;
        let target = target_of(record)?;
        let macs = crate::pairing::macs(cx);
        let name = name_of(macs, target, record.mac.is_some());
        let at = step_of(self.working(id, cx));
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let said: SharedString = moving_said(&name).into();
        let head = div()
            .id("moving-head")
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().id("moving-mark").flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(amber).tooltip(tip("Moving")))
            .child(said.clone());
        let chips = STEPS.iter().map(|step| {
            let words: SharedString = step.said(&name).into();
            let (ink, ground) = if *step == at { (look.color(token::FG_INVERSE), amber) } else { (look.color(token::FG_MUTED), look.color(token::FG_DEFAULT).opacity(0.08)) };
            labelled(
                crate::operator::view::chip(words.clone(), ink, ground, look).id(SharedString::from(format!("moving-step:{}", step_key(*step)))),
                Role::Status,
                if *step == at { format!("{words}, current") } else { words.to_string() },
            )
        });
        Some(
            labelled(
                div()
                    .id("moving")
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(amber)
                    .bg(look.color(token::BG_SUNKEN))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .child(labelled(head, Role::Heading, said))
                    .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_1)).children(chips)),
                Role::Region,
                "Moving",
            )
            .into_any_element(),
        )
    }
}

fn step_key(step: Step) -> &'static str {
    match step {
        Step::TurnEnds => "turn-ends",
        Step::BranchPushed => "branch-pushed",
        Step::Resumed => "resumed",
    }
}
