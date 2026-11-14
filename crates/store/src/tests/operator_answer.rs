use crate::tests::{open, TempDir};
use crate::KeptAnswer;

fn answer(manifest_id: &str, question: &str) -> KeptAnswer {
    KeptAnswer {
        session: "01JOB".to_string(),
        manifest_id: manifest_id.to_string(),
        job: Some("Build the pane".to_string()),
        item_id: "full".to_string(),
        question: question.to_string(),
        options: r#"["Yes","No"]"#.to_string(),
        choice: Some("Yes".to_string()),
        answered_at: "2026-11-12T14:03:00.000Z".to_string(),
        via: "job".to_string(),
        ..KeptAnswer::default()
    }
}

#[test]
fn his_answers_are_kept_and_read_newest_first_for_the_repositories_asked() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.operator_answers_empty().expect("read"));
    let a = store.add_operator_answer(&answer("01A", "first")).expect("kept");
    let b = store.add_operator_answer(&answer("01B", "second")).expect("kept");
    let c = store.add_operator_answer(&answer("01A", "third")).expect("kept");
    assert!(!store.operator_answers_empty().expect("read"));

    let both = store.operator_answers_of(&["01A".to_string(), "01B".to_string()], 10).expect("read");
    assert_eq!(both.iter().map(|one| one.n).collect::<Vec<_>>(), vec![c, b, a]);
    assert_eq!(store.operator_answers_of(&["01A".to_string()], 1).expect("read").iter().map(|one| one.n).collect::<Vec<_>>(), vec![c], "a limit, newest first");
    assert!(store.operator_answers_of(&[], 10).expect("read").is_empty());
    assert_eq!(store.operator_answer(b).expect("read").expect("kept").question, "second");
    assert!(store.operator_answer(9999).expect("read").is_none());
}
