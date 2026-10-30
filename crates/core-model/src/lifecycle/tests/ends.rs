//! How a step's `ends_on` writes a lifecycle state, and that the five words are left as they were.

use alloc::vec::Vec;

use crate::lifecycle::{LifecycleEnd, NotALifecycleEnd, Waits};
use crate::workflow::StepEnd;

#[test]
fn a_lifecycle_end_reads_back_as_written_and_any_is_the_only_other_count() {
    let every = LifecycleEnd::from_wire("pr: approved").expect("an end");
    assert_eq!((every.lifecycle().as_str(), every.state().as_str(), every.waits()), ("pr", "approved", Waits::Every));
    let any = LifecycleEnd::from_wire("any ticket: closed").expect("an end");
    assert_eq!((any.lifecycle().as_str(), any.waits()), ("ticket", Waits::Any));
    for written in ["pr: approved", "any ticket: closed", "job: landed"] {
        assert_eq!(LifecycleEnd::from_wire(written).expect("an end").as_wire(), written);
    }
}

#[test]
fn a_word_that_is_not_the_pair_is_refused_and_not_taken_for_a_lifecycle() {
    assert_eq!(LifecycleEnd::from_wire("approved"), Err(NotALifecycleEnd::NoState));
    assert!(matches!(LifecycleEnd::from_wire("Pr: approved"), Err(NotALifecycleEnd::Lifecycle(_))));
    assert!(matches!(LifecycleEnd::from_wire("pr: in review"), Err(NotALifecycleEnd::State(_))));
}

#[test]
fn the_five_words_keep_their_spelling_and_the_new_variant_is_after_them() {
    let words: Vec<_> = StepEnd::ALL.iter().map(|end| end.as_wire().into_owned()).collect();
    assert_eq!(words, ["replied", "walk_approved", "sketch_approved", "draft_approved", "idea_picked"]);
    for end in StepEnd::ALL {
        assert_eq!(StepEnd::from_wire(&end.as_wire()).as_ref(), Some(end));
    }
    let lifecycle = StepEnd::from_wire("any pr: approved").expect("a lifecycle end");
    assert_eq!(lifecycle.as_wire(), "any pr: approved");
    assert!(StepEnd::IdeaPicked < lifecycle, "ordered after every word, so a list written before sorts as it did");
}
