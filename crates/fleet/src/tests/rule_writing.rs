//! What a saved rule writes to the tracker and the forge: made at once under Acts, asked about first
//! under Asks, and recorded as not carried, with why, where the repository cannot make it.
//! **Nothing here reaches a real tracker or forge**: both are fakes that record what they were asked.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::TicketState;
use api::Next;
use ipc::{Event, SessionId};
use store::rule_act_standing as standing;
use testkit::{Delivered, FakeTracker, FakeTrackers, IssueWritten};

use super::rule_acting::{approved, notices_in};
use super::rule_asking::{items, pressed};
use super::session_host::{rig_fitted, Rig};

/// The tracker's own words for the ticket states, which a move writes the first of.
const TICKETS: &str = "version: 1\nlifecycle_id: ticket\nname: Ticket\nstates:\n  \
    - { id: todo, label: To do, read: { tracker: [To Do] } }\n  \
    - { id: in_progress, label: In progress, read: { tracker: [In Progress, Doing] } }\n  \
    - { id: closed, label: Closed, read: { tracker: [Done, Closed] } }\n\
    transitions:\n  - { from: todo, to: in_progress }\n  - { from: in_progress, to: closed }\nends: [closed]\n";

const WRITES_AT_ONCE: &str = "version: 1
rule_id: write_on_approval
name: Write on approval
when: { enters: { lifecycle: pr, state: approved } }
autonomy: acts
do:
  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
  - tracker_write: { assign: ana, on: { held: ticket, in: todo } }
  - forge_write: { request_reviewers: [ana, bob], on: { entered: true } }
  - forge_write: { mark_ready: true, on: { entered: true } }
  - forge_write: { auto_merge: true, on: { entered: true } }
  - forge_write: { merge: true, on: { entered: true } }
";

const MOVES_ANY_TICKET: &str = "version: 1
rule_id: move_any_ticket
name: Move any ticket
when: { enters: { lifecycle: pr, state: approved } }
do:
  - tracker_write: { move: in_progress, on: { held: ticket } }
";

const ASKS_FIRST: &str = "version: 1
rule_id: write_when_told
name: Write when told
when: { enters: { lifecycle: pr, state: approved } }
do:
  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
  - forge_write: { request_reviewers: [ana], on: { entered: true } }
";

fn kit_file(rig: &Rig, folder: &str, name: &str, text: &str) {
    let folder = Path::new(&rig.fleet.host().kit_home).join(folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(folder.join(name), text).expect("a kit file");
}

/// A Fleet whose tracker is `tracker`, with the ticket words and the rule saved before the Job starts.
fn rig_with(tracker: &Arc<FakeTracker>, rule: &str, lifecycle: bool) -> Rig {
    let held = Arc::clone(tracker);
    let rig = rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(held)));
    if lifecycle {
        kit_file(&rig, "lifecycles", "tickets.yml", TICKETS);
    }
    kit_file(&rig, "rules", "write.yml", rule);
    rig
}

/// The Job holds ARM-41 in To do, and its pull request is approved.
async fn approving_with_a_ticket(rig: &Rig, job: &SessionId) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "ticket", "ARM-41", &[], &at).expect("taken on");
    assert!(rig.fleet.lifecycle_seen(job.as_str(), "ticket", "ARM-41", core_model::Observed::Tracker("To Do")).await.is_some());
    approved(rig, job).await;
}

async fn kept(rig: &Rig, job: &SessionId) -> Vec<(String, String, u32, String)> {
    let rows = rig.fleet.store().lock().await.rule_acts_of(job.as_str()).expect("reads");
    rows.into_iter().map(|one| (one.text, one.state, one.attempts, one.said)).collect()
}

fn states(rows: &[(String, String, u32, String)]) -> Vec<&str> {
    rows.iter().map(|one| one.1.as_str()).collect()
}

