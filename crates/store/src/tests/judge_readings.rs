//! A Job's Judge readings: one a `(kind, artifact)`, replaced whole when asked again, the oldest asked
//! read first, a note settled once and a reading marked read.

use crate::tests::{open, TempDir};
use crate::{KeptJudgeNote, KeptJudgeReading};

fn note(id: &str, about: Option<&str>) -> KeptJudgeNote {
    KeptJudgeNote {
        id: id.into(),
        about: about.map(str::to_string),
        claim: "Two rotations can land on one day.".into(),
        place: about.map(|part| format!("Plan, task {part}")),
        expected: Some("A lock.".into()),
        found: None,
        impact: Some("Someone rotating their keys finds two on one day.".into()),
        cites: vec!["crates/store/src/keys.rs:2".into(), "crates/store/src/lib.rs".into()],
        settled: None,
    }
}

fn reading(id: &str, kind: &str, artifact: &str, asked: &str, state: &str, notes: Vec<KeptJudgeNote>) -> KeptJudgeReading {
    KeptJudgeReading {
        id: id.into(),
        kind: kind.into(),
        artifact_id: artifact.into(),
        title: format!("The {kind}"),
        prompt: String::new(),
        state: state.into(),
        asked_at: asked.into(),
        finished_at: (state != "reading").then(|| "2026-10-31T09:01:00.000Z".into()),
        failed: (state == "failed").then(|| "the answer was refused".into()),
        read: false,
        cost_micros: Some(400_000),
        notes,
    }
}

#[test]
fn a_reading_is_kept_per_artifact_replaced_whole_and_read_back_oldest_asked_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.judge_readings("s1").unwrap(), Vec::new());

    let plan = reading("r1", "plan", "2", "2026-10-31T09:00:00.000Z", "done", vec![note("n1", Some("t2")), note("n2", None)]);
    let pull = reading("r2", "pull_request", "812", "2026-10-31T09:05:00.000Z", "reading", Vec::new());
    store.keep_judge_reading("s1", &pull).unwrap();
    store.keep_judge_reading("s1", &plan).unwrap();
    assert_eq!(store.judge_readings("s1").unwrap(), vec![plan.clone(), pull.clone()], "it reads back as it was kept, oldest asked first");
    assert_eq!(store.judge_reading("s1", "r1").unwrap(), Some(plan.clone()));
    assert_eq!(store.judge_reading_of("s1", "pull_request", "812").unwrap(), Some(pull.clone()));

    // Asked again for the plan: a new id, and the old reading and its notes are gone.
    let again = reading("r3", "plan", "2", "2026-10-31T09:10:00.000Z", "failed", Vec::new());
    store.keep_judge_reading("s1", &again).unwrap();
    assert_eq!(store.judge_readings("s1").unwrap(), vec![pull, again.clone()]);
    assert_eq!(store.judge_reading("s1", "r1").unwrap(), None, "a reading asked again is gone");
    assert_eq!(store.judge_reading_of("s1", "plan", "2").unwrap(), Some(again));
    assert_eq!(store.judge_readings("s2").unwrap(), Vec::new(), "another Job's are its own");
}

#[test]
fn a_note_is_settled_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_judge_reading("s1", &reading("r1", "plan", "1", "2026-10-31T09:00:00.000Z", "done", vec![note("n1", Some("g1"))])).unwrap();

    assert!(store.settle_judge_note("s1", "r1", "n1", "sent", "2026-10-31T09:02:00.000Z").unwrap());
    assert!(!store.settle_judge_note("s1", "r1", "n1", "dismissed", "2026-10-31T09:03:00.000Z").unwrap(), "settled once");
    assert!(!store.settle_judge_note("s1", "r1", "n9", "sent", "2026-10-31T09:03:00.000Z").unwrap(), "no such note");
    assert!(!store.settle_judge_note("s2", "r1", "n1", "sent", "2026-10-31T09:03:00.000Z").unwrap(), "another Job's reading");
    let kept = store.judge_reading("s1", "r1").unwrap().expect("kept");
    assert_eq!(kept.notes[0].settled, Some(("sent".into(), "2026-10-31T09:02:00.000Z".into())));
}

#[test]
fn a_reading_is_marked_read_and_asking_again_clears_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_judge_reading("s1", &reading("r1", "doc", "notes.html", "2026-10-31T09:00:00.000Z", "done", vec![note("n1", Some("p:2"))])).unwrap();

    assert!(store.mark_judge_read("s1", "r1").unwrap());
    assert!(!store.mark_judge_read("s1", "r9").unwrap(), "no such reading");
    assert!(!store.mark_judge_read("s2", "r1").unwrap(), "another Job's reading");
    assert!(store.judge_reading("s1", "r1").unwrap().expect("kept").read);

    store.keep_judge_reading("s1", &reading("r2", "doc", "notes.html", "2026-10-31T09:05:00.000Z", "done", Vec::new())).unwrap();
    assert!(!store.judge_reading_of("s1", "doc", "notes.html").unwrap().expect("kept").read, "a fresh reading is unread");
}
