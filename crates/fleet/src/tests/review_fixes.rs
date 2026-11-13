//! A fix run: refused without a finding on the head's lines of a file the change touches, and
//! kept with the head's own lines as `before`, whatever the agent says they were.
//! `docs/concepts/pull-review.md`, *Agent runs*.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::slot_path;
use api::PullRequestPanel;
use ipc::{DiffSide, ReviewFinding, ReviewPlace, ReviewResult, ReviewRunKind, ReviewRunState, ReviewSeverity, StartReviewRun};
use testkit::FakeJudge;

use super::pull_requesting::{code, manifest};
use super::review_runs::{a_fleet_judged_by, settled};
use crate::review_runs::fix_answer_read;
use crate::tests::tmp::TempDir;

/// `a.rs` at the head, as the bay holds it.
const AT_HEAD: &str = "fn total(a: u8, b: u8) -> u8 {\n    let sum = a + b;\n    sum\n}\n";

fn at(path: &str, side: DiffSide, start: u64, end: u64) -> ReviewPlace {
    ReviewPlace { path: path.into(), side, start, end }
}

fn fixing(finding: Option<ReviewPlace>) -> StartReviewRun {
    StartReviewRun {
        number: 12,
        kind: ReviewRunKind::Fix,
        name: "a.rs:2-3 Overflow".into(),
        again: false,
        place: None,
        finding: finding.map(|at| ReviewFinding {
            title: "Overflow".into(),
            body: "`a + b` wraps past 255.".into(),
            severity: ReviewSeverity::High,
            at,
            comment: "This sum wraps past 255. Use a saturating add.".into(),
        }),
        since: None,
    }
}

/// The head's `a.rs` in the directory the first bay is.
fn bay_holding_a_rs(root: &str) {
    let bay = slot_path(root, 1);
    std::fs::create_dir_all(&bay).expect("a bay");
    std::fs::write(Path::new(&bay).join("a.rs"), AT_HEAD).expect("written");
}

#[tokio::test]
async fn a_fix_with_no_finding_on_old_lines_or_off_the_change_is_refused_before_anything_is_spent() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"after":"x","why":"y"}"#));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, _) = manifest(&fleet);
    for (finding, refused_as) in [
        (None, "fleet.review_fix_needs_a_finding"),
        (Some(at("a.rs", DiffSide::Left, 2, 3)), "fleet.review_fix_on_old_lines"),
        (Some(at("elsewhere.rs", DiffSide::Right, 2, 3)), "fleet.review_fix_off_the_change"),
    ] {
        let refused = Arc::clone(&fleet).start_review_run(id.clone(), fixing(finding)).await.unwrap_err();
        assert_eq!((refused.status(), code(&refused)), (422, refused_as));
    }
    assert!(judge.asked().is_empty() && fleet.vcs().review_bays_leased().is_empty());
    assert!(fleet.get_review_runs(id, 12).await.expect("read").runs.is_empty(), "nothing is kept");
}

#[tokio::test]
async fn a_fix_keeps_the_heads_lines_and_the_findings_place_not_the_agents() {
    let home = TempDir::new();
    // The agent also says where and what was there; Fleet keeps neither.
    let judge = Arc::new(FakeJudge::saying(
        r#"{"after":"    let sum = a.saturating_add(b);\n    sum\n","why":"The sum stops at 255.","at":{"path":"b.rs","side":"left","start":9,"end":9},"before":"made up"}"#,
    ));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, root) = manifest(&fleet);
    bay_holding_a_rs(&root);
    let place = at("a.rs", DiffSide::Right, 2, 3);

    Arc::clone(&fleet).start_review_run(id.clone(), fixing(Some(place.clone()))).await.expect("started");
    let runs = settled(&fleet, &id).await;

    assert_eq!((runs[0].kind, runs[0].state, runs[0].name.as_str()), (ReviewRunKind::Fix, ReviewRunState::Done, "a.rs:2-3 Overflow"));
    assert_eq!(
        runs[0].result,
        Some(ReviewResult::Fix {
            at: place,
            before: "    let sum = a + b;\n    sum".into(),
            after: "    let sum = a.saturating_add(b);\n    sum".into(),
            why: "The sum stops at 255.".into(),
        })
    );
    let asked = judge.asked();
    for told in ["Overflow", "wraps past 255", "Lines 2 to 3 of `a.rs`", "    let sum = a + b;", "\"after\""] {
        assert!(asked[0].contains(told), "the brief lacks {told}: {}", asked[0]);
    }
}

#[tokio::test]
async fn a_finding_past_the_files_end_fails_without_asking() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"after":"x","why":"y"}"#));
    let fleet = a_fleet_judged_by(&home, Arc::clone(&judge));
    let (id, root) = manifest(&fleet);
    bay_holding_a_rs(&root);

    Arc::clone(&fleet).start_review_run(id.clone(), fixing(Some(at("a.rs", DiffSide::Right, 3, 9)))).await.expect("started");
    let runs = settled(&fleet, &id).await;

    assert_eq!(runs[0].state, ReviewRunState::Failed);
    let failed = runs[0].failed.as_deref().unwrap_or_default();
    assert!(failed.contains("lines 3 to 9") && failed.contains("has 4 lines"), "{failed}");
    assert!(judge.asked().is_empty(), "nothing was spent");
    assert_eq!(fleet.vcs().review_bays_released().len(), 1, "the bay went back");
}

#[test]
fn a_fix_that_is_empty_or_changes_nothing_is_refused() {
    let dir = TempDir::new();
    std::fs::write(dir.path().join("a.rs"), AT_HEAD).expect("written");
    let place = at("a.rs", DiffSide::Right, 2, 3);
    let read = |answer: &str| fix_answer_read(answer, dir.path(), &place);
    let empty = read(r#"{"after":"  \n","why":"Nothing."}"#).unwrap_err();
    assert!(empty.contains("the fix is empty"), "{empty}");
    let same = read(r#"{"after":"    let sum = a + b;\n    sum\n","why":"Same."}"#).unwrap_err();
    assert!(same.contains("changes nothing"), "{same}");
    let no_why = read(r#"{"after":"    a.saturating_add(b)","why":" "}"#).unwrap_err();
    assert!(no_why.contains("why is empty"), "{no_why}");
    assert!(matches!(read(r#"{"after":"    a.saturating_add(b)","why":"Stops at 255."}"#), Ok(ReviewResult::Fix { .. })));
}
