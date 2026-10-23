//! A tracker mod names the shape of its ticket references in `[links]`, so a pull request that
//! names one links to that ticket, as Jira's browse links do. A mod with no `[links]` links nothing.

use std::sync::Arc;

use adapter_traits::{Tracker, TrackerChoice};

use super::taught_tracker::{GUIDE, READ_MAP, SEARCH_MAP};
use crate::taught_tracker::{self, TaughtTracker, TaughtTrackers, TrackerFile};
use crate::tracker_providers::tracker_over;

/// A tracker file whose `[links]` is `links`, written as the mod writes it (none for no section).
fn file(links: &str) -> String {
    format!(
        "{GUIDE}\n[search]\ntransport = \"cli\"\nargv = [\"/usr/bin/true\", \"{{query}}\"]\nmap = '''{SEARCH_MAP}'''\n\n\
         [read]\ntransport = \"cli\"\nargv = [\"/usr/bin/true\", \"{{reference}}\"]\nmap = '''{READ_MAP}'''\n\n{links}"
    )
}

fn linking(pattern: &str) -> TrackerFile {
    let text = file(&format!("[links]\nreference = '{pattern}'\n"));
    taught_tracker::read(&text).unwrap_or_else(|problems| panic!("{problems:?}"))
}

fn linked(pattern: &str, text: &str) -> Vec<String> {
    TaughtTracker::over(linking(pattern)).issues_linked_in("/repo", text)
}

#[test]
fn a_pull_request_naming_a_reference_of_the_taught_shape_links_it() {
    assert_eq!(linked(r"\bLIN-\d+\b", "Fixes LIN-42, and see (LIN-7). lin-8 and ALIN-9 are not ours."), ["LIN-42", "LIN-7"]);
}

#[test]
fn text_naming_no_reference_links_nothing() {
    assert!(linked(r"\bLIN-\d+\b", "Rename the thing, no ticket. #12 and ABC-3 are other trackers'.").is_empty());
}

#[test]
fn a_reference_named_twice_links_once_in_the_order_first_written() {
    assert_eq!(linked(r"\bLIN-\d+\b", "LIN-9 first, then LIN-3, then LIN-9 again"), ["LIN-9", "LIN-3"]);
}

#[test]
fn a_pattern_with_a_group_links_what_the_group_holds() {
    let text = "https://linear.app/acme/issue/LIN-42/a-title and https://linear.app/acme/issue/LIN-42 and https://linear.app/other/issue/LIN-1";
    assert_eq!(linked(r"linear\.app/acme/issue/(LIN-\d+)", text), ["LIN-42"]);
}

#[test]
fn a_mod_with_no_links_section_links_nothing() {
    let file = taught_tracker::read(&file("")).expect("it reads");
    assert!(TaughtTracker::over(file).issues_linked_in("/repo", "LIN-42").is_empty());
}

#[test]
fn a_manifest_naming_the_mod_links_by_it() {
    struct Planted(TrackerFile);
    impl TaughtTrackers for Planted {
        fn taught(&self, name: &str) -> Option<Result<TrackerFile, String>> {
            (name == "linear").then(|| Ok(self.0.clone()))
        }
    }
    let mods: Arc<dyn TaughtTrackers> = Arc::new(Planted(linking(r"\bLIN-\d+\b")));
    let choice = TrackerChoice::named(Some("linear"), &Default::default());
    assert_eq!(tracker_over(&choice, None, Some(&mods)).issues_linked_in("/repo", "Fixes LIN-42"), ["LIN-42"]);
}

#[test]
fn a_links_section_that_cannot_be_used_is_refused_with_its_reason() {
    let cases: [(String, &str); 5] = [
        (file("[links]\nreference = 'LIN-(\\d+'\n"), "`[links]` `reference` is not a pattern: "),
        (file("[links]\nreference = ''\n"), "`[links]` `reference` matches the empty text"),
        (file("[links]\nreference = 'x*'\n"), "`[links]` `reference` matches the empty text"),
        (file("[links]\nreference = 3\n"), "`[links]` `reference` must be a string"),
        (file("[links]\nreferenc = 'LIN-\\d+'\n"), "`[links]` has `referenc`, which the links does not take"),
    ];
    for (text, reason) in cases {
        let problems = taught_tracker::problems(&text);
        assert!(problems.iter().any(|one| one.contains(reason)), "wanted {reason:?} in {problems:?}");
    }
    let problems = taught_tracker::problems(&file("[links]\nreference = 'LIN-(\\d+'\n"));
    assert!(problems.iter().any(|one| one.contains("unclosed group")), "the regex's own reason: {problems:?}");
    assert!(taught_tracker::problems(&file("[links]\nreference = '\\bLIN-\\d+\\b'\n")).is_empty());
}
