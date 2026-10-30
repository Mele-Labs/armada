//! What a Job holds: instances, what they are linked to, and what is still going.

use super::*;
use crate::lifecycle::{
    AlreadyHeld, FleetFact, ForgeFact, HeldInstances, InstanceRef, LifecycleInstance, Moved, Observed, Standing,
};
use crate::watch::ReadingState;

fn held(lifecycle: &Lifecycle, key: &str) -> LifecycleInstance {
    LifecycleInstance::taken_on(InstanceRef::of(lifecycle, key).expect("a key"))
}

fn at(lifecycle: &Lifecycle, key: &str) -> InstanceRef {
    InstanceRef::of(lifecycle, key).expect("a key")
}

fn see(all: &mut HeldInstances, lifecycle: &Lifecycle, key: &str, seen: Observed<'_>) {
    let standing = lifecycle.standing_of(&seen);
    all.get_mut(&at(lifecycle, key)).expect("held").observe(standing);
}

#[test]
fn a_second_read_of_the_same_standing_moves_nothing() {
    let pull_request = Lifecycle::pull_request();
    let mut one = held(&pull_request, "#812");
    assert_eq!(one.standing(), &Standing::Unread);

    let approved = pull_request.standing_of(&Observed::Forge(ForgeFact::Approved));
    assert_eq!(one.observe(approved.clone()), Some(Moved { from: Standing::Unread, to: approved.clone() }));
    assert_eq!(one.observe(approved), None, "a rule runs once for an instance entering a state");

    let merged = pull_request.standing_of(&Observed::Forge(ForgeFact::Merged));
    assert_eq!(one.observe(merged.clone()).map(|moved| moved.to), Some(merged));
}

#[test]
fn a_blank_key_names_nothing_and_a_link_to_itself_or_twice_adds_nothing() {
    let ticket = Lifecycle::ticket();
    assert!(InstanceRef::of(&ticket, "  ").is_err());
    let one = held(&ticket, "ARM-41")
        .linked_to(at(&ticket, "ARM-41"))
        .linked_to(at(&ticket, "ARM-42"))
        .linked_to(at(&ticket, " ARM-42 "));
    assert_eq!(one.links(), [at(&ticket, "ARM-42")]);
}

#[test]
fn a_pull_request_knows_its_tickets_and_a_started_job_knows_who_started_it() {
    let (ticket, pull_request, job) = (Lifecycle::ticket(), Lifecycle::pull_request(), Lifecycle::job());
    let mut all = HeldInstances::new();
    all.take_on(held(&ticket, "ARM-41")).expect("held");
    all.take_on(held(&ticket, "ARM-42")).expect("held");
    all.take_on(held(&pull_request, "#812").linked_to(at(&ticket, "ARM-41")).linked_to(at(&ticket, "ARM-42")))
        .expect("held");
    // The Job that started it is not among the instances: it is the Job holding them.
    all.take_on(held(&job, "33b2b07b-child").linked_to(at(&job, "33b2b07b"))).expect("held");

    let keys = |found: Vec<&LifecycleInstance>| -> Vec<String> {
        found.iter().map(|one| String::from(one.subject().key())).collect()
    };
    assert_eq!(keys(all.linked(&at(&pull_request, "#812"))), ["ARM-41", "ARM-42"]);
    assert!(all.linked(&at(&ticket, "ARM-41")).is_empty(), "a ticket came from nothing the Job holds");
    assert!(all.linked(&at(&pull_request, "#999")).is_empty(), "a thing not held links to nothing");
    assert_eq!(keys(all.linking(&at(&ticket, "ARM-42"))), ["#812"]);
    assert_eq!(keys(all.linking(&at(&job, "33b2b07b"))), ["33b2b07b-child"]);
    assert_eq!(all.linked(&at(&job, "33b2b07b-child")).len(), 0, "the starter is linked and not held");
}

#[test]
fn a_thing_is_held_once() {
    let ticket = Lifecycle::ticket();
    let mut all = HeldInstances::new();
    all.take_on(held(&ticket, "ARM-41")).expect("held");
    assert_eq!(all.take_on(held(&ticket, "ARM-41")), Err(AlreadyHeld(at(&ticket, "ARM-41"))));
    assert_eq!(all.all().len(), 1);
}

#[test]
fn what_is_in_flight_is_what_no_end_state_has_taken() {
    let copy = Lifecycle::built_in();
    let (ticket, pull_request, deploy) = (&copy[0], &copy[1], &copy[2]);
    let mut all = HeldInstances::new();
    for (lifecycle, key) in [(ticket, "ARM-41"), (ticket, "ARM-42"), (pull_request, "#812"), (deploy, "prod")] {
        all.take_on(held(lifecycle, key)).expect("held");
    }
    see(&mut all, pull_request, "#812", Observed::Forge(ForgeFact::Merged));
    see(&mut all, deploy, "prod", Observed::Watch(ReadingState::Running));
    see(&mut all, ticket, "ARM-42", Observed::Tracker("Awaiting QA"));

    let flying: Vec<String> = all.in_flight(&copy).iter().map(|one| one.subject().to_string()).collect();
    // ARM-41 was never read, ARM-42 cannot be placed, the deploy is rolling out: none is over.
    assert_eq!(flying, ["ticket ARM-41", "ticket ARM-42", "deploy prod"]);

    see(&mut all, deploy, "prod", Observed::Watch(ReadingState::Passed));
    let flying: Vec<String> = all.in_flight(&copy).iter().map(|one| one.subject().to_string()).collect();
    assert_eq!(flying, ["ticket ARM-41", "ticket ARM-42"]);
}

#[test]
fn an_instance_whose_lifecycle_the_job_does_not_hold_is_in_flight() {
    let mut all = HeldInstances::new();
    let job = Lifecycle::job();
    all.take_on(held(&job, "child")).expect("held");
    see(&mut all, &job, "child", Observed::Fleet(FleetFact::Landed));
    assert!(all.in_flight(&[job]).is_empty());
    assert_eq!(all.in_flight(&[]).len(), 1, "nothing can say it ended");
}
