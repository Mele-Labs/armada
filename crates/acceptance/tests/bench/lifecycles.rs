//! Lifecycles and rules' apparatus: the ticket lifecycle two Workspaces (sets of repositories) keep
//! in their own tracker's words, and the helpers a case reads a Job's held instances with.
//!
//! The piece files are held as text and read through `config`, so a case drives the same path a
//! person's file takes. Nothing here opens a file, a tracker or a forge.

use std::path::{Path, PathBuf};

use config::{PieceWritten, Rules, Workspaces};
use core_model::rule::{AutonomyDefaults, Engine, JobFacts, Occurrence, PlannedAct, RanOnce, Rule};
use core_model::{
    HeldInstances, InstanceRef, Lifecycle, LifecycleInstance, ManifestId, Observed, Standing, StateId, StepId, Ulid,
};

/// my-store keeps its tickets in a tracker whose review column has two names.
pub const MY_STORE_TICKETS: &str = "version: 1
lifecycle_id: ticket
name: Ticket
scope: my-store
states:
  - { id: todo, label: To do, read: { tracker: [To Do, Backlog] } }
  - { id: in_progress, label: In progress, read: { tracker: [In Progress] } }
  - { id: in_review, label: In review, read: { tracker: [Code Review, In Review] } }
  - { id: closed, label: Closed, read: { tracker: [Done] } }
transitions:
  - { from: todo, to: in_progress }
  - { from: in_progress, to: in_review }
  - { from: in_review, to: closed }
ends: [closed]
";

/// back-office is another Workspace, and its tracker calls the same column something else.
pub const BACK_OFFICE_TICKETS: &str = "version: 1
lifecycle_id: ticket
name: Ticket
scope: back-office
states:
  - { id: in_progress, label: In progress, read: { tracker: [Doing] } }
  - { id: in_review, label: In review, read: { tracker: [Peer Review] } }
  - { id: closed, label: Closed, read: { tracker: [Shipped] } }
transitions:
  - { from: in_progress, to: in_review }
  - { from: in_review, to: closed }
ends: [closed]
";

pub fn lifecycle(text: &str) -> Lifecycle {
    config::parse_lifecycle(Path::new("lifecycle.yml"), text).expect("a lifecycle that loads")
}

pub fn state(written: &str) -> StateId {
    StateId::new(written).expect("a state id")
}

pub fn in_state(written: &str) -> Standing {
    Standing::In(state(written))
}

pub fn at(lifecycle: &Lifecycle, key: &str) -> InstanceRef {
    InstanceRef::of(lifecycle, key).expect("a key")
}

pub fn taken_on(lifecycle: &Lifecycle, key: &str) -> LifecycleInstance {
    LifecycleInstance::taken_on(at(lifecycle, key))
}

/// What a Job started in my-store freezes: my-store's tickets, and the three Armada carries.
pub fn the_jobs_copy() -> Vec<Lifecycle> {
    let mut copy = vec![lifecycle(MY_STORE_TICKETS)];
    copy.extend([Lifecycle::pull_request(), Lifecycle::deploy(), Lifecycle::job()]);
    copy
}

/// A reader reports what it saw of one held instance, against the lifecycle that instance is of.
pub fn see(held: &mut HeldInstances, lifecycle: &Lifecycle, key: &str, seen: Observed<'_>) {
    held.get_mut(&at(lifecycle, key))
        .expect("the Job holds it")
        .observe(lifecycle, &seen)
        .expect("it is of that lifecycle");
}

/// The keys of some instances, in the order they were given.
pub fn keys(instances: &[&LifecycleInstance]) -> Vec<String> {
    instances.iter().map(|one| one.subject().key().to_string()).collect()
}

/// my-store's rule for the first step: any ticket still in To do moves into In progress.
pub const MOVES_TICKETS: &str = "version: 1
rule_id: plan_moves_tickets
name: Move the tickets when planning starts
scope: my-store
when: { step_starts: plan }
if:
  - any: { held: ticket, in: todo }
do:
  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
";

/// What store-web has: the rules in `files`, kept in Kit, resolved against the Job's lifecycles.
pub fn rules_from(files: &[&str]) -> Rules {
    let workspace = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n";
    let workspaces = Workspaces::of([PieceWritten::in_repository(PathBuf::from("store-web/.armada/workspaces/my-store.yml"), workspace.into())]);
    let copy = the_jobs_copy();
    let lifecycles: Vec<&Lifecycle> = copy.iter().collect();
    let written = files
        .iter()
        .enumerate()
        .map(|(n, text)| PieceWritten::in_kit(PathBuf::from(format!("kit/rules/{n}.yml")), (*text).into()));
    Rules::applying_to(&ManifestId::carried(Ulid::carried("store-web")), &workspaces, &lifecycles, written)
}

/// The Job holds three tickets, read from the tracker: two in To do (one under its other name) and
/// one in progress.
pub fn a_job_holding_three_tickets() -> HeldInstances {
    let copy = the_jobs_copy();
    let tickets = &copy[0];
    let mut held = HeldInstances::new();
    for (key, status) in [("ARM-41", "To Do"), ("ARM-42", "Backlog"), ("ARM-43", "In Progress")] {
        held.take_on(taken_on(tickets, key)).expect("held");
        see(&mut held, tickets, key, Observed::Tracker(status));
    }
    held
}

/// What the rules plan when `step` starts at `when`, for a Job that follows no workflow in particular.
pub fn plan_step_start(
    step: &str,
    when: &str,
    rules: &[Rule],
    defaults: &AutonomyDefaults,
    held: &HeldInstances,
    ran: &mut RanOnce,
) -> Vec<PlannedAct> {
    let started = Occurrence::step_started(StepId::new(step), when);
    Engine::new(rules, defaults).plan(&started, held, &JobFacts::default(), ran)
}
