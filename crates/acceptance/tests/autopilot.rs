//! Autopilot's claim: **I start Autopilot on a repository with one sentence
//! and a cap, leave it, and come back to Sessions that ran, picks it was unsure of
//! set aside with their reason, and nothing past the caps I set; Pause and
//! Stop do what they say.** #2092, `docs/capabilities/autopilot.md`.
//!
//! Written before the code it names, and asserted in the order the owner meets it.
//!
//! | Proved | Not proved |
//! |---|---|
//! | One sentence is read as settings, and reading starts nothing | That Bridge shows them for the person to confirm. The Cockpit lane is #2092's Bridge half |
//! | Settings nothing could run under are refused at start | That a Session starts from a sure pick, unasked, with the issue as its brief. `fleet`'s `tests::autopilot` drives that through a Fleet, since a Session here would need a store |
//! | The picker is offered only issues nothing holds, and a pick it is unsure of is set aside with its reason | That the pass takes the next issue straight after one it set aside. The same tests |
//! | A Session waiting on a person, or ended, does not count against at once | That a Session ran or landed its work. That is every other milestone's claim, and an Autopilot Session is an ordinary one |
//! | The first cap reached is named and pauses it, each stops only the next pick, and Resume waits until a person raises it | That the pass pauses it and a raised cap picks again. `tests::autopilot` again |
//! | Pause stops picking and Stop withdraws only what never started | That a withdrawn Session is closed. `tests::autopilot` again |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use std::collections::BTreeSet;

use core_model::Timestamp;
use fleet::autopilot::{
    candidates, picked, read_pick, read_settings, room, settings_from, Candidate, Cap, Caps,
    NotPicked, NotResumed, Pick, Settings, Spent, State,
};

use bench::autopilot::{
    asking, listed, now, standing, the_backlog, the_workflows, url_of, LOCAL_NOW, READ_AS, SENTENCE,
};

// ---------------------------------------------------------------------------
// One sentence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn one_sentence_is_read_as_the_settings_a_person_then_confirms() {
    let (asking, model) = asking(&[READ_AS]);

    let proposed = settings_from(SENTENCE, &now(), Some(LOCAL_NOW), &asking)
        .await
        .expect("the sentence is read");

    assert_eq!(
        proposed.instruction,
        "work through the open issues that don't need design decisions"
    );
    assert_eq!(proposed.at_once, 2);
    assert_eq!(
        proposed.until,
        Some(Timestamp::from_rfc3339("2026-10-12T11:00:00.000Z")),
        "7am on the owner's clock, held in UTC"
    );
    assert_eq!(proposed.spend_cap_micros, Some(30_000_000));
    assert_eq!(proposed.job_cap, None);
    assert_eq!(
        proposed.quoted.until.as_deref(),
        Some("until 7am"),
        "the start form quotes the words behind each setting"
    );
    assert_eq!(proposed.quoted.spend.as_deref(), Some("$30 max"));
    assert_eq!(proposed.quoted.jobs, None, "the sentence named no count");
    let asked = model.asked();
    assert_eq!(asked.len(), 1, "one call");
    assert!(asked[0].contains(SENTENCE) && asked[0].contains(LOCAL_NOW));
}

#[test]
fn a_reading_that_names_no_instruction_is_refused_rather_than_guessed() {
    let answer = "at_once: 2\nuntil: none\nspend: none\njobs: none";
    assert!(read_settings(answer, SENTENCE, &now()).is_err());
    assert!(read_settings("instruction: fix things\nat_once: lots", SENTENCE, &now()).is_err());
}

#[test]
fn settings_nothing_could_run_under_are_refused_at_start() {
    let caps = Caps::default();
    assert!(
        Settings::confirmed("   ", 2, caps.clone(), &now()).is_err(),
        "no instruction"
    );
    assert!(
        Settings::confirmed("fix the flakes", 0, caps.clone(), &now()).is_err(),
        "no room"
    );
    let past = Caps {
        until: Some(Timestamp::from_rfc3339("2026-10-12T02:00:00Z")),
        ..Caps::default()
    };
    assert!(
        Settings::confirmed("fix the flakes", 2, past, &now()).is_err(),
        "already over"
    );
    assert!(Settings::confirmed("fix the flakes", 2, caps, &now()).is_ok());
}

