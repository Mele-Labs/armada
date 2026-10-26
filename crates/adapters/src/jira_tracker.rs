//! The Jira tracker: the same seam as `crate::issue_tracker`, through `acli`
//! and `jq`. #91, `docs/concepts/tracker.md`, *Jira*.
//!
//! **A runner stands between this and the machine**, so a test hands it what
//! `acli` and `jq` would print and reads the commands it was given. Nothing here
//! parses JSON; `jq` reduces it to `@tsv` and this reads lines.

use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

use adapter_traits::{FromOutside, IssueComment, IssueField, NotTracked, OpenIssue, QueryGuide, ReadIssue, Tracker};

use crate::delivery::said;
use crate::jira::{self, is_key, JiraSettings};
use crate::under_review::as_written;

/// How many open issues one listing reads, newest edit first.
const LISTED: &str = "200";

/// How much of a description the list carries, in characters.
const EXCERPT: usize = 240;

/// How many issues one search reads.
const SEARCHED: &str = "100";

/// `acli`'s search refuses `updated`, so a listing keeps the order the JQL asked for.
const SEARCH_FIELDS: &str = "key,summary,labels,assignee,description";

/// What runs a program: given its arguments and, where it takes any, its stdin.
pub(crate) trait Run: Send + Sync {
    fn run(&self, program: &str, args: &[String], stdin: Option<&[u8]>) -> std::io::Result<Output>;
}

/// The machine's own processes.
pub(crate) struct Process;

impl Run for Process {
    fn run(&self, program: &str, args: &[String], stdin: Option<&[u8]>) -> std::io::Result<Output> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(match stdin {
                Some(_) => Stdio::piped(),
                None => Stdio::null(),
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        // Written from its own thread: a filter that answers while it is still
        // being fed would otherwise fill its pipe and wait on a writer that waits on it.
        let feeding = stdin.zip(child.stdin.take()).map(|(bytes, mut pipe)| {
            let bytes = bytes.to_vec();
            std::thread::spawn(move || pipe.write_all(&bytes))
        });
        let output = child.wait_with_output()?;
        if let Some(feeding) = feeding {
            let _ = feeding.join();
        }
        Ok(output)
    }
}

pub(crate) struct JiraTracker<R = Process> {
    settings: JiraSettings,
    run: R,
}

impl JiraTracker<Process> {
    pub(crate) fn over(settings: JiraSettings) -> JiraTracker<Process> {
        JiraTracker {
            settings,
            run: Process,
        }
    }
}

impl<R: Run> JiraTracker<R> {
    #[cfg(test)]
    pub(crate) fn with_runner(settings: JiraSettings, run: R) -> JiraTracker<R> {
        JiraTracker { settings, run }
    }

    /// `acli` with these arguments, its JSON through `jq` with this filter, and
    /// what `jq` printed. **A tool that is not there says so**, since that is
    /// the likeliest reason on a machine that has never run this.
    fn reduced(
        &self,
        args: &[&str],
        filter: &str,
        doing: &'static str,
    ) -> Result<(bool, String), NotTracked> {
        let args: Vec<String> = args.iter().map(|one| one.to_string()).collect();
        let asked = self
            .run
            .run("acli", &args, None)
            .map_err(|why| missing("acli", &why, doing))?;
        if !asked.status.success() {
            return Ok((false, said(&asked)));
        }
        let filtered = self
            .run
            .run("jq", &["-r".to_string(), filter.to_string()], Some(&asked.stdout))
            .map_err(|why| missing("jq", &why, doing))?;
        match filtered.status.success() {
            true => Ok((true, String::from_utf8_lossy(&filtered.stdout).into_owned())),
            false => Err(NotTracked::of(doing, said(&filtered))),
        }
    }

}

pub(crate) fn missing(tool: &str, why: &std::io::Error, doing: &'static str) -> NotTracked {
    match why.kind() {
        ErrorKind::NotFound => NotTracked::lacking(doing, tool),
        _ => NotTracked::of(doing, why.to_string()),
    }
}

impl<R: Run> Tracker for JiraTracker<R> {
    fn open_issues(&self, _in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        let doing = "listing the open issues";
        let jql = self.settings.search();
        let (worked, text) = self.reduced(
            &[
                "jira", "workitem", "search", "--jql", &jql, "--fields", SEARCH_FIELDS, "--json",
                "--limit", LISTED,
            ],
            &jira::listing_filter(EXCERPT),
            doing,
        )?;
        if !worked {
            return Err(NotTracked::of(doing, text));
        }
        let mut issues: Vec<OpenIssue> = text
            .lines()
            .filter_map(|line| issue_of_line(&self.settings, line))
            .collect();
        // RFC 3339 in one zone sorts as text; the search already asked for this order.
        issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(issues)
    }

