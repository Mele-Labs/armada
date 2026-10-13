//! A Commander's card outlives the turn that raised it, and a Job has one out
//! at a time. `#2166`.

use core_model::Timestamp;

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{CardAnswered, CardRaised, CardRaiser, NewCard, Store, WriteError};

const JOB: &str = "01COMMANDER00000000000003";

fn a_job(store: &mut Store) {
    store
        .insert_job(&top_level(JOB), &created_at())
        .expect("the job is stored");
}

fn card(id: &str) -> NewCard {
    NewCard {
        card_id: id.to_string(),
        raised_by: CardRaiser::Commander,
        raised_at: Timestamp::from_rfc3339("2026-10-13T09:00:00.000Z"),
        text: "Group 3 is red on desktop_test".to_string(),
        reach: "A Check is the owner's to skip".to_string(),
        question: "Skip it?".to_string(),
        options: vec!["Skip desktop_test".to_string(), "Leave it".to_string()],
    }
}

#[test]
fn a_card_is_out_until_it_is_answered_and_is_still_out_after_a_restart() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        a_job(&mut store);
        assert_eq!(
            store.raise_commander_card(&job_id(JOB), &card("c1")).expect("raised"),
            CardRaised::Out
        );
    }
    let mut reopened = open(&dir);
    let out = reopened
        .open_commander_card(&job_id(JOB))
        .expect("reads")
        .expect("the card survived the restart");
    assert_eq!(out.card_id, "c1");
    assert_eq!(out.options, ["Skip desktop_test", "Leave it"]);
    assert_eq!(out.chose, None);

    let at = Timestamp::from_rfc3339("2026-10-13T11:00:00.000Z");
    let answered = reopened
        .answer_commander_card(&job_id(JOB), "c1", "1", &at)
        .expect("answered");
    let CardAnswered::Recorded(card) = answered else {
        panic!("{answered:?}");
    };
    assert_eq!(card.label_of("1"), Some("Skip desktop_test"));
    assert_eq!(card.chose.as_deref(), Some("1"));
    assert_eq!(reopened.open_commander_card(&job_id(JOB)).expect("reads"), None);
}

#[test]
fn a_job_has_one_card_out_and_a_second_is_told_which() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store.raise_commander_card(&job_id(JOB), &card("c1")).expect("raised");

    let second = store
        .raise_commander_card(&job_id(JOB), &card("c2"))
        .expect("answered");

    let CardRaised::AlreadyOut(out) = second else {
        panic!("{second:?}");
    };
    assert_eq!(out.card_id, "c1", "the card out is left as it was");
    assert_eq!(store.open_commander_cards().expect("reads").len(), 1);
}

#[test]
fn an_answer_to_a_card_that_is_not_the_one_out_is_not_recorded() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store.raise_commander_card(&job_id(JOB), &card("c1")).expect("raised");
    let at = Timestamp::from_rfc3339("2026-10-13T11:00:00.000Z");

    assert_eq!(
        store.answer_commander_card(&job_id(JOB), "c0", "1", &at).expect("answered"),
        CardAnswered::NotOut,
        "a card id that is not the one out"
    );
    assert!(matches!(
        store.answer_commander_card(&job_id(JOB), "c1", "3", &at).expect("answered"),
        CardAnswered::NoSuchOption(_)
    ));
    assert!(matches!(
        store.answer_commander_card(&job_id(JOB), "c1", "0", &at).expect("answered"),
        CardAnswered::NoSuchOption(_)
    ));
    store.answer_commander_card(&job_id(JOB), "c1", "2", &at).expect("answered");
    assert_eq!(
        store.answer_commander_card(&job_id(JOB), "c1", "2", &at).expect("answered"),
        CardAnswered::NotOut,
        "answered once"
    );
    assert_eq!(
        store.raise_commander_card(&job_id(JOB), &card("c2")).expect("raised"),
        CardRaised::Out,
        "an answered card leaves room for the next"
    );
}

#[test]
fn a_card_for_a_job_that_is_not_there_is_refused_and_a_forgotten_job_takes_its_cards() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(matches!(
        store.raise_commander_card(&job_id(JOB), &card("c1")),
        Err(WriteError::NoSuchJob { .. })
    ));
    a_job(&mut store);
    store.raise_commander_card(&job_id(JOB), &card("c1")).expect("raised");
    store.forget_job(&job_id(JOB)).expect("forgotten");
    assert!(store.open_commander_cards().expect("reads").is_empty());
}
