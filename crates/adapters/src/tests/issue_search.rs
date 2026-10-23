//! The forge's issue search through `gh`: the arguments it is given, and how what its `--jq`
//! prints reads. The filter is run by the real `jq` over the JSON `gh issue list --json` prints,
//! as `gh` runs it in its own process.

use adapter_traits::{FromOutside, IssueField, Tracker};

use super::jira::through_jq;
use crate::issue_tracker::{issue_of_line, search_args};
use crate::IssueTracker;

fn jq_of(args: &[String]) -> &str {
    let at = args.iter().position(|one| one == "--jq").expect("a --jq");
    &args[at + 1]
}

fn field(name: &str, values: &[&str]) -> IssueField {
    IssueField {
        name: name.to_string(),
        values: values.iter().map(|one| FromOutside::verbatim(*one)).collect(),
    }
}

#[test]
fn the_search_asks_gh_for_every_state_newest_edit_first_with_the_fields_a_row_can_show() {
    let args = search_args("is:open label:bug", true, true);
    assert_eq!(
        args[..9],
        [
            "issue", "list", "--search=is:open label:bug sort:updated-desc", "--state", "all", "--limit", "100",
            "--json",
            "number,title,url,labels,assignees,updatedAt,body,closedByPullRequestsReferences,author,state,milestone,projectItems",
        ]
    );
    let without = search_args("-label:bug sort:created-asc", false, false);
    assert_eq!(without[2], "--search=-label:bug sort:created-asc", "a query's own sort is kept");
    assert_eq!(without[8], "number,title,url,labels,assignees,updatedAt,body,author,state,milestone");
}

const SEARCHED: &str = r#"[
 {"number":12,"title":"Search drops a row","url":"https://github.com/o/r/issues/12",
  "labels":[{"id":"L1","name":"bug","description":"","color":"d73a4a"}],
  "assignees":[{"id":"U1","login":"sam","name":"Sam"}],"updatedAt":"2026-10-19T10:00:00Z",
  "body":"It  drops\none","closedByPullRequestsReferences":[{"number":40}],
  "author":{"id":"U2","is_bot":false,"login":"nick","name":"Nick"},"state":"CLOSED",
  "milestone":{"number":3,"title":"v2","description":"","dueOn":null},
  "projectItems":[{"status":{"optionId":"o1","name":"In progress"},"title":"Roadmap"},
                  {"status":{"optionId":"","name":""},"title":"Backlog"}]},
 {"number":13,"title":"Bare","url":"https://github.com/o/r/issues/13","labels":[],"assignees":[],
  "updatedAt":"2026-10-20T10:00:00Z","body":"","closedByPullRequestsReferences":[],
  "author":null,"state":"OPEN","milestone":null,"projectItems":[]}
]"#;

#[test]
fn a_searched_issue_reads_its_author_state_milestone_and_projects() {
    let printed = through_jq(jq_of(&search_args("label:bug", true, true)), SEARCHED);
    let issues: Vec<_> = printed.lines().filter_map(issue_of_line).collect();
    assert_eq!(issues.len(), 2);
    let full = &issues[0];
    assert_eq!(full.reference, "#12");
    assert_eq!(full.author.as_written(), "nick");
    assert!(full.closed);
    assert!(full.linked_pull_request);
    assert_eq!(full.excerpt.as_written(), "It drops one");
    assert_eq!(
        full.fields,
        [
            field("Labels", &["bug"]),
            field("Milestone", &["v2"]),
            field("Projects", &["Roadmap: In progress", "Backlog"]),
        ]
    );
    let bare = &issues[1];
    assert_eq!(bare.author.as_written(), "", "an account that is gone is no author");
    assert!(!bare.closed);
    assert!(bare.fields.is_empty(), "a field with no values is not listed");
}

#[test]
fn a_search_without_projects_reads_every_other_field() {
    let printed = through_jq(jq_of(&search_args("label:bug", false, false)), SEARCHED);
    let first = issue_of_line(printed.lines().next().expect("a line")).expect("an issue");
    assert_eq!(first.fields, [field("Labels", &["bug"]), field("Milestone", &["v2"])]);
    assert!(first.closed);
    assert!(!first.linked_pull_request, "not asked, so not said");
}

#[test]
fn the_forges_guide_carries_its_open_and_assigned_to_me() {
    let guide = IssueTracker.query_guide();
    assert_eq!(guide.open, "is:open");
    assert_eq!(guide.assigned_to_me, "is:open assignee:@me");
    assert_eq!(guide.fields, ["Labels", "Milestone", "Projects"]);
    assert!(guide.guide.contains("assignee:@me") && guide.guide.contains("sort:"), "{}", guide.guide);
}
