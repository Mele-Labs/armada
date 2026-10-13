//! The Commander's claim: **When a Job's Drone ends with nothing handed in, or
//! a group is red with its retries spent, the Job's Commander steers it
//! without the owner. It reads the Job, then raises the cap inside its
//! ceiling, splits or restarts a task, redirects a Drone, or sends a repair
//! Drone onto the Job's branch. The owner sees in the Job's thread and on each
//! task's mark that the Commander did it, and he can write to the Commander
//! and get an answer. Where the fix is out of its reach (skip or edit a Check,
//! edit the Manifest, pass a gate, attest, merge, push), it puts one card to
//! him. The Job never advances on anything the Commander said.** #2163.

//! Merged green. Each step below cannot be asserted yet; the issue that builds
//! it adds its assertion beside the code and deletes its row.
//!
//! | Step of the claim | Carried by |
//! |---|---|
//! | The owner writes to it and gets an answer, a message queues, and one card goes to him | #2166 |
//! | A Drone with nothing handed in, or a red group, wakes it; three wakes on one problem stop it | #2167 |
//! | The owner sees in the thread and on each task's mark that it acted, and answers a card | #2169 |
//! | A repair Drone goes onto the Job's own branch | #2170 |

//! What is asserted today is the fourth signer, the signer of a plan act, and the ceiling on a cap it raises: the record a Commander's act
//! would leave, made by moving the machines as `Actor::Commander`, through
//! `ipc::encode` and back, and the reach: what the door grants a Commander,
//! read off the list the door itself enforces. That a call from a placed
//! connection is refused, answered and recorded as the Commander's is `api`'s
//! `tests::commander_reach`, through the real router, and that a plan act is
//! placed as a person's, Helm's or the Commander's is `api`'s
//! `tests::plan_signing` and `fleet`'s `tests::plan_signed`; the process that hosts a
//! Commander, and so the first real connection to place, comes with #2166.

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use api::{offerable, CommanderReach, COMMANDER_ALLOWED};
use core_model::{Actor, Attempt, JobStatus, PlanAuthor, StepId, Via};
use testkit::FakeWorkProduct;

use bench::commander::{
    a_plan_four_acts_were_made_on, a_plan_recorded_by, a_step_the_commander_stopped,
    status_moves_received, stopped_by_fleet_and_resumed_by_the_commander,
};
use bench::plan::PLAN;
use bench::Bench;

fn a_bench() -> Bench {
    Bench::with(FakeWorkProduct::untouched())
}

#[test]
fn a_commanders_move_on_a_job_reaches_a_bridge_as_the_commanders() {
    let bench = a_bench();
    stopped_by_fleet_and_resumed_by_the_commander(&bench);

    let received = status_moves_received(&bench);

    let signers: Vec<&str> = received.iter().map(|moved| moved.actor.as_wire()).collect();
    assert_eq!(
        signers,
        vec!["human", "fleet", "fleet", "commander"],
        "approved by a person, run and stopped by Fleet, resumed by the Commander"
    );
    let resumed = received.last().expect("the Commander moved the Job");
    assert_eq!(
        (resumed.from.domain(), resumed.to.domain()),
        (JobStatus::Escalated, JobStatus::Running)
    );
}

#[test]
fn a_step_the_commander_stopped_reads_as_the_commanders_beside_fleets_own() {
    let bench = a_bench();
    let mut run = bench.created("raise the bound");
    bench.approved_and_dispatched(&mut run);
    let step = bench.step(0);

    let (moved, bytes) = a_step_the_commander_stopped(&bench, &mut run, &step);

    assert_eq!(moved.actor.as_wire(), "commander");
    assert!(
        bytes.contains(r#""actor":"commander""#),
        "the spelling a client dispatches on is on the wire: {bytes}"
    );
    assert_eq!(
        bench::arc::step_signers(&bench),
        vec![Actor::Fleet, Actor::Commander],
        "the dispatch was Fleet's, the stop the Commander's"
    );
}

#[test]
fn a_plan_the_commander_recorded_is_told_apart_from_a_steps_and_a_persons() {
    let by_commander = a_plan_recorded_by(PlanAuthor::Commander);
    let by_person = a_plan_recorded_by(PlanAuthor::Person);
    let by_step = a_plan_recorded_by(PlanAuthor::Step {
        step_id: StepId::new(PLAN),
        attempt: Attempt::stored(1).expect("one-based"),
    });

    assert_eq!(by_commander.0.recorded_by, ipc::ChangedBy::Commander);
    assert!(
        by_commander.1.contains(r#""by":"commander""#),
        "{}",
        by_commander.1
    );
    assert_eq!(by_person.0.recorded_by, ipc::ChangedBy::Person);
    assert!(matches!(
        by_step.0.recorded_by,
        ipc::ChangedBy::Step { ref step_id, attempt: 1 } if step_id.as_str() == PLAN
    ));
}

#[test]
fn the_commander_is_one_more_signer_and_one_more_door_and_not_a_person() {
    let signers: Vec<&str> = Actor::ALL.iter().map(Actor::as_wire).collect();
    let mut distinct = signers.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), signers.len(), "no two signers share a spelling");
    assert_eq!(Actor::from_wire("commander"), Some(Actor::Commander));

    assert_eq!(Via::from_wire("commander"), Some(Via::Commander));
    assert!(!Via::Commander.is_a_person());
    assert_ne!(Via::Commander, Via::Helm, "Helm's door is not the Commander's");
}

