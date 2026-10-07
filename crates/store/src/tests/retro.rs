//! What a Job's retro keeps: the door each move came through, what its Drones
//! said got in the way, and the retro itself once one is written.
//! `docs/concepts/retro.md`.

use core_model::{Actor, Job, LandsIn, LessonState, StepId, Target, Via, Whose};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::attempt::on_its_first_run;
use crate::tests::{at, created_at, job_id, open, top_level, TempDir};
use crate::{Reflected, RetroLine, Store};

/// A stored Job, killed at `when`. **Ended**, which is what owes a retro.
fn ended(store: &mut Store, id: &str, when: &str) -> Job {
    let created = top_level(id);
    store
        .insert_job(&created, &created_at())
        .expect("the job is stored");
    let moved = created
        .transition(Target::Killed, Actor::Human, at(when))
        .expect("a person may stop a Job at the gate");
    store.record_transition(&moved).expect("recorded");
    moved.job
}

fn line(whose: Whose, said: &str, evidence: &[&str]) -> RetroLine {
    RetroLine {
        whose,
        title: None,
        what: None,
        fix: None,
        said: said.to_string(),
        evidence: evidence.iter().map(|one| one.to_string()).collect(),
        lands_in: Some(LandsIn::Armada),
        change: None,
    }
}

/// **The door is kept against the row it moved**, so a history read can say
/// which moves an agent made and which a person pressed in Bridge.
#[test]
fn a_move_keeps_the_door_it_came_through() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = on_its_first_run(&mut store, "01RETROVIA");
    let moved = job
        .transition(Target::Killed, Actor::Helm, at("2026-08-26T10:05:00.000Z"))
        .expect("a kill");
    let seq = store.record_transition(&moved).expect("recorded");

    store
        .record_via(job.id(), seq, Via::Http)
        .expect("the door is kept");

    assert_eq!(
        store.vias_for(job.id()).expect("read back"),
        vec![(seq, Via::Http)]
    );
}

/// What a Drone said got in its way, in the order it said it, under the step
/// it said it on.
#[test]
fn what_a_drone_said_got_in_the_way_is_kept_in_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = on_its_first_run(&mut store, "01RETRONOTE");
    let step = StepId::new("fix");
    store
        .record_drone_note(
            job.id(),
            &step,
            "the asked run log was cut",
            &at("2026-08-26T10:03:00.000Z"),
        )
        .expect("kept");
    store
        .record_drone_note(
            job.id(),
            &step,
            "grep was refused",
            &at("2026-08-26T10:04:00.000Z"),
        )
        .expect("kept");

    let notes = store.drone_notes_for(job.id()).expect("read back");
    let said: Vec<&str> = notes.iter().map(|note| note.said.as_str()).collect();
    assert_eq!(said, vec!["the asked run log was cut", "grep was refused"]);
    assert_eq!(notes[0].step_id, step);
}

