//! **Fleet never writes to an issue tracker on the owner's behalf.** The
//! `Tracker` trait is the whole door to one, so the rule is held on the trait:
//! it lists, reads and searches, and says how a search is written, and a method
//! added to it that does anything else fails here by name.

const TRAIT: &str = include_str!("../../../adapter-traits/src/tracker.rs");

/// The names of the `fn`s declared inside `pub trait Tracker { ... }`.
fn methods_of_tracker() -> Vec<String> {
    let start = TRAIT
        .find("pub trait Tracker {")
        .expect("the Tracker trait is declared in tracker.rs");
    let body = &TRAIT[start..];
    let end = body.find("\n}\n").expect("the trait closes");
    body[..end]
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("fn "))
        .map(|rest| rest.split(['(', '<']).next().unwrap_or_default().to_string())
        .collect()
}

#[test]
fn the_tracker_trait_has_no_write_method() {
    assert_eq!(
        methods_of_tracker(),
        ["open_issues", "read_issue", "search_issues", "query_guide", "address_template", "issues_linked_in", "issues_named_by_pull_request"],
        "a Tracker lists and reads; commenting, labelling and editing an issue are not Fleet's to do"
    );
}
