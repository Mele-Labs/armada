//! The tracker Fleet reads issues from, for the one forge this workspace
//! already assumes elsewhere. #91, `docs/concepts/tracker.md`.
//!
//! **No parse**, `crate::main_ci`'s way: `--jq` reduces the listing to `@tsv`.
//! **Nothing here writes.**

use adapter_traits::{
    FromOutside, IssueComment, IssueField, LinkedPullRequest, LinkedPullRequestState, NotTracked, OpenIssue,
    QueryGuide, ReadIssue, Tracker,
};

use crate::delivery::{said, FORGE};
use crate::under_review::as_written;

/// How many open issues one listing reads. A backlog past this is read newest
/// edit first, which is the order a person chooses from.
pub(crate) const LISTED: &str = "200";

/// How much of a body the list carries, in characters.
pub(crate) const EXCERPT: usize = 240;

/// Each issue as one line: number, title, address, labels and assignees joined
/// by `US`, last edit, the body's start, and whether a pull request closes it. A search adds
/// [`SEARCHED_MORE`].
const OPEN_ISSUES: &str = "\
    .[] | [(.number | tostring), (.title // \"\"), (.url // \"\"), \
    ([.labels[]?.name] | join(\"\\u001f\")), ([.assignees[]?.login] | join(\"\\u001f\")), \
    (.updatedAt // \"\"), ((.body // \"\") | gsub(\"\\\\s+\"; \" \") | .[0:EXCERPT]), \
    LINKED MORE] | @tsv";

/// Who opened it, its state, its milestone and its projects.
const SEARCHED_MORE: &str = ", (.author.login // \"\"), (.state // \"\"), (.milestone.title // \"\"), PROJECTS";

/// Each project's title, with its status where it has one: `Roadmap: In progress`.
const PROJECTS: &str = "([.projectItems[]? | (.title // \"\") \
    + (if (.status.name // \"\") != \"\" then \": \" + .status.name else \"\" end)] | join(\"\\u001f\"))";

/// An issue's projects in a GraphQL read, as [`PROJECTS`] reads them from `gh`'s listing.
pub(crate) const PROJECT_ITEMS: &str = "projectItems(first:20){nodes{project{title} \
    fieldValueByName(name:\"Status\"){... on ProjectV2ItemFieldSingleSelectValue{name}}}}";

/// One issue, whole, in one `api graphql` read: the pull requests that close it
/// (`closedByPullRequestsReferences`) and the ones that name it (a cross-reference whose source
/// is a pull request) come back with it, so a linked pull request needs no second request. A
/// pull request's number is not an issue's to the forge's `issue` field, which is how it is none.
/// Its milestone and, where the token may read them, its projects too.
pub(crate) fn read_issue_query(with_projects: bool) -> String {
    let projects = if with_projects { PROJECT_ITEMS } else { "" };
    READ_ISSUE_QUERY.replace("PROJECTS", projects)
}

const READ_ISSUE_QUERY: &str = "query($owner:String!,$name:String!,$number:Int!){\
    repository(owner:$owner,name:$name){issue(number:$number){\
    number title url state body createdAt author{login} \
    labels(first:50){nodes{name}} assignees(first:20){nodes{login}} milestone{title} PROJECTS \
    comments(first:100){nodes{author{login} createdAt body}} \
    closedByPullRequestsReferences(first:20,includeClosedPrs:true){nodes{number title url state}} \
    timelineItems(first:50,itemTypes:[CROSS_REFERENCED_EVENT]){nodes{\
    ... on CrossReferencedEvent{source{... on PullRequest{number title url state}}}}}}}}";

/// Tagged `@tsv` records, `crate::pull_request_threads`' way: an `issue` record, a `comment`
/// record for each comment and a `pr` record for each pull request linked. `// ""` on every
/// field so a `null` cannot stop jq mid-stream. A cross-reference from an issue has an empty
/// `source`, which has no number and prints nothing.
const READ_ISSUE: &str = "\
    .data.repository.issue | select(. != null) | \
    ([\"issue\", (.number | tostring), (.title // \"\"), (.url // \"\"), (.state // \"\"), \
      (.author.login // \"\"), (.createdAt // \"\"), \
      ([.labels.nodes[]?.name] | join(\"\\u001f\")), \
      ([.assignees.nodes[]?.login] | join(\"\\u001f\")), (.body // \"\"), (.milestone.title // \"\"), \
      ([.projectItems.nodes[]? | (.project.title // \"\") \
        + (if (.fieldValueByName.name // \"\") != \"\" then \": \" + .fieldValueByName.name else \"\" end)] \
       | join(\"\\u001f\"))], \
     (.comments.nodes[]? | [\"comment\", (.author.login // \"\"), (.createdAt // \"\"), (.body // \"\")]), \
     ((.closedByPullRequestsReferences.nodes[]?, (.timelineItems.nodes[]?.source // empty)) \
      | select(.number != null) \
      | [\"pr\", (.number | tostring), (.title // \"\"), (.url // \"\"), (.state // \"\")])) \
    | @tsv";

