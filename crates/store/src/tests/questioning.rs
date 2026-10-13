//! A Drone's question survives a reopen unanswered, and again answered, and
//! delivering it clears every column. The hours before a person answers are the
//! ordinary case for a Fleet restart, not an exotic one.

use core_model::{Decision, DroneQuestion, GivenAnswer, OfferedAnswer, StepId, Ulid};

use crate::tests::{at, job_id, open, top_level, TempDir};

fn asked() -> DroneQuestion {
    DroneQuestion {
        id: Ulid::carried("01QUESTION"),
        step: StepId::new("implement"),
        asked_at: at("2026-10-11T01:00:00.000Z"),
        question: String::from("Keep the old setting name, or rename it everywhere?"),
        options: vec![
            OfferedAnswer {
                label: String::from("Keep it"),
                consequence: String::from("nothing outside this crate changes"),
            },
            OfferedAnswer {
                label: String::from("Rename it"),
                consequence: String::from("every reader of the setting changes with it"),
            },
        ],
        answer: None,
        decisions: Vec::new(),
    }
}

fn an_interview() -> DroneQuestion {
    let offered = |label: &str| OfferedAnswer {
        label: String::from(label),
        consequence: format!("{label} is what gets built"),
    };
    DroneQuestion {
        question: String::new(),
        options: Vec::new(),
        decisions: vec![
            Decision {
                id: String::from("d1"),
                question: String::from("Which store?"),
                options: vec![offered("SQLite"), offered("Files")],
                chose: None,
            },
            Decision {
                id: String::from("d2"),
                question: String::from("Which order?"),
                options: vec![offered("Reader first"), offered("Writer first")],
                chose: None,
            },
        ],
        ..asked()
    }
}

#[test]
fn a_question_survives_a_reopen_asked_and_answered_and_clears_on_delivery() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01ASKED");
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");

    let asking = job.asking(asked());
    store
        .record_question(&asking)
        .expect("the question is kept");
    drop(store);

    let mut store = open(&dir);
    let read = store.load_job(&job_id("01ASKED")).expect("it reads back");
    assert_eq!(
        read.question(),
        Some(&asked()),
        "asked, whole, after a reopen"
    );
    assert!(read.waiting_on_an_answer());

    let answered = read.question_answered(GivenAnswer {
        label: Some(String::from("Rename it")),
        deciding: None,
        at: at("2026-10-11T08:30:00.000Z"),
    });
    store
        .record_question(&answered)
        .expect("the answer is kept");
    drop(store);

    let mut store = open(&dir);
    let read = store.load_job(&job_id("01ASKED")).expect("it reads back");
    assert!(!read.waiting_on_an_answer(), "answered, so nothing waits");
    assert_eq!(
        read.question()
            .and_then(DroneQuestion::chosen)
            .map(|o| o.label.as_str()),
        Some("Rename it"),
        "and the answer is still there for the next Drone's brief"
    );

    store
        .record_question(&read.question_delivered())
        .expect("the delivery is kept");
    let read = store.load_job(&job_id("01ASKED")).expect("it reads back");
    assert_eq!(
        read, job,
        "delivered, and the record is back where it started"
    );
}

/// An interview is several decisions on the one row. Asked, they come back whole
/// after a reopen; answered, each decision keeps its own pick.
#[test]
fn an_interview_survives_a_reopen_asked_and_answered() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01DECIDED");
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
    store
        .record_question(&job.asking(an_interview()))
        .expect("the interview is kept");
    drop(store);

    let mut store = open(&dir);
    let read = store.load_job(&job_id("01DECIDED")).expect("it reads back");
    assert_eq!(read.question(), Some(&an_interview()), "asked, whole");
    assert!(read.waiting_on_an_answer());

    let picks = [
        (String::from("d1"), String::from("Files")),
        (String::from("d2"), String::from("Reader first")),
    ];
    store
        .record_question(&read.question_decided(&picks, at("2026-10-11T08:30:00.000Z")))
        .expect("the answers are kept");
    drop(store);

    let mut store = open(&dir);
    let read = store.load_job(&job_id("01DECIDED")).expect("it reads back");
    assert!(!read.waiting_on_an_answer(), "answered, so nothing waits");
    let held = read.question().expect("still held for the next brief");
    assert_eq!(
        held.decision("d1").and_then(|d| d.chose.as_deref()),
        Some("Files")
    );
    assert_eq!(
        held.decision("d2").and_then(|d| d.chose.as_deref()),
        Some("Reader first")
    );

    store
        .record_question(&read.question_delivered())
        .expect("the delivery is kept");
    let read = store.load_job(&job_id("01DECIDED")).expect("it reads back");
    assert_eq!(read, job, "delivered, and every column is clear");
}
