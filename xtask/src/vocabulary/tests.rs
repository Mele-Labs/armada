//! The emitter, proved against the files it writes and against the shapes it reads.

use super::*;

/// The claim the gate makes, held where a person can run it alone: what is checked in is what the
/// registries emit today. A registry edit nobody regenerated against fails here with the file named.
#[test]
fn the_checked_in_files_are_what_the_registries_emit() {
    let root = crate::repo_root();
    let emitted = outputs(&root).expect("the registries emit");
    assert_eq!(emitted.len(), 2, "the two files native reads");
    for (rel, want) in emitted {
        let have = fs::read_to_string(root.join(&rel)).expect("a checked-in file");
        assert!(have == want, "STALE {rel} — run `{WRITE}` and commit what it writes");
    }
}

/// The same comparison, shown to fail: one verb changed in the registry text moves the output, so a
/// registry edit with no regeneration cannot match what is checked in.
#[test]
fn a_changed_registry_changes_the_output() {
    let root = crate::repo_root();
    let verbs = read(&root, VERBS).expect("enum-verbs.toml");
    let outcomes = read(&root, OUTCOMES).expect("check-outcomes.toml");
    let before = emit::vocabulary(&vocabularies(&verbs, &outcomes).expect("rows"));

    let edited = verbs.replacen("verb = \"passed\"", "verb = \"went through\"", 1);
    assert_ne!(edited, verbs, "the registry has a `passed` verb to edit");
    let after = emit::vocabulary(&vocabularies(&edited, &outcomes).expect("rows"));

    assert_ne!(before, after);
    assert!(after.contains("Some(\"went through\")"));
}

#[test]
fn a_literal_string_keeps_its_trailing_backslash() {
    let found = tables("[actions.toggle_sidebar]\nshortcut = '⌘\\'\n", "t.toml").unwrap();
    assert_eq!(found[0].1["shortcut"], "⌘\\");
}

#[test]
fn a_quoted_key_with_a_colon_is_the_wire_value() {
    assert_eq!(unquoted("\"manifest_rule:auto_merge\""), "manifest_rule:auto_merge");
    assert_eq!(unquoted("bare"), "bare");
}

#[test]
fn a_line_that_is_not_toml_stops_the_run() {
    assert!(tables("[a]\nnot a key\n", "t.toml").unwrap_err().contains("t.toml:2"));
    assert!(tables("key = 1\n", "t.toml").unwrap_err().contains("outside every table"));
    assert!(tables("[a\n", "t.toml").unwrap_err().contains("unreadable table header"));
}

#[test]
fn a_literal_is_escaped_as_rust_reads_it() {
    assert_eq!(emit::rust_str("a\"b\\c\nd\u{1}"), "\"a\\\"b\\\\c\\nd\\u{1}\"");
}

#[test]
fn a_scope_becomes_a_variant_and_a_key_context() {
    assert_eq!(emit::variant("list and detail"), "ListAndDetail");
    assert_eq!(emit::variant("by design"), "ByDesign");
    assert_eq!(emit::context("list and detail"), "list-and-detail");
}

#[test]
fn an_outcome_without_an_advances_is_refused() {
    let why = advances("[outcomes.passed]\n[outcomes.failed]\nadvances = false\n").unwrap_err();
    assert!(why.contains("[outcomes.passed] names no advances"), "{why}");
}
