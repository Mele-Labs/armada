//! The claim, written before the code: **a step whose `ends_on` names a lifecycle state holds the Job
//! until instances of that lifecycle get there, says so to the agent and the person without raising a
//! wait for him, and is moved on by the world, by his writing on the step, or by a skip.** The owner
//! approved the design on 29 Oct 2026. The rules here are pure; the rest is `lifecycle_ends_read` and
//! `lifecycle_waking`.

use core_model::{LifecycleEnd, StepEnd};
use ipc::{Instant, SessionRow, SessionVoice};

use super::session_rail::{approved, at, delivers, follows, plan_step, step};
use crate::rail::{Facts, RailStep, Skipped};
use crate::rail_ends::{self, ends_any, ends_when, End, Waiter};
use crate::rail_keeping::messaged_on;
use crate::rail_lifecycle::StepBits;

pub(super) fn end(written: &str) -> StepEnd {
    StepEnd::Lifecycle(LifecycleEnd::from_wire(written).expect("a lifecycle end"))
}

/// Plan, Implement, Review (ends when every pull request is approved), Land.
pub(super) fn reviewed() -> Vec<RailStep> {
    let review = RailStep { ends_on: vec![end("pr: approved")], instructions: Some("Answer the review.".into()), ..step("Review") };
    vec![plan_step(), follows("Implement"), review, delivers("Land")]
}

/// Plan, Implement, Land (delivers, and `pr: approved` gates its merge).
fn landing() -> Vec<RailStep> {
    let land = RailStep {
        delivers: true,
        ends_on: vec![end("pr: approved")],
        instructions: Some("Open the pull request and merge it.".into()),
        ..step("Land")
    };
    vec![plan_step(), follows("Implement"), land]
}

fn holding(steps: &[usize], met: &[usize]) -> Facts {
    Facts { lifecycle_held: StepBits::of(steps.iter().copied()), lifecycle_met: StepBits::of(met.iter().copied()), ..approved() }
}

#[test]
fn a_step_that_ends_on_a_state_is_entered_when_the_job_holds_an_instance_and_left_when_it_gets_there() {
    let steps = reviewed();
    assert_eq!(at(&steps, approved()), Some(1), "before the Job holds a pull request");
    assert_eq!(at(&steps, holding(&[2], &[])), Some(2), "holding one that is not approved");
    assert_eq!(at(&steps, Facts { pushed: true, ..holding(&[2], &[]) }), Some(2), "a push does not carry the Job past the wait");
    assert_eq!(at(&steps, holding(&[2], &[2])), Some(1), "approved, and no push yet");
    assert_eq!(at(&steps, Facts { pushed: true, ..holding(&[2], &[2]) }), Some(3), "approved and pushed: the next step");
}

#[test]
fn a_job_holding_none_is_not_on_the_step_and_a_message_or_a_skip_moves_a_job_that_waits_for_nothing() {
    let steps = reviewed();
    let pushed = Facts { pushed: true, ..approved() };
    assert_eq!(at(&steps, pushed), Some(1), "none held: the wait is not entered, and the push does not enter Land past it");
    assert_eq!(at(&steps, Facts { messaged: StepBits::of([2]), ..pushed }), Some(3), "he wrote on the step");
    assert_eq!(at(&steps, Facts { skipped: Skipped::of(&[2]), ..pushed }), Some(3), "he skipped it");
    let waiting = holding(&[2], &[]);
    assert_eq!(at(&steps, Facts { messaged: StepBits::of([2]), pushed: true, ..waiting }), Some(3));
    assert_eq!(at(&steps, Facts { skipped: Skipped::of(&[2]), pushed: true, ..waiting }), Some(3));
}

#[test]
fn a_step_that_nothing_observable_enters_follows_the_wait_and_a_wait_that_is_last_ends_the_rail() {
    let wait = RailStep { ends_on: vec![end("ticket: closed")], ..step("Wait for the ticket") };
    let steps = [plan_step(), follows("Implement"), wait.clone(), step("Close out")];
    assert_eq!(at(&steps, holding(&[2], &[])), Some(2));
    assert_eq!(at(&steps, holding(&[2], &[2])), Some(3), "a step with no fact to enter it is entered when the wait before it ends");
    let last = [plan_step(), follows("Implement"), wait];
    assert_eq!(at(&last, holding(&[2], &[])), Some(2));
    assert_eq!(at(&last, holding(&[2], &[2])), None, "nothing follows the last step once its end comes");
}

#[test]
fn any_one_of_the_ends_a_step_lists_ends_it_the_lifecycle_ones_among_the_words() {
    let both = RailStep { ends_on: vec![end("pr: approved"), end("ticket: closed")], ..step("Review") };
    let steps = [plan_step(), follows("Implement"), both, delivers("Land")];
    assert_eq!(at(&steps, holding(&[2], &[])), Some(2));
    assert_eq!(at(&steps, Facts { pushed: true, ..holding(&[2], &[2]) }), Some(3), "met is per step: one of its ends came");
}