/// **An ended Job is owed a retro until one is kept**, and the items read back
/// across Jobs newest first — the order the Lessons page lists them in.
#[test]
fn an_ended_job_is_owed_a_retro_until_one_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = ended(&mut store, "01RETROFIRST", "2026-08-26T10:00:00.000Z");
    let second = ended(&mut store, "01RETROSECOND", "2026-08-26T11:00:00.000Z");
    on_its_first_run(&mut store, "01RETRORUNNING");

    assert_eq!(
        store.retros_owed().expect("read"),
        vec![first.id().clone(), second.id().clone()],
        "both ended Jobs, oldest first, and never the one still running"
    );

    store
        .record_retro(
            first.id(),
            &Reflected::Written {
                model: "the-cheap-model".to_string(),
                items: vec![
                    line(Whose::Drone, "the log was cut", &["refusal:1"]),
                    line(Whose::Owner, "asked twice", &["question:1", "question:2"]),
                ],
            },
            &at("2026-08-26T12:00:00.000Z"),
        )
        .expect("kept");
    store
        .record_retro(
            second.id(),
            &Reflected::Written {
                model: "the-cheap-model".to_string(),
                items: vec![line(
                    Whose::Fleet,
                    "a gate refused a docs change",
                    &["check:1"],
                )],
            },
            &at("2026-08-26T13:00:00.000Z"),
        )
        .expect("kept");

    assert!(store.retros_owed().expect("read").is_empty());
    let kept = store.retro_for(first.id()).expect("read").expect("a retro");
    assert_eq!(
        kept.reflected,
        Reflected::Written {
            model: "the-cheap-model".to_string(),
            items: vec![
                line(Whose::Drone, "the log was cut", &["refusal:1"]),
                line(Whose::Owner, "asked twice", &["question:1", "question:2"]),
            ],
        }
    );
    let newest: Vec<(String, &str)> = store
        .lessons(10, None, None)
        .expect("read")
        .iter()
        .map(|lesson| {
            (
                lesson.job_id.as_str().to_string(),
                lesson.line.whose.as_wire(),
            )
        })
        .collect();
    assert_eq!(
        newest,
        vec![
            ("01RETROSECOND".to_string(), "fleet"),
            ("01RETROFIRST".to_string(), "drone"),
            ("01RETROFIRST".to_string(), "owner"),
        ],
        "newest retro first, and a retro's own items in the order written"
    );
}

/// **A failed call is kept, and not owed again.** A retry on every turn would
/// spend on a quota that is already gone.
#[test]
fn a_retro_that_could_not_be_written_is_not_owed_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01RETROFAILED", "2026-08-26T10:00:00.000Z");
    store
        .record_retro(
            job.id(),
            &Reflected::Failed {
                why: "the model said nothing".to_string(),
            },
            &at("2026-08-26T10:01:00.000Z"),
        )
        .expect("kept");

    assert!(store.retros_owed().expect("read").is_empty());
    assert!(store.lessons(10, None, None).expect("read").is_empty());
}

/// **A Job that ended before retros existed is owed none.** Without this, the
/// first turn after the migration would spend a model call on every Job the
/// file has ever finished.
#[test]
fn a_job_that_ended_before_retros_existed_is_owed_none() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("a file to put version 1 in");
    conn.execute_batch(MIGRATIONS[0].sql).expect("version 1");
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '1')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 1");
    conn.execute(
        "INSERT INTO jobs (
             job_id, status, workflow_id, owner_manifest_id, origin, urgency, atomic,
             model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES ('01ENDEDLONGAGO', 'killed', '01WORKFLOW', '01OWNERMANIFEST', 'manual',
                   'normal', 0, 'a-model-name', '[]', '[]', '', '[]', 0,
                   '2026-08-26T09:00:00.000Z')",
        [],
    )
    .expect("a Job as version 1 wrote them");
    drop(conn);

    let store = Store::open(&dir.db()).expect("migrated");

    assert!(store.retros_owed().expect("read").is_empty());
    let kept = store
        .retro_for(&job_id("01ENDEDLONGAGO"))
        .expect("read")
        .expect("marked");
    assert!(
        matches!(kept.reflected, Reflected::Skipped { .. }),
        "{:?}",
        kept.reflected
    );
}

/// Forgetting a Job takes its retro, its notes and its doors with it, and
/// counts them.
#[test]
fn forgetting_a_job_takes_its_retro() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01RETROFORGET", "2026-08-26T10:00:00.000Z");
    store
        .record_drone_note(
            job.id(),
            &StepId::new("fix"),
            "slow",
            &at("2026-08-26T09:59:00.000Z"),
        )
        .expect("kept");
    store
        .record_retro(
            job.id(),
            &Reflected::Written {
                model: "m".to_string(),
                items: vec![line(Whose::Drone, "slow", &["note:1"])],
            },
            &at("2026-08-26T10:01:00.000Z"),
        )
        .expect("kept");

    let forgotten = store.forget_job(job.id()).expect("forgotten");

    assert_eq!(forgotten.retros, 3, "the note, the retro and its one item");
    assert_eq!(forgotten.other, 0);
}

