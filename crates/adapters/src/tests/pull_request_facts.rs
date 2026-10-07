//! The one line `gh pr view` is asked to answer with, read as a pull request's
//! standing. The forge is never called here: `gh` needs an account and a
//! network, and what is under test is what the line means.

use adapter_traits::{PullRequestFacts, PullRequestStanding};

use crate::pull_request_facts::parsed;

fn line(state: &str, draft: &str, auto: &str) -> String {
    format!("{state}\t{draft}\t{auto}\tfleet/pr-acts\thttps://forge.invalid/a/b/pull/7\tTake a draft out")
}

#[test]
fn an_open_pull_request_that_is_a_draft_is_a_draft() {
    assert_eq!(
        parsed(&line("OPEN", "true", "false")),
        Some(PullRequestFacts {
            standing: PullRequestStanding::Draft,
            branch: "fleet/pr-acts".into(),
            auto_merge: false,
            title: "Take a draft out".into(),
            url: "https://forge.invalid/a/b/pull/7".into(),
        })
    );
}

#[test]
fn each_state_the_forge_names_reads_as_its_own() {
    let standing = |state, draft| parsed(&line(state, draft, "false")).map(|facts| facts.standing);
    assert_eq!(standing("OPEN", "false"), Some(PullRequestStanding::Open));
    assert_eq!(standing("MERGED", "false"), Some(PullRequestStanding::Merged));
    assert_eq!(standing("CLOSED", "false"), Some(PullRequestStanding::Closed));
}

/// A draft that was merged or closed is not a draft: the flag stays on the
/// forge's record and means nothing once the state has moved.
#[test]
fn a_merged_pull_request_is_not_a_draft_whatever_its_flag_says() {
    assert_eq!(
        parsed(&line("MERGED", "true", "false")).map(|facts| facts.standing),
        Some(PullRequestStanding::Merged)
    );
}

#[test]
fn auto_merge_is_whether_the_forge_holds_a_request_for_it() {
    assert!(parsed(&line("OPEN", "false", "true")).expect("read").auto_merge);
}

/// **A state this build has no word for is silence**, never a guess.
#[test]
fn a_state_nobody_named_is_no_reading() {
    assert_eq!(parsed(&line("LOCKED", "false", "false")), None);
}

#[test]
fn a_short_or_blank_line_is_no_reading() {
    assert_eq!(parsed("OPEN\ttrue"), None);
    assert_eq!(parsed(""), None);
    assert_eq!(
        parsed("OPEN\tfalse\tfalse\t\thttps://forge.invalid/a/b/pull/7\ttitle"),
        None
    );
}
