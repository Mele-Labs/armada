//! The one line `gh pr view` is asked to answer with, read as a pull request's
//! standing. The forge is never called here: `gh` needs an account and a
//! network, and what is under test is what the line means.

use adapter_traits::{PullRequestFacts, PullRequestStanding};

use crate::pull_request_facts::parsed;

fn line(state: &str, draft: &str, auto: &str) -> String {
    with_body(state, draft, auto, "")
}

fn with_body(state: &str, draft: &str, auto: &str, body: &str) -> String {
    closing(state, draft, auto, "", body)
}

fn closing(state: &str, draft: &str, auto: &str, closes: &str, body: &str) -> String {
    format!("{state}\t{draft}\t{auto}\tfleet/pr-acts\thttps://forge.invalid/a/b/pull/7\t{closes}\t{body}\tTake a draft out")
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
            body: None,
            closes: Vec::new(),
        })
    );
}

#[test]
fn each_state_the_forge_names_reads_as_its_own() {
    let standing = |state, draft| parsed(&line(state, draft, "false")).map(|facts| facts.standing);
    assert_eq!(standing("OPEN", "false"), Some(PullRequestStanding::Open));
    assert_eq!(
        standing("MERGED", "false"),
        Some(PullRequestStanding::Merged)
    );
    assert_eq!(
        standing("CLOSED", "false"),
        Some(PullRequestStanding::Closed)
    );
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
    assert!(
        parsed(&line("OPEN", "false", "true"))
            .expect("read")
            .auto_merge
    );
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
        parsed("OPEN\tfalse\tfalse\t\thttps://forge.invalid/a/b/pull/7\t\t\ttitle"),
        None
    );
}

/// `@tsv` writes a description's newlines and tabs as escapes, and what reaches
/// a person is the text the author wrote.
#[test]
fn a_description_comes_through_as_written_and_an_absent_one_stays_none() {
    let said = with_body("OPEN", "false", "false", "Adds `body`.\\n\\n- one\\n- two\\tbullets\\\\");
    assert_eq!(
        parsed(&said).expect("read").body.as_deref(),
        Some("Adds `body`.\n\n- one\n- two\tbullets\\")
    );
    assert_eq!(parsed(&line("OPEN", "false", "false")).expect("read").body, None);
    assert_eq!(
        parsed(&with_body("OPEN", "false", "false", "\\n \\n"))
            .expect("read")
            .body,
        None
    );
}

/// The forge names the issues a pull request closes in its own order, and a row
/// reads them ascending. None is an empty list, never a reading refused.
#[test]
fn the_issues_a_pull_request_closes_are_read_ascending_and_none_is_empty() {
    assert_eq!(
        parsed(&closing("OPEN", "false", "false", "31,7", "")).expect("read").closes,
        vec![7, 31]
    );
    assert_eq!(parsed(&line("OPEN", "false", "false")).expect("read").closes, Vec::<u64>::new());
}
