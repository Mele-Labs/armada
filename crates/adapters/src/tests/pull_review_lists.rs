//! What jq prints for a list of pull requests waiting on a person, read as rows, and the search
//! each list asks the forge. The forge is never called.

use adapter_traits::{LatestReview, LatestReviewSaid, ListedChecks, NumberedPull, ReviewList, ReviewListAsk, ReviewStanding};

use crate::pull_review_lists::{listed_folded, numbered_query, search_of, Teams, QUERY};

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
        "NickMele",
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
        let rows = listed_folded(&[row(said, "", "")], Teams::Unwanted, "NickMele");
        assert_eq!(rows[0].checks, checks, "{said:?}");
    }
}

#[test]
fn a_decision_the_page_has_no_word_for_is_no_review() {
    let rows = listed_folded(&[row("", "SOMETHING_NEW", "")], Teams::Unwanted, "NickMele");
    assert_eq!(rows[0].review, ReviewStanding::None);
    let rows = listed_folded(&[row("", "CHANGES_REQUESTED", "")], Teams::Unwanted, "NickMele");
    assert_eq!(rows[0].review, ReviewStanding::ChangesRequested);
}

#[test]
fn a_title_keeps_what_the_tsv_escaped_and_a_line_with_no_number_is_dropped() {
    let mut fields: Vec<String> = row("", "", "").split('\t').map(str::to_string).collect();
    fields[2] = "Fix a\\ttab and a\\nnewline".to_string();
    let titled = fields.join("\t");
    let rows = listed_folded(&[titled, "a notice gh printed".to_string()], Teams::Unwanted, "NickMele");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Fix a\ttab and a\nnewline");
}

const ASKED: &str = "acme/platform\u{1f}Platform\u{1e}acme/web\u{1f}Web";

#[test]
fn the_team_a_row_names_is_the_first_asked_that_the_person_is_on() {
    let on = ["acme/web".to_string()];
    let rows = listed_folded(&[row("", "", ASKED)], Teams::Held(&on), "NickMele");
    assert_eq!(rows[0].team.as_deref(), Some("Web"));
}

#[test]
fn where_the_forge_would_not_list_their_teams_the_first_asked_is_named() {
    let rows = listed_folded(&[row("", "", ASKED)], Teams::Unread, "NickMele");
    assert_eq!(rows[0].team.as_deref(), Some("Platform"));
    let on_none = ["other/team".to_string()];
    let rows = listed_folded(&[row("", "", ASKED)], Teams::Held(&on_none), "NickMele");
    assert_eq!(rows[0].team.as_deref(), Some("Platform"), "none of theirs: still the first");
    let rows = listed_folded(&[row("", "", "")], Teams::Unread, "NickMele");
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
fn the_open_list_is_every_open_pull_request_of_the_repositories_and_needs_no_login() {
    let repositories = ["mele-labs/armada".to_string(), "acme/site".to_string()];
    let mut open = ask(ReviewList::Open, &repositories);
    open.login = "";
    assert_eq!(
        search_of(&open).as_deref(),
        Some("is:pr is:open sort:updated-desc repo:mele-labs/armada repo:acme/site")
    );
    assert_eq!(search_of(&ask(ReviewList::Open, &[])), None, "still no repository, no search");
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

#[test]
fn a_row_carries_the_persons_own_latest_review_and_a_pending_one_is_none() {
    let reviewed = format!("{}\tCHANGES_REQUESTED\t{SHA}\t2026-10-22T04:30:00Z", row("", "", ""));
    let pending = format!("{}\tPENDING\t{SHA}", row("", "", ""));
    let rows = listed_folded(&[reviewed, pending, row("", "", "")], Teams::Unwanted, "NickMele");
    assert_eq!(
        rows[0].my_latest,
        Some(LatestReview {
            said: LatestReviewSaid::ChangesRequested,
            commit: Some(SHA.to_string()),
            submitted_at: Some("2026-10-22T04:30:00Z".to_string()),
        })
    );
    assert_eq!(rows[1].my_latest, None);
    assert_eq!(rows[2].my_latest, None);
}

const ASKED_GH: &str = include_str!("github_answers/review_list_asked.tsv");

fn asked_lines() -> Vec<String> {
    ASKED_GH.lines().filter(|line| !line.is_empty()).map(str::to_string).collect()
}

#[test]
fn the_time_he_was_asked_is_the_latest_request_naming_him_off_the_team_list() {
    let rows = listed_folded(&asked_lines(), Teams::Unwanted, "mira");
    let asked: Vec<Option<&str>> = rows.iter().map(|row| row.asked_at.as_deref()).collect();
    // #41 asked Mira, her team, tobias, then mira again; #42 only tobias; #43 read no requests.
    assert_eq!(asked, [Some("2026-11-14T09:00:00Z"), None, None]);
}

#[test]
fn on_the_team_list_the_time_is_the_latest_request_of_a_team_he_is_on() {
    let on = ["acme/billing-reviewers".to_string()];
    let rows = listed_folded(&asked_lines(), Teams::Held(&on), "mira");
    let asked: Vec<Option<&str>> = rows.iter().map(|row| row.asked_at.as_deref()).collect();
    assert_eq!(asked, [Some("2026-11-12T09:00:00Z"), None, None], "not acme/design's, a team he is not on");
    let rows = listed_folded(&asked_lines(), Teams::Unread, "mira");
    assert_eq!(rows[0].asked_at.as_deref(), Some("2026-11-15T09:00:00Z"), "his teams unread: any team's");
    assert_eq!(rows[1].asked_at, None, "a request naming nobody is no team's");
}

fn numbered(repository: &str, number: u64) -> NumberedPull {
    NumberedPull { repository: repository.to_string(), number }
}

#[test]
fn a_read_by_number_is_one_query_with_each_repository_once_and_the_lists_own_fields() {
    let pulls = [numbered("acme/billing", 41), numbered("acme/ledger", 9), numbered("Acme/Billing", 38), numbered("acme/billing", 41)];
    let query = numbered_query(&pulls).expect("a query");
    assert!(query.starts_with(concat!(
        r#"query($me:String,$asked:Boolean!){r0:repository(owner:"acme",name:"billing"){"#,
        r#"p41:pullRequest(number:41){...Row} p38:pullRequest(number:38){...Row} }"#,
        r#"r1:repository(owner:"acme",name:"ledger"){p9:pullRequest(number:9){...Row} }}"#,
    )), "{query}");
    let fields = QUERY.split_once("... on PullRequest{").expect("the search's fields").1;
    let fields = fields.strip_suffix("}}}}").expect("closed");
    assert!(query.ends_with(&format!(" fragment Row on PullRequest{{state {fields}}}")), "{query}");
}

#[test]
fn a_name_that_could_say_more_than_a_name_is_not_read_by_number() {
    let unsafe_names = [numbered(r#"acme/billing"){x}"#, 1), numbered("acme", 2), numbered("acme/a/b", 3), numbered("/billing", 4)];
    assert_eq!(numbered_query(&unsafe_names), None);
    let query = numbered_query(&[unsafe_names[0].clone(), numbered("acme/ledger", 9)]).expect("the safe one");
    assert!(!query.contains("billing") && query.contains("p9:"), "{query}");
}
