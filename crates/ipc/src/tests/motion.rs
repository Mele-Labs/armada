//! The owner's motion settings' rules: `ringSeconds`, a whole number from 2 to 20, and nothing else.

use crate::motion::{problems, read, Motion, DEFAULT_RING_SECONDS};
use crate::{decode, encode, Preferences};

#[test]
fn whole_seconds_inside_the_range_are_accepted() {
    for text in [r#"{"ringSeconds":2}"#, r#"{"ringSeconds":6}"#, r#"{"ringSeconds":20}"#] {
        assert_eq!(problems(text), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn anything_else_is_named() {
    for (text, first) in [
        ("{", "not JSON"),
        ("[]", "not an object"),
        ("{}", "ringSeconds is missing"),
        (r#"{"ringSeconds":1}"#, "ringSeconds is outside 2 to 20 seconds"),
        (r#"{"ringSeconds":21}"#, "ringSeconds is outside 2 to 20 seconds"),
        (r#"{"ringSeconds":0}"#, "ringSeconds is outside 2 to 20 seconds"),
        (r#"{"ringSeconds":4.5}"#, "ringSeconds is not a whole number of seconds"),
        (r#"{"ringSeconds":-3}"#, "ringSeconds is not a whole number of seconds"),
        (r#"{"ringSeconds":"6"}"#, "ringSeconds is not a whole number of seconds"),
        (r#"{"ringSeconds":6,"spin":1}"#, "spin is not a motion setting"),
    ] {
        assert_eq!(problems(text).first().map(String::as_str), Some(first), "{text}");
    }
}

#[test]
fn a_saved_value_reads_back_and_anything_else_reads_as_what_ships() {
    let chosen = Motion { ring_seconds: 12 };
    assert_eq!(chosen.text(), r#"{"ringSeconds":12}"#);
    assert_eq!(read(&chosen.text()), chosen);
    for text in ["", "  ", "{", r#"{"ringSeconds":1}"#, r#"{"ringSeconds":99}"#, r#"{"ringSeconds":4.5}"#, "[]"] {
        assert_eq!(read(text), Motion { ring_seconds: DEFAULT_RING_SECONDS }, "{text}");
    }
    assert_eq!(DEFAULT_RING_SECONDS, 6);
}

#[test]
fn motion_is_left_out_of_the_answer_while_none_is_chosen_and_round_trips_once_chosen() {
    assert!(!encode(&Preferences::default()).expect("plain data").contains("motion"));
    let chosen = Preferences { motion: r#"{"ringSeconds":9}"#.into(), ..Preferences::default() };
    let json = encode(&chosen).expect("plain data");
    assert!(json.contains(r#""motion":"{\"ringSeconds\":9}""#), "{json}");
    assert_eq!(decode::<Preferences>("preferences", json.as_bytes()).expect("round-trips"), chosen);
    let before_it: Preferences = decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
    assert_eq!(before_it.motion, "");
}
