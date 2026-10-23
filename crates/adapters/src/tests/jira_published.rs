//! The Jira filters over the JSON Atlassian publishes. **Atlassian publishes no sample of what
//! `acli jira workitem search --json` or `view --json` prints**: the acli reference lists the flags and
//! its output guide reads `view --json | jq '.fields.summary'`, so `fields` holds the fields. What
//! these fixtures hold is the Jira Cloud REST v3 shapes acli's issues are, each cited, and they
//! pin that the filters read them as published. Where acli wraps them differently is unconfirmed:
//! `docs/concepts/tracker.md`, *Jira*.

use super::jira::{settings, through_jq};
use crate::jira;
use crate::jira_tracker::{issue_of_line, read_issue_of_line};

/// The 200 response example of `POST /rest/api/3/search`, verbatim in what it holds:
/// https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-search/
/// Its status and assignee carry only a name, which is all the example gives them.
const SEARCH_RESPONSE: &str = r#"{
  "expand": ["names", "schema", "operations"], "startAt": 0, "maxResults": 15, "total": 2,
  "issues": [{"id": "10001", "key": "HSP-1",
    "fields": {"summary": "Test issue", "status": {"name": "To Do"}, "assignee": {"displayName": "John Doe"}}}]
}"#;

/// An issue whose `description` is the ADF document of the REST v3 comment example
/// (https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-comments/), and whose
/// `updated` and user are in the shape that page and the worklog page give:
/// `2021-01-18T23:45:00.000+0000`, `{accountId, active, displayName, self}`.
const ISSUE: &str = r#"{"id": "10010", "key": "HSP-2", "fields": {
  "summary": "Written up",
  "assignee": {"accountId": "5b10a2844c20165700ede21g", "active": false, "displayName": "Mia Krystof",
    "self": "https://your-domain.atlassian.net/rest/api/3/user?accountId=5b10a2844c20165700ede21g"},
  "updated": "2021-01-18T23:45:00.000+0000",
  "description": {"type": "doc", "version": 1, "content": [{"type": "paragraph", "content": [
    {"type": "text", "text": "Lorem ipsum dolor sit amet, consectetur adipiscing elit."}]}]}}}"#;

#[test]
fn a_search_response_wrapping_its_issues_is_read_with_only_the_fields_the_example_gives() {
    let it = settings();
    let printed = through_jq(&jira::search_filter(240, &it), SEARCH_RESPONSE);
    let issue = issue_of_line(&it, printed.lines().next().expect("a line")).expect("an issue");
    assert_eq!(issue.reference, "HSP-1");
    assert_eq!(issue.title.as_written(), "Test issue");
    assert_eq!(issue.assignees[0].as_written(), "John Doe");
    assert!(!issue.closed);
    assert!(issue.labels.is_empty() && issue.updated_at.is_empty());
}

#[test]
fn an_issue_in_the_published_shapes_gives_its_description_its_user_and_an_rfc_3339_instant() {
    let it = settings();
    let printed = through_jq(&jira::listing_filter(240), ISSUE);
    let issue = issue_of_line(&it, printed.lines().next().expect("a line")).expect("an issue");
    assert_eq!(issue.assignees[0].as_written(), "Mia Krystof");
    assert_eq!(issue.excerpt.as_written(), "Lorem ipsum dolor sit amet, consectetur adipiscing elit.");
    assert_eq!(issue.updated_at, "2021-01-18T23:45:00.000+00:00");
    assert_eq!(
        through_jq(&jira::edited_filter(), ISSUE).trim(),
        "2021-01-18T23:45:00.000+00:00",
        "what a Job's staleness check compares"
    );
}

/// The status category examples of the REST v3 status pages
/// (https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-workflow-statuses/), which
/// key a finished one `completed` and a running one `in-flight`, as Jira's own screens say `done`.
#[test]
fn a_finished_status_category_is_done_under_each_key_jira_publishes_for_it() {
    let it = settings();
    for (key, closed) in [("completed", true), ("done", true), ("in-flight", false), ("new", false), ("indeterminate", false)] {
        let json = format!(r#"{{"key":"HSP-3","fields":{{"summary":"S","status":{{"name":"X","statusCategory":{{"key":"{key}"}}}},"resolution":null}}}}"#);
        let printed = through_jq(&jira::reading_filter(), &json);
        let read = read_issue_of_line(&it, printed.lines().next().expect("a line")).expect("an issue");
        assert_eq!(read.closed, closed, "statusCategory key {key}");
    }
}
