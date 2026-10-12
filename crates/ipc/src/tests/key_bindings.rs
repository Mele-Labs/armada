//! The owner's key bindings' rules, which `parseKeyBindings` in `packages/components/src/keymap.ts`
//! applies to the same text: one wrong shape refuses the whole of it, and an act this build does
//! not have is ignored.

use crate::key_bindings::{problems, MOST_BYTES};

#[test]
fn bindings_of_known_shapes_are_accepted() {
    for text in [
        r#"{"version":1}"#,
        r#"{"version":1,"bindings":{}}"#,
        r#"{"version":1,"bindings":{"kill":["q"],"history":["⌥⌘←","⌥⌘→"],"close":[]}}"#,
        r#"{"version":1,"bindings":{"no_such_act":["z"]}}"#,
    ] {
        assert_eq!(problems(text), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn a_wrong_shape_refuses_the_whole_of_it() {
    for text in [
        "{",
        "[]",
        r#"{"version":2}"#,
        r#"{"version":1,"bindings":[]}"#,
        r#"{"version":1,"bindings":{"kill":"q"}}"#,
        r#"{"version":1,"bindings":{"kill":[1]}}"#,
        r#"{"version":1,"bindings":{"kill":["a","b","c","d","e","f","g","h","i"]}}"#,
        r#"{"version":1,"bindings":{"kill":["abcdefghijklmnopqrstuvwxyz"]}}"#,
    ] {
        assert!(!problems(text).is_empty(), "should be refused: {text}");
    }
    let heavy = format!(r#"{{"version":1,"pad":"{}"}}"#, "x".repeat(MOST_BYTES));
    assert_eq!(problems(&heavy), vec![format!("larger than {MOST_BYTES} bytes")]);
}
