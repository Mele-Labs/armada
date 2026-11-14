//! The moving card: a Job on its way to his other Mac, pinned at the thread's end as the claims card is. Its head is an
//! amber dot and "Moving to <Mac>"; under it stand the move's three steps, the one it is at filled in amber. It has no
//! act, since Fleet's move cannot be cancelled. What the move is comes from `crate::move_to_mac`.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};

use super::rows::tip;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::move_to_mac::moving_to;

/// What a move says: the Mac it goes to, in the words a row and the card share.
pub(crate) fn moving_said(mac: &str) -> String {
    format!("Moving to {mac}")
}

/// The name of the Mac with this id, where the list holds it. A Job held from another Mac that is moving here names the
/// Mac it goes to by the id that Mac knows this one by, so a name the list lacks is this Mac's.
pub(crate) fn name_of(macs: &[ipc::Mac], id: &str, held_away: bool) -> String {
    match macs.iter().find(|one| one.id == id) {
        Some(one) => one.name.clone(),
        None if held_away => macs.iter().find(|one| one.this_mac).map_or_else(|| "this Mac".to_string(), |one| one.name.clone()),
        None => "another Mac".to_string(),
    }
}

/// One of a move's three steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    TurnEnds,
    BranchPushed,
    Resumed,
}

/// The steps in the order a move takes them.
pub(crate) const STEPS: [Step; 3] = [Step::TurnEnds, Step::BranchPushed, Step::Resumed];

impl Step {
    /// What the chip says. The last names the Mac the Job goes to.
    pub(crate) fn said(self, mac: &str) -> String {
        match self {
            Step::TurnEnds => "Turn ends".to_string(),
            Step::BranchPushed => "Branch pushed".to_string(),
            Step::Resumed => format!("Resumed on {mac}"),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Step::TurnEnds => "turn-ends",
            Step::BranchPushed => "branch-pushed",
            Step::Resumed => "resumed",
        }
    }
}

/// The step a move is at. Fleet sends no progress, so this is read off the Job: the move waits for the turn, so while it
/// works the move is at Turn ends, and once it has stopped the branch is being pushed. Resumed is the other Mac's, and the
/// row ends here when it is reached, so no record is ever at it.
pub(crate) fn step_of(working: bool) -> Step {
    if working { Step::TurnEnds } else { Step::BranchPushed }
}

/// Whether the card stands: only while Fleet's record says the Job is on its way, so it goes when the move ends.
pub(super) fn on_its_way(job: &ipc::SessionRecord) -> bool {
    moving_to(job).is_some()
}

impl JobView {
    /// The card, or `None` where the Job is not on its way to another Mac.
    pub(super) fn moving_card(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let record = self.record(id, cx)?;
        let target = moving_to(record)?;
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
                crate::operator::view::chip(words.clone(), ink, ground, look).id(SharedString::from(format!("moving-step:{}", step.key()))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{Mac, MacColour};

    fn mac(id: &str, name: &str, this: bool) -> Mac {
        Mac { id: id.into(), name: name.into(), colour: MacColour::Teal, this_mac: this, ..Default::default() }
    }

    #[test]
    fn the_step_is_read_off_whether_the_job_still_works() {
        assert_eq!(step_of(true), Step::TurnEnds);
        assert_eq!(step_of(false), Step::BranchPushed);
        let said: Vec<String> = STEPS.iter().map(|step| step.said("Studio")).collect();
        assert_eq!(said, ["Turn ends", "Branch pushed", "Resumed on Studio"]);
    }

    #[test]
    fn the_card_stands_only_while_the_job_is_moving() {
        let mut job = crate::mock::jobs("jobs-screens").expect("the fixture decodes").into_iter().find(|job| job.id.as_str() == "mock-02").expect("a hosted Job");
        assert!(!on_its_way(&job), "a quiet Job has no card");
        job.hosted.as_mut().expect("hosted").moving_to_mac = Some("air".into());
        assert!(on_its_way(&job), "the move is on");
        job.hosted.as_mut().expect("hosted").moving_to_mac = None;
        job.state = ipc::SessionState::Ended;
        job.end_reason = Some(crate::move_to_mac::ENDED_AS.into());
        assert!(!on_its_way(&job), "the card goes when the move has ended");
    }

    #[test]
    fn the_mac_a_move_goes_to_is_named_and_a_job_coming_here_names_this_mac() {
        let macs = [mac("air", "Air", true), mac("studio", "Studio", false)];
        assert_eq!(moving_said(&name_of(&macs, "studio", false)), "Moving to Studio");
        assert_eq!(name_of(&macs, "unknown", true), "Air", "an id the list lacks, on a Job held away, is this Mac");
        assert_eq!(name_of(&macs, "unknown", false), "another Mac");
    }
}
