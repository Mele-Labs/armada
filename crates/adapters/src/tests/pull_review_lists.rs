//! What jq prints for a list of pull requests waiting on a person, read as rows, and the search
//! each list asks the forge. The forge is never called.

use adapter_traits::{ListedChecks, ReviewList, ReviewListAsk, ReviewStanding};

use crate::pull_review_lists::{listed_folded, search_of, Teams};

const SHA: &str = "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e";

fn line(fields: &[&str]) -> String {
    fields.join("\t")
}

fn row(rollup: &str, decision: &str, asked: &str) -> String {
    line(&[
        "mele-labs/armada",
        "12",
        "Take a draft out",
        "fleet/pr-acts",
        "https://forge.invalid/mele-labs/armada/pull/12",
        "false",
        SHA,
        "ada",
        "2026-10-15T09:30:00Z",
        decision,
        rollup,
        asked,
    ])
}

fn ask<'a>(list: ReviewList, repositories: &'a [String]) -> ReviewListAsk<'a> {
    ReviewListAsk {
        list,
        login: "NickMele",
        repositories,
        most: 50,
    }
}

#[test]
fn a_line_is_a_row_in_the_wires_words() {
    let rows = listed_folded(
        &[row("SUCCESS", "REVIEW_REQUIRED", "")],
        Teams::Unwanted,
    );
    let pull = &rows[0];
    assert_eq!((pull.repository.as_str(), pull.number), ("mele-labs/armada", 12));
    assert_eq!(pull.head, SHA);
    assert_eq!(pull.checks, ListedChecks::Passed);
    assert_eq!(pull.review, ReviewStanding::Required);
    assert!(!pull.draft);
    assert_eq!(pull.team, None);
}

#[test]
fn a_rollup_is_passed_failed_or_still_running() {
    for (said, checks) in [
        ("SUCCESS", ListedChecks::Passed),
        ("FAILURE", ListedChecks::Failed),
        ("ERROR", ListedChecks::Failed),
        ("PENDING", ListedChecks::Running),
        ("EXPECTED", ListedChecks::Running),
        ("", ListedChecks::Running),
    ] {
        let rows = listed_folded(&[row(said, "", "")], Teams::Unwanted);
        assert_eq!(rows[0].checks, checks, "{said:?}");
    }
}

#[test]
fn a_decision_the_page_has_no_word_for_is_no_review() {
    let rows = listed_folded(&[row("", "SOMETHING_NEW", "")], Teams::Unwanted);
    assert_eq!(rows[0].review, ReviewStanding::None);
    let rows = listed_folded(&[row("", "CHANGES_REQUESTED", "")], Teams::Unwanted);
    assert_eq!(rows[0].review, ReviewStanding::ChangesRequested);
}

#[test]
fn a_title_keeps_what_the_tsv_escaped_and_a_line_with_no_number_is_dropped() {
    let mut fields: Vec<String> = row("", "", "").split('\t').map(str::to_string).collect();
    fields[2] = "Fix a\\ttab and a\\nnewline".to_string();
    let titled = fields.join("\t");
    let rows = listed_folded(&[titled, "a notice gh printed".to_string()], Teams::Unwanted);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Fix a\ttab and a\nnewline");
}

const ASKED: &str = "acme/platform\u{1f}Platform\u{1e}acme/web\u{1f}Web";

#[test]
fn the_team_a_row_names_is_the_first_asked_that_the_person_is_on() {
    let on = ["acme/web".to_string()];
    let rows = listed_folded(&[row("", "", ASKED)], Teams::Held(&on));
    assert_eq!(rows[0].team.as_deref(), Some("Web"));
}

#[test]
fn where_the_forge_would_not_list_their_teams_the_first_asked_is_named() {
    let rows = listed_folded(&[row("", "", ASKED)], Teams::Unread);
    assert_eq!(rows[0].team.as_deref(), Some("Platform"));
    let on_none = ["other/team".to_string()];
    let rows = listed_folded(&[row("", "", ASKED)], Teams::Held(&on_none));
    assert_eq!(rows[0].team.as_deref(), Some("Platform"), "none of theirs: still the first");
    let rows = listed_folded(&[row("", "", "")], Teams::Unread);
    assert_eq!(rows[0].team, None, "no team asked, none named");
}

#[test]
fn each_list_is_one_search_naming_every_repository_and_the_login() {
    let repositories = ["mele-labs/armada".to_string(), "acme/site".to_string()];
    let base = "is:pr is:open sort:updated-desc repo:mele-labs/armada repo:acme/site";
    for (list, qualifier) in [
        (ReviewList::Requested, "user-review-requested:NickMele"),
        (
            ReviewList::Team,
            "review-requested:NickMele -user-review-requested:NickMele",
        ),
        (ReviewList::Assigned, "assignee:NickMele"),
        (ReviewList::Mine, "author:NickMele"),
    ] {
        assert_eq!(
            search_of(&ask(list, &repositories)).as_deref(),
            Some(format!("{base} {qualifier}").as_str())
        );
    }
}

#[test]
fn a_search_that_would_not_stay_inside_the_served_repositories_is_not_asked() {
    assert_eq!(search_of(&ask(ReviewList::Mine, &[])), None, "no repository is every repository");
    let spaced = ["a/b author:someone".to_string()];
    assert_eq!(search_of(&ask(ReviewList::Mine, &spaced)), None);
    let repositories = ["a/b".to_string()];
    let mut loose = ask(ReviewList::Mine, &repositories);
    loose.login = "me author:other";
    assert_eq!(search_of(&loose), None);
}
