//! What a gate's policies resolved to, per run: V87, and whether the rule
//! decided: V88. #1683.
//!
//! **A run written the old way reads absent.** The first run here is recorded
//! through the writers that existed before V87 and nothing else, which is
//! every run an older Fleet left on the record. It has no row, and the read
//! says so rather than answering a default.

use core_model::{AutoMerge, ResolvedPolicies, ReviewGate};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
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
        decided: true,
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
        decided: false,
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

/// **A row V87 wrote reads `decided = true`.** V87 was written only on a run
/// that reached the advance gate, so the column's default is the fact for
/// every row already there. Tested from the version before it,
/// `studio_unfrozen`'s way: a V87 file written through raw SQL.
#[test]
fn a_row_v87_wrote_reads_decided() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..87] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '87');
         INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES ('01V87ROW', 'a job V87 gated', 'awaiting_review', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-10-01T09:00:00.000Z');
         INSERT INTO job_step_policies
             (job_id, step_id, attempt, auto_merge, review_gate, resolved_at)
         VALUES ('01V87ROW', 'fix', 1, 'never', 'human_always', '2026-10-01T09:05:00.000Z');"
    ))
    .expect("a gated run as V87 wrote it");
    drop(conn);

    let store = open(&dir);
    let kept = store
        .resolved_policies_every_attempt(&job_id("01V87ROW"))
        .expect("migrates and reads");
    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0].record,
        ResolvedPolicies {
            auto_merge: AutoMerge::Never,
            review_gate: ReviewGate::HumanAlways,
            decided: true,
        }
    );
}