#[tokio::test]
async fn under_acts_every_write_is_made_at_once_in_the_tracker_and_forge_and_asks_nothing() {
    let tracker = Arc::new(FakeTracker::empty());
    let rig = rig_with(&tracker, WRITES_AT_ONCE, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;
    assert_eq!(states(&kept(&rig, &job).await), [standing::PLANNED; 6], "planned for doing, and none asked");
    assert!(items(&rig, &job).await.is_empty(), "no item to press");

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let done = kept(&rig, &job).await;
    assert_eq!(states(&done), [standing::DONE; 6]);
    assert_eq!(done.iter().map(|one| one.2).collect::<Vec<_>>(), [1; 6]);
    assert_eq!(
        done.iter().map(|one| one.3.as_str()).collect::<Vec<_>>(),
        [
            "Moved ARM-41 to In Progress",
            "Assigned ARM-41 to ana",
            "Asked ana, bob to review PR #812",
            "Marked PR #812 ready",
            "Turned on auto-merge for PR #812",
            "The forge took the merge of PR #812",
        ]
    );

    // The first word the state lists, and not the label the lifecycle shows the owner.
    assert_eq!(
        tracker.writes(),
        [
            IssueWritten::Moved { reference: "ARM-41".into(), status: "In Progress".into() },
            IssueWritten::Assigned { reference: "ARM-41".into(), to: "ana".into() },
        ]
    );
    let forge = rig.fleet.vcs().delivered();
    assert!(forge.contains(&Delivered::RequestedReviewers { pull_request: "812".into(), reviewers: vec!["ana".into(), "bob".into()] }), "{forge:?}");
    assert!(forge.contains(&Delivered::MarkedReady { pull_request: "812".into() }));
    assert!(forge.contains(&Delivered::AutoMerge { pull_request: "812".into() }));
    assert!(forge.contains(&Delivered::Merged { pull_request: "812".into() }), "the merge is the forge's own, which a queue takes");
    assert!(!forge.iter().any(|one| matches!(one, Delivered::MergedByPush { .. })), "the base is never pushed");
    assert!(notices_in(&rig, &job).await.iter().all(|one| one.0.as_deref() != Some("lifecycle_rule")), "the agent was told nothing");

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(tracker.writes().len(), 2, "a done write is not made twice");
}

#[tokio::test]
async fn under_asks_each_write_waits_as_do_it_or_skip_in_the_owners_words_and_is_made_only_on_do_it() {
    let tracker = Arc::new(FakeTracker::empty());
    let rig = rig_with(&tracker, ASKS_FIRST, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;

    let asked = items(&rig, &job).await;
    assert_eq!(asked.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), ["Move ARM-41 to In progress", "Request review on PR #812 from ana"]);
    assert!(asked.iter().all(|one| one.options.iter().map(|option| option.label.as_str()).eq(["Do it", "Skip"])));
    assert!(tracker.writes().is_empty(), "nothing is written before he says");
    assert!(rig.fleet.vcs().delivered().iter().all(|one| !matches!(one, Delivered::RequestedReviewers { .. })));

    pressed(&rig, &job, &asked[0].id, 0).await.expect("answered");
    pressed(&rig, &job, &asked[1].id, 1).await.expect("skipped");
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(states(&kept(&rig, &job).await), [standing::DONE, standing::SKIPPED]);
    assert_eq!(tracker.writes(), [IssueWritten::Moved { reference: "ARM-41".into(), status: "In Progress".into() }]);
    assert!(rig.fleet.vcs().delivered().iter().all(|one| !matches!(one, Delivered::RequestedReviewers { .. })), "a skipped write is not made");
    assert!(items(&rig, &job).await.is_empty());
}

#[tokio::test]
async fn a_write_the_tracker_cannot_make_is_kept_as_not_carried_with_why_and_raises_nothing() {
    let tracker = Arc::new(FakeTracker::empty());
    tracker.cannot("move", "this tracker cannot move an issue");
    let rig = rig_with(&tracker, ASKS_FIRST, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;

    let rows = kept(&rig, &job).await;
    assert_eq!(rows[0], ("Move ARM-41 to In progress".to_string(), standing::NOT_CARRIED.to_string(), 0, "this tracker cannot move an issue".to_string()));
    assert_eq!(rows[1].1, standing::ASKED, "the forge write beside it is still asked");
    assert_eq!(items(&rig, &job).await.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), ["Request review on PR #812 from ana"], "only what can be done is asked");

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert!(tracker.writes().is_empty());
    assert_eq!(kept(&rig, &job).await[0].2, 0, "never tried");
}

#[tokio::test]
async fn a_move_to_a_state_the_ticket_lifecycle_gives_no_tracker_word_is_not_carried_and_says_so() {
    let tracker = Arc::new(FakeTracker::empty());
    // This tracker declares no words, so Armada's own ticket lifecycle lists none for In progress: the Workspace says what its tracker calls it.
    // No tracker word reads a ticket into To do either, so the rule names any ticket the Job holds.
    let rig = rig_with(&tracker, MOVES_ANY_TICKET, false);
    let job = rig.start().await;
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "ticket", "ARM-41", &[], &at).expect("taken on");
    approved(&rig, &job).await;

    let rows = kept(&rig, &job).await;
    let moved: Vec<_> = rows.iter().filter(|one| one.0 == "Move ARM-41 to In progress").collect();
    assert_eq!(moved.len(), 1, "{rows:?}");
    assert_eq!((moved[0].1.as_str(), moved[0].3.as_str()), (standing::NOT_CARRIED, "The ticket lifecycle names no tracker word for In progress"));
    assert!(items(&rig, &job).await.iter().all(|one| one.text != "Move ARM-41 to In progress"), "and nothing is asked");
}

