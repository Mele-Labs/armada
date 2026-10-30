//! What a rule planned is written to the tracker and the forge, or is not carried. **The tracker
//! and the forge are fakes that record what they were asked**, and the functions called are the
//! ones Fleet's worker calls: `tracker_written` and `forge_written` make the write, and
//! `Writes::carries` says beforehand whether the repository can. That an act under Asks waits as
//! a Do it / Skip item, that one under Acts raises none, and that a failed write is tried again and
//! then asked about are `fleet`'s own tests (`rule_writing`), which need a store this crate does not have.

use std::sync::Arc;

use adapter_traits::TrackerWrites;
use config::Rules;
use core_model::rule::{Act, AutonomyDefaults, Course, Engine, ForgeWrite, Occurrence, PlannedAct, RanOnce};
use core_model::{HeldInstances, Lifecycle, Moved};
use fleet::lifecycles::saying::said;
use fleet::lifecycles::writing::{forge_written, tracker_written, Writes};
use testkit::{Delivered, FakeTracker, FakeVcs, IssueWritten};

use crate::bench::lifecycles::{
    a_job_holding_three_tickets, at, in_state, plan_step_start, rules_from, taken_on, the_jobs_copy, MOVES_TICKETS,
};

/// A rule that asks for review, readies the pull request and lets it merge, once it is approved.
const LANDS_THE_PULL_REQUEST: &str = "version: 1
rule_id: land_when_approved
name: Land when approved
scope: my-store
when: { enters: { lifecycle: pr, state: approved } }
do:
  - forge_write: { request_reviewers: [ana, bob], on: { entered: true } }
  - forge_write: { mark_ready: true, on: { entered: true } }
  - forge_write: { auto_merge: true, on: { entered: true } }
  - forge_write: { merge: true, on: { entered: true } }
";

/// What the act is done to, as the pair the writes take.
fn on_of(planned: &PlannedAct) -> Option<(&str, &str)> {
    planned.on().map(|on| (on.lifecycle().as_str(), on.key()))
}

fn writes_over(tracker: &Arc<FakeTracker>) -> Writes {
    Writes::new(Arc::clone(tracker) as Arc<dyn TrackerWrites + Send + Sync>, Ok(()))
}

/// The acts a rule plans when the Job's pull request 812 is approved, and the lifecycles it reads against.
fn approved_pull_request() -> (Vec<PlannedAct>, Vec<Lifecycle>) {
    let copy = the_jobs_copy();
    let rules: Rules = rules_from(&[LANDS_THE_PULL_REQUEST]);
    let mut held = HeldInstances::new();
    held.take_on(taken_on(&copy[1], "812")).expect("held");
    let approved = Occurrence::entered(at(&copy[1], "812"), Moved { from: in_state("in_review"), to: in_state("approved") }, "10:00");
    let frozen = rules.frozen();
    let planned = Engine::new(&frozen, &AutonomyDefaults::new()).plan(&approved, &held, &Default::default(), &mut RanOnce::new());
    (planned, copy)
}

#[test]
fn the_planned_ticket_moves_are_made_in_the_trackers_own_word_and_no_other_ticket_is_touched() {
    let copy = the_jobs_copy();
    let rules = rules_from(&[MOVES_TICKETS]).frozen();
    let moves = plan_step_start("plan", "10:00", &rules, &AutonomyDefaults::new(), &a_job_holding_three_tickets(), &mut RanOnce::new());
    assert_eq!(moves.len(), 2, "the two tickets still in To do");

    let tracker = Arc::new(FakeTracker::empty());
    let writes = writes_over(&tracker);
    let mut said_done = Vec::new();
    for one in &moves {
        assert_eq!(writes.carries(one.act(), on_of(one), &copy), Ok(()), "{}", said(one.act(), one.on(), &copy));
        let Act::Tracker { write, .. } = one.act() else { panic!("{:?}", one.act()) };
        said_done.push(tracker_written(&*tracker, "/store-web", write, on_of(one), &copy).expect("made"));
    }
    assert_eq!(said_done, ["Moved ARM-41 to In Progress", "Moved ARM-42 to In Progress"]);
    assert_eq!(
        tracker.writes(),
        [
            IssueWritten::Moved { reference: "ARM-41".into(), status: "In Progress".into() },
            IssueWritten::Moved { reference: "ARM-42".into(), status: "In Progress".into() },
        ],
        "the first word the state lists in the Workspace's tracker, and ARM-43 is already in progress"
    );
}

