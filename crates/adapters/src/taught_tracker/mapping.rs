//! A mod's `map` and what it yields: `jq` runs the mod's filter, then this module's, which checks
//! each issue has what Armada needs and writes it as one `@tsv` line. **Nothing here parses
//! JSON**, `crate::jira`'s way: lines come back, and lists inside a column are kept apart by the
//! ASCII separators, which no value is let hold.

use adapter_traits::{
    FromOutside, IssueComment, IssueField, LinkedPullRequest, LinkedPullRequestState, OpenIssue, ReadIssue,
};

use crate::under_review::as_written;

/// How much of a body a listed issue carries, in characters.
const EXCERPT: usize = 240;

/// Between the values of one list.
const UNIT: char = '\u{1f}';
/// Between the parts of one entry: a field's name and values, a comment's author, time and body.
const PART: char = '\u{1e}';
/// Between the entries of one list of entries.
const ENTRY: char = '\u{1d}';

/// Checks one issue and writes it as a line. `$n` counts from one, for the reason a row is refused.
const ROWS: &str = r#"
def clean: if . == null then "" else tostring end | gsub("[\u001c-\u001f]"; "");
def many: if . == null then [] elif type == "array" then map(clean) else [clean] end
  | map(select(. != "")) | join("\u001f");
def need($n; $k): if .[$k] == null then error("row \($n) has no `\($k)`") else . end;
def row($n):
  (if type == "object" then . else error("row \($n) is \(type), not an issue") end)
  | need($n; "reference") | need($n; "title") | need($n; "url") | need($n; "closed")
  | (if (.reference | clean) == "" then error("row \($n) has an empty `reference`") else . end)
  | (if (.closed | type) == "boolean" then . else error("row \($n) `closed` is \(.closed | type), not true or false") end)
  | [ (.reference | clean), (.title | clean), (.url | clean), (if .closed then "1" else "" end),
      (.author | clean), (.assignees | many), (.updated_at | clean),
      ((.excerpt // .body) | clean | gsub("\\s+"; " ") | ltrimstr(" ") | .[0:EXCERPT]),
      (.labels | many),
      (if ((.linked_pull_requests // []) | length) > 0 then "1" else "" end),
      ([(.fields // [])[] | ((.name | clean) + "\u001e" + (.values | many))] | join("\u001d")),
      ((.body // .excerpt) | clean),
      (.opened_at | clean),
      ([(.comments // [])[] | ((.author | clean) + "\u001e" + (.at | clean) + "\u001e" + (.body | clean))]
        | join("\u001d")),
      ([(.linked_pull_requests // [])[] | ((.number | clean) + "\u001e" + (.title | clean) + "\u001e"
        + (.url | clean) + "\u001e" + (.state | clean | ascii_downcase))] | join("\u001d"))
    ] | @tsv;
"#;

/// The whole program `jq -r` runs: the mod's `map` in parentheses, its issues spread one by one
/// whether it yielded a list, one issue or none, and each written by [`ROWS`].
pub(crate) fn program(map: &str) -> String {
    format!(
        "{}\n[({map}\n) | if . == null then empty elif type == \"array\" then .[] else . end]\n\
         | to_entries[] | (.key + 1) as $n | .value | row($n)",
        ROWS.replace("EXCERPT", &EXCERPT.to_string())
    )
}

/// What `jq` said when it refused, without its own prefix: `jq: error (at <stdin>:3): row 2 has
/// no `title`` is `row 2 has no `title``.
pub(crate) fn refusal_of(stderr: &str) -> String {
    let said = stderr.trim();
    match said.strip_prefix("jq: error (at ").and_then(|rest| rest.split_once("): ")) {
        Some((_, why)) => why.trim().to_string(),
        None => format!("the map did not run: {said}"),
    }
}

/// One line as an issue read whole. A line that is not one of [`program`]'s is not an issue.
pub(crate) fn read_issue_of(line: &str) -> Option<ReadIssue> {
    let column: Vec<&str> = line.split('\t').collect();
    let at = |n: usize| column.get(n).copied().unwrap_or_default();
    if column.len() < 15 || at(0).is_empty() {
        return None;
    }
    Some(ReadIssue {
        reference: as_written(at(0)),
        title: verbatim(at(1)),
        url: verbatim(at(2)),
        closed: !at(3).is_empty(),
        author: verbatim(at(4)),
        assignees: listed(at(5)),
        labels: listed(at(8)),
        fields: fields_of(at(10)),
        body: verbatim(at(11)),
        opened_at: as_written(at(12)),
        comments: entries(at(13))
            .map(|part| IssueComment { author: verbatim(part(0)), at: as_written(part(1)), body: verbatim(part(2)) })
            .collect(),
        linked_pull_requests: entries(at(14))
            .filter_map(|part| {
                Some(LinkedPullRequest {
                    number: part(0).parse().ok()?,
                    title: verbatim(part(1)),
                    url: verbatim(part(2)),
                    state: match part(3) {
                        "merged" => LinkedPullRequestState::Merged,
                        "closed" => LinkedPullRequestState::Closed,
                        _ => LinkedPullRequestState::Open,
                    },
                })
            })
            .collect(),
    })
}

/// One line as an issue in a list.
pub(crate) fn open_issue_of(line: &str) -> Option<OpenIssue> {
    let column: Vec<&str> = line.split('\t').collect();
    let read = read_issue_of(line)?;
    Some(OpenIssue {
        updated_at: as_written(column[6]),
        excerpt: verbatim(column[7]),
        linked_pull_request: !column[9].is_empty(),
        reference: read.reference,
        title: read.title,
        url: read.url,
        labels: read.labels,
        assignees: read.assignees,
        author: read.author,
        closed: read.closed,
        fields: read.fields,
    })
}

fn verbatim(column: &str) -> FromOutside {
    FromOutside::verbatim(as_written(column))
}

fn listed(column: &str) -> Vec<FromOutside> {
    column.split(UNIT).filter(|one| !one.is_empty()).map(verbatim).collect()
}

fn fields_of(column: &str) -> Vec<IssueField> {
    entries(column)
        .map(|part| IssueField { name: as_written(part(0)), values: listed(part(1)) })
        .filter(|field| !field.name.is_empty() && !field.values.is_empty())
        .collect()
}

/// Each entry of a column, as a reader of its parts by place.
fn entries<'a>(column: &'a str) -> impl Iterator<Item = impl Fn(usize) -> &'a str> {
    column.split(ENTRY).filter(|one| !one.is_empty()).map(|entry| {
        let parts: Vec<&'a str> = entry.split(PART).collect();
        move |n: usize| parts.get(n).copied().unwrap_or_default()
    })
}
