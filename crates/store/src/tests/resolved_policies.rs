//! What a gate's policies resolved to, per run: V87. #1683.
//!
//! **A run written the old way reads absent.** The first run here is recorded
//! through the writers that existed before V87 and nothing else, which is
//! every run an older Fleet left on the record. It has no row, and the read
//! says so rather than answering a default.

use core_model::{AutoMerge, ResolvedPolicies, ReviewGate};

use crate::tests::attempt::{on_its_first_run, record_a_whole_run, run_it_again, step_id};
use crate::tests::{at, job_id, open, TempDir};

#[test]
fn a_run_recorded_before_v87_reads_absent_and_the_next_reads_what_it_resolved_to() {
    let dir = TempDir::new();
    let id = "01RESOLVED";
    let mut store = open(&dir);
    let job = on_its_first_run(&mut store, id);
    store
        .record_step_checks(
            &job_id(id),
            &step_id(),
            &[],
            &at("2026-08-26T10:05:00.000Z"),
        )
        .expect("a run's Checks, as an older Fleet wrote them");
    run_it_again(
        &mut store,
        &job,
        "2026-08-26T10:06:00.000Z",
        "2026-08-26T10:07:00.000Z",
    );
    let resolved = ResolvedPolicies {
        auto_merge: AutoMerge::ChecksPass,
        review_gate: ReviewGate::AutoIfJudgePasses,
    };
    store
        .record_resolved_policies(
            &job_id(id),
            &step_id(),
            resolved,
            &at("2026-08-26T10:08:00.000Z"),
        )
        .expect("the second run's gate is kept");

    let kept = store
        .resolved_policies_every_attempt(&job_id(id))
        .expect("reads back");
    let runs: Vec<(u32, ResolvedPolicies)> = kept
        .iter()
        .map(|row| (row.attempt.number(), row.record))
        .collect();
    assert_eq!(
        runs,
        vec![(2, resolved)],
        "the first run has no row and the second has its own"
    );
}

/// **A second gate on one run replaces the first**, which a re-run of the
/// Checks is: the run ends on the later answer.
#[test]
fn a_second_gate_on_the_same_run_replaces_the_first() {
    let dir = TempDir::new();
    let id = "01REGATED";
    let mut store = open(&dir);
    on_its_first_run(&mut store, id);
    record_a_whole_run(&mut store, id, "the first pass", "2026-08-26T10:05:00.000Z");
    let later = ResolvedPolicies {
        auto_merge: AutoMerge::Always,
        review_gate: ReviewGate::HumanAlways,
    };
    store
        .record_resolved_policies(
            &job_id(id),
            &step_id(),
            later,
            &at("2026-08-26T10:06:00.000Z"),
        )
        .expect("the re-gate is kept");

    let kept = store
        .resolved_policies_every_attempt(&job_id(id))
        .expect("reads back");
    assert_eq!(kept.len(), 1, "one row per run");
    assert_eq!(kept[0].record, later);
}
