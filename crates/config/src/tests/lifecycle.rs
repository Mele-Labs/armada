//! A lifecycle piece file: what it reads to, and what it is refused for.

use std::path::Path;

use core_model::{
    FleetFact, ForgeFact, Lifecycle, LifecycleScope, MalformedLifecycle, Observed, ReadingState, ScopeId, StateId,
    StateRead, Standing, StepId,
};

use crate::{parse_lifecycle, LifecycleError};

const TICKET: &str = "version: 1
lifecycle_id: ticket
name: Ticket
states:
  - { id: todo, label: To do, read: { tracker: [] } }
  - { id: in_progress, label: In progress, read: { tracker: [] } }
  - { id: in_review, label: In review, read: { tracker: [] } }
  - { id: closed, label: Closed, read: { tracker: [closed] } }
transitions:
  - { from: todo, to: in_progress }
  - { from: in_progress, to: in_review }
  - { from: in_review, to: closed }
ends: [closed]
";

const PULL_REQUEST: &str = "version: 1
lifecycle_id: pr
name: Pull request
states:
  - { id: draft, label: Draft, read: { forge: draft } }
  - { id: ready, label: Ready, read: { forge: ready } }
  - { id: reviewers_on, label: Reviewers on, read: { forge: reviewers_on } }
  - { id: in_review, label: In review, read: { forge: in_review } }
  - { id: approved, label: Approved, read: { forge: approved } }
  - { id: merged, label: Merged, read: { forge: merged } }
  - { id: closed, label: Closed, read: { forge: closed } }
transitions:
  - { from: draft, to: ready }
  - { from: ready, to: reviewers_on }
  - { from: reviewers_on, to: in_review }
  - { from: in_review, to: approved }
  - { from: approved, to: merged }
ends: [merged, closed]
";

const DEPLOY: &str = "version: 1
lifecycle_id: deploy
name: Deploy
states:
  - { id: rolling_out, label: Rolling out, read: { watch: running } }
  - { id: healthy, label: Healthy, read: { watch: passed } }
  - { id: failed, label: Failed, read: { watch: failed } }
transitions:
  - { from: rolling_out, to: healthy }
  - { from: rolling_out, to: failed }
ends: [healthy, failed]
";

const JOB: &str = "version: 1
lifecycle_id: job
name: Job
states:
  - { id: running, label: Running, read: { fleet: started } }
  - { id: landed, label: Landed, read: { fleet: landed } }
  - { id: ended, label: Ended, read: { fleet: ended } }
transitions:
  - { from: running, to: landed }
  - { from: running, to: ended }
ends: [landed, ended]
";

const MY_STORE_TICKETS: &str = "version: 1
lifecycle_id: ticket
name: Ticket
scope: my-store
states:
  - { id: in_progress, label: In progress, read: { tracker: [In Progress] } }
  - { id: in_review, label: In review, read: { tracker: [Code Review, In Review] } }
  - { id: closed, label: Closed, read: { tracker: [Done] } }
transitions:
  - { from: in_progress, to: in_review }
  - { from: in_review, to: closed }
ends: [closed]
";

fn parsed(text: &str) -> Lifecycle {
    parse_lifecycle(Path::new("lifecycle.yml"), text).expect("a lifecycle that loads")
}

fn said(text: &str) -> String {
    parse_lifecycle(Path::new("lifecycle.yml"), text).expect_err("a lifecycle that does not load").to_string()
}

#[test]
fn the_four_armada_carries_read_back_from_files_that_spell_them_out() {
    assert_eq!(parsed(TICKET), Lifecycle::ticket());
    assert_eq!(parsed(PULL_REQUEST), Lifecycle::pull_request());
    assert_eq!(parsed(DEPLOY), Lifecycle::deploy());
    assert_eq!(parsed(JOB), Lifecycle::job());
}

#[test]
fn a_scoped_lifecycle_lists_the_trackers_words_for_its_states() {
    let tickets = parsed(MY_STORE_TICKETS);
    assert_eq!(tickets.scope(), &LifecycleScope::Within(ScopeId::new("my-store").expect("a scope")));
    assert_eq!(tickets.standing_of(&Observed::Tracker("Code Review")), Standing::In(StateId::new("in_review").unwrap()));
    assert_eq!(tickets.standing_of(&Observed::Tracker("Peer Review")), Standing::Unmapped("Peer Review".into()));
}

