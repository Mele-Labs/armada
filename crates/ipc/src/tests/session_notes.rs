//! A pinned note on the wire: `SendSessionNote` keeps its flat shape now that it is built on
//! `SessionNote`, and `SendSessionNotes` and `NotesSent` round-trip.

use crate::{Candidate, NoteElement, NotePlace, NoteState, SendSessionNote, SendSessionNotes, SessionId, SessionNote};

const ONE: &str = r#"{"session_id":"s1","said":"too wide","address":"http://127.0.0.1:5173/","note_id":"n-1","place":{"selector":"li","element":{"tag":"li","text":"x"},"location":"/usage"},"state":{},"crop":{"media_type":"image/png","data":"aGk="}}"#;

#[test]
fn a_single_note_is_flat_on_the_wire_as_it_was_before_the_batch() {
    let note: SendSessionNote = crate::decode("a note", ONE.as_bytes()).unwrap();
    assert_eq!(note.session_id, SessionId::carried("s1"));
    assert_eq!((note.note.said.as_str(), note.note.note_id.as_deref()), ("too wide", Some("n-1")));
    assert_eq!(crate::encode(&note).unwrap(), ONE, "the same bytes back, with no `note` key");
}

#[test]
fn a_batch_of_notes_and_its_answer_round_trip_in_order() {
    let note = |said: &str| SessionNote {
        said: said.to_string(),
        address: "http://127.0.0.1:5173/".to_string(),
        note_id: None,
        place: NotePlace {
            selector: "li".to_string(),
            element: NoteElement { tag: "li".to_string(), text: "x".to_string(), label: None },
            location: "/".to_string(),
            ..NotePlace::default()
        },
        state: NoteState::default(),
        crop: None,
    };
    let batch = SendSessionNotes { session_id: SessionId::carried("s1"), notes: vec![note("one"), note("two")] };
    let wire = crate::encode(&batch).unwrap();
    assert!(wire.starts_with(r#"{"session_id":"s1","notes":[{"said":"one","#), "{wire}");
    let back: SendSessionNotes = crate::decode("notes", wire.as_bytes()).unwrap();
    assert_eq!(back, batch);

    let lines = vec![vec![Candidate { file: "a.tsx".into(), line: 3, why: "w".into(), text: "t".into() }], Vec::new()];
    let wire = crate::encode(&lines).unwrap();
    assert_eq!(wire, r#"[[{"file":"a.tsx","line":3,"why":"w","text":"t"}],[]]"#, "one list a note, an empty one kept");
}
