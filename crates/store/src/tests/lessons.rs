//! What a person's answer to a Session's retro item keeps: the state, the
//! Session started for it, and the headline, what and fix an item is written
//! with. `docs/concepts/retro.md`.

use core_model::{Change, LandsIn, LessonState, Whose};

use crate::tests::{at, open, TempDir};
use crate::{RetroLine, Store};

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

fn written(store: &mut Store, session: &str, items: Vec<RetroLine>) -> i64 {
    store
        .record_session_retro(
            session,
            None,
            "01END",
            "m",
            None,
            &items,
            &at("2026-08-26T10:01:00.000Z"),
        )
        .expect("kept")
}

/// **An item answers once, and the answer is the claim**: the second press on
/// an `open` item finds it taken, so a Session is started once however many
/// press. The listing narrows by state.
#[test]
fn an_item_is_answered_once_and_the_lessons_narrow_by_state() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let retro = written(
        &mut store,
        "01LESSONSTATE",
        vec![line("grep was refused"), line("a stale base")],
    );
    let with_state =
        |store: &Store, state| store.session_lessons(10, None, Some(state)).expect("read");
    assert_eq!(
        with_state(&store, LessonState::Open).len(),
        2,
        "every item starts open"
    );

    assert!(store
        .answer_session_lesson(retro, 0, LessonState::Discarded)
        .expect("answered"));
    assert!(
        !store
            .answer_session_lesson(retro, 0, LessonState::Agreed)
            .expect("answered"),
        "the first answer stands"
    );

    let standing = store.session_lesson(retro, 0).expect("read").expect("held");
    assert_eq!(standing.state, LessonState::Discarded);
    assert_eq!(with_state(&store, LessonState::Open).len(), 1);
    assert_eq!(with_state(&store, LessonState::Discarded).len(), 1);
    assert!(store.session_lesson(retro, 7).expect("read").is_none());
}

/// **A Session that exists is never forgotten**: an agreed item with no
/// Session is given back, and one that has a Session is left alone.
#[test]
fn an_agreed_item_with_no_session_is_given_back_and_one_with_a_session_is_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let retro = written(&mut store, "01LESSONGIVEN", vec![line("a stale base")]);

    assert!(store
        .answer_session_lesson(retro, 0, LessonState::Agreed)
        .expect("answered"));
    store.reopen_session_lesson(retro, 0).expect("given back");
    let given_back = store.session_lesson(retro, 0).expect("read").expect("held");
    assert_eq!(given_back.state, LessonState::Open);

    assert!(store
        .answer_session_lesson(retro, 0, LessonState::Agreed)
        .expect("answered"));
    store
        .keep_session_lesson_session(retro, 0, "01PROPOSEDSESSION")
        .expect("kept");
    store.reopen_session_lesson(retro, 0).expect("left alone");
    let agreed = store.session_lesson(retro, 0).expect("read").expect("held");
    assert_eq!(agreed.state, LessonState::Agreed);
    assert_eq!(agreed.session_proposed.as_deref(), Some("01PROPOSEDSESSION"));
}

/// **A change is kept with its item, accepting applies it once**, and an item
/// with no change is never marked applied.
#[test]
fn a_change_is_kept_and_accepting_applies_it_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
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
    let retro = written(
        &mut store,
        "01LESSONCHANGE",
        vec![changing.clone(), plain],
    );

    let kept = store.session_lesson(retro, 0).expect("read").expect("held");
    assert_eq!(kept.line, changing, "the change is kept with the item");
    assert!(!kept.applied, "nothing is applied until Accept");

    assert!(
        store
            .accept_session_lesson_applied(retro, 0)
            .expect("claimed"),
        "the first press claims it"
    );
    assert!(
        !store
            .accept_session_lesson_applied(retro, 0)
            .expect("claimed"),
        "the second press finds it taken"
    );
    let accepted = store.session_lesson(retro, 0).expect("read").expect("held");
    assert_eq!(accepted.state, LessonState::Accepted);
    assert!(accepted.applied);

    assert!(
        !store
            .accept_session_lesson_applied(retro, 1)
            .expect("claimed"),
        "an item with no change is not one this claims"
    );
    let untouched = store.session_lesson(retro, 1).expect("read").expect("held");
    assert_eq!(untouched.state, LessonState::Open);
    assert!(!untouched.applied);
}

/// **The headline, what and fix are kept with the item.**
#[test]
fn the_title_what_and_fix_are_kept_with_the_item() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let item = RetroLine {
        title: Some("Gate measured a stale main".to_string()),
        what: Some("Two upstream commits counted as the Session's work.".to_string()),
        fix: Some("Fetch main before the gate measures.".to_string()),
        ..line("Two upstream commits counted as the Session's work.")
    };
    let retro = written(&mut store, "01LESSONTEXTS", vec![item.clone()]);

    let kept = store.session_lesson(retro, 0).expect("read").expect("held");
    assert_eq!(kept.line, item);
    assert_eq!(kept.state, LessonState::Open);
    assert_eq!(kept.session_proposed, None);
}
