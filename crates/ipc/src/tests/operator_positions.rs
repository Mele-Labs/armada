//! `features.operatorRepositories`: the roles that differ per repository. Read leniently, so a hand edit
//! gone wrong leaves each role at the machine's position.

use crate::operator_positions::{OperatorPosition, OperatorRepositories, OperatorRole};

#[test]
fn text_that_is_absent_or_not_the_shape_reads_as_no_repository_differing() {
    for text in ["", "null", "not json", "[\"answer\"]", r#"{"a":"off"}"#, r#"{"a":{"steer":"decide"}}"#] {
        let read = OperatorRepositories::read(text);
        assert_eq!(read, OperatorRepositories::default(), "{text:?}");
        assert_eq!(read.of("a", OperatorRole::Answer), None, "{text:?}");
    }
}

#[test]
fn a_repository_sets_only_the_roles_it_names() {
    let read = OperatorRepositories::read(r#"{"a":{"answer":"read_only","file":"suggest"}}"#);
    assert_eq!(read.of("a", OperatorRole::Answer), Some(OperatorPosition::ReadOnly));
    assert_eq!(read.of("a", OperatorRole::File), Some(OperatorPosition::Suggest));
    assert_eq!(read.of("a", OperatorRole::Dispatch), None);
    assert_eq!(read.of("b", OperatorRole::Answer), None);
}

#[test]
fn the_new_words_read_as_themselves_the_old_ones_as_the_ones_they_meant_and_any_other_as_read_only() {
    for (word, position) in [
        ("read_only", OperatorPosition::ReadOnly),
        ("suggest", OperatorPosition::Suggest),
        ("decide", OperatorPosition::Decide),
        ("off", OperatorPosition::ReadOnly),
        ("on", OperatorPosition::Decide),
    ] {
        assert_eq!(OperatorPosition::read(word), position, "{word:?}");
    }
    for word in ["", "true", "On", "sideways"] {
        assert_eq!(OperatorPosition::read(word), OperatorPosition::ReadOnly, "{word:?}");
    }
}

#[test]
fn a_repository_saved_with_the_old_words_or_a_word_that_is_none_reads_as_the_position_it_meant() {
    let read = OperatorRepositories::read(r#"{"a":{"answer":"off","file":"on","dispatch":"suggest","delegate":"sideways"}}"#);
    assert_eq!(read.of("a", OperatorRole::Answer), Some(OperatorPosition::ReadOnly));
    assert_eq!(read.of("a", OperatorRole::File), Some(OperatorPosition::Decide));
    assert_eq!(read.of("a", OperatorRole::Dispatch), Some(OperatorPosition::Suggest));
    assert_eq!(read.of("a", OperatorRole::Delegate), Some(OperatorPosition::ReadOnly));
}

#[test]
fn the_wire_words_are_read_only_suggest_and_decide() {
    for (position, word) in [(OperatorPosition::ReadOnly, "read_only"), (OperatorPosition::Suggest, "suggest"), (OperatorPosition::Decide, "decide")] {
        assert_eq!(serde_json::to_string(&position).expect("written"), format!("\"{word}\""));
    }
}
