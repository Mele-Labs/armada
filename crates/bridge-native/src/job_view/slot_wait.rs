//! The slot card: a Job waiting for a worktree slot, pinned at the thread's end as the moving card is,
//! while Fleet's record carries the wait (`slot_wait`). Waiting is the default, so it asks nothing: its
//! two buttons are for when he would rather not wait. Add has Fleet make a slot for this Job
//! (`answer_waiting` on `slot-choice`, choice 1); Free gives this Job the slot the quietest idle Job
//! holds (`free_slot`). The words are `crate::slot_wait`'s.

use chrono::Utc;
use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::{AnswerWaiting, FreeSlot, SessionId, SlotWait};

use super::claim_waits::waited;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::slot_wait::{freeable, place, WAITING};
use crate::titlebar::parts::{Btn, Weight};

/// The id Add and Free answer, though no item lists it.
const SLOT_CHOICE: &str = "slot-choice";

fn encoded<T: serde::Serialize>(body: &T) -> Result<String, String> {
    ipc::encode(body).map_err(|why| format!("{why:?}"))
}

/// The line under the heading: its place, how long it has waited, and that it moves in by itself.
pub(super) fn under(wait: &SlotWait, now: chrono::DateTime<Utc>) -> String {
    let mut parts = vec![place(wait.ahead)];
    parts.extend(waited(&wait.since, now));
    parts.push("moves in by itself when one frees".to_string());
    parts.join(" \u{b7} ")
}

impl JobView {
    /// The card, or `None` where the Job waits for no slot.
    pub(super) fn slot_wait_card(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let wait = self.record(id, cx)?.slot_wait.clone()?;
        let now = crate::mock::fixed_now().map_or_else(Utc::now, |now| now.with_timezone(&Utc));
        let adding = wait.creating_since.is_some();
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph(look, "hard-drive", Size::Small, look.color(token::FG_MUTED)))
            .child(WAITING);
        let said = div()
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(under(&wait, now));
        let refused = wait.create_refused.clone().map(|why| {
            div()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                .child(format!("Could not add a slot: {why}"))
        });
        let job = id.clone();
        let add_said = if adding { "Adding a slot" } else { "Add a slot" };
        let add = labelled(
            Btn::new("slot-wait-add", add_said, Weight::Secondary)
                .pending(adding)
                .draw(look, cx)
                .tooltip(tip("Fleet adds a slot to the pool, and this Job takes it"))
                .when(!adding, |button| button.on_click(cx.listener(move |this, _, window, cx| this.slot_added(job.clone(), window, cx)))),
            Role::Button,
            add_said,
        );
        let free = freeable(&wait).map(|holder| {
            let (job, slot) = (id.clone(), holder.slot);
            let words: SharedString = format!("Free slot {slot}").into();
            let why = match waited(&holder.quiet_since, now) {
                Some(quiet) => format!("Slot {slot} is held by {}, quiet {quiet}. It keeps its work, and this Job takes the slot", holder.job.title),
                None => format!("Slot {slot} is held by {}. It keeps its work, and this Job takes the slot", holder.job.title),
            };
            labelled(
                Btn::new("slot-wait-free", words.clone(), Weight::Secondary)
                    .draw(look, cx)
                    .tooltip(tip(why))
                    .on_click(cx.listener(move |this, _, window, cx| this.slot_freed(job.clone(), slot, window, cx))),
                Role::Button,
                words,
            )
        });
        let words = div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).child(head).child(said).children(refused);
        let acts = div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).child(add).children(free);
        Some(
            labelled(
                div()
                    .id("slot-wait")
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .child(words)
                    .child(acts),
                Role::Region,
                WAITING,
            )
            .into_any_element(),
        )
    }

    /// Add: Fleet makes a slot and this Job takes it. A refusal is said as a toast; the card's own line says a failed make.
    fn slot_added(&mut self, job: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let body = encoded(&AnswerWaiting { session_id: SessionId::carried(job.to_string()), item_id: SLOT_CHOICE.into(), choice: Some(1), text: None, mode: None, fixed: None, extras: Default::default() });
        self.post_with(job, "/sessions/waiting/answer", body, |_, why, cx| cx.defer(move |cx| crate::toasts::tell(why, cx)), window, cx);
    }

    /// Free: the slot goes back from the Job holding it, and this Job takes it.
    fn slot_freed(&mut self, job: SharedString, slot: u32, window: &mut Window, cx: &mut Context<Self>) {
        let body = encoded(&FreeSlot { session_id: SessionId::carried(job.to_string()), item_id: SLOT_CHOICE.into(), slot });
        self.post_with(job, "/sessions/slot_choice/free", body, |_, why, cx| cx.defer(move |cx| crate::toasts::tell(why, cx)), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_under_says_its_place_how_long_and_that_it_moves_in_alone() {
        let wait = SlotWait { since: ipc::Instant::carried("2026-11-14T16:00:00.000Z"), ahead: 1, holders: Vec::new(), creating_since: None, create_refused: None };
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-14T16:12:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(under(&wait, now), "2nd in line \u{b7} 12 min \u{b7} moves in by itself when one frees");
    }
}
