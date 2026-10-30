//! How what a reader saw lands in a state, and what does not.

use super::*;
use crate::lifecycle::{FleetFact, ForgeFact, Observed, Standing, StateSource};
use crate::watch::ReadingState;
use crate::workflow::StepId;

#[test]
fn a_tracker_word_is_matched_exactly_and_one_no_state_lists_is_unmapped() {
    let tickets = my_stores_tickets();
    for review in ["Code Review", "In Review", "  In Review "] {
        assert_eq!(tickets.standing_of(&Observed::Tracker(review)), Standing::In(id("in_review")), "{review:?}");
    }
    // Not folded to the nearest: a different case, and a status of its own, are facts.
    for other in ["In review", "Awaiting QA", ""] {
        assert_eq!(
            tickets.standing_of(&Observed::Tracker(other)),
            Standing::Unmapped(text(other.trim())),
            "{other:?}"
        );
    }
}

#[test]
fn the_ticket_armada_carries_lists_no_words_so_every_status_is_unmapped() {
    let carried = Lifecycle::ticket();
    for status in ["To Do", "In Progress", "Done"] {
        assert_eq!(carried.standing_of(&Observed::Tracker(status)), Standing::Unmapped(text(status)));
    }
}

#[test]
fn a_tracker_that_declares_words_gives_the_ticket_them_and_one_that_declares_none_leaves_it_as_carried() {
    let declared = [("todo", "open"), ("in_progress", "in_progress"), ("closed", "closed")];
    let worded = Lifecycle::ticket_reading(&declared);
    for (status, state) in [("open", "todo"), ("in_progress", "in_progress"), ("closed", "closed")] {
        assert_eq!(worded.standing_of(&Observed::Tracker(status)), Standing::In(id(state)), "{status}");
    }
    // What the tracker did not declare stays a fact of its own, and so does every other word.
    for other in ["blocked", "deferred", "Open"] {
        assert_eq!(worded.standing_of(&Observed::Tracker(other)), Standing::Unmapped(text(other)), "{other}");
    }
    assert_eq!(Lifecycle::ticket_reading(&[]), Lifecycle::ticket());
    assert_eq!(Lifecycle::built_in_reading(&[]), Lifecycle::built_in());
}

#[test]
fn a_declared_word_that_is_blank_repeated_or_claimed_by_two_states_is_left_out_and_the_rest_stand() {
    let worded = Lifecycle::ticket_reading(&[
        ("todo", "open"),
        ("todo", " open "),
        ("todo", "  "),
        ("in_progress", "shared"),
        ("closed", "shared"),
        ("closed", "closed"),
        ("no_such_state", "ghost"),
    ]);
    assert_eq!(standing(&worded, "open"), Standing::In(id("todo")), "a word repeated on one state is kept once");
    assert_eq!(standing(&worded, "closed"), Standing::In(id("closed")));
    assert_eq!(standing(&worded, "shared"), Standing::Unmapped(text("shared")), "two states claim it, so neither does");
    assert_eq!(standing(&worded, "ghost"), Standing::Unmapped(text("ghost")));
}

fn standing(lifecycle: &Lifecycle, status: &str) -> Standing {
    lifecycle.standing_of(&Observed::Tracker(status))
}

#[test]
fn a_fact_from_the_forge_a_watch_or_fleet_lands_in_the_state_that_stands_for_it() {
    assert_eq!(
        Lifecycle::pull_request().standing_of(&Observed::Forge(ForgeFact::Approved)),
        Standing::In(id("approved"))
    );
    assert_eq!(
        Lifecycle::deploy().standing_of(&Observed::Watch(ReadingState::Running)),
        Standing::In(id("rolling_out"))
    );
    assert_eq!(Lifecycle::job().standing_of(&Observed::Fleet(FleetFact::Landed)), Standing::In(id("landed")));
}