    fn read_issue(&self, _in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        let doing = "reading the issue";
        // Only a key reaches the CLI; a number alone names no issue across projects.
        let key = reference.trim();
        if !is_key(key) {
            return Ok(None);
        }
        let (worked, text) = self.reduced(
            &["jira", "workitem", "view", key, "--fields", &jira::read_fields(), "--json"],
            &jira::reading_filter(),
            doing,
        )?;
        if !worked {
            let lower = text.to_ascii_lowercase();
            return match ["does not exist", "not found", "404"]
                .iter()
                .any(|sign| lower.contains(sign))
            {
                true => Ok(None),
                false => Err(NotTracked::of(doing, text)),
            };
        }
        Ok(read_issue_of_records(&self.settings, &text))
    }

    fn search_issues(&self, _in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        let doing = "searching the issues";
        let jql = self.settings.search_for(query).map_err(|why| NotTracked::of(doing, why))?;
        let fields = self.settings.search_fields();
        let (worked, text) = self.reduced(
            &["jira", "workitem", "search", "--jql", &jql, "--fields", &fields, "--json", "--limit", SEARCHED],
            &jira::search_filter(EXCERPT, &self.settings),
            doing,
        )?;
        if !worked {
            return Err(NotTracked::of(doing, text));
        }
        let mut issues: Vec<OpenIssue> = text
            .lines()
            .filter_map(|line| issue_of_line(&self.settings, line))
            .collect();
        issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(issues)
    }

    fn issues_linked_in(&self, _in_repo: &str, text: &str) -> Vec<String> {
        self.settings.keys_in(text)
    }

    fn issues_named_by_pull_request(&self, title: &str, branch: &str) -> Vec<String> {
        self.settings.keys_named_by(title, branch)
    }

    fn query_guide(&self) -> QueryGuide {
        self.settings.guide()
    }

    /// The site's `browse` page, for an example that is a Jira key.
    fn address_template(&self, example: &str) -> Option<String> {
        is_key(example.trim()).then(|| self.settings.browse("{match}"))
    }
}

/// One `@tsv` line of the listing, or of a search, which goes on with the reporter, the status,
/// the resolution and each field. **A line whose first field is not a key is not an issue**;
/// which projects are listed was the search's to say, and is not checked again here.
pub(crate) fn issue_of_line(settings: &JiraSettings, line: &str) -> Option<OpenIssue> {
    let mut field = line.split('\t');
    let key = field.next().filter(|key| is_key(key))?;
    let title = field.next().unwrap_or_default();
    let labels = field.next().unwrap_or_default();
    let assignee = field.next().unwrap_or_default();
    let updated_at = field.next().unwrap_or_default();
    let excerpt = field.next().unwrap_or_default();
    let reporter = field.next().unwrap_or_default();
    let status = field.next().unwrap_or_default();
    let resolution = field.next().unwrap_or_default();
    let fields = settings
        .named_fields()
        .into_iter()
        .map(|(name, _)| IssueField { name, values: listed(field.next().unwrap_or_default()) })
        .filter(|one| !one.values.is_empty())
        .collect();
    Some(OpenIssue {
        reference: key.to_string(),
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(settings.browse(key)),
        labels: listed(labels),
        assignees: Some(as_written(assignee))
            .filter(|one| !one.is_empty())
            .map(FromOutside::verbatim)
            .into_iter()
            .collect(),
        updated_at: updated_at.to_string(),
        excerpt: FromOutside::verbatim(as_written(excerpt)),
        linked_pull_request: false,
        author: FromOutside::verbatim(as_written(reporter)),
        closed: is_done(status) || !resolution.is_empty(),
        fields,
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

/// A status category, or a status's own name, that means the issue is finished.
fn is_done(status: &str) -> bool {
    ["done", "closed", "resolved", "complete", "completed"].contains(&status)
}

/// The records [`jira::reading_filter`] printed, as an issue: its `issue` line, then each `comment`.
/// `None` where no `issue` line holds a key.
pub(crate) fn read_issue_of_records(settings: &JiraSettings, printed: &str) -> Option<ReadIssue> {
    let mut issue: Option<ReadIssue> = None;
    let mut comments = Vec::new();
    for line in printed.lines() {
        let mut field = line.split('\t');
        let mut next = || field.next().unwrap_or_default();
        match next() {
            "issue" if issue.is_none() => {
                let key = Some(next()).filter(|key| is_key(key))?;
                let title = next();
                let status = next();
                let resolution = next();
                let mut read = ReadIssue::bare(
                    key.to_string(),
                    FromOutside::verbatim(as_written(title)),
                    FromOutside::verbatim(settings.browse(key)),
                    is_done(status) || !resolution.is_empty(),
                );
                read.author = FromOutside::verbatim(as_written(next()));
                read.opened_at = next().to_string();
                read.labels = listed(next());
                read.assignees = listed(next());
                read.body = FromOutside::verbatim(as_written(next()));
                read.fields = jira::read_field_names()
                    .map(|name| IssueField { name: name.to_string(), values: listed(next()) })
                    .filter(|one| !one.values.is_empty())
                    .collect();
                issue = Some(read);
            }
            "comment" => comments.push(IssueComment {
                author: FromOutside::verbatim(as_written(next())),
                at: next().to_string(),
                body: FromOutside::verbatim(as_written(next())),
            }),
            _ => {}
        }
    }
    let mut issue = issue?;
    issue.comments = comments;
    Some(issue)
}
