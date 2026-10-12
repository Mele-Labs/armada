//! Autopilot's claim: **I start Autopilot on a repository with one sentence
//! and a cap, leave it, and come back to Jobs that ran, picks it was unsure of
//! set aside with their reason, and nothing past the caps I set; Pause and
//! Stop do what they say.** #2092, `docs/capabilities/autopilot.md`.
//!
//! Written before the code it names, and asserted in the order the owner meets it.
//!
//! | Proved | Not proved |
//! |---|---|
//! | One sentence is read as settings, and reading starts nothing | That Bridge shows them for the person to confirm. The Cockpit lane is #2092's Bridge half |
//! | Settings nothing could run under are refused at start | That a Job starts from a sure pick, at `queued` and unasked, with the issue as its source. `fleet`'s `tests::autopilot` drives that through a Fleet, since a Job here would need a store |
//! | The picker is offered only issues nothing holds, and a pick it is unsure of is set aside with its reason | That the pass takes the next issue straight after one it set aside. The same tests |
//! | A Job waiting on a person does not count against at once | That a Job ran, landed or waits at review. That is every other milestone's claim, and an Autopilot Job is an ordinary one |
//! | The first cap reached is named, and each stops only the next pick | |
//! | Pause stops picking and Stop withdraws only what never started | That a withdrawn Job is killed. `tests::autopilot` again |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use std::collections::BTreeSet;

use core_model::{JobStatus, Timestamp};
use fleet::autopilot::{
    candidates, picked, read_settings, room, settings_from, Cap, Caps, NotPicked, Pick, Settings,
    Spent, State,
};

use bench::autopilot::{
    asking, listed, now, standing, the_backlog, the_workflows, url_of, LOCAL_NOW, READ_AS,
    SENTENCE,
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
    let asked = model.asked();
    assert_eq!(asked.len(), 1, "one call");
    assert!(asked[0].contains(SENTENCE) && asked[0].contains(LOCAL_NOW));
}

#[test]
fn a_reading_that_names_no_instruction_is_refused_rather_than_guessed() {
    assert!(read_settings("at_once: 2\nuntil: none\nspend: none\njobs: none", &now()).is_err());
    assert!(read_settings("instruction: fix things\nat_once: lots", &now()).is_err());
}

#[test]
fn settings_nothing_could_run_under_are_refused_at_start() {
    let caps = Caps::default();
    assert!(Settings::confirmed("   ", 2, caps.clone(), &now()).is_err(), "no instruction");
    assert!(Settings::confirmed("fix the flakes", 0, caps.clone(), &now()).is_err(), "no room");
    let past = Caps {
        until: Some(Timestamp::from_rfc3339("2026-10-12T02:00:00Z")),
        ..Caps::default()
    };
    assert!(Settings::confirmed("fix the flakes", 2, past, &now()).is_err(), "already over");
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

    let offered: Vec<u64> = candidates(&listed(&tracker), &taken)
        .iter()
        .map(|one| one.number)
        .collect();

    assert_eq!(offered, vec![12, 13], "14 has a pull request that closes it");
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
            assert_eq!(sure.issue().number, 12);
            assert_eq!(sure.workflow().as_str(), "bug");
        }
        other => panic!("a sure pick, not {other:?}"),
    }
    match second.expect("a pick") {
        Pick::Unsure(aside) => {
            assert_eq!(aside.issue.number, 13);
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
    let only_12: Vec<_> = offered.into_iter().filter(|one| one.number == 12).collect();

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

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

#[test]
fn a_job_waiting_on_a_person_does_not_count_against_at_once() {
    let at_work = [
        standing(JobStatus::Running, false, true),
        standing(JobStatus::Queued, true, true),
        standing(JobStatus::Escalated, false, true),
        standing(JobStatus::AwaitingReview, false, true),
        standing(JobStatus::AwaitingRepair, false, true),
        standing(JobStatus::CompletedSuccess, false, true),
    ];
    assert_eq!(room(2, &at_work), 1, "only the running one holds a place");

    let full = [
        standing(JobStatus::Running, false, true),
        standing(JobStatus::Queued, false, false),
    ];
    assert_eq!(room(2, &full), 0, "a Job ready to run holds one too");
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
    assert_eq!(caps.first_reached(&now(), &under), None, "under every cap, it picks");

    let spent = Spent {
        micros: 30_000_000,
        ..under
    };
    assert_eq!(caps.first_reached(&now(), &spent), Some(Cap::Spend));

    let morning = Timestamp::from_rfc3339("2026-10-12T11:00:00.000Z");
    assert_eq!(caps.first_reached(&morning, &spent), Some(Cap::Until));

    let five = Spent { micros: 0, started: 5 };
    assert_eq!(caps.first_reached(&now(), &five), Some(Cap::Jobs));
    assert_eq!(Caps::default().first_reached(&now(), &five), None, "no cap, none reached");
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

    assert!(standing(JobStatus::Queued, false, false).withdrawn_at_stop());
    assert!(!standing(JobStatus::Running, false, true).withdrawn_at_stop(), "a running Job finishes");
    assert!(
        !standing(JobStatus::Queued, true, true).withdrawn_at_stop(),
        "one that ran and is waiting on an answer is started"
    );
    assert!(!standing(JobStatus::AwaitingReview, false, true).withdrawn_at_stop());
}
