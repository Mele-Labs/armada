//! V90: a Judge and a Check sign the rows their answer wrote, and a task may
//! stand `handed_in` or `failed`. Spike 022, slice 1a.
//!
//! **An older Fleet refuses the file rather than folding it.** A build that
//! knows 89 migrations meets a file recording 90 at open, and `open`'s
//! `found > known` rule refuses it, which `migrate`'s own tests hold. What is
//! held here is that a file this build writes records past 89.

use core_model::{Actor, EscalationTrigger, StepLevelTrigger, StepTarget, Target};
use rusqlite::Connection;

use crate::migrations::{KNOWN_SCHEMA_VERSION, MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};

/// The last migration a build without the two signers knew.
const BEFORE_THE_SIGNERS: u32 = 89;

#[test]
fn a_judge_and_a_check_sign_and_read_back_and_an_older_fleet_cannot_open_the_file() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(
        KNOWN_SCHEMA_VERSION > BEFORE_THE_SIGNERS,
        "a file holding a Judge's signature records a version a build before it refuses"
    );
    for (id, signer) in [
        ("01SIGNEDJUDGE", Actor::Judge),
        ("01SIGNEDCHECK", Actor::Check),
    ] {
        let job = on_its_first_run(&mut store, id);
        let trigger = StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("step-level");
        let stopped = job
            .transition_step(
                &step_id(),
                StepTarget::Stopped(trigger),
                signer,
                at("2026-10-02T10:03:00.000Z"),
            )
            .expect("a legal stop");
        store.record_step_transition(&stopped).expect("recorded");
        let moved = stopped
            .job
            .transition(
                Target::Escalated(EscalationTrigger::GateFailure),
                signer,
                at("2026-10-02T10:04:00.000Z"),
            )
            .expect("a legal escalation");
        store.record_transition(&moved).expect("recorded");
    }
    drop(store);

    let store = open(&dir);
    for (id, signer) in [
        ("01SIGNEDJUDGE", Actor::Judge),
        ("01SIGNEDCHECK", Actor::Check),
    ] {
        let signed: Vec<Actor> = store
            .events_for(&job_id(id))
            .expect("reads")
            .iter()
            .map(|event| event.actor())
            .collect();
        assert_eq!(
            signed[signed.len() - 2..],
            [signer, signer],
            "the stop and the escalation are both signed by who answered"
        );
        store.load_job(&job_id(id)).expect("and the Job folds");
    }
}

/// The rebuild keeps every plan change V89 held, and admits the two states.
#[test]
fn a_plan_v89_wrote_survives_and_the_two_states_are_admitted() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..BEFORE_THE_SIGNERS as usize] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '89');
         INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES ('01V89PLAN', 'a planned job', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-10-01T09:00:00.000Z');
         INSERT INTO job_work_plan_changes (job_id, seq, change, by_step, by_attempt, at, approach)
         VALUES ('01V89PLAN', 1, 'recorded', 'plan', 1, '2026-10-01T09:01:00.000Z', 'Bound it');
         INSERT INTO job_work_plan_tasks (job_id, seq, ordinal, title, detail, scope, expects)
         VALUES ('01V89PLAN', 1, 1, 'Stop at the end', '', 'crates/store/src/read.rs', '');
         INSERT INTO job_work_plan_changes (job_id, seq, change, by_step, by_attempt, at,
                                            task_id, state, shown)
         VALUES ('01V89PLAN', 2, 'updated', 'plan', 1, '2026-10-01T09:02:00.000Z', 1,
                 'working', NULL);"
    ))
    .expect("a plan as V89 wrote it");
    drop(conn);

    let store = open(&dir);
    let plan = store
        .work_plan(&job_id("01V89PLAN"))
        .expect("migrates and reads")
        .expect("a plan");
    assert_eq!(plan.tasks()[0].state(), core_model::TaskState::Working);
    assert_eq!(
        plan.tasks()[0].scope()[0].as_str(),
        "crates/store/src/read.rs"
    );

    let update = |seq: u32, state: &str, reason: Option<&str>| {
        store.conn.execute(
            "INSERT INTO job_work_plan_changes (job_id, seq, change, at, task_id, state, reason)
             VALUES ('01V89PLAN', ?1, 'updated', '2026-10-02T09:00:00.000Z', 1, ?2, ?3)",
            rusqlite::params![seq, state, reason],
        )
    };
    update(3, "handed_in", None).expect("the store admits a handed-in task");
    // A failure says why since V94, as a drop does: spike 022, slice 2.
    assert!(
        update(4, "failed", None).is_err(),
        "never a failure without a reason"
    );
    update(4, "failed", Some("G1's Checks were still red on run 3")).expect("and a failed one");
    assert!(
        update(5, "finished", None).is_err(),
        "and still no word it lacks"
    );
    assert!(
        store
            .conn
            .execute("DELETE FROM job_work_plan_changes WHERE seq = 3", [])
            .is_err(),
        "and the rebuilt table is still append-only"
    );
}
