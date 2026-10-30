//! What a lifecycle must be before it exists.

use alloc::vec;

use super::*;
use crate::lifecycle::{ForgeFact, MalformedLifecycle};
use crate::watch::ReadingState;

fn declare(
    states: Vec<LifecycleState>,
    transitions: Vec<LifecycleTransition>,
    ended_by: &[&str],
) -> Result<Lifecycle, MalformedLifecycle> {
    Lifecycle::declared(
        LifecycleId::new("release").expect("an id"),
        Title::new("Release").expect("a name"),
        LifecycleScope::Everywhere,
        states,
        transitions,
        ends(ended_by),
    )
}

#[test]
fn an_id_is_one_word_of_lower_case_letters_digits_and_underscores() {
    for written in ["pr", "in_review", "step2"] {
        assert_eq!(StateId::new(written).expect("an id").as_str(), written);
        assert_eq!(LifecycleId::new(written).expect("an id").as_str(), written);
    }
    for written in ["", "In_review", "in review", "pr: approved", "2fast", "in-review", "_a"] {
        assert!(StateId::new(written).is_err(), "{written:?} is not an id");
        assert!(LifecycleId::new(written).is_err(), "{written:?} is not an id");
    }
    assert!(ScopeId::new("my-store").is_ok(), "a scope is an id the set of repositories or the repository chose");
    assert!(ScopeId::new("  ").is_err());
}

#[test]
fn a_transition_or_an_end_naming_a_state_it_lacks_is_refused() {
    let states = || vec![state("a", words(&["A"])), state("b", words(&["B"]))];
    assert_eq!(
        declare(states(), moves(&[("a", "c")]), &["b"]),
        Err(MalformedLifecycle::TransitionNamesNoState { state: id("c") })
    );
    assert_eq!(
        declare(states(), moves(&[("z", "a")]), &["b"]),
        Err(MalformedLifecycle::TransitionNamesNoState { state: id("z") })
    );
    assert_eq!(
        declare(states(), moves(&[("a", "b")]), &["c"]),
        Err(MalformedLifecycle::EndNamesNoState { state: id("c") })
    );
}

#[test]
fn a_lifecycle_with_no_end_or_no_state_is_refused() {
    assert_eq!(declare(vec![], vec![], &[]), Err(MalformedLifecycle::NoStates));
    assert_eq!(declare(vec![state("a", words(&["A"]))], vec![], &[]), Err(MalformedLifecycle::NoEnd));
}

#[test]
fn a_state_declared_twice_is_refused() {
    assert_eq!(
        declare(vec![state("a", words(&["A"])), state("a", words(&["B"]))], vec![], &["a"]),
        Err(MalformedLifecycle::StateTwice { state: id("a") })
    );
}

#[test]
fn one_word_or_one_fact_that_would_put_a_thing_in_two_states_is_refused() {
    assert_eq!(
        declare(vec![state("a", words(&["Done", "Closed"])), state("b", words(&["Resolved", "Done"]))], vec![], &["a"]),
        Err(MalformedLifecycle::WordClaimedTwice { word: text("Done"), first: id("a"), second: id("b") })
    );
    assert_eq!(
        declare(
            vec![state("a", StateRead::Forge(ForgeFact::Merged)), state("b", StateRead::Forge(ForgeFact::Merged))],
            vec![],
            &["a"]
        ),
        Err(MalformedLifecycle::FactClaimedTwice { first: id("a"), second: id("b") })
    );
    // The same word in different sources claims nothing twice: a tracker's `passed` is not a watch's.
    assert!(declare(
        vec![state("a", words(&["running"])), state("b", StateRead::Watch(ReadingState::Running))],
        vec![],
        &["a"]
    )
    .is_ok());
}

#[test]
fn tracker_words_are_trimmed_and_a_blank_or_repeated_one_is_refused() {
    let listed = TrackerWords::of(["  Code Review ", "In Review"]).expect("words");
    assert_eq!(listed.as_slice(), ["Code Review", "In Review"]);
    assert_eq!(TrackerWords::of(["Done", "  "]), Err(MalformedLifecycle::BlankWord));
    assert_eq!(
        TrackerWords::of(["Done", " Done"]),
        Err(MalformedLifecycle::WordTwice { word: text("Done") })
    );
}
