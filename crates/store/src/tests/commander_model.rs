//! The model and effort chosen for one Job's Commander. `#2166`.

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{CommanderChoice, LoadJobError, WriteError};

const JOB: &str = "01COMMANDER00000000000004";

#[test]
fn a_job_nobody_chose_for_has_no_choice_and_a_choice_is_read_back_and_cleared() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.insert_job(&top_level(JOB), &created_at()).expect("stored");
    assert_eq!(store.commander_choice(&job_id(JOB)).expect("reads"), CommanderChoice::default());

    let chosen = CommanderChoice {
        model: Some("a-model".to_string()),
        effort: Some("harness default".to_string()),
    };
    store.set_commander_choice(&job_id(JOB), &chosen).expect("set");
    assert_eq!(store.commander_choice(&job_id(JOB)).expect("reads"), chosen);

    store
        .set_commander_choice(&job_id(JOB), &CommanderChoice::default())
        .expect("cleared");
    assert_eq!(store.commander_choice(&job_id(JOB)).expect("reads"), CommanderChoice::default());
}

#[test]
fn a_choice_for_a_job_that_is_not_there_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(matches!(
        store.set_commander_choice(&job_id(JOB), &CommanderChoice::default()),
        Err(WriteError::NoSuchJob { .. })
    ));
    assert!(matches!(
        store.commander_choice(&job_id(JOB)),
        Err(LoadJobError::NoSuchJob { .. })
    ));
}
