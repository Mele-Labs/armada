//! What a person's answer to a retro item keeps: the state, the Job proposed,
//! and the headline, what and fix an item is written with. `docs/concepts/retro.md`.

use core_model::{Actor, Change, Job, LandsIn, LessonState, Target, Whose};

use crate::tests::{at, created_at, job_id, open, top_level, TempDir};
use crate::{Reflected, RetroLine, Store};

fn ended(store: &mut Store, id: &str) -> Job {
    let created = top_level(id);
    store
        .insert_job(&created, &created_at())
        .expect("the job is stored");
    let moved = created
        .transition(Target::Killed, Actor::Human, at("2026-08-26T10:00:00.000Z"))
        .expect("a person may stop a Job at the gate");
    store.record_transition(&moved).expect("recorded");
    moved.job
}

fn line(said: &str) -> RetroLine {
    RetroLine {
        whose: Whose::Fleet,
        title: None,
        what: None,
        fix: None,
        said: said.to_string(),
        evidence: vec!["check:1".to_string()],
        lands_in: Some(LandsIn::Armada),
        change: None,
    }
}

fn written(store: &mut Store, job: &Job, items: Vec<RetroLine>) {
    store
        .record_retro(
            job.id(),
            &Reflected::Written {
                model: "m".to_string(),
                items,
            },
            &at("2026-08-26T10:01:00.000Z"),
        )
        .expect("kept");
}

/// **An item answers once, and the answer is the claim**: the second press on
/// an `open` item finds it taken, so a Job is proposed once however many
/// press. The listing narrows by state.
#[test]
fn an_item_is_answered_once_and_the_lessons_narrow_by_state() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01LESSONSTATE");
    written(
        &mut store,
        &job,
        vec![line("grep was refused"), line("a stale base")],
    );
    let with_state = |store: &Store, state| store.lessons(10, None, Some(state)).expect("read");
    assert_eq!(
        with_state(&store, LessonState::Open).len(),
        2,
        "every item starts open"
    );

    assert!(store
        .answer_lesson(job.id(), 0, LessonState::Discarded)
        .expect("answered"));
    assert!(
        !store
            .answer_lesson(job.id(), 0, LessonState::Agreed)
            .expect("answered"),
        "the first answer stands"
    );

    let standing = store.lesson(job.id(), 0).expect("read").expect("held");
    assert_eq!(standing.state, LessonState::Discarded);
    assert_eq!(with_state(&store, LessonState::Open).len(), 1);
    assert_eq!(with_state(&store, LessonState::Discarded).len(), 1);
    assert!(store.lesson(job.id(), 7).expect("read").is_none());
}

/// **A Job that exists is never forgotten**: an agreed item with no Job is
/// given back, and one that has a Job is left alone.
#[test]
fn an_agreed_item_with_no_job_is_given_back_and_one_with_a_job_is_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01LESSONGIVEN");
    written(&mut store, &job, vec![line("a stale base")]);

    assert!(store
        .answer_lesson(job.id(), 0, LessonState::Agreed)
        .expect("answered"));
    store.reopen_lesson(job.id(), 0).expect("given back");
    let given_back = store.lesson(job.id(), 0).expect("read").expect("held");
    assert_eq!(given_back.state, LessonState::Open);

    assert!(store
        .answer_lesson(job.id(), 0, LessonState::Agreed)
        .expect("answered"));
    store
        .keep_lesson_job(job.id(), 0, &job_id("01PROPOSEDJOB"))
        .expect("kept");
    store.reopen_lesson(job.id(), 0).expect("left alone");
    let agreed = store.lesson(job.id(), 0).expect("read").expect("held");
    assert_eq!(agreed.state, LessonState::Agreed);
    assert_eq!(
        agreed.job_proposed.map(|id| id.as_str().to_string()),
        Some("01PROPOSEDJOB".to_string())
    );
}

/// **A change is kept with its item, accepting applies it once**, and an item
/// with no change is never marked applied. V110.
#[test]
fn a_change_is_kept_and_accepting_applies_it_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01LESSONCHANGE");
    let changing = RetroLine {
        lands_in: Some(LandsIn::Kit),
        change: Some(Change::AllowCommand {
            command: "grep -a -c".to_string(),
        }),
        ..line("grep was refused")
    };
    let plain = RetroLine {
        lands_in: Some(LandsIn::Kit),
        ..line("a skill was missing")
    };
    written(&mut store, &job, vec![changing.clone(), plain]);

    let kept = store.lesson(job.id(), 0).expect("read").expect("held");
    assert_eq!(kept.line, changing, "the change is kept with the item");
    assert!(!kept.applied, "nothing is applied until Accept");

    assert!(
        store.accept_lesson_applied(job.id(), 0).expect("claimed"),
        "the first press claims it"
    );
    assert!(
        !store.accept_lesson_applied(job.id(), 0).expect("claimed"),
        "the second press finds it taken"
    );
    let accepted = store.lesson(job.id(), 0).expect("read").expect("held");
    assert_eq!(accepted.state, LessonState::Accepted);
    assert!(accepted.applied);

    assert!(
        !store.accept_lesson_applied(job.id(), 1).expect("claimed"),
        "an item with no change is not one this claims"
    );
    let untouched = store.lesson(job.id(), 1).expect("read").expect("held");
    assert_eq!(untouched.state, LessonState::Open);
    assert!(!untouched.applied);
}

/// **The headline, what and fix are kept with the item.**
#[test]
fn the_title_what_and_fix_are_kept_with_the_item() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = ended(&mut store, "01LESSONTEXTS");
    let item = RetroLine {
        title: Some("Gate measured a stale main".to_string()),
        what: Some("Two upstream commits counted as the Drone's work.".to_string()),
        fix: Some("Fetch main before the gate measures.".to_string()),
        ..line("Two upstream commits counted as the Drone's work.")
    };
    written(&mut store, &job, vec![item.clone()]);

    let kept = store.lesson(job.id(), 0).expect("read").expect("held");
    assert_eq!(kept.line, item);
    assert_eq!(kept.state, LessonState::Open);
    assert_eq!(kept.job_proposed, None);
}