/// The field that says a pull request closes it. Newer than the rest of the
/// listing, so a CLI that does not know it is asked again without it.
const CLOSED_BY: &str = "closedByPullRequestsReferences";

pub struct IssueTracker;

impl Tracker for IssueTracker {
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        run_listing(in_repo, None, "listing the open issues")
    }

    fn read_issue(&self, in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        let doing = "reading the issue";
        // `#12` or `12`; text that is not a number names no issue here, and never reaches the CLI.
        let Ok(number) = reference.trim_start_matches('#').parse::<u64>() else {
            return Ok(None);
        };
        let number = format!("number={number}");
        let mut with_projects = true;
        let run = loop {
            let query = format!("query={}", read_issue_query(with_projects));
            let run = crate::background::command(FORGE)
                .args(["api", "graphql", "-F", "owner={owner}", "-F", "name={repo}", "-F"])
                .args([number.as_str(), "-f", query.as_str(), "--jq", READ_ISSUE])
                .current_dir(in_repo)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .map_err(|why| NotTracked::of(doing, why.to_string()))?;
            match run.status.success() || !with_projects || !said(&run).contains(PROJECT_SCOPE) {
                true => break run,
                false => with_projects = false,
            }
        };
        if !run.status.success() {
            let refusal = said(&run);
            return match refusal.contains("Could not resolve to an") || refusal.contains("not found") {
                true => Ok(None),
                false => Err(NotTracked::of(doing, refusal)),
            };
        }
        Ok(read_issue_of_records(&String::from_utf8_lossy(&run.stdout)))
    }

    fn search_issues(&self, in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        run_listing(in_repo, Some(query), "searching the issues")
    }

    fn query_guide(&self) -> QueryGuide {
        forge_guide()
    }
}

/// How many issues one search reads, which is as many as one page of the forge's search.
pub(crate) const SEARCHED: usize = 100;

/// The scope a token needs to read an issue's projects. One without it is asked again without
/// them, so a search never fails for want of a column.
pub(crate) const PROJECT_SCOPE: &str = "read:project";

/// The open issues, or the ones `search` matches, newest edit first. A CLI that does not know
/// [`CLOSED_BY`], or a token that may not read projects, is asked again without them.
fn run_listing(in_repo: &str, search: Option<&str>, doing: &'static str) -> Result<Vec<OpenIssue>, NotTracked> {
    let (mut with_links, mut with_projects) = (true, search.is_some());
    loop {
        let run = crate::background::command(FORGE)
            .args(listing_args(search, with_links, with_projects))
            .current_dir(in_repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|why| NotTracked::of(doing, why.to_string()))?;
        if run.status.success() {
            let mut issues: Vec<OpenIssue> = String::from_utf8_lossy(&run.stdout)
                .lines()
                .filter_map(issue_of_line)
                .collect();
            // RFC 3339 in one zone sorts as text.
            issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            return Ok(issues);
        }
        let refusal = said(&run);
        if with_links && refusal.contains(CLOSED_BY) {
            with_links = false;
        } else if with_projects && refusal.contains(PROJECT_SCOPE) {
            with_projects = false;
        } else {
            return Err(NotTracked::of(doing, refusal));
        }
    }
}

/// The search's arguments to `gh`: every state, since the query says which, and the fields a row
/// can show. `--search=` keeps a query that starts with a dash a value.
#[cfg(test)]
pub(crate) fn search_args(query: &str, with_links: bool, with_projects: bool) -> Vec<String> {
    listing_args(Some(query), with_links, with_projects)
}