/// **Where a fix lands narrows the Lessons**, and is kept with the item.
#[test]
fn the_lessons_narrow_to_where_a_fix_lands() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01RETROLANDS", "2026-08-26T10:00:00.000Z");
    let kit = RetroLine {
        lands_in: Some(LandsIn::Kit),
        ..line(Whose::Drone, "grep was refused", &["refusal:1"])
    };
    let manifest = RetroLine {
        lands_in: Some(LandsIn::Manifest),
        ..line(Whose::Fleet, "a docs edit ran every test", &["check:1"])
    };
    store
        .record_retro(
            job.id(),
            &Reflected::Written {
                model: "m".to_string(),
                items: vec![kit.clone(), manifest.clone()],
            },
            &at("2026-08-26T10:01:00.000Z"),
        )
        .expect("kept");

    let narrowed: Vec<RetroLine> = store
        .lessons(10, Some(LandsIn::Kit), None)
        .expect("read")
        .into_iter()
        .map(|lesson| lesson.line)
        .collect();
    assert_eq!(narrowed, vec![kit.clone()]);
    assert_eq!(store.lessons(10, None, None).expect("read").len(), 2);
}

/// **An item kept before V102 reads with `lands_in` absent**, never guessed,
/// and is listed only where nothing narrows the Lessons.
#[test]
fn an_item_kept_before_lands_in_reads_with_it_absent() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("a file");
    for migration in &MIGRATIONS[..101] {
        conn.execute_batch(migration.sql).expect("a migration");
    }
    conn.execute_batch(&format!(
        "INSERT INTO armada_meta (key, value) VALUES ('{SCHEMA_VERSION_KEY}', '101');
         INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES ('01RETROOLD', 'a job V101 reflected on', 'killed', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-08-26T09:00:00.000Z');
         INSERT INTO job_retros (job_id, state, model, at)
             VALUES ('01RETROOLD', 'written', 'm', '2026-08-26T10:01:00.000Z');
         INSERT INTO job_retro_items (job_id, ordinal, whose, said, evidence)
             VALUES ('01RETROOLD', 0, 'drone', 'grep was refused', 'refusal:1');"
    ))
    .expect("a retro as V101 kept it");
    drop(conn);

    let store = Store::open(&dir.db()).expect("migrated");

    let old = RetroLine {
        lands_in: None,
        ..line(Whose::Drone, "grep was refused", &["refusal:1"])
    };
    let kept = store
        .retro_for(&job_id("01RETROOLD"))
        .expect("read")
        .expect("kept");
    assert_eq!(
        kept.reflected,
        Reflected::Written {
            model: "m".to_string(),
            items: vec![old.clone()],
        }
    );
    let all: Vec<RetroLine> = store
        .lessons(10, None, None)
        .expect("read")
        .into_iter()
        .map(|lesson| lesson.line)
        .collect();
    assert_eq!(all, vec![old]);
    assert!(
        store
            .lessons(10, None, None)
            .expect("read")
            .iter()
            .all(|lesson| lesson.line.change.is_none() && !lesson.applied),
        "V110 gives an item kept before it no change and nothing applied"
    );
    for place in LandsIn::ALL {
        assert!(
            store
                .lessons(10, Some(*place), None)
                .expect("read")
                .is_empty(),
            "an old item is under no place: {place:?}"
        );
    }
    let old = store
        .lesson(&job_id("01RETROOLD"), 0)
        .expect("read")
        .expect("held");
    assert_eq!(
        old.state,
        LessonState::Open,
        "an item kept before V105 is open"
    );
    assert_eq!(old.job_proposed, None);
    assert_eq!(old.line.title, None);
}
