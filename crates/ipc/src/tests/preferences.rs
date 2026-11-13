//! What `get_preferences` and `save_preferences` must keep true: the answer
//! is flat, and a save is a name and a value with nothing else read from it.

use crate::{decode, encode, AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, SavePreference, TakeIn};

#[test]
fn nothing_saved_reads_as_the_shipped_default() {
    assert_eq!(
        Preferences::default(),
        Preferences {
            where_things_are_open: false,
            ..Default::default()
        }
    );
}

#[test]
fn preferences_round_trip_flat() {
    let preferences = Preferences {
        where_things_are_open: true,
        ..Default::default()
    };
    let json = encode(&preferences).expect("plain data");
    assert_eq!(json, r#"{"where_things_are_open":true}"#);
    assert_eq!(
        decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"),
        preferences
    );
}

#[test]
fn a_save_names_one_preference_and_a_value() {
    let body = br#"{"name":"where_things_are_open","value":true}"#;
    let save = decode::<SavePreference>("a preference to save", body).expect("plain data");
    assert_eq!(
        save,
        SavePreference {
            name: "where_things_are_open".to_string(),
            value: true,
            text: None,
        }
    );
}

/// **The name is a plain string, not a closed wire type.** An unrecognised
/// name still decodes — refusing it by name is `store`'s and `fleet`'s, not a
/// deserializer's, so the message can say exactly what was sent.
#[test]
fn an_unrecognised_name_still_decodes() {
    let body = br#"{"name":"where_things_are_purple","value":true}"#;
    decode::<SavePreference>("a preference to save", body).expect("the wire type takes any name");
}

#[test]
fn the_draft_default_is_left_out_while_false_and_read_back_when_true() {
    let drafting = Preferences {
        draft_pull_requests: true,
        ..Preferences::default()
    };
    let json = encode(&drafting).expect("plain data");
    assert_eq!(
        json,
        r#"{"where_things_are_open":false,"draft_pull_requests":true}"#
    );
    assert_eq!(
        decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"),
        drafting
    );
    let before_it: Preferences =
        decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
    assert!(!before_it.draft_pull_requests);
}

#[test]
fn how_he_works_is_left_out_while_it_is_the_default() {
    let json = encode(&Preferences::default()).expect("plain data");
    assert_eq!(json, r#"{"where_things_are_open":false}"#);
    let from_before: Preferences = decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
    assert_eq!(from_before, Preferences { where_things_are_open: true, ..Preferences::default() });
    assert!(from_before.listen_end_summary && from_before.listen_needs_you && from_before.buttons_words);
}

#[test]
fn a_choice_away_from_the_default_rides_by_its_word_and_a_false_switch_is_sent() {
    let chosen = Preferences {
        take_in: TakeIn::Listen,
        answer_with: AnswerWith::Buttons,
        first_run_answered: true,
        read_results: ReadResults::ThreadOnly,
        look_results: LookResults::WalkVisible,
        look_questions: LookQuestions::PicturesWhole,
        listen_end_summary: false,
        listen_needs_you: false,
        listen_progress: true,
        listen_operator: true,
        listen_voice: "Samantha".into(),
        buttons_words: false,
        ..Preferences::default()
    };
    let json = encode(&chosen).expect("plain data");
    assert_eq!(
        json,
        concat!(
            r#"{"where_things_are_open":false,"take_in":"listen","answer_with":"buttons","first_run_answered":true,"#,
            r#""read_results":"thread_only","look_results":"walk_visible","look_questions":"pictures_whole","#,
            r#""listen_end_summary":false,"listen_needs_you":false,"listen_progress":true,"listen_operator":true,"#,
            r#""listen_voice":"Samantha","buttons_words":false}"#
        )
    );
    assert_eq!(decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"), chosen);
}

#[test]
fn every_word_of_a_set_reads_back_and_the_default_leads() {
    fn walked<T: Default + PartialEq + std::fmt::Debug + serde::Serialize>(words: &[&str], from_word: fn(&str) -> Option<T>) {
        for word in words {
            let one = from_word(word).unwrap_or_else(|| panic!("`{word}` is a word of the set"));
            assert_eq!(serde_json::to_value(&one).unwrap(), serde_json::Value::String((*word).to_string()));
        }
        assert_eq!(from_word(words[0]), Some(T::default()));
        assert_eq!(from_word("sideways"), None);
        assert_eq!(from_word(""), None);
    }
    walked(TakeIn::WORDS, TakeIn::from_word);
    walked(AnswerWith::WORDS, AnswerWith::from_word);
    walked(ReadResults::WORDS, ReadResults::from_word);
    walked(LookResults::WORDS, LookResults::from_word);
    walked(LookQuestions::WORDS, LookQuestions::from_word);
}