fn listing_args(search: Option<&str>, with_links: bool, with_projects: bool) -> Vec<String> {
    let mut fields = "number,title,url,labels,assignees,updatedAt,body".to_string();
    let linked = match with_links {
        true => {
            fields.push_str(&format!(",{CLOSED_BY}"));
            format!("(if ((.{CLOSED_BY} // []) | length) > 0 then \"1\" else \"0\" end)")
        }
        false => "\"0\"".to_string(),
    };
    let mut more = String::new();
    if search.is_some() {
        fields.push_str(",author,state,milestone");
        if with_projects {
            fields.push_str(",projectItems");
        }
        more = SEARCHED_MORE.replace("PROJECTS", if with_projects { PROJECTS } else { "\"\"" });
    }
    let jq = OPEN_ISSUES
        .replace("EXCERPT", &EXCERPT.to_string())
        .replace("LINKED", &linked)
        .replace("MORE", &more);
    let mut args: Vec<String> = vec!["issue".into(), "list".into()];
    match search {
        None => args.extend(["--state", "open", "--limit", LISTED].map(String::from)),
        Some(query) => args.extend([
            format!("--search={}", newest_edit_first(query)),
            "--state".into(),
            "all".into(),
            "--limit".into(),
            SEARCHED.to_string(),
        ]),
    }
    args.extend(["--json".into(), fields, "--jq".into(), jq]);
    args
}

/// `query` asking for the newest edit first, unless it already says how it sorts.
pub(crate) fn newest_edit_first(query: &str) -> String {
    let query = query.trim();
    match query.split_whitespace().any(|word| word.starts_with("sort:")) {
        true => query.to_string(),
        false => format!("{query} sort:updated-desc").trim_start().to_string(),
    }
}

/// The fields a forge issue carries that Armada has no slot for, each left out where empty.
pub(crate) fn forge_fields(labels: &[FromOutside], milestone: &str, projects: Vec<FromOutside>) -> Vec<IssueField> {
    let milestone: Vec<FromOutside> = Some(milestone).filter(|it| !it.is_empty()).map(FromOutside::verbatim).into_iter().collect();
    [("Labels", labels.to_vec()), ("Milestone", milestone), ("Projects", projects)]
        .into_iter()
        .filter(|(_, values)| !values.is_empty())
        .map(|(name, values)| IssueField { name: name.to_string(), values })
        .collect()
}

/// How a model writes the forge's issue search.
pub(crate) fn forge_guide() -> QueryGuide {
    QueryGuide {
        language: "GitHub issue search".to_string(),
        guide: FORGE_GUIDE.to_string(),
        open: "is:open".to_string(),
        assigned_to_me: "is:open assignee:@me".to_string(),
        fields: ["Labels", "Milestone", "Projects"].map(String::from).to_vec(),
    }
}

const FORGE_GUIDE: &str = "GitHub's issue search syntax. The query is already held to this \
repository's issues, so never write `repo:`, `is:issue` or `is:pr`. Terms separated by spaces must \
all match; `OR` between two terms means either, parentheses group, and `-` before a qualifier negates \
it. Qualifiers: `is:open`, `is:closed` (neither means both), `author:LOGIN`, `assignee:LOGIN`, \
`assignee:@me` (whoever is signed in), `no:assignee`, `mentions:@me`, `label:NAME` (quote a name with \
spaces: `label:\"good first issue\"`; `label:a,b` is either), `no:label`, `milestone:\"TITLE\"`, \
`no:milestone`, `project:OWNER/NUMBER`, `linked:pr`, `-linked:pr`, `reason:completed`, \
`reason:\"not planned\"`, `comments:>10`, `created:>=2026-01-01`, `updated:>=2026-10-01`, \
`closed:<2026-10-01`, a range `updated:2026-09-01..2026-09-30`, and plain words, which match the title \
and body (`in:title WORD` for the title alone). Results come newest edit first, at most 100, so write \
no `sort:`. Examples: `is:open label:bug -label:wontfix`, `is:open assignee:@me updated:>=2026-10-01`, \
`is:closed milestone:\"v2\" reason:completed`, `is:open no:assignee -linked:pr`. It cannot filter \
on a project's Status column, compare two fields, or match a label by part of its name.";

