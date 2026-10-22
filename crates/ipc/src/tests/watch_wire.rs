//! A watch's command line read into a reading, and the three bodies an agent sends.

use core_model::ReadingState;

use crate::{decode, encode, read_watch_line, StartWatch, WatchStarts, WatchVerdict, WeighWatch};

/// The design's own example, after two lines of the script's chatter.
const PRINTED: &str = "fetching run 4411\n\n{\"state\": \"running\", \"finished\": false, \"says\": \"Deploy running, two of three services done\", \"link\": \"https://ci.example.com/runs/4411\", \"parts\": [{\"name\": \"build\", \"state\": \"passed\"}, {\"name\": \"deploy-api\", \"state\": \"running\"}, {\"name\": \"unit-tests-other\", \"state\": \"failed\"}], \"provider\": \"ignored\"}\n";

#[test]
fn the_last_line_is_the_reading_and_nothing_else_is_read() {
    let reading = read_watch_line(PRINTED).expect("the design's line reads");
    assert_eq!(reading.state, ReadingState::Running);
    assert!(!reading.finished);
    assert_eq!(reading.link.as_deref(), Some("https://ci.example.com/runs/4411"));
    let parts: Vec<(&str, ReadingState)> = reading.parts.iter().map(|part| (part.name.as_str(), part.state)).collect();
    assert_eq!(
        parts,
        [("build", ReadingState::Passed), ("deploy-api", ReadingState::Running), ("unit-tests-other", ReadingState::Failed)]
    );
}

#[test]
fn anything_but_the_line_is_a_broken_reading_that_says_why() {
    let cases = [
        ("", "printed nothing"),
        ("all good\n", "last line"),
        ("{\"state\": \"success\", \"finished\": true, \"says\": \"done\"}", "`state` is `success`"),
        ("{\"state\": \"passed\", \"says\": \"done\"}", "finished"),
        ("{\"state\": \"passed\", \"finished\": true, \"says\": \" \"}", "`says` is blank"),
        ("{\"state\": \"passed\", \"finished\": true, \"says\": \"ok\", \"parts\": [{\"name\": \"a\", \"state\": \"skipped\"}]}", "`parts[0].state`"),
        ("{\"state\": \"passed\", \"finished\": true, \"says\": \"ok\", \"parts\": [{\"name\": \"\", \"state\": \"passed\"}]}", "`parts[0].name`"),
    ];
    for (printed, said) in cases {
        let why = read_watch_line(printed).expect_err(printed).why;
        assert!(why.contains(said), "{printed:?} said {why:?}, not {said:?}");
    }
}

#[test]
fn a_start_is_a_recipe_or_a_command_and_never_both() {
    let recipe: StartWatch = decode(
        "a watch to start",
        br#"{"name":"QA deploy","recipe":"qa-deploy","values":{"sha":"abc123","environment":"qa"},"follows":42}"#,
    )
    .expect("a recipe start");
    assert!(matches!(&recipe.starts, WatchStarts::Recipe { values, .. } if values["sha"] == "abc123"));
    assert_eq!(recipe.follows, Some(42));

    let own: StartWatch = decode(
        "a watch to start",
        br#"{"name":"CI","command":["scripts/ci-state","abc"],"every":"60s","set_aside":[{"part":"lint","sentence":"red on main"}]}"#,
    )
    .expect("an own command");
    assert!(matches!(&own.starts, WatchStarts::Command { gives_up_after: None, .. }));
    assert_eq!(own.set_aside[0].part, "lint");
    let again: StartWatch = decode("a watch to start", encode(&own).expect("plain data").as_bytes()).expect("round trip");
    assert_eq!(again, own);

    for refused in [
        r#"{"name":"x","recipe":"r","command":["a"],"every":"60s"}"#,
        r#"{"name":"x","command":["a"]}"#,
        r#"{"name":"x","command":["a"],"every":"60s","values":{"sha":"a"}}"#,
        r#"{"name":"x"}"#,
        r#"{"name":"x","recipe":"r","shell":"a | b"}"#,
    ] {
        assert!(decode::<StartWatch>("a watch to start", refused.as_bytes()).is_err(), "{refused}");
    }
}

#[test]
fn ours_carries_nothing_and_not_ours_carries_its_sentence() {
    let ours: WeighWatch = decode("a weighing", br#"{"watch":"w1","verdict":"ours"}"#).expect("ours");
    assert_eq!(ours.verdict, WatchVerdict::Ours);
    let not_ours: WeighWatch = decode(
        "a weighing",
        br#"{"watch":"w1","verdict":"not_ours","sentence":"Another service's tests.","parts":["unit-tests-other"]}"#,
    )
    .expect("not ours");
    assert!(matches!(not_ours.verdict, WatchVerdict::NotOurs { ref parts, .. } if parts == &["unit-tests-other"]));
    for refused in [
        r#"{"watch":"w1","verdict":"ours","sentence":"why"}"#,
        r#"{"watch":"w1","verdict":"not_ours","parts":["a"]}"#,
        r#"{"watch":"w1","verdict":"maybe"}"#,
    ] {
        assert!(decode::<WeighWatch>("a weighing", refused.as_bytes()).is_err(), "{refused}");
    }
}
