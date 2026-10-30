//! An act in the owner's words: the one short line he reads in Needs you, and a draft's template
//! filled for the thing it is left on.

use core_model::rule::{Act, Among, Target, TrackerWrite, Words};
use core_model::{InstanceRef, Lifecycle, LifecycleId, StateId};

use crate::lifecycles::saying::{failed, filled, said, thing, titled};

fn words(text: &str) -> Words {
    Words::new(text).expect("words")
}

fn instance(lifecycle: &str, key: &str) -> InstanceRef {
    let lifecycles = Lifecycle::built_in();
    let one = lifecycles.iter().find(|one| one.id().as_str() == lifecycle).expect("a built-in lifecycle");
    InstanceRef::of(one, key).expect("a key")
}

#[test]
fn a_move_is_said_by_the_label_its_lifecycle_gives_the_state_and_a_pull_request_by_its_number() {
    let lifecycles = Lifecycle::built_in();
    let moved = Act::Tracker {
        write: TrackerWrite::Move { to: StateId::new("in_progress").expect("an id") },
        target: Target::held(LifecycleId::new("ticket").expect("an id")),
    };
    assert_eq!(said(&moved, Some(&instance("ticket", "ARM-41")), &lifecycles), "Move ARM-41 to In progress");
    let draft = Act::Draft { template: words("Ready for {pr}"), target: Target { among: Among::Entered, in_state: None } };
    assert_eq!(said(&draft, Some(&instance("pr", "812")), &lifecycles), "Draft a message on #812: Ready for #812");
}

#[test]
fn the_acts_done_to_nothing_are_said_without_a_thing_and_a_long_prompt_is_cut() {
    let lifecycles = Lifecycle::built_in();
    let watch = Act::StartWatch { recipe: words("qa-deploy"), values: Default::default(), deploy: Some(words("web")) };
    assert_eq!(said(&watch, None, &lifecycles), "Start the web deploy watch");
    let plain = Act::StartWatch { recipe: words("qa-deploy"), values: Default::default(), deploy: None };
    assert_eq!(said(&plain, None, &lifecycles), "Start the qa-deploy watch");
    assert_eq!(said(&Act::Wake, None, &lifecycles), "Wake the agent");
    let long = Act::StartJob { workflow: None, prompt: words(&format!("{}\nsecond line", "a".repeat(80))) };
    assert_eq!(said(&long, None, &lifecycles), format!("Start a Job: {}...", "a".repeat(60)));
    assert_eq!(titled(&words("Look at the deploy\nand the logs")), "Look at the deploy");
}

#[test]
fn a_failure_is_said_in_the_acts_own_words_and_a_template_names_only_what_it_is_left_on() {
    assert_eq!(failed("Start the web deploy watch", "  no such recipe \n"), "Start the web deploy watch failed: no such recipe");
    assert_eq!(filled("Ready for {pr}, see {ticket}", Some(("pr", "812"))), "Ready for #812, see {ticket}");
    assert_eq!(filled("Ready for {pr}", None), "Ready for {pr}");
    assert_eq!((thing("pr", "812"), thing("ticket", "ARM-41")), ("#812".to_string(), "ARM-41".to_string()));
}
