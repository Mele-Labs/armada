//! `features.operatorRepositories`: the roles that differ per repository. Read leniently, so a hand edit
//! gone wrong leaves each role at the machine's position.

use crate::operator_positions::{OperatorPosition, OperatorRepositories, OperatorRole};

#[test]
fn text_that_is_absent_or_not_the_shape_reads_as_no_repository_differing() {
    for text in ["", "null", "not json", "[\"answer\"]", r#"{"a":"off"}"#, r#"{"a":{"answer":"sideways"}}"#, r#"{"a":{"steer":"on"}}"#] {
        let read = OperatorRepositories::read(text);
        assert_eq!(read, OperatorRepositories::default(), "{text:?}");
        assert_eq!(read.of("a", OperatorRole::Answer), None, "{text:?}");
    }
}

#[test]
fn a_repository_sets_only_the_roles_it_names() {
    let read = OperatorRepositories::read(r#"{"a":{"answer":"off","file":"suggest"}}"#);
    assert_eq!(read.of("a", OperatorRole::Answer), Some(OperatorPosition::Off));
    assert_eq!(read.of("a", OperatorRole::File), Some(OperatorPosition::Suggest));
    assert_eq!(read.of("a", OperatorRole::Dispatch), None);
    assert_eq!(read.of("b", OperatorRole::Answer), None);
}

#[test]
fn a_word_that_is_no_position_is_off() {
    assert_eq!(OperatorPosition::read("on"), OperatorPosition::On);
    assert_eq!(OperatorPosition::read("suggest"), OperatorPosition::Suggest);
    for word in ["off", "", "true", "On"] {
        assert_eq!(OperatorPosition::read(word), OperatorPosition::Off, "{word:?}");
    }
}
