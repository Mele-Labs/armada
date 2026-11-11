//! An open issue a merged pull request finished without naming it, and a new issue that repeats an open
//! one, as the Operator offers to close them. **Each is asked first**: nothing is written to the tracker
//! until Do it or a rule he kept, and a merge with no open issue near its work makes no call. A close
//! names the pull request in the Log, and a group closes only the new issue, with a link to the one it repeats.

use std::sync::Arc;

use api::Sessions;
use ipc::operator_page::{OperatorRule, RuleScope};
use store::{KeptOrder, KeptSuggestion};
use testkit::{FakeBeads, FakeBeadsRemote, FakeTracker};

use super::backlog::machine;
use super::operator_ends::merge;
use super::operator_sources::{on, rulings, Answers};
use super::session_host::{eventually, Rig};
use crate::operator::{keep_the_ends_told, keep_the_raises};

const SCROLLBACK: &str = "Terminal scrollback is lost when the pane redraws";
const MERGED: &str = "Keep terminal scrollback when the pane redraws";
const FLICKER: &str = "Bay grid flickers when the window resizes";
const FLICKER_AGAIN: &str = "Bay grid flickers on resize";

/// A rig over beads with the Operator on, and the copy its issues are made in.
async fn beads() -> (Rig, Arc<FakeBeads>) {
    let copy = Arc::new(FakeBeads::on(&FakeBeadsRemote::new()));
    let rig = machine("north", &copy).await;
    on(&rig).await;
    (rig, copy)
}

fn closes(reference: &str) -> String {
    format!("item: 1\nway: 1\nissue: {reference}\nbecause: the pull request did what it asks")
}

fn repeats(reference: &str) -> String {
    format!("item: 1\nway: 1\nissue: {reference}\nkind: duplicate\nbecause: the same flicker")
}

fn belongs(reference: &str) -> String {
    format!("item: 1\nway: 1\nissue: {reference}\nkind: part\nbecause: one piece of that work")
}

async fn standing(rig: &Rig) -> Vec<KeptSuggestion> {
    rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")
}

async fn do_it(rig: &Rig, suggestion: &KeptSuggestion) {
    let _ = Arc::clone(&rig.fleet).suggestion_chosen("", &format!("suggestion:{}", suggestion.n), Some(0)).await;
}

/// The rule a go-ahead of `what` is done by, kept for every repository.
async fn rule_for(rig: &Rig, what: &str) {
    let rule = OperatorRule::PickRule { scope: RuleScope::Everywhere, what: what.to_string(), pick: "do_it".to_string() };
    let one = KeptOrder { n: 0, form: rule.kept(), said: "a rule".to_string(), until: None, at: "2026-11-10T09:00:00.000Z".to_string() };
    rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
}

/// Job "Fix the scrollback" with a pull request it merges once the tap hears it.
async fn merging(rig: &Rig, number: &str) {
    let job = rig.start_titled(Some("Fix the scrollback")).await;
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    merge(rig, &job, number, MERGED).await;
}