#[tokio::test]
async fn a_move_writes_the_word_the_trackers_own_set_gives_where_no_lifecycle_lists_one() {
    let tracker = Arc::new(FakeTracker::empty());
    let held = Arc::clone(&tracker);
    let fixed: &'static [(TicketState, &str)] = &[(TicketState::InProgress, "in_progress")];
    let rig = rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(held).ticket_words(fixed)));
    kit_file(&rig, "rules", "write.yml", &MOVES_ANY_TICKET.replace("do:", "autonomy: acts\ndo:"));
    let job = rig.start().await;
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "ticket", "ARM-41", &[], &at).expect("taken on");
    approved(&rig, &job).await;
    assert_eq!(states(&kept(&rig, &job).await), [standing::PLANNED]);

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let done = kept(&rig, &job).await;
    assert_eq!((done[0].1.as_str(), done[0].3.as_str()), (standing::DONE, "Moved ARM-41 to in_progress"));
    assert_eq!(tracker.writes(), [IssueWritten::Moved { reference: "ARM-41".into(), status: "in_progress".into() }]);
}

#[tokio::test]
async fn a_write_that_can_no_longer_be_made_when_he_says_do_it_is_settled_as_not_carried_and_not_tried() {
    let tracker = Arc::new(FakeTracker::empty());
    let rig = rig_with(&tracker, ASKS_FIRST, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;
    let asked = items(&rig, &job).await;
    tracker.cannot("move", "this tracker cannot move an issue");

    pressed(&rig, &job, &asked[0].id, 0).await.expect("answered");
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let rows = kept(&rig, &job).await;
    assert_eq!((rows[0].1.as_str(), rows[0].2, rows[0].3.as_str()), (standing::NOT_CARRIED, 0, "this tracker cannot move an issue"));
    assert!(tracker.writes().is_empty());
}

#[tokio::test]
async fn a_write_the_tracker_refuses_is_tried_up_to_the_rules_cap_and_then_asks_with_the_trackers_own_words() {
    let tracker = Arc::new(FakeTracker::empty());
    tracker.will_not_write("ARM-41 is locked");
    let rule = "version: 1\nrule_id: move_it\nname: Move it\nwhen: { enters: { lifecycle: pr, state: approved } }\nautonomy: acts\ndo:\n  - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }\non_failure: { retries: 1, then: ask_owner }\n";
    let rig = rig_with(&tracker, rule, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let rows = kept(&rig, &job).await;
    assert_eq!((rows[0].1.as_str(), rows[0].2), (standing::STUCK, 2), "the try and its one retry");
    assert!(rows[0].3.starts_with("Move ARM-41 to In progress failed: "), "{}", rows[0].3);
    assert!(rows[0].3.contains("ARM-41 is locked"), "{}", rows[0].3);
    assert_eq!(items(&rig, &job).await.len(), 1, "and it asks");
}

#[tokio::test]
async fn a_forge_write_the_forge_refuses_fails_in_the_forges_words() {
    let tracker = Arc::new(FakeTracker::empty());
    let rule = "version: 1\nrule_id: ready_it\nname: Ready it\nwhen: { enters: { lifecycle: pr, state: approved } }\nautonomy: acts\ndo:\n  - forge_write: { mark_ready: true, on: { entered: true } }\non_failure: { retries: 0, then: ask_owner }\n";
    let rig = rig_with(&tracker, rule, true);
    rig.fleet.vcs().ready_refuses("the pull request is closed");
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let rows = kept(&rig, &job).await;
    assert_eq!((rows[0].1.as_str(), rows[0].2), (standing::STUCK, 1));
    assert!(rows[0].3.contains("the pull request is closed"), "{}", rows[0].3);
}

/// How many times the stream said what the Job holds, in the next moments.
async fn lifecycle_events(watching: &mut api::Subscription) -> usize {
    let mut found = 0;
    while let Ok(next) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        match next {
            Some(Next::Send(delivered)) => found += usize::from(matches!(delivered.event, Event::JobLifecyclesChanged(_))),
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    found
}

#[tokio::test]
async fn the_workflow_view_is_told_the_moment_a_rule_act_is_answered_done_or_found_not_carried() {
    let tracker = Arc::new(FakeTracker::empty());
    let rig = rig_with(&tracker, ASKS_FIRST, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;
    let asked = items(&rig, &job).await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &job, &asked[1].id, 1).await.expect("skipped");
    assert!(lifecycle_events(&mut watching).await >= 1, "a Skip tells it");

    pressed(&rig, &job, &asked[0].id, 0).await.expect("answered");
    lifecycle_events(&mut watching).await;
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert!(lifecycle_events(&mut watching).await >= 1, "a write made tells it");

    // A tracker that stops being able to make a write before he says Do it.
    let tracker = Arc::new(FakeTracker::empty());
    let rig = rig_with(&tracker, ASKS_FIRST, true);
    let job = rig.start().await;
    approving_with_a_ticket(&rig, &job).await;
    let asked = items(&rig, &job).await;
    tracker.cannot("move", "this tracker cannot move an issue");
    pressed(&rig, &job, &asked[0].id, 0).await.expect("answered");
    let mut watching = rig.fleet.events().subscribe();
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert!(lifecycle_events(&mut watching).await >= 1, "an act found not carried tells it");
}

/// Every file under `src` outside the tests, with its text, as the path below `src` and what it holds.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, below: &str, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("a folder") {
            let entry = entry.expect("an entry");
            let name = format!("{below}{}", entry.file_name().to_string_lossy());
            match entry.file_type().expect("a type").is_dir() {
                true if name == "tests" => {}
                true => walk(&entry.path(), &format!("{name}/"), out),
                false if name.ends_with(".rs") => out.push((name, std::fs::read_to_string(entry.path()).expect("a source"))),
                false => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), "", &mut out);
    out
}

#[test]
fn only_a_rules_act_reaches_the_trackers_writes() {
    let calling = |needle: &str| -> Vec<String> {
        let mut named: Vec<String> = sources()
            .into_iter()
            .filter(|(_, text)| text.lines().any(|line| !line.trim_start().starts_with("//") && line.contains(needle)))
            .map(|(name, _)| name)
            .collect();
        named.sort();
        named
    };
    assert_eq!(calling(".writer("), ["lifecycles/writing.rs"], "the one place Fleet asks for the writer of a tracker");
    assert_eq!(calling(".request_reviewers("), ["lifecycles/writing.rs"], "the one place Fleet asks a forge to request reviewers");
    assert_eq!(calling("rule_tracker_written("), ["lifecycles/acting.rs", "lifecycles/writing.rs"], "done by the worker that does a rule's acts, and by nothing else");
    assert_eq!(calling("rule_forge_written("), ["lifecycles/acting.rs", "lifecycles/writing.rs"]);
}