/// One `@tsv` line of [`OPEN_ISSUES`]. A line with no number is not an issue.
pub(crate) fn issue_of_line(line: &str) -> Option<OpenIssue> {
    let mut field = line.split('\t');
    let number: u64 = field.next()?.parse().ok()?;
    let title = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let labels = field.next().unwrap_or_default();
    let assignees = field.next().unwrap_or_default();
    let updated_at = field.next().unwrap_or_default();
    let excerpt = field.next().unwrap_or_default();
    let linked = field.next().unwrap_or_default();
    // A search's: the open listing stops before them.
    let author = field.next().unwrap_or_default();
    let state = field.next().unwrap_or_default();
    let milestone = field.next().unwrap_or_default();
    let projects = field.next().unwrap_or_default();
    let labels = listed(labels);
    Some(OpenIssue {
        reference: format!("#{number}"),
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(as_written(url)),
        fields: forge_fields(&labels, &as_written(milestone), listed(projects)),
        labels,
        assignees: listed(assignees),
        updated_at: updated_at.to_string(),
        excerpt: FromOutside::verbatim(as_written(excerpt)),
        linked_pull_request: linked == "1",
        author: FromOutside::verbatim(as_written(author)),
        closed: state.eq_ignore_ascii_case("closed"),
    })
}

/// A `US`-joined column, each value apart.
fn listed(joined: &str) -> Vec<FromOutside> {
    as_written(joined)
        .split('\u{1f}')
        .filter(|one| !one.is_empty())
        .map(FromOutside::verbatim)
        .collect()
}