#[tokio::test]
async fn a_merge_that_finished_an_open_issue_is_offered_as_a_close_and_the_issue_is_closed_only_on_do_it() {
    let (rig, copy) = beads().await;
    let finished = copy.create(SCROLLBACK);
    let other = copy.create("Usage meter shows yesterday's tokens");
    let call = Answers::with(&[&closes(&finished)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    merging(&rig, "3042").await;

    eventually(|| async { standing(&rig).await.len() == 1 }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in [
        "A pull request merged, one of Fleet's own: \"Keep terminal scrollback when the pane redraws\"",
        &format!("Open issue its work may have finished: {finished} \"{SCROLLBACK}\""),
        "The ways it is offered: 1. close the open issue it lists, which the merged work finished; 2. leave it open, and nothing follows\n",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(!asked[0].contains(&other), "an issue sharing nothing with the work is not shown:\n{}", asked[0]);
    let suggestion = standing(&rig).await.remove(0);
    assert_eq!(suggestion.text, format!("The Operator would close {finished} \"{SCROLLBACK}\", done by #3042: the pull request did what it asks"));
    assert_eq!(copy.bead(&finished).expect("there").status, "open", "nothing is written until he presses");

    do_it(&rig, &suggestion).await;

    eventually(|| async { copy.bead(&finished).expect("there").status == "closed" }).await;
    assert!(copy.bead(&finished).expect("there").reason.contains("#3042"), "{:?}", copy.bead(&finished));
    assert_eq!(copy.bead(&other).expect("there").status, "open");
    let log = rulings(&rig).await;
    let row = log.iter().find(|row| row.outcome == "close").expect("a close in the Log");
    assert_eq!((row.arrived.as_str(), row.by.as_str(), row.issue.as_deref()), ("merged", "human", Some(finished.as_str())));
    assert!(row.because.starts_with("Done by #3042: "), "{}", row.because);
}

#[tokio::test]
async fn a_merge_with_no_open_issue_near_its_work_makes_no_call_and_writes_nothing() {
    let (rig, copy) = beads().await;
    let unrelated = copy.create("Usage meter shows yesterday's tokens");
    let call = Answers::with(&[&closes(&unrelated)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    merging(&rig, "3042").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    assert!(call.asked().is_empty(), "{:#?}", call.asked());
    assert!(standing(&rig).await.is_empty());
    assert!(rulings(&rig).await.is_empty());
    assert_eq!(copy.bead(&unrelated).expect("there").status, "open");
}

#[tokio::test]
async fn a_merge_does_not_offer_to_close_an_issue_its_job_named() {
    let (rig, copy) = beads().await;
    let named = copy.create(SCROLLBACK);
    let call = Answers::with(&[&closes(&named)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let job = rig.start_titled(Some("Fix the scrollback")).await;
    rig.fleet.claim_issue(None, ipc::ClaimIssue { issue: named.clone(), session_id: Some(job.clone()), pull_requests: Vec::new() }).await.expect("claimed");
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;

    merge(&rig, &job, "3042", MERGED).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    assert!(call.asked().is_empty(), "{:#?}", call.asked());
    assert!(standing(&rig).await.is_empty());
}

#[tokio::test]
async fn a_close_is_done_as_it_is_offered_by_a_rule_he_kept_and_the_log_says_which() {
    let (rig, copy) = beads().await;
    let finished = copy.create(SCROLLBACK);
    let call = Answers::with(&[&closes(&finished)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    rule_for(&rig, "merged:close").await;

    merging(&rig, "3042").await;

    eventually(|| async { copy.bead(&finished).expect("there").status == "closed" }).await;
    assert!(copy.bead(&finished).expect("there").reason.contains("#3042"));
    assert_eq!(call.asked().len(), 1, "one call to read it, and none to review what a rule did");
    eventually(|| async { rulings(&rig).await.iter().any(|row| row.outcome == "close") }).await;
    let log = rulings(&rig).await;
    let row = log.iter().find(|row| row.outcome == "close").expect("a close in the Log");
    assert_eq!((row.by.as_str(), row.issue.as_deref()), ("operator", Some(finished.as_str())));
    assert!(row.because.starts_with("By a rule, Close an open issue a merged pull request finished: Done by #3042: "), "{}", row.because);
    let done = rig.fleet.operator_board_wire().await.expect("read").done;
    let shown = done.iter().find(|one| one.outcome == format!("Closed: {finished}")).expect("what a rule did is in Done");
    assert_eq!(shown.by, ipc::operator_board::OperatorDecider::Operator);
}

/// A new issue `new` and an older one `old`, in beads and as Fleet's reads of the tracker saw them.
async fn two_issues(rig: &Rig, copy: &FakeBeads, old: &str, new: &str) -> (String, String) {
    let (old, new) = (copy.create(old), copy.create(new));
    let seen = |reference: &str, title: &str| {
        let mut one = FakeTracker::issue(1, title);
        one.reference = reference.to_string();
        one
    };
    let seat = rig.fleet.operator_seat();
    let titled = |reference: &str| copy.bead(reference).expect("there").title;
    seat.issues_read(rig.manifest.as_str(), &[seen(&old, &titled(&old))]);
    seat.issues_read(rig.manifest.as_str(), &[seen(&old, &titled(&old)), seen(&new, &titled(&new))]);
    (old, new)
}

#[tokio::test]
async fn a_new_issue_that_repeats_an_open_one_is_offered_as_a_duplicate_and_closed_only_on_do_it() {
    let (rig, copy) = beads().await;
    let (old, new) = two_issues(&rig, &copy, FLICKER, FLICKER_AGAIN).await;
    let call = Answers::with(&[&repeats(&old)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    rig.fleet.sources_handed().await;

    eventually(|| async { standing(&rig).await.len() == 1 }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in [
        "A new issue was filed, from the tracker: \"Bay grid flickers on resize\"",
        &format!("Open issues it may repeat: {old} \"{FLICKER}\""),
        "The ways it is offered: 1. group it with the open issue it lists, as its duplicate (closed) or a part of it (put under it), saying which in kind; 2. raise\n",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    let suggestion = standing(&rig).await.remove(0);
    assert_eq!(suggestion.text, format!("The Operator would close {new} \"{FLICKER_AGAIN}\" as a duplicate of {old} \"{FLICKER}\": the same flicker"));
    assert_eq!(copy.bead(&new).expect("there").status, "open", "nothing is written until he presses");

    do_it(&rig, &suggestion).await;

    eventually(|| async { copy.bead(&new).expect("there").status == "closed" }).await;
    assert!(copy.bead(&new).expect("there").reason.contains(&old), "{:?}", copy.bead(&new));
    assert_eq!(copy.bead(&old).expect("there").status, "open", "the one it repeats is left alone");
    let log = rulings(&rig).await;
    let row = log.iter().find(|row| row.outcome == "group").expect("a group in the Log");
    assert_eq!((row.arrived.as_str(), row.issue.as_deref()), ("issue", Some(old.as_str())));
}

#[tokio::test]
async fn a_part_is_put_under_the_open_issue_on_do_it_and_nothing_is_closed() {
    let (rig, copy) = beads().await;
    let (old, new) = two_issues(&rig, &copy, FLICKER, FLICKER_AGAIN).await;
    let call = Answers::with(&[&belongs(&old)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    rig.fleet.sources_handed().await;

    eventually(|| async { standing(&rig).await.len() == 1 }).await;
    let suggestion = standing(&rig).await.remove(0);
    assert_eq!(suggestion.text, format!("The Operator would put {new} \"{FLICKER_AGAIN}\" under {old} \"{FLICKER}\": one piece of that work"));
    assert_eq!(copy.bead(&new).expect("there").parent, "", "nothing is written until he presses");

    do_it(&rig, &suggestion).await;

    eventually(|| async { copy.bead(&new).expect("there").parent == old }).await;
    assert_eq!((copy.bead(&new).expect("there").status.as_str(), copy.bead(&old).expect("there").status.as_str()), ("open", "open"));
    eventually(|| async { rulings(&rig).await.iter().any(|row| row.outcome == "group") }).await;
    let log = rulings(&rig).await;
    let row = log.iter().find(|row| row.outcome == "group").expect("a group in the Log");
    assert_eq!(row.issue.as_deref(), Some(old.as_str()));
    assert!(row.because.starts_with(&format!("Put under {old}: ")), "{}", row.because);
}

#[tokio::test]
async fn a_group_that_names_neither_duplicate_nor_part_is_refused_and_nothing_is_written() {
    let (rig, copy) = beads().await;
    let (old, new) = two_issues(&rig, &copy, FLICKER, FLICKER_AGAIN).await;
    let call = Answers::with(&[&closes(&old)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    rig.fleet.sources_handed().await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    assert!(standing(&rig).await.is_empty());
    assert_eq!(rulings(&rig).await[0].outcome, "passed_on");
    assert_eq!((copy.bead(&new).expect("there").status.as_str(), copy.bead(&new).expect("there").parent.as_str()), ("open", ""));
}

#[tokio::test]
async fn a_duplicate_is_closed_as_it_is_offered_by_a_rule_he_kept() {
    let (rig, copy) = beads().await;
    let (old, new) = two_issues(&rig, &copy, FLICKER, FLICKER_AGAIN).await;
    let call = Answers::with(&[&repeats(&old)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    rule_for(&rig, "issue:group").await;

    rig.fleet.sources_handed().await;

    eventually(|| async { copy.bead(&new).expect("there").status == "closed" }).await;
    assert_eq!(copy.bead(&old).expect("there").status, "open");
    assert_eq!(call.asked().len(), 1);
    eventually(|| async { rulings(&rig).await.iter().any(|row| row.outcome == "group") }).await;
    let log = rulings(&rig).await;
    let row = log.iter().find(|row| row.outcome == "group").expect("a group in the Log");
    assert_eq!(row.by, "operator");
    assert!(row.because.starts_with("By a rule, Group new issues with an open issue they repeat or belong under: "), "{}", row.because);
}

#[tokio::test]
async fn a_new_issue_with_no_open_issue_near_it_is_not_offered_a_group() {
    let (rig, copy) = beads().await;
    let (_, new) = two_issues(&rig, &copy, SCROLLBACK, FLICKER_AGAIN).await;
    let call = Answers::with(&["item: 1\nway: 1\nbecause: a new fault, and no Job works on it"]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    rig.fleet.sources_handed().await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert!(asked[0].contains("The ways it is offered: 1. raise\n"), "{}", asked[0]);
    assert!(!asked[0].contains("duplicate of the open issue it lists;"), "{}", asked[0]);
    assert_eq!(copy.bead(&new).expect("there").status, "open");
    assert!(standing(&rig).await.is_empty());
}

#[tokio::test]
async fn a_close_the_call_names_for_an_issue_it_was_not_shown_is_refused_and_nothing_is_written() {
    let (rig, copy) = beads().await;
    let finished = copy.create(SCROLLBACK);
    let other = copy.create("Usage meter shows yesterday's tokens");
    let call = Answers::with(&[&closes(&other)]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    merging(&rig, "3042").await;

    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    assert!(standing(&rig).await.is_empty());
    assert_eq!(copy.bead(&finished).expect("there").status, "open");
    assert_eq!(copy.bead(&other).expect("there").status, "open");
    assert_eq!(rulings(&rig).await[0].outcome, "passed_on");
}
