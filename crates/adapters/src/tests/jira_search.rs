//! The Jira tracker's search: the JQL it composes from the block and a query, the command it
//! runs, and how its filter reads each field from both of `acli`'s possible shapes.

use adapter_traits::{FromOutside, IssueField, Tracker};

use super::jira::{block, settings, through_jq, Scripted};
use crate::jira::{self, JiraSettings};
use crate::jira_tracker::{issue_of_line, JiraTracker};

fn field(name: &str, values: &[&str]) -> IssueField {
    IssueField {
        name: name.to_string(),
        values: values.iter().map(|one| FromOutside::verbatim(*one)).collect(),
    }
}

fn with_fields(fields: &str) -> JiraSettings {
    JiraSettings::read(&block(&[("site", "acme"), ("project", "ABC"), ("fields", fields)])).expect("a block")
}

// ------------------------------------------------------------------ the JQL

#[test]
fn a_search_is_the_blocks_selection_anded_with_the_query_newest_edit_first() {
    assert_eq!(
        settings().search_for("assignee = currentUser()"),
        Ok("project = \"ABC\" AND (assignee = currentUser()) ORDER BY updated DESC".to_string())
    );
    let both = JiraSettings::read(&block(&[("site", "acme"), ("project", "ABC, OPS"), ("jql", "labels = web")]))
        .expect("a block");
    assert_eq!(
        both.search_for("statusCategory != Done"),
        Ok("project in (\"ABC\", \"OPS\") AND (labels = web) AND (statusCategory != Done) ORDER BY updated DESC"
            .to_string())
    );
    let only_jql = JiraSettings::read(&block(&[("site", "acme"), ("jql", "labels = web")])).expect("a block");
    assert_eq!(
        only_jql.search_for("sprint in openSprints()"),
        Ok("(labels = web) AND (sprint in openSprints()) ORDER BY updated DESC".to_string())
    );
    assert_eq!(
        settings().search_for("  "),
        Ok("project = \"ABC\" ORDER BY updated DESC".to_string()),
        "an empty query is the whole selection"
    );
}

#[test]
fn a_query_that_orders_itself_is_refused_and_never_reaches_acli() {
    let refused = settings().search_for("labels = web order by rank").expect_err("refused");
    assert!(refused.contains("ORDER BY"), "{refused}");
    let run = Scripted::saying(&[]);
    let said = JiraTracker::with_runner(settings(), &run)
        .search_issues("/repo", "labels = web ORDER BY rank")
        .expect_err("refused");
    assert_eq!(said.doing, "searching the issues");
    assert!(said.said.contains("ORDER BY"), "{}", said.said);
    assert!(run.calls().is_empty());
}

// ------------------------------------------------------------------ the command

#[test]
fn the_search_runs_through_acli_asking_for_every_field_a_row_can_show() {
    let line = "ABC-5\tFive\tweb\tSam\t2026-10-14T10:00:00Z\tBody\tPat\tdone\t\tDone\tHigh\tBug\t\t\tweb\t\t5\n";
    let run = Scripted::saying(&[(0, "[]", ""), (0, line, "")]);
    let found = JiraTracker::with_runner(with_fields("customfield_10016"), &run)
        .search_issues("/repo", "assignee = currentUser()")
        .expect("found");
    let calls = run.calls();
    assert_eq!(
        calls[0].1,
        [
            "jira", "workitem", "search", "--jql",
            "project = \"ABC\" AND (assignee = currentUser()) ORDER BY updated DESC",
            "--fields",
            "key,summary,labels,assignee,updated,description,reporter,status,resolution,\
             priority,issuetype,components,fixVersions,sprint,customfield_10016",
            "--json", "--limit", "100",
        ]
    );
    assert_eq!(calls[1].0, "jq");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].author.as_written(), "Pat");
    assert!(found[0].closed);
    assert_eq!(
        found[0].fields,
        [
            field("Status", &["Done"]),
            field("Priority", &["High"]),
            field("Issue type", &["Bug"]),
            field("Labels", &["web"]),
            field("customfield_10016", &["5"]),
        ]
    );
}

#[test]
fn a_fields_key_names_extra_fields_and_refuses_one_that_cannot_be_quoted() {
    let extra = with_fields("customfield_10016, Story Points ,");
    let guide = extra.guide();
    assert_eq!(
        guide.fields,
        ["Status", "Priority", "Issue type", "Components", "Fix versions", "Labels", "Sprint", "customfield_10016", "Story Points"]
    );
    let refused = JiraSettings::read(&block(&[("site", "acme"), ("project", "ABC"), ("fields", "a\"b")]))
        .expect_err("refused");
    assert!(refused.contains("`fields`"), "{refused}");
}