/// The records [`READ_ISSUE`] printed, as an issue. `None` where there is no `issue` record: the
/// forge had none by that number. Split from `read_issue` so a test hands it what jq printed.
pub(crate) fn read_issue_of_records(printed: &str) -> Option<ReadIssue> {
    let mut issue: Option<ReadIssue> = None;
    let mut comments = Vec::new();
    let mut linked: Vec<LinkedPullRequest> = Vec::new();
    for line in printed.lines() {
        let mut field = line.split('\t');
        let mut next = || field.next().unwrap_or_default();
        match next() {
            "issue" if issue.is_none() => {
                let number: u64 = next().parse().ok()?;
                let title = next();
                let url = next();
                let state = next();
                let author = next();
                let opened_at = next();
                let labels = next();
                let assignees = next();
                let body = next();
                let milestone = next();
                let projects = next();
                let mut read = ReadIssue::bare(
                    format!("#{number}"),
                    FromOutside::verbatim(as_written(title)),
                    FromOutside::verbatim(as_written(url)),
                    !state.eq_ignore_ascii_case("open"),
                );
                read.body = FromOutside::verbatim(as_written(body));
                read.author = FromOutside::verbatim(as_written(author));
                read.opened_at = opened_at.to_string();
                read.labels = listed(labels);
                read.assignees = listed(assignees);
                read.fields = forge_fields(&read.labels, &as_written(milestone), listed(projects));
                issue = Some(read);
            }
            "comment" => comments.push(IssueComment {
                author: FromOutside::verbatim(as_written(next())),
                at: next().to_string(),
                body: FromOutside::verbatim(as_written(next())),
            }),
            "pr" => {
                let Ok(number) = next().parse::<u64>() else { continue };
                let title = as_written(next());
                let url = as_written(next());
                let state = match next() {
                    "MERGED" => LinkedPullRequestState::Merged,
                    "CLOSED" => LinkedPullRequestState::Closed,
                    _ => LinkedPullRequestState::Open,
                };
                // A pull request that closes the issue is often named in it too.
                if linked.iter().all(|one| one.url.as_written() != url) {
                    linked.push(LinkedPullRequest {
                        number,
                        title: FromOutside::verbatim(title),
                        url: FromOutside::verbatim(url),
                        state,
                    });
                }
            }
            _ => {}
        }
    }
    let mut issue = issue?;
    issue.comments = comments;
    issue.linked_pull_requests = linked;
    Some(issue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_issue_reads_as_closed_and_an_open_one_as_open() {
        let closed = read_issue_of_records("issue\t12\tOld\thttps://forge.example/o/r/issues/12\tCLOSED\t\t\t\t\t")
            .expect("an issue");
        assert!(closed.closed);
        assert_eq!(closed.title.as_written(), "Old");
        let open = read_issue_of_records("issue\t13\tNew\thttps://forge.example/o/r/issues/13\tOPEN\t\t\t\t\t")
            .expect("an issue");
        assert!(!open.closed);
    }

    /// What the forge's answer reduces to for a closed issue with two comments, a pull request that
    /// closes it and the same one naming it, and another that only names it.
    #[test]
    fn an_issue_with_comments_and_linked_pull_requests_reads_into_every_field() {
        let printed = "\
issue\t214\tDoctor reports a stale pid\thttps://github.com/o/r/issues/214\tCLOSED\tnick\t2026-10-01T09:00:00Z\tbug\u{1f}armada: in progress\tnick\u{1f}sam\tIt says running.\\n\\n- one\\n- two\\tTabbed
comment\tsam\t2026-10-02T10:00:00Z\tSeen here too.
comment\t\t2026-10-03T11:30:00Z\tFrom a deleted account\\nsecond line
pr\t230\tFix the stale pid\thttps://github.com/o/r/pull/230\tMERGED
pr\t230\tFix the stale pid\thttps://github.com/o/r/pull/230\tMERGED
pr\t228\tTry another way\thttps://github.com/o/r/pull/228\tCLOSED
pr\t240\tFollow-up\thttps://github.com/o/r/pull/240\tOPEN
";
        let read = read_issue_of_records(printed).expect("an issue");
        assert_eq!(read.reference, "#214");
        assert!(read.closed);
        assert_eq!(read.body.as_written(), "It says running.\n\n- one\n- two\tTabbed");
        assert_eq!(read.author.as_written(), "nick");
        assert_eq!(read.opened_at, "2026-10-01T09:00:00Z");
        assert_eq!(
            read.labels,
            vec![FromOutside::verbatim("bug"), FromOutside::verbatim("armada: in progress")]
        );
        assert_eq!(
            read.assignees,
            vec![FromOutside::verbatim("nick"), FromOutside::verbatim("sam")]
        );
        assert_eq!(read.comments.len(), 2);
        assert_eq!(read.comments[0].author.as_written(), "sam");
        assert_eq!(read.comments[0].at, "2026-10-02T10:00:00Z");
        assert_eq!(read.comments[0].body.as_written(), "Seen here too.");
        assert_eq!(read.comments[1].author.as_written(), "");
        assert_eq!(read.comments[1].body.as_written(), "From a deleted account\nsecond line");
        let linked: Vec<(u64, LinkedPullRequestState)> = read
            .linked_pull_requests
            .iter()
            .map(|one| (one.number, one.state))
            .collect();
        assert_eq!(
            linked,
            vec![
                (230, LinkedPullRequestState::Merged),
                (228, LinkedPullRequestState::Closed),
                (240, LinkedPullRequestState::Open),
            ]
        );
        assert_eq!(read.linked_pull_requests[0].title.as_written(), "Fix the stale pid");
        assert_eq!(read.linked_pull_requests[0].url.as_written(), "https://github.com/o/r/pull/230");
    }

    #[test]
    fn nothing_printed_is_no_issue() {
        assert_eq!(read_issue_of_records(""), None);
        assert_eq!(read_issue_of_records("comment\tsam\t2026-10-02T10:00:00Z\tOrphan"), None);
    }

    #[test]
    fn text_that_is_not_a_number_names_no_issue_and_never_reaches_the_cli() {
        for text in ["ABC-12", "--repo=evil", "", "#"] {
            assert_eq!(IssueTracker.read_issue("/nowhere", text), Ok(None), "{text}");
        }
    }

    #[test]
    fn a_line_reads_into_an_issue_with_its_labels_apart() {
        let line = "91\tTicket system link\thttps://github.com/o/r/issues/91\t\
                    idea\u{1f}armada: in progress\tnick\t2026-10-11T19:15:42Z\tThe idea\\tis\t1";
        let issue = issue_of_line(line).expect("an issue");
        assert_eq!(issue.reference, "#91");
        assert_eq!(issue.title.as_written(), "Ticket system link");
        assert_eq!(
            issue.labels,
            vec![
                FromOutside::verbatim("idea"),
                FromOutside::verbatim("armada: in progress")
            ]
        );
        assert_eq!(issue.assignees, vec![FromOutside::verbatim("nick")]);
        assert_eq!(issue.excerpt.as_written(), "The idea\tis");
        assert!(issue.linked_pull_request);
    }

    #[test]
    fn no_labels_and_no_link_read_as_none_and_false() {
        let issue = issue_of_line("7\tt\tu\t\t\t2026-01-01T00:00:00Z\t\t0").expect("an issue");
        assert!(issue.labels.is_empty());
        assert!(issue.assignees.is_empty());
        assert!(!issue.linked_pull_request);
    }

    #[test]
    fn a_line_with_no_number_is_not_an_issue() {
        assert_eq!(issue_of_line("\ttitle"), None);
        assert_eq!(issue_of_line(""), None);
    }

}
