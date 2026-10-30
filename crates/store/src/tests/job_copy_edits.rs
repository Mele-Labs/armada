//! What a Job changed in its copy: a workflow of its own once a step changed, a mark for each change that
//! the latest change replaces, and pieces put in and taken out of the copy it froze.

use crate::tests::{open, TempDir};
use crate::{KeptCopy, KeptCopyMark, KeptPiece, KeptWorkflowCopy, PieceKind};

fn mark(lane: &str, item: &str, text: &str, at: &str) -> KeptCopyMark {
    KeptCopyMark { lane: lane.into(), item: item.into(), text: text.into(), by: "job".into(), at: at.into() }
}

fn rule(text: &str) -> KeptPiece {
    KeptPiece { kind: PieceKind::Rule, piece_id: "watch".into(), path: "job copy".into(), text: text.into() }
}

#[test]
fn a_job_holds_its_own_workflow_only_once_it_changed_a_step_and_gives_it_up_with_the_marks_on_its_steps() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.job_workflow_copy("s1").expect("reads"), None);
    let copy = KeptWorkflowCopy { workflow_id: "feature".into(), path: "feature.json".into(), text: "steps".into() };
    store.keep_job_workflow_copy("s1", &copy).expect("keeps");
    store.keep_job_workflow_copy("s1", &KeptWorkflowCopy { text: "more steps".into(), ..copy.clone() }).expect("keeps again");
    assert_eq!(store.job_workflow_copy("s1").expect("reads").map(|one| one.text), Some("more steps".into()));
    assert_eq!(store.job_workflow_copy("s2").expect("reads"), None, "another Job's is its own");

    store.keep_job_copy_mark("s1", &mark("workflow", "dispatch", "Added", "t0")).expect("marks");
    store.keep_job_copy_mark("s1", &mark("rule", "watch", "Added", "t0")).expect("marks");
    store.drop_job_workflow_copy("s1").expect("drops");
    assert_eq!(store.job_workflow_copy("s1").expect("reads"), None);
    let left: Vec<String> = store.job_copy_marks("s1").expect("reads").into_iter().map(|one| one.lane).collect();
    assert_eq!(left, ["rule"], "the marks on its steps go with the steps, and a rule's stays");
}

#[test]
fn the_latest_change_to_a_node_is_its_mark_and_undoing_it_takes_the_mark_away() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_job_copy_mark("s1", &mark("workflow", "walk", "Skipped", "t0")).expect("marks");
    store.keep_job_copy_mark("s1", &mark("workflow", "dispatch", "Added", "t1")).expect("marks");
    store.keep_job_copy_mark("s1", &mark("workflow", "walk", "Moved", "t2")).expect("marks again");
    let marks = store.job_copy_marks("s1").expect("reads");
    assert_eq!(marks.iter().map(|one| (one.item.as_str(), one.text.as_str())).collect::<Vec<_>>(), [("dispatch", "Added"), ("walk", "Moved")]);
    store.drop_job_copy_mark("s1", "workflow", "walk").expect("drops");
    assert_eq!(store.job_copy_marks("s1").expect("reads").len(), 1);
    assert!(store.job_copy_marks("s2").expect("reads").is_empty());
}

#[test]
fn a_piece_is_put_in_the_copy_a_job_froze_replaced_and_taken_out_and_a_job_with_no_copy_takes_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(!store.keep_job_copy_piece("s1", &rule("first")).expect("keeps"), "no copy was frozen, and none is made here");
    store.freeze_job_copy("s1", "t0", &KeptCopy::default()).expect("freezes");
    assert!(store.keep_job_copy_piece("s1", &rule("first")).expect("keeps"));
    assert!(store.keep_job_copy_piece("s1", &rule("second")).expect("keeps again"));
    let kept = store.job_copy_of("s1").expect("reads").expect("a copy");
    assert_eq!(kept.pieces, [rule("second")], "one of each id");
    assert!(store.drop_job_copy_piece("s1", PieceKind::Rule, "watch").expect("drops"));
    assert!(!store.drop_job_copy_piece("s1", PieceKind::Rule, "watch").expect("drops again"), "it was not there");
    assert!(store.job_copy_of("s1").expect("reads").expect("a copy").pieces.is_empty());
}