#[test]
fn jiras_guide_carries_its_open_and_assigned_to_me() {
    let guide = settings().guide();
    assert_eq!(guide.language, "JQL");
    assert_eq!(guide.open, "statusCategory != Done");
    assert_eq!(guide.assigned_to_me, "assignee = currentUser() AND statusCategory != Done");
    for taught in ["currentUser()", "openSprints()", "ORDER BY", "project = \"ABC\""] {
        assert!(guide.guide.contains(taught), "{taught}: {}", guide.guide);
    }
}

// ------------------------------------------------------------------ the filter

const NESTED: &str = r#"[{"key":"ABC-12","fields":{"summary":"Reader drops a row","labels":["web","api"],
  "assignee":{"displayName":"Sam Doe"},"reporter":{"displayName":"Pat Doe","accountId":"p1"},
  "updated":"2026-10-14T10:11:12.123+0200","description":"Some text",
  "status":{"name":"In Progress","statusCategory":{"key":"indeterminate"}},"resolution":null,
  "priority":{"name":"High","id":"2"},"issuetype":{"name":"Bug"},
  "components":[{"name":"Fleet"},{"name":"Bridge"}],"fixVersions":[],
  "sprint":[{"id":4,"name":"Sprint 4","state":"active"}],
  "customfield_10016":5,"customfield_10050":{"value":"Team A","id":"9"}}}]"#;

const FLAT: &str = r#"{"issues":[{"key":"ABC-14","summary":"Flat","labels":[],"reporter":"Ana",
  "updated":"2026-10-14T09:00:00Z","status":"Done","priority":"Low","issuetype":{"name":"Task"},
  "components":["Fleet"],"customfield_10016":null,"customfield_10050":[{"value":"Team B"},{"value":"Team C"}]}]}"#;

#[test]
fn the_search_filter_reads_each_field_from_the_nested_shape() {
    let it = with_fields("customfield_10016,customfield_10050");
    let printed = through_jq(&jira::search_filter(240, &it), NESTED);
    let issue = issue_of_line(&it, printed.lines().next().expect("a line")).expect("an issue");
    assert_eq!(issue.reference, "ABC-12");
    assert_eq!(issue.author.as_written(), "Pat Doe");
    assert!(!issue.closed);
    assert_eq!(
        issue.fields,
        [
            field("Status", &["In Progress"]),
            field("Priority", &["High"]),
            field("Issue type", &["Bug"]),
            field("Components", &["Fleet", "Bridge"]),
            field("Labels", &["web", "api"]),
            field("Sprint", &["Sprint 4"]),
            field("customfield_10016", &["5"]),
            field("customfield_10050", &["Team A"]),
        ]
    );
}

#[test]
fn the_search_filter_reads_the_flat_shape_too() {
    let it = with_fields("customfield_10016,customfield_10050");
    let printed = through_jq(&jira::search_filter(240, &it), FLAT);
    let issue = issue_of_line(&it, printed.lines().next().expect("a line")).expect("an issue");
    assert_eq!(issue.reference, "ABC-14");
    assert_eq!(issue.author.as_written(), "Ana");
    assert!(issue.closed, "a status called Done is done");
    assert_eq!(
        issue.fields,
        [
            field("Status", &["Done"]),
            field("Priority", &["Low"]),
            field("Issue type", &["Task"]),
            field("Components", &["Fleet"]),
            field("customfield_10050", &["Team B", "Team C"]),
        ]
    );
}

// ------------------------------------------------------------------ the address of a key

#[test]
fn a_jira_key_opens_on_the_blocks_site_and_anything_else_has_no_address() {
    let run = Scripted::saying(&[]);
    let tracker = JiraTracker::with_runner(settings(), &run);
    assert_eq!(tracker.address_template("ABC-123"), Some("https://acme.atlassian.net/browse/{match}".to_string()));
    assert_eq!(
        tracker.address_template(" OPS-7 "),
        Some("https://acme.atlassian.net/browse/{match}".to_string()),
        "any project's key opens on the same site"
    );
    for not_a_key in ["#123", "abc-1", "ABC", "ABC-", "see ABC-1"] {
        assert_eq!(tracker.address_template(not_a_key), None, "{not_a_key}");
    }
    let by_host = JiraTracker::with_runner(
        JiraSettings::read(&block(&[("site", "Jira.Example.Org"), ("project", "A1")])).expect("a block"),
        &run,
    );
    assert_eq!(by_host.address_template("A1-9"), Some("https://jira.example.org/browse/{match}".to_string()));
}