/// Every act the claim says a Commander never takes. All are offered by the
/// door to Helm, so a refusal here is the allowlist's and not an absence.
const NEVER_THE_COMMANDERS: &[&str] = &[
    // Pass a gate.
    "override_verdict",
    "rerun_gate",
    "rerun_failed_checks",
    "set_when_blocked",
    "submit_for_verification",
    "approve_dispatch",
    // Attest, merge, push.
    "attest_complete",
    "merge_pull_request",
    "merge_pull_request_by_number",
    "enable_auto_merge",
    "enable_job_auto_merge",
    "delete_branch",
    // A Check, the Manifest.
    "edit_manifest",
    "write_manifest_proposal",
    "save_manifest_file",
];

#[test]
fn a_commander_is_offered_every_read_and_ten_acts_and_nothing_that_passes_a_gate() {
    let reach = CommanderReach::of_the_allowlist();

    let mut granted: Vec<&str> = offerable()
        .filter(|row| row.kind == "command" && reach.may(row.operation))
        .map(|row| row.operation)
        .collect();
    granted.sort_unstable();
    let mut settled = vec![
        "raise_turn_cap",
        "raise_cost_cap",
        "redirect_drone",
        "redirect_one_drone",
        "restart_step",
        "restart_task",
        "edit_task",
        "add_task",
        "drop_task",
        "move_plan",
    ];
    settled.sort_unstable();
    assert_eq!(granted, settled, "the acts a Commander may take, and no more");
    assert_eq!(COMMANDER_ALLOWED.len(), settled.len());

    for row in offerable().filter(|row| row.kind == "query") {
        assert!(reach.may(row.operation), "every read is its: {}", row.operation);
    }
    for operation in NEVER_THE_COMMANDERS {
        assert!(
            offerable().any(|row| row.operation == *operation),
            "`{operation}` is offered to someone, so refusing it is the allowlist's doing"
        );
        assert!(!reach.may(operation), "`{operation}` is not a Commander's");
    }
}

/// #2165. The ceiling is a multiple of what the workflow would inherit, one
/// for each raiser, and a raise has no field to say which it is.
#[test]
fn a_cap_a_commander_raises_stops_at_three_times_the_workflows_and_names_no_raiser() {
    let inherited = 300;
    let most = |raiser| fleet::raising::Ceiling::on(raiser, inherited).most();
    assert_eq!(most(ipc::RaisedBy::Commander), Some(900), "three times");
    assert_eq!(most(ipc::RaisedBy::Helm), Some(600), "Helm keeps two");
    assert_eq!(most(ipc::RaisedBy::Person), None, "a person has none");
    assert!(fleet::raising::Ceiling::on(ipc::RaisedBy::Commander, inherited).admits(900));
    assert!(!fleet::raising::Ceiling::on(ipc::RaisedBy::Commander, inherited).admits(901));

    // A body that says it is a person's decodes to a raise that says nothing
    // of the kind: Fleet places the raiser, and nothing in the bytes can.
    let claimed = br#"{"turn_cap":900,"raised_by":"person"}"#;
    let raise: ipc::TurnRaise = ipc::decode("a turn cap", claimed).expect("decodes");
    let carried = ipc::encode(&raise).expect("encodes");
    assert!(!carried.contains("raised_by"), "{carried}");
}

/// #2168. The same four acts, made by each of the three callers a plan has: a
/// task each one touched wears its hand on the wire and every event names it,
/// and the task none of them touched wears nobody's.
#[test]
fn a_plan_act_is_signed_by_whoever_made_it_and_the_owners_is_not_the_commanders() {
    let callers = [
        (PlanAuthor::Person, Actor::Human, "person"),
        (PlanAuthor::Helm, Actor::Helm, "helm"),
        (PlanAuthor::Commander, Actor::Commander, "commander"),
    ];
    for (author, actor, spelled) in callers {
        let (plan, bytes, told) = a_plan_four_acts_were_made_on(author, actor);

        let mark = |id: &str| {
            plan.tasks
                .iter()
                .find(|task| task.id == id)
                .map(|task| task.by)
                .unwrap_or_else(|| panic!("the plan holds {id}"))
        };
        let theirs = Some(match spelled {
            "person" => ipc::TaskBy::Person,
            "helm" => ipc::TaskBy::Helm,
            _ => ipc::TaskBy::Commander,
        });
        assert_eq!(mark("T4"), theirs, "{spelled}: the task it added and moved");
        assert_eq!(mark("T1"), theirs, "{spelled}: the task it edited");
        assert_eq!(mark("T2"), theirs, "{spelled}: the task it dropped");
        assert_eq!(mark("T3"), None, "{spelled}: nobody touched it, a step wrote it");
        assert!(
            bytes.contains(&format!(r#""by":{{"who":"{spelled}"}}"#)),
            "the spelling a client dispatches on is on the wire: {bytes}"
        );
        let signers: Vec<&str> = told.iter().map(|moved| moved.actor.as_wire()).collect();
        assert_eq!(signers, vec![actor.as_wire(); 4], "{spelled}: four acts, four events");
    }
}
