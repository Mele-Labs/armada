//! A Job's walk notes: kept oldest first, removed only while unsent, marked
//! sent once, and never edited.

use crate::tests::{at, created_at, open, top_level, TempDir};
use crate::{KeptWalkNote, WalkNoteRemoved, WalkServed};

fn note(id: &str, said: &str, when: &str) -> KeptWalkNote {
    KeptWalkNote {
        note_id: id.to_string(),
        said: said.to_string(),
        at: at(when),
        element: "button “Save”".to_string(),
        selector: "main > button.save".to_string(),
        location: "/settings".to_string(),
        served: Some(WalkServed {
            run: "01RUN".to_string(),
            name: "mock".to_string(),
            address: "http://127.0.0.1:5173".to_string(),
        }),
        frame: Some("/machine/walks/01JOB/01A.png".to_string()),
        sent: false,
    }
}

#[test]
fn walk_notes_come_back_oldest_first_and_are_marked_sent_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01WALKED");
    store.insert_job(&job, &created_at()).expect("inserted");
    let later = note("01B", "the header is too tall", "2026-10-04T10:02:00.000Z");
    let first = note(
        "01A",
        "this button is too small",
        "2026-10-04T10:01:00.000Z",
    );
    store.keep_walk_note(job.id(), &later).expect("kept");
    store.keep_walk_note(job.id(), &first).expect("kept");

    let kept = store.walk_notes(job.id()).expect("read");
    assert_eq!(kept, vec![first.clone(), later.clone()]);

    store
        .mark_walk_notes_sent(
            job.id(),
            &["01A".to_string()],
            &at("2026-10-04T10:03:00.000Z"),
        )
        .expect("marked");
    // Marking it again moves nothing and does not trip the trigger.
    store
        .mark_walk_notes_sent(
            job.id(),
            &["01A".to_string()],
            &at("2026-10-04T10:04:00.000Z"),
        )
        .expect("a second mark is a no-op");
    let kept = store.walk_notes(job.id()).expect("read");
    assert!(kept[0].sent && !kept[1].sent, "{kept:?}");

    assert_eq!(
        store.remove_walk_note(job.id(), "01A").expect("answered"),
        WalkNoteRemoved::AlreadySent
    );
    assert_eq!(
        store
            .remove_walk_note(job.id(), "01NOPE")
            .expect("answered"),
        WalkNoteRemoved::NoSuchNote
    );
    assert_eq!(
        store.remove_walk_note(job.id(), "01B").expect("answered"),
        WalkNoteRemoved::Removed {
            frame: later.frame.clone()
        }
    );
    assert_eq!(store.walk_notes(job.id()).expect("read").len(), 1);
}

#[test]
fn a_walk_note_is_never_edited() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01WALKED");
    store.insert_job(&job, &created_at()).expect("inserted");
    store
        .keep_walk_note(
            job.id(),
            &note("01A", "too small", "2026-10-04T10:01:00.000Z"),
        )
        .expect("kept");
    let edited = store.conn.execute(
        "UPDATE job_walk_notes SET said = 'something else' WHERE note_id = '01A'",
        (),
    );
    assert!(edited.is_err(), "the trigger refuses an edit");
}