// ---------------------------------------------------------------------------
// The picks
// ---------------------------------------------------------------------------

#[test]
fn the_picker_is_offered_only_the_open_issues_nothing_holds() {
    let tracker = the_backlog();
    // 15 has a Job on it; 11 was already picked.
    let taken: BTreeSet<String> = [url_of(15), url_of(11)].into_iter().collect();

    let offered: Vec<String> = candidates(&listed(&tracker), &taken)
        .into_iter()
        .map(|one| one.reference)
        .collect();

    assert_eq!(
        offered,
        vec!["#12", "#13"],
        "14 has a pull request that closes it"
    );
}

#[tokio::test]
async fn a_sure_pick_names_its_issue_and_workflow_and_an_unsure_one_is_set_aside_with_its_reason() {
    let tracker = the_backlog();
    let offered = candidates(&listed(&tracker), &BTreeSet::new());
    let (asking, _) = asking(&[
        "issue: 12\nworkflow: bug\nsure: yes\nbecause: a wrong label, one file",
        "issue: 13\nworkflow: plan-first\nsure: no\nbecause: it asks a design question",
    ]);
    let instruction = "work through the open issues that don't need design decisions";

    let first = picked(instruction, &offered, &the_workflows(), &asking).await;
    let second = picked(instruction, &offered, &the_workflows(), &asking).await;

    match first.expect("a pick") {
        Pick::Sure(sure) => {
            assert_eq!(sure.issue().reference, "#12");
            assert_eq!(sure.workflow().as_str(), "bug");
        }
        other => panic!("a sure pick, not {other:?}"),
    }
    match second.expect("a pick") {
        Pick::Unsure(aside) => {
            assert_eq!(aside.issue.reference, "#13");
            assert_eq!(aside.reason, "it asks a design question");
        }
        other => panic!("set aside, not {other:?}"),
    }
}

#[tokio::test]
async fn a_pick_of_an_issue_not_offered_or_a_workflow_not_held_starts_nothing() {
    let tracker = the_backlog();
    let offered = candidates(&listed(&tracker), &BTreeSet::new());
    let (asking, _) = asking(&[
        "issue: 15\nworkflow: bug\nsure: yes\nbecause: x",
        "issue: 12\nworkflow: hotfix\nsure: yes\nbecause: x",
    ]);
    let only_12: Vec<_> = offered.into_iter().filter(|one| one.reference == "#12").collect();

    let not_offered = picked("x", &only_12, &the_workflows(), &asking).await;
    let not_held = picked("x", &only_12, &the_workflows(), &asking).await;

    assert!(
        matches!(&not_offered, Err(NotPicked::Unreadable { why, .. }) if why.contains("15")),
        "{not_offered:?}"
    );
    assert!(
        matches!(&not_held, Err(NotPicked::Unreadable { why, .. }) if why.contains("hotfix")),
        "{not_held:?}"
    );
}