/// A lifecycle of one state, so the fact under test is the only one it reads.
fn read_as(read: &str) -> StateRead {
    let text = format!("version: 1\nlifecycle_id: one\nname: One\nstates:\n  - {{ id: only, label: Only, read: {{ {read} }} }}\nends: [only]\n");
    parsed(&text).states()[0].read().clone()
}

#[test]
fn every_fact_a_source_has_is_a_word_a_file_can_read() {
    for fact in ForgeFact::ALL {
        assert_eq!(read_as(&format!("forge: {}", fact.as_wire())), StateRead::Forge(*fact));
    }
    for state in ReadingState::ALL {
        assert_eq!(read_as(&format!("watch: {}", state.as_wire())), StateRead::Watch(*state));
    }
    for fact in [
        FleetFact::Started,
        FleetFact::PullRequestOpened,
        FleetFact::Landed,
        FleetFact::Ended,
        FleetFact::StepEntered(StepId::new("plan")),
    ] {
        assert_eq!(read_as(&format!("fleet: {}", fact.as_wire())), StateRead::Fleet(fact));
    }
}

#[test]
fn a_file_is_refused_for_every_key_that_is_wrong_and_names_each() {
    let said = said(
        "version: 2
lifecycle_id: 'pr: approved'
name: Pull request
colour: green
states:
  - { id: draft, label: Draft, read: { forge: shipped } }
  - { id: ready, label: Ready, read: { forge: ready, tracker: [Done] } }
  - { id: merged, label: Merged, read: {} }
  - { id: closed, label: Closed, read: { tracker: [Done, '  '] } }
  - { id: parked, label: Parked, read: { tracker: [Parked, Parked] } }
ends: ['merged: yes']
",
    );
    for expected in [
        "`version` is `2`",
        "`lifecycle_id` wanted an id",
        "`colour` is not a key Armada reads",
        "`states[0].read.forge` is `shipped`",
        "`states[1].read` wanted one of `tracker`, `forge`, `watch` or `fleet` and holds more than one",
        "`states[2].read` wanted one of `tracker`, `forge`, `watch` or `fleet` and holds none of them",
        "`states[3].read.tracker[1]` is empty",
        "`states[4].read.tracker` wanted a list of different words and holds a word listed twice",
        "`ends[0]` wanted an id",
    ] {
        assert!(said.contains(expected), "{expected:?} is not in:\n{said}");
    }
}

#[test]
fn a_file_missing_its_keys_says_which() {
    let said = said("version: 1\nname: Thing\n");
    for expected in ["`lifecycle_id` is required", "`states` is required", "`ends` is required"] {
        assert!(said.contains(expected), "{expected:?} is not in:\n{said}");
    }
    assert!(self::said("- a\n- b\n").contains("wanted a map"));
    assert!(self::said("states: [").contains("is not YAML"));
}

#[test]
fn keys_that_are_right_but_make_a_lifecycle_that_is_not_come_back_as_the_lifecycles_own_refusal() {
    let broken = |states: &str, rest: &str| {
        let text = format!("version: 1\nlifecycle_id: release\nname: Release\nstates:\n{states}{rest}");
        match parse_lifecycle(Path::new("release.yml"), &text) {
            Err(LifecycleError::Malformed { why, .. }) => why,
            other => panic!("wanted the lifecycle's own refusal, got {other:?}"),
        }
    };
    let id = |written: &str| StateId::new(written).expect("an id");
    let two = "  - { id: a, label: A, read: { tracker: [Done] } }\n  - { id: b, label: B, read: { tracker: [Shipped] } }\n";

    assert_eq!(
        broken(two, "transitions:\n  - { from: a, to: c }\nends: [b]\n"),
        MalformedLifecycle::TransitionNamesNoState { state: id("c") }
    );
    assert_eq!(broken(two, "ends: [c]\n"), MalformedLifecycle::EndNamesNoState { state: id("c") });
    assert_eq!(
        broken("  - { id: a, label: A, read: { tracker: [Done] } }\n  - { id: a, label: A again, read: { tracker: [] } }\n", "ends: [a]\n"),
        MalformedLifecycle::StateTwice { state: id("a") }
    );
    assert_eq!(
        broken("  - { id: a, label: A, read: { tracker: [Done] } }\n  - { id: b, label: B, read: { tracker: [Done] } }\n", "ends: [a]\n"),
        MalformedLifecycle::WordClaimedTwice { word: "Done".into(), first: id("a"), second: id("b") }
    );
    assert!(parse_lifecycle(Path::new("release.yml"), &format!("version: 1\nlifecycle_id: release\nname: R\nstates:\n{two}ends: [a]\n")).is_ok());
}
