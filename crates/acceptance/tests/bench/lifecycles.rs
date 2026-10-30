//! Lifecycles and rules' apparatus: the ticket lifecycle two Workspaces (sets of repositories) keep
//! in their own tracker's words, and the helpers a case reads a Job's held instances with.
//!
//! The piece files are held as text and read through `config`, so a case drives the same path a
//! person's file takes. Nothing here opens a file, a tracker or a forge.

use std::path::Path;

use core_model::{
    HeldInstances, InstanceRef, Lifecycle, LifecycleInstance, Observed, Standing, StateId,
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