#[test]
fn a_fact_a_lifecycle_has_no_state_for_is_unmapped_in_the_words_it_was_seen_in() {
    assert_eq!(
        Lifecycle::deploy().standing_of(&Observed::Watch(ReadingState::Waiting)),
        Standing::Unmapped(text("waiting"))
    );
    assert_eq!(
        Lifecycle::job().standing_of(&Observed::Fleet(FleetFact::StepEntered(StepId::new("plan")))),
        Standing::Unmapped(text("step_entered:plan"))
    );
    // A forge fact is never read against a state that a tracker's words stand for.
    assert_eq!(
        my_stores_tickets().standing_of(&Observed::Forge(ForgeFact::Merged)),
        Standing::Unmapped(text("merged"))
    );
}

#[test]
fn only_a_state_that_ends_it_ends_an_instance() {
    let pull_request = Lifecycle::pull_request();
    assert!(pull_request.is_over(&Standing::In(id("merged"))));
    assert!(pull_request.is_over(&Standing::In(id("closed"))));
    assert!(!pull_request.is_over(&Standing::In(id("approved"))));
    assert!(!pull_request.is_over(&Standing::Unread));
    assert!(!pull_request.is_over(&Standing::Unmapped(text("parked"))), "a thing nobody can place is not let go");
}

#[test]
fn a_move_off_the_listed_transitions_is_a_standing_all_the_same() {
    let pull_request = Lifecycle::pull_request();
    assert!(pull_request.allows(&id("in_review"), &id("approved")));
    assert!(!pull_request.allows(&id("approved"), &id("draft")));
    // Straight to Merged was never listed, and a pull request that did is merged.
    assert!(!pull_request.allows(&id("ready"), &id("merged")));
    assert_eq!(pull_request.standing_of(&Observed::Forge(ForgeFact::Merged)), Standing::In(id("merged")));
}

#[test]
fn each_built_in_is_read_from_its_own_source() {
    let built = Lifecycle::built_in();
    let sources: Vec<(&str, StateSource)> =
        built.iter().map(|one| (one.id().as_str(), one.states()[0].read().source())).collect();
    assert_eq!(
        sources,
        [
            ("ticket", StateSource::Tracker),
            ("pr", StateSource::Forge),
            ("deploy", StateSource::Watch),
            ("job", StateSource::Fleet),
        ]
    );
    for one in &built {
        let kinds: Vec<StateSource> = one.states().iter().map(|state| state.read().source()).collect();
        assert!(kinds.iter().all(|kind| *kind == kinds[0]), "{} is read from one source", one.id());
        assert!(!one.ends().is_empty(), "{} ends", one.id());
    }
}

#[test]
fn a_standing_survives_being_written_down_and_a_spelling_nobody_wrote_is_refused() {
    for standing in [
        Standing::Unread,
        Standing::In(id("in_review")),
        Standing::Unmapped(text("Awaiting QA")),
        Standing::Unmapped(text("in:review")),
    ] {
        assert_eq!(Standing::from_wire(&standing.as_wire()), Some(standing.clone()), "{standing:?}");
    }
    for refused in ["", "in:", "in:Two Words", "somewhere:else"] {
        assert_eq!(Standing::from_wire(refused), None, "{refused:?}");
    }
}

#[test]
fn what_was_seen_is_read_back_from_its_spelling_through_the_source_that_said_it() {
    let seen = [
        Observed::Tracker("Code Review"),
        Observed::Forge(ForgeFact::ReviewersOn),
        Observed::Watch(ReadingState::Running),
        Observed::Fleet(FleetFact::StepEntered(StepId::new("plan"))),
    ];
    for one in seen {
        let spelled = one.spelled();
        assert_eq!(Observed::read(one.source(), &spelled), Some(one.clone()), "{one:?}");
    }
    // A word the source has no name for is not made into one.
    assert_eq!(Observed::read(StateSource::Forge, "queued"), None);
    assert_eq!(Observed::read(StateSource::Watch, "healthy"), None);
}
