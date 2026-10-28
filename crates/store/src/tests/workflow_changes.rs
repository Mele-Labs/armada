//! The changes kept to a workflow: begun drafting, drafted or failed, sent back, approved or dismissed,
//! and listed while open for the workflow and place they belong to.

use crate::tests::{open, TempDir};
use crate::{NewWorkflowChange, CHANGE_APPROVED, CHANGE_CANNOT, CHANGE_DISMISSED, CHANGE_DRAFTING, CHANGE_FAILED, CHANGE_READY};

const AT: &str = "2026-10-27T09:00:00.000Z";
const LATER: &str = "2026-10-27T09:05:00.000Z";

fn asked<'a>(manifest: &'a str, source: &'a str) -> NewWorkflowChange<'a> {
    NewWorkflowChange {
        manifest_id: manifest,
        workflow_id: "feature",
        source,
        step_id: None,
        asked: Some("split Refine in two"),
        nudges: &[],
        before_text: "{before}",
    }
}

#[test]
fn a_change_is_begun_drafting_and_kept_across_reopens() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    drop(store);
    let kept = open(&dir).workflow_change(id).unwrap().expect("kept");
    assert_eq!(kept.state, CHANGE_DRAFTING);
    assert_eq!(kept.asked.as_deref(), Some("split Refine in two"));
    assert_eq!((kept.before_text.as_str(), kept.after_text.as_deref()), ("{before}", None));
    assert!(open(&dir).workflow_change(id + 1).unwrap().is_none());
}

#[test]
fn a_suggestion_keeps_the_step_and_the_nudges_it_was_drawn_from() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let new = NewWorkflowChange { step_id: Some("walk"), asked: None, nudges: &[4, 9], ..asked("repo", "repository") };
    let id = store.begin_workflow_change(&new, AT).unwrap();
    let kept = store.workflow_change(id).unwrap().unwrap();
    assert_eq!((kept.step_id.as_deref(), kept.asked.as_deref(), kept.nudges), (Some("walk"), None, vec![4, 9]));
}

#[test]
fn a_draft_makes_it_ready_and_a_failure_keeps_the_reason_but_only_from_drafting() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let one = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    let two = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    assert!(store.workflow_change_drafted(one, "{after}", LATER).unwrap());
    assert!(store.workflow_change_failed(two, "it did not fit", LATER).unwrap());
    let ready = store.workflow_change(one).unwrap().unwrap();
    assert_eq!((ready.state.as_str(), ready.after_text.as_deref()), (CHANGE_READY, Some("{after}")));
    let failed = store.workflow_change(two).unwrap().unwrap();
    assert_eq!((failed.state.as_str(), failed.failed_because.as_deref()), (CHANGE_FAILED, Some("it did not fit")));
    assert!(!store.workflow_change_failed(one, "late", LATER).unwrap(), "a ready change does not fail");
    assert!(!store.workflow_change_drafted(two, "{late}", LATER).unwrap(), "a failed one is not drafted by a straggler");
}

#[test]
fn sending_back_drafts_again_with_the_notes_and_only_from_ready_or_failed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    assert!(!store.workflow_change_sent_back(id, "no", LATER).unwrap(), "drafting is not sent back");
    store.workflow_change_drafted(id, "{after}", AT).unwrap();
    assert!(store.workflow_change_sent_back(id, "keep Deliver as it was", LATER).unwrap());
    let back = store.workflow_change(id).unwrap().unwrap();
    assert_eq!((back.state.as_str(), back.notes.as_deref()), (CHANGE_DRAFTING, Some("keep Deliver as it was")));
    assert_eq!(back.after_text.as_deref(), Some("{after}"), "the draft sent back stays for the next to read");
    assert!(back.failed_because.is_none());
}

#[test]
fn approving_needs_a_ready_change_and_dismissing_takes_any_open_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let drafting = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    assert!(!store.workflow_change_approved(drafting, LATER).unwrap());
    store.workflow_change_drafted(drafting, "{after}", AT).unwrap();
    assert!(store.workflow_change_approved(drafting, LATER).unwrap());
    assert_eq!(store.workflow_change(drafting).unwrap().unwrap().state, CHANGE_APPROVED);
    assert!(!store.workflow_change_dismissed(drafting, LATER).unwrap(), "a settled change is left alone");
    let other = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    assert!(store.workflow_change_dismissed(other, LATER).unwrap());
    assert_eq!(store.workflow_change(other).unwrap().unwrap().state, CHANGE_DISMISSED);
}

#[test]
fn only_open_changes_are_listed_for_the_workflow_and_place_and_a_repositorys_is_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mine = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    let settled = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    store.workflow_change_dismissed(settled, LATER).unwrap();
    let kit = store.begin_workflow_change(&asked("repo", "kit"), AT).unwrap();
    store.begin_workflow_change(&asked("elsewhere", "repository"), AT).unwrap();
    store.begin_workflow_change(&NewWorkflowChange { workflow_id: "bug", ..asked("repo", "repository") }, AT).unwrap();
    let ids = |found: Vec<crate::KeptWorkflowChange>| found.into_iter().map(|one| one.id).collect::<Vec<_>>();
    assert_eq!(ids(store.open_workflow_changes("feature", "repository", "repo").unwrap()), [mine]);
    assert_eq!(ids(store.open_workflow_changes("feature", "kit", "another").unwrap()), [kit], "Kit's is every repository's");
}

#[test]
fn a_change_armada_cannot_make_keeps_what_it_lacks_is_listed_and_is_dismissed_but_not_sent_back() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    let missing = vec![String::from("A step can't end when a document is approved."), String::from("A step can't end on a sketch\nbeing approved.")];
    assert!(store.workflow_change_cannot(id, &missing, LATER).unwrap());
    drop(store);
    let mut store = open(&dir);
    let kept = store.workflow_change(id).unwrap().unwrap();
    assert_eq!(kept.state, CHANGE_CANNOT);
    assert_eq!(kept.missing, ["A step can't end when a document is approved.", "A step can't end on a sketch being approved."], "one line each");
    assert_eq!(kept.after_text, None);
    assert_eq!(store.open_workflow_changes("feature", "repository", "repo").unwrap().len(), 1, "it is open");
    assert!(!store.workflow_change_drafted(id, "{late}", LATER).unwrap(), "a straggler does not draft it");
    assert!(!store.workflow_change_sent_back(id, "try again", LATER).unwrap());
    assert!(!store.workflow_change_cannot(id, &missing, LATER).unwrap(), "only from drafting");
    assert!(store.workflow_change_dismissed(id, LATER).unwrap());
    let dismissed = store.workflow_change(id).unwrap().unwrap();
    assert_eq!((dismissed.state.as_str(), dismissed.missing.len()), (CHANGE_DISMISSED, 2));
    assert!(store.open_workflow_changes("feature", "repository", "repo").unwrap().is_empty());
    let other = store.begin_workflow_change(&asked("repo", "repository"), AT).unwrap();
    assert!(store.workflow_change(other).unwrap().unwrap().missing.is_empty(), "every other change lacks nothing");
}