#[test]
fn a_step_that_ends_on_a_state_is_the_worlds_to_end_and_raises_no_wait_and_no_stall() {
    let steps = reviewed();
    let ends = rail_ends::ends_of(&steps, 2);
    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0].waits_on(), Waiter::TheWorld, "{:?}", ends[0]);
    assert!(ends[0].by_the_world(), "{:?}", ends[0]);
    assert!(!ends[0].by_the_job(), "the agent has nothing to do to end it, so it is never nudged as stalled");
    for other in [End::Merged, End::PlanApproved, End::Replied, End::Pushed, End::Edited] {
        assert!(!other.by_the_world(), "{other:?}");
    }
    assert_eq!(rail_ends::end_of(&steps, 2).map(|one| one.by_the_job()), Some(false));
}

#[test]
fn what_ends_the_step_is_said_with_the_lifecycle_and_state_in_the_words_a_person_reads() {
    let steps = reviewed();
    assert_eq!(ends_any(&steps, 2), ["every pr is approved"]);
    assert_eq!(ends_when(&steps, 2).as_deref(), Some("every pr is approved"));
    let any = [RailStep { ends_on: vec![end("any ticket: in_review")], ..step("Review") }, plan_step()];
    assert_eq!(ends_when(&any, 0).as_deref(), Some("any ticket is in review"), "any, and an underscore as a space");
    let both_kinds = [RailStep { ends_on: vec![end("pr: approved"), StepEnd::Replied], ..step("Review") }, plan_step()];
    assert_eq!(ends_any(&both_kinds, 0), ["every pr is approved", "you reply"], "the words the step also lists are still said");
}

fn from_you(n: u32, text: &str) -> SessionRow {
    row(n, SessionVoice::You, text)
}

fn row(n: u32, from: SessionVoice, text: &str) -> SessionRow {
    SessionRow::Message {
        id: format!("m{n}"),
        at: Instant::carried(&format!("2026-10-29T09:00:{n:02}.000Z")),
        from,
        text: text.into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: None,
        queued: false,
    }
}

fn visit(step: u32, n: u32) -> (u32, String) {
    (step, format!("2026-10-29T09:00:{n:02}.000Z"))
}

#[test]
fn only_what_the_person_wrote_after_a_step_began_counts_as_writing_on_it() {
    let rows = [from_you(1, "go"), row(4, SessionVoice::Agent, "waiting"), from_you(6, "skip the review")];
    let began = [visit(0, 0), visit(2, 3)];
    assert!(messaged_on(&rows, &began).has(2), "written at :06, after the step began at :03");
    assert!(messaged_on(&rows, &began).has(0), "and the first step, written on at :01");
    let before = [visit(2, 7)];
    assert!(!messaged_on(&rows, &before).has(2), "everything he wrote came before");
    let only_the_agent = [row(4, SessionVoice::Agent, "waiting"), row(5, SessionVoice::Session { id: "fleet".into(), title: "Fleet".into() }, "Fleet: ...")];
    assert_eq!(messaged_on(&only_the_agent, &[visit(2, 3)]), StepBits::default(), "neither the agent's words nor Fleet's");
    let twice = [visit(2, 1), visit(3, 2), visit(2, 5)];
    assert!(!messaged_on(&[from_you(3, "earlier")], &twice).has(2), "only the latest visit to a step counts");
}

#[test]
fn a_delivering_step_that_lists_a_lifecycle_end_is_entered_as_always_and_only_landing_passes_it() {
    let steps = landing();
    assert_eq!(at(&steps, approved()), Some(1), "not pushed yet");
    let pushed = Facts { pushed: true, ..approved() };
    assert_eq!(at(&steps, pushed), Some(2), "entered on the push with nothing held: the agent opens the pull request here");
    let waiting = Facts { pushed: true, ..holding(&[2], &[]) };
    assert_eq!(at(&steps, waiting), Some(2));
    let met = Facts { pushed: true, ..holding(&[2], &[2]) };
    assert_eq!(at(&steps, met), Some(2), "the state lifts the wait and does not end the step");
    let written = Facts { messaged: StepBits::of([2]), ..waiting };
    assert_eq!(at(&steps, written), Some(2), "nor does a message");
    assert_eq!(at(&steps, Facts { landed: true, ..met }), None, "landing passes it, as it does with no gate");
    assert_eq!(at(&steps, Facts { landed: true, ..waiting }), None, "and it passes without the state, since the merge is done");
}

#[test]
fn a_delivering_step_that_waits_still_ends_on_its_merge_and_the_page_says_the_gate_before_it() {
    let steps = landing();
    assert_eq!(ends_when(&steps, 2).as_deref(), Some("it is merged"), "the merge is still what ends it");
    assert_eq!(ends_any(&steps, 2), ["every pr is approved, then it is merged"], "and the page says the gate before it");
    let plain_land = [plan_step(), delivers("Land")];
    assert_eq!(ends_any(&plain_land, 1), ["it is merged"], "a Land with no gate says only the merge");
    let several = RailStep { ends_on: vec![end("pr: approved"), end("any ticket: closed")], ..steps[2].clone() };
    let gated = [plan_step(), several];
    assert_eq!(ends_any(&gated, 1), ["every pr is approved, or any ticket is closed, then it is merged"]);
}
