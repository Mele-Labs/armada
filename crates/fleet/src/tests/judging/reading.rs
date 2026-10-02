//! A Judge reads the repository's own checkout, and is told it may. Driven
//! through a whole Fleet, because what is under test is the path Fleet takes
//! from the repository it serves to the call it starts, and that the Job's
//! worktree is never on it. Decided 2 Oct 2026.

use std::sync::Arc;

use adapter_traits::WorktreeSpec;
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, diff_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// One step, gated on a non-empty diff and on one narrow question.
fn one_judged_step() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[("c1", "Does the fix address the cause the note names?")],
        scope: None,
        gaming: None,
    }])
}

/// The Judge that ruled on a Job's first step, and the Job's worktree.
async fn judged_once(home: &TempDir) -> (Arc<FakeJudge>, String) {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let fleet = a_fleet_judged_by(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n"),
        one_judged_step(),
        Arc::clone(&judge),
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the gate");
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the tool took it");
    fleet.turn().await.expect("the gate ruled");
    let worktree = WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle())
        .expect("a legal spec")
        .worktree_path()
        .to_string();
    (judge, worktree)
}

/// **The checkout and never the worktree.** A Drone writes in its worktree, so
/// a Judge reading there would read what the Drone chose to show it.
#[tokio::test]
async fn a_judge_reads_the_repository_s_checkout_and_never_the_job_s_worktree() {
    let home = TempDir::new();
    let (judge, worktree) = judged_once(&home).await;

    let directories = judge.directories();
    assert_eq!(directories.len(), 1, "one criterion is one call");
    let root = home.path().to_string_lossy();
    for directory in &directories {
        let directory = directory.as_deref().expect("a Judge's call can read");
        assert_eq!(directory, root.trim_end_matches('/'));
        assert_ne!(directory, worktree.trim_end_matches('/'));
    }
}

/// The brief says what the call can do, and what that reading is for.
#[tokio::test]
async fn a_judge_is_told_it_may_read_the_repository_and_what_it_requires_is_not_scope() {
    let home = TempDir::new();
    let (judge, _) = judged_once(&home).await;

    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one criterion is one call");
    let question = &asked[0];
    assert!(
        question.contains("You can read this repository's own checkout, read-only"),
        "{question}"
    );
    assert!(
        question.contains("Work the repository requires of a change is not scope expansion"),
        "{question}"
    );
    let told = question
        .find("You can read this repository's own checkout")
        .expect("told");
    let asks = question
        .find("The question, which is yes or no")
        .expect("the question");
    assert!(told < asks, "told before it is asked: {question}");
}