#[test]
fn a_tracker_that_cannot_make_the_move_is_not_carried_with_why_and_is_never_called() {
    let copy = the_jobs_copy();
    let rules = rules_from(&[MOVES_TICKETS]).frozen();
    let moves = plan_step_start("plan", "10:00", &rules, &AutonomyDefaults::new(), &a_job_holding_three_tickets(), &mut RanOnce::new());
    let tracker = Arc::new(FakeTracker::empty());
    tracker.cannot("move", "this tracker cannot move an issue");
    let writes = writes_over(&tracker);

    for one in &moves {
        assert_eq!(writes.carries(one.act(), on_of(one), &copy), Err("this tracker cannot move an issue".to_string()));
        let Act::Tracker { write, .. } = one.act() else { panic!("{:?}", one.act()) };
        assert!(tracker_written(&*tracker, "/store-web", write, on_of(one), &copy).is_err(), "and doing it says no as well");
    }
    assert!(tracker.writes().is_empty());

    let nowhere = Writes::nowhere("the Job is in no repository this Fleet serves");
    assert!(nowhere.carries(moves[0].act(), on_of(&moves[0]), &copy).is_err(), "a Job outside every repository writes nowhere");

    // Armada's own ticket lifecycle lists no tracker word for any state, so a move under it names no status to write.
    let built_in = vec![Lifecycle::ticket()];
    let why = writes_over(&Arc::new(FakeTracker::empty())).carries(moves[0].act(), on_of(&moves[0]), &built_in).expect_err("no word");
    assert!(why.contains("no tracker word"), "{why}");
}

#[test]
fn the_planned_forge_writes_are_made_through_the_forge_and_a_merge_is_the_forges_own_so_a_queue_takes_it() {
    let (planned, copy) = approved_pull_request();
    let lines: Vec<String> = planned.iter().map(|one| said(one.act(), one.on(), &copy)).collect();
    assert_eq!(lines, ["Request review on #812 from ana, bob", "Mark #812 ready", "Turn on auto-merge for #812", "Merge #812"]);
    assert!(planned.iter().all(|one| one.course() == Course::Asks), "a write to the forge asks until the owner says otherwise");

    let tracker = Arc::new(FakeTracker::empty());
    let forge = FakeVcs::default();
    let writes = writes_over(&tracker);
    let mut said_done = Vec::new();
    for one in &planned {
        assert_eq!(writes.carries(one.act(), on_of(one), &copy), Ok(()), "{}", said(one.act(), one.on(), &copy));
        let Act::Forge { write, .. } = one.act() else { panic!("{:?}", one.act()) };
        said_done.push(forge_written(&forge, "/store-web", write, on_of(one)).expect("made"));
    }
    assert_eq!(
        said_done,
        ["Asked ana, bob to review #812", "Marked #812 ready", "Turned on auto-merge for #812", "The forge took the merge of #812"]
    );

    let delivered = forge.delivered();
    assert!(delivered.contains(&Delivered::RequestedReviewers { pull_request: "812".into(), reviewers: vec!["ana".into(), "bob".into()] }), "{delivered:?}");
    assert!(delivered.contains(&Delivered::MarkedReady { pull_request: "812".into() }));
    assert!(delivered.contains(&Delivered::AutoMerge { pull_request: "812".into() }));
    assert!(delivered.contains(&Delivered::Merged { pull_request: "812".into() }));
    assert!(
        !delivered.iter().any(|one| matches!(one, Delivered::MergedByPush { .. } | Delivered::MergedTheBaseIn { .. } | Delivered::Pushed { .. })),
        "a merge asks the forge and never puts the branch on the base: {delivered:?}"
    );
    assert!(tracker.writes().is_empty(), "and no ticket was touched");
}

#[test]
fn a_forge_write_names_a_pull_request_of_this_repository_and_nothing_else() {
    let forge = FakeVcs::default();
    for other in ["acme/web#5", "", "5 && true"] {
        assert!(forge_written(&forge, "/store-web", &ForgeWrite::Merge, Some(("pr", other))).is_err(), "{other:?}");
    }
    assert!(forge_written(&forge, "/store-web", &ForgeWrite::Merge, Some(("ticket", "ARM-41"))).is_err(), "a ticket is not a pull request");
    assert!(forge_written(&forge, "/store-web", &ForgeWrite::Merge, None).is_err(), "nothing named");
    assert!(forge.delivered().is_empty(), "the forge was not reached");
}
