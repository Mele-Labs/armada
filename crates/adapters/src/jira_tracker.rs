//! The Jira tracker: the same seam as `crate::issue_tracker`, through `acli`
//! and `jq`. #91, `docs/concepts/tracker.md`, *Jira*.
//!
//! **A runner stands between this and the machine**, so a test hands it what
//! `acli` and `jq` would print and reads the commands it was given. Nothing here
//! parses JSON; `jq` reduces it to `@tsv` and this reads lines.

use std::io::{ErrorKind, Write};
use std::process::{Command, Output, Stdio};

use adapter_traits::{FromOutside, IssueAddress, NotTracked, OpenIssue, ReadIssue, Tracker};

use crate::delivery::said;
use crate::jira::{self, as_label, is_key, JiraSettings};
use crate::under_review::as_written;

/// How many open issues one listing reads, newest edit first.
const LISTED: &str = "200";

/// How much of a description the list carries, in characters.
const EXCERPT: usize = 240;

const SEARCH_FIELDS: &str = "key,summary,labels,assignee,updated,description";

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

    /// `acli` alone, for a write: whether it worked is its only answer.
    fn written(&self, args: &[&str], doing: &'static str) -> Result<(), NotTracked> {
        let args: Vec<String> = args.iter().map(|one| one.to_string()).collect();
        let run = self
            .run
            .run("acli", &args, None)
            .map_err(|why| missing("acli", &why, doing))?;
        match run.status.success() {
            true => Ok(()),
            false => Err(NotTracked::of(doing, said(&run))),
        }
    }

    /// The key an address names in this tracker, refused where it names none:
    /// nothing that is not an issue's key reaches the CLI.
    fn key_of(&self, issue: &IssueAddress, doing: &'static str) -> Result<String, NotTracked> {
        match self.settings.key_in(issue.url()) {
            Some(key) if is_key(&key) => Ok(key),
            _ => Err(NotTracked::of(
                doing,
                format!("`{}` is not an issue's address on this site", issue.url()),
            )),
        }
    }
}

fn missing(tool: &str, why: &std::io::Error, doing: &'static str) -> NotTracked {
    match why.kind() {
        ErrorKind::NotFound => NotTracked::of(
            doing,
            format!("`{tool}` is not installed, or not on the PATH Fleet runs with"),
        ),
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

    fn read_issue(&self, _in_repo: &str, number: u64) -> Result<Option<ReadIssue>, NotTracked> {
        let doing = "reading the issue";
        let key = self.settings.key_of(number);
        let (worked, text) = self.reduced(
            &["jira", "workitem", "view", &key, "--fields", "key,summary,status,resolution", "--json"],
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
        Ok(text
            .lines()
            .find_map(|line| read_issue_of_line(&self.settings, line)))
    }

    fn comment(&self, issue: &IssueAddress, text: &str) -> Result<(), NotTracked> {
        let doing = "commenting on the issue";
        let key = self.key_of(issue, doing)?;
        self.written(
            &["jira", "workitem", "comment", "create", "--key", &key, "--body", text],
            doing,
        )
    }

    /// **The whole set is written, not only the change.** Whether `--labels`
    /// adds or replaces is unverified (`docs/concepts/tracker.md`), and the
    /// current labels plus the added ones, less the removed, is right either way.
    fn relabel(
        &self,
        issue: &IssueAddress,
        add: &[&str],
        remove: &[&str],
    ) -> Result<(), NotTracked> {
        let doing = "moving the issue's labels";
        let key = self.key_of(issue, doing)?;
        let add: Vec<String> = add.iter().map(|one| as_label(one)).collect();
        let remove: Vec<String> = remove.iter().map(|one| as_label(one)).collect();
        if add.is_empty() && remove.is_empty() {
            return Ok(());
        }
        let (worked, held) = self.reduced(
            &["jira", "workitem", "view", &key, "--fields", "labels", "--json"],
            &jira::labels_filter(),
            doing,
        )?;
        if !worked {
            return Err(NotTracked::of(doing, held));
        }
        let mut labels: Vec<String> = held.lines().map(str::to_string).collect();
        for one in &add {
            if !labels.contains(one) {
                labels.push(one.clone());
            }
        }
        labels.retain(|one| !remove.contains(one));
        let (labels, remove) = (labels.join(","), remove.join(","));
        let mut args = vec!["jira", "workitem", "edit", "--key", key.as_str()];
        if !add.is_empty() {
            args.extend(["--labels", labels.as_str()]);
        }
        if !remove.is_empty() {
            args.extend(["--remove-labels", remove.as_str()]);
        }
        args.push("--yes");
        self.written(&args, doing)
    }

    /// Nothing in a body closes a Jira issue: its integration with the forge reads the key.
    fn closing_line(&self, _issue: &IssueAddress) -> Option<String> {
        None
    }

    fn title_mark(&self, issue: &IssueAddress) -> Option<String> {
        self.settings.key_in(issue.url()).filter(|key| is_key(key))
    }
}

/// One `@tsv` line of the listing. A key of another project is not this
/// project's issue, whatever a custom filter let through.
pub(crate) fn issue_of_line(settings: &JiraSettings, line: &str) -> Option<OpenIssue> {
    let mut field = line.split('\t');
    let key = field.next()?;
    let number = settings.number_of(key)?;
    let title = field.next().unwrap_or_default();
    let labels = field.next().unwrap_or_default();
    let assignee = field.next().unwrap_or_default();
    let updated_at = field.next().unwrap_or_default();
    let excerpt = field.next().unwrap_or_default();
    Some(OpenIssue {
        number,
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(settings.browse(key)),
        labels: as_written(labels)
            .split('\u{1f}')
            .filter(|one| !one.is_empty())
            .map(FromOutside::verbatim)
            .collect(),
        assignees: Some(as_written(assignee))
            .filter(|one| !one.is_empty())
            .map(FromOutside::verbatim)
            .into_iter()
            .collect(),
        updated_at: updated_at.to_string(),
        excerpt: FromOutside::verbatim(as_written(excerpt)),
        linked_pull_request: false,
    })
}

/// One `@tsv` line of the reading: key, summary, status, resolution.
pub(crate) fn read_issue_of_line(settings: &JiraSettings, line: &str) -> Option<ReadIssue> {
    let mut field = line.split('\t');
    let key = field.next()?;
    let number = settings.number_of(key)?;
    let title = field.next().unwrap_or_default();
    let status = field.next().unwrap_or_default();
    let resolution = field.next().unwrap_or_default();
    let done = ["done", "closed", "resolved", "complete", "completed"].contains(&status);
    Some(ReadIssue {
        number,
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(settings.browse(key)),
        closed: done || !resolution.is_empty(),
    })
}
