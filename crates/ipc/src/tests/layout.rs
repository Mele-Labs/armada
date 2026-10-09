//! `layout.json`'s rules, which `parseLayout` in `packages/shell/src/layout.tsx` applies to the
//! same text: one wrong shape refuses the whole file, and an id or region this build does not have
//! is ignored. The registry file both sides check is `packages/shell/layout-registry.json`.

use crate::layout::{problems, regions, MOST_BYTES};

fn refused(text: &str) -> String {
    let found = problems(text);
    assert!(!found.is_empty(), "should be refused: {text}");
    found.join("; ")
}

#[test]
fn a_file_of_known_shapes_is_accepted() {
    for text in [
        r#"{"version":1}"#,
        r#"{"version":1.0}"#,
        r#"{"version":1,"dashboard.panels":{"order":["merge-line","fleet"]},"rail":{"hidden":["lessons"]}}"#,
        r#"{"version":1,"job.tabs":{"order":["overview","record","plan"],"hidden":["pulse"],"first":"plan"}}"#,
        r#"{"version":1,"job.tabs":{}}"#,
    ] {
        assert_eq!(problems(text), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn what_this_build_does_not_have_is_ignored_and_never_a_reason_to_refuse() {
    for text in [
        // A region it does not have, and whatever shape that region's body is.
        r#"{"version":1,"sidebar":{"order":["x"]}}"#,
        r#"{"version":1,"sidebar":7}"#,
        // An id it does not have, in every place an id is read.
        r#"{"version":1,"rail":{"hidden":["a-panel-removed-in-a-release"]}}"#,
        r#"{"version":1,"job.tabs":{"order":["gone"],"first":"gone"}}"#,
        // Fields a region does not use: read as ids, then ignored.
        r#"{"version":1,"rail":{"order":["lessons"]}}"#,
        r#"{"version":1,"dashboard.panels":{"first":"fleet"}}"#,
    ] {
        assert_eq!(problems(text), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn one_wrong_shape_refuses_the_whole_file() {
    let long_ids = format!(r#"{{"version":1,"rail":{{"hidden":[{}]}}}}"#, vec![r#""lessons""#; 33].join(","));
    let long_id = format!(r#"{{"version":1,"rail":{{"hidden":["{}"]}}}}"#, "a".repeat(41));
    let oversize = format!(r#"{{"version":1,"x":"{}"}}"#, "a".repeat(MOST_BYTES));
    for (text, why) in [
        ("", "not JSON"),
        ("{", "not JSON"),
        ("[]", "not an object"),
        ("7", "not an object"),
        ("null", "not an object"),
        (r#"{}"#, "version is not 1"),
        (r#"{"version":2}"#, "version is not 1"),
        (r#"{"version":"1"}"#, "version is not 1"),
        (r#"{"version":1,"rail":[]}"#, "rail is not an object"),
        (r#"{"version":1,"rail":null}"#, "rail is not an object"),
        (r#"{"version":1,"rail":{"colour":"red"}}"#, "rail.colour is not a field"),
        (r#"{"version":1,"rail":{"hidden":"lessons"}}"#, "rail.hidden is not a list"),
        (r#"{"version":1,"rail":{"hidden":[1]}}"#, "rail.hidden is not a list"),
        (r#"{"version":1,"rail":{"hidden":["Lessons"]}}"#, "rail.hidden is not a list"),
        (r#"{"version":1,"rail":{"hidden":["../x"]}}"#, "rail.hidden is not a list"),
        (r#"{"version":1,"rail":{"hidden":["1x"]}}"#, "rail.hidden is not a list"),
        (r#"{"version":1,"job.tabs":{"first":["plan"]}}"#, "job.tabs.first is not an id"),
        (r#"{"version":1,"job.tabs":{"first":"Plan"}}"#, "job.tabs.first is not an id"),
        (&long_ids, "rail.hidden is not a list"),
        (&long_id, "rail.hidden is not a list"),
        (&oversize, "larger than 4096 bytes"),
    ] {
        assert!(refused(text).contains(why), "{text}: {:?}", problems(text));
    }
}

#[test]
fn a_file_at_the_limit_is_accepted_and_one_byte_over_is_not() {
    let head = r#"{"version":1,"p":""#;
    let tail = r#""}"#;
    let at = format!("{head}{}{tail}", " ".repeat(MOST_BYTES - head.len() - tail.len()));
    assert_eq!(at.len(), MOST_BYTES);
    assert_eq!(problems(&at), Vec::<String>::new());
    assert!(refused(&format!("{at} ")).contains("larger than"));
}

/// The regions are read from the file `layout.tsx`'s registry is tested against, so a region added
/// there is added here by editing one file, and a test on the TypeScript side fails when the two drift.
#[test]
fn the_regions_are_the_ones_the_registry_file_names() {
    let mut found = regions();
    found.sort();
    assert_eq!(found, ["dashboard.panels", "dashboard.tabs", "job.tabs", "rail"]);
}
