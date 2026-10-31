//! The line Fleet adds ahead of a message that names something, as a Session and Helm are both
//! told it.

use ipc::{SessionTag, TagKind};

use crate::session_host::mention_line;

fn tag(kind: TagKind, id: &str, title: &str, url: Option<&str>) -> (SessionTag, Option<String>) {
    (
        SessionTag {
            kind,
            id: id.to_string(),
            title: title.to_string(),
            url: url.map(str::to_string),
        },
        None,
    )
}

#[test]
fn an_issue_is_named_by_number_title_and_address() {
    let line = mention_line(&[tag(
        TagKind::Issue,
        "214",
        "The title",
        Some("https://forge.invalid/o/r/issues/214"),
    )]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: issue #214 "The title" (https://forge.invalid/o/r/issues/214)."#)
    );
}

#[test]
fn an_issue_named_by_a_trackers_key_is_said_as_the_key_without_a_hash() {
    let line = mention_line(&[tag(TagKind::Issue, "ABC-12", "Walk it", Some("https://acme.invalid/browse/ABC-12"))]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: issue ABC-12 "Walk it" (https://acme.invalid/browse/ABC-12)."#)
    );
}

#[test]
fn the_number_bridge_puts_in_a_title_is_not_said_twice() {
    let line = mention_line(&[
        tag(TagKind::Issue, "219", "#219 Walk it", None),
        tag(TagKind::PullRequest, "220", "#220 Land it", None),
    ])
    .expect("a line");
    assert_eq!(
        line,
        r#"The person named: issue #219 "Walk it"; pull request #220 "Land it"."#
    );
}

#[test]
fn a_title_that_only_looks_like_the_prefix_is_left_alone() {
    let line = mention_line(&[tag(TagKind::Issue, "21", "#219 Walk it", None)]).expect("a line");
    assert_eq!(line, r##"The person named: issue #21 "#219 Walk it"."##);
}

#[test]
fn a_pull_request_carries_its_address_only_when_it_has_one() {
    let with = mention_line(&[tag(
        TagKind::PullRequest,
        "5",
        "Five",
        Some("https://forge.invalid/o/r/pull/5"),
    )])
    .expect("a line");
    assert_eq!(
        with,
        r#"The person named: pull request #5 "Five" (https://forge.invalid/o/r/pull/5)."#
    );
    let without = mention_line(&[tag(TagKind::PullRequest, "5", "Five", None)]).expect("a line");
    assert_eq!(without, r#"The person named: pull request #5 "Five"."#);
}

#[test]
fn a_reported_bug_is_named_by_its_title() {
    let line = mention_line(&[tag(TagKind::Report, "01jreport", "Preview restart drops the lease", None)]);
    assert_eq!(
        line.as_deref(),
        Some(r#"The person named: reported bug "Preview restart drops the lease"."#)
    );
}