#[test]
fn a_tracker_that_names_issues_by_key_is_picked_by_key() {
    let issue = |key: &str| Candidate {
        reference: key.to_string(),
        title: "A title".to_string(),
        url: format!("https://acme.atlassian.net/browse/{key}"),
        excerpt: String::new(),
        labels: Vec::new(),
    };
    let offered = vec![issue("ABC-12"), issue("OPS-12")];
    let answer = "issue: ops-12\nworkflow: bug\nsure: yes\nbecause: x";
    match read_pick(answer, &offered, &the_workflows()).expect("a pick") {
        Pick::Sure(sure) => assert_eq!(sure.issue().url, "https://acme.atlassian.net/browse/OPS-12"),
        other => panic!("a sure pick, not {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

#[test]
fn a_session_waiting_on_a_person_or_ended_does_not_count_against_at_once() {
    let at_work = [
        standing(true, false, true),
        standing(true, true, true),
        standing(false, false, true),
    ];
    assert_eq!(room(2, &at_work), 1, "only the live one with nothing owed holds a place");

    let full = [standing(true, false, true), standing(true, false, false)];
    assert_eq!(room(2, &full), 0, "a Session not yet run holds one too");
}

// ---------------------------------------------------------------------------
// The caps
// ---------------------------------------------------------------------------

#[test]
fn the_first_cap_reached_stops_new_picks_and_says_which() {
    let caps = Caps {
        until: Some(Timestamp::from_rfc3339("2026-10-12T11:00:00.000Z")),
        spend_cap_micros: Some(30_000_000),
        job_cap: Some(5),
    };
    let under = Spent {
        micros: 29_999_999,
        started: 4,
    };
    assert_eq!(
        caps.first_reached(&now(), &under),
        None,
        "under every cap, it picks"
    );

    let spent = Spent {
        micros: 30_000_000,
        ..under
    };
    assert_eq!(caps.first_reached(&now(), &spent), Some(Cap::Spend));

    let morning = Timestamp::from_rfc3339("2026-10-12T11:00:00.000Z");
    assert_eq!(caps.first_reached(&morning, &spent), Some(Cap::Until));

    let five = Spent {
        micros: 0,
        started: 5,
    };
    assert_eq!(caps.first_reached(&now(), &five), Some(Cap::Jobs));
    assert_eq!(
        Caps::default().first_reached(&now(), &five),
        None,
        "no cap, none reached"
    );
}

/// Nothing past the caps I set still holds: a cap stops the next pick, and now pauses rather than
/// ends. The owner's *fix the gaps*, 12 Oct 2026.
#[test]
fn a_reached_cap_pauses_and_resume_waits_until_a_person_raises_it() {
    let three = Caps {
        job_cap: Some(3),
        ..Caps::default()
    };
    let spent = Spent {
        micros: 0,
        started: 3,
    };
    assert_eq!(three.first_reached(&now(), &spent), Some(Cap::Jobs));
    let paused = State::Running.paused().expect("the cap pauses it");

    assert_eq!(
        paused.resumed_under(&three, &now(), &spent),
        Err(NotResumed::Capped(Cap::Jobs)),
        "resuming past the cap would pick once and pause again"
    );
    let four = Caps {
        job_cap: Some(4),
        ..Caps::default()
    };
    assert_eq!(
        paused.resumed_under(&four, &now(), &spent),
        Ok(State::Running)
    );
}

// ---------------------------------------------------------------------------
// Pause and Stop
// ---------------------------------------------------------------------------

#[test]
fn pause_stops_picking_and_resume_carries_on() {
    let paused = State::Running.paused().expect("a running one pauses");
    assert_eq!(paused, State::Paused);
    assert!(!paused.picks());
    assert_eq!(paused.resumed().expect("resumes"), State::Running);
    assert!(State::Running.picks());
    assert!(State::Running.resumed().is_err(), "nothing to resume");
}

#[test]
fn stop_ends_it_and_withdraws_only_what_never_started() {
    assert_eq!(State::Paused.stopped().expect("stops"), State::Stopped);
    assert!(State::Stopped.resumed().is_err(), "stopped is the end");
    assert!(State::Stopped.paused().is_err());

    assert!(standing(true, false, false).withdrawn_at_stop());
    assert!(
        !standing(true, false, true).withdrawn_at_stop(),
        "a Session that ran finishes"
    );
    assert!(
        !standing(true, true, true).withdrawn_at_stop(),
        "one that ran and is waiting on an answer is started"
    );
    assert!(
        !standing(false, false, false).withdrawn_at_stop(),
        "an ended one is not live to withdraw"
    );
}
