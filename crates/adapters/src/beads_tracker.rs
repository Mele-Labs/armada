//! The beads tracker: an issue tracker on Dolt with a copy of the database on each machine, read
//! and written through `bd`, its JSON reduced by `jq` as Jira's is. `docs/concepts/tracker.md`,
//! *Beads*.
//!
//! **Every call runs in the main checkout**, found from whatever root it is given through git's
//! common directory. Beads keeps one database for a repository and every worktree of it, so one
//! folder for every call is one database and one server per machine, whichever slot asked.
//!
//! **The one tracker Fleet writes to**, so it is a [`Backlog`] as well as a [`Tracker`].

use std::path::{Path, PathBuf};

use adapter_traits::{
    Backlog, BeadState, CopyMark, FromOutside, IssueField, IssueWrite, NotTracked, OpenIssue, QueryGuide, ReadIssue, TicketState,
    Tracker, TrackerWrites,
};

use crate::delivery::{last_line, said};
use crate::jira_tracker::{missing, Process, Run};
use crate::under_review::as_written;

pub(crate) const PROVIDER: &str = "beads";

/// What beads calls the ticket's states. **Its statuses are fixed**, so the words are Armada's to
/// give: `blocked` and `deferred` are no step of the ticket's way and stay unmapped, and none of
/// beads' five says In review.
pub(crate) const TICKET_WORDS: &[(TicketState, &str)] =
    &[(TicketState::ToDo, "open"), (TicketState::InProgress, "in_progress"), (TicketState::Closed, "closed")];

/// How much of a description the list carries, in characters.
const EXCERPT: usize = 240;

/// How many issues one search reads.
const SEARCHED: &str = "100";

const OPEN: &str = "status=open OR status=in_progress OR status=blocked";

const GUIDE: &str = "A query is `field=value` comparisons joined by AND, OR and NOT, with parentheses. \
Fields: status (open, in_progress, blocked, deferred, closed), priority (0 to 4), \
type (bug, feature, task, epic, chore, decision), assignee (none for unassigned), label, title and \
description (both contain), created, updated, closed (dates, or 7d for seven days ago), id (wildcards: armada-*). \
Quote a value with spaces: assignee=\"Ana Ruiz\". \
Examples: `status=open AND label=bug`, `status=open AND title=shell`, `priority<=1 AND NOT status=closed`. \
It cannot search comments.";

/// The columns every listing and search line carries, `@tsv`.
fn line_filter() -> String {
    format!(
        r#"(if type == "array" then .[] else . end)
| [(.id // ""), (.title // ""), ((.labels // []) | join("\u001f")), (.assignee // ""), (.updated_at // ""),
   ((.description // "") | gsub("\\s+"; " ") | gsub("^ | $"; "") | .[0:{EXCERPT}]),
   (.created_by // ""), (.status // ""), (.priority // "" | tostring), (.issue_type // ""), (.external_ref // "")]
| @tsv"#
    )
}

/// One issue read whole, one `@tsv` line.
const READ_FILTER: &str = r#"(if type == "array" then .[0] else . end)
| [(.id // ""), (.title // ""), (.status // ""), (.created_by // ""), (.created_at // ""),
   ((.labels // []) | join("\u001f")), (.assignee // ""), (.description // ""),
   (.priority // "" | tostring), (.issue_type // ""), (.external_ref // "")]
| @tsv"#;

pub(crate) struct BeadsTracker<R = Process> {
    run: R,
}

impl BeadsTracker<Process> {
    pub(crate) fn over() -> BeadsTracker<Process> {
        BeadsTracker { run: Process }
    }
}

impl<R: Run> BeadsTracker<R> {
    #[cfg(test)]
    pub(crate) fn with_runner(run: R) -> BeadsTracker<R> {
        BeadsTracker { run }
    }

    /// `bd -C <main checkout>` with `args`, and whether it exited 0 with what it said.
    fn bd(&self, in_repo: &str, args: &[&str], doing: &'static str) -> Result<std::process::Output, NotTracked> {
        let mut all = vec!["-C".to_string(), main_checkout(Path::new(in_repo)).display().to_string()];
        all.extend(args.iter().map(|one| one.to_string()));
        self.run.run("bd", &all, None).map_err(|why| missing("bd", &why, doing))
    }

    /// A read through `bd --readonly … --json`, reduced by `jq` with `filter`: what `jq` printed,
    /// or `Err` with what `bd` said where it did not exit 0.
    fn read(&self, in_repo: &str, args: &[&str], filter: &str, doing: &'static str) -> Result<Result<String, String>, NotTracked> {
        let mut all = vec!["--readonly"];
        all.extend_from_slice(args);
        all.push("--json");
        let asked = self.bd(in_repo, &all, doing)?;
        if !asked.status.success() {
            return Ok(Err(said(&asked)));
        }
        let filtered = self
            .run
            .run("jq", &["-r".to_string(), filter.to_string()], Some(&asked.stdout))
            .map_err(|why| missing("jq", &why, doing))?;
        match filtered.status.success() {
            true => Ok(Ok(String::from_utf8_lossy(&filtered.stdout).into_owned())),
            false => Err(NotTracked::of(doing, said(&filtered))),
        }
    }

    fn listed(&self, in_repo: &str, query: &str, limit: &str, doing: &'static str) -> Result<Vec<OpenIssue>, NotTracked> {
        let text = self
            .read(in_repo, &["query", query, "-n", limit], &line_filter(), doing)?
            .map_err(|said| refused(doing, said))?;
        let mut issues: Vec<OpenIssue> = text.lines().filter_map(issue_of_line).collect();
        // RFC 3339 in UTC sorts as text.
        issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(issues)
    }
}

impl<R: Run> Tracker for BeadsTracker<R> {
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.listed(in_repo, OPEN, "0", "listing the open issues")
    }

    fn read_issue(&self, in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked> {
        let doing = "reading the issue";
        let reference = reference.trim();
        if reference.is_empty() || reference.starts_with('-') {
            return Ok(None);
        }
        match self.read(in_repo, &["show", reference], READ_FILTER, doing)? {
            Ok(text) => Ok(text.lines().find_map(read_of_line)),
            Err(said) if said.contains("no issue found") => Ok(None),
            Err(said) => Err(refused(doing, said)),
        }
    }

    fn search_issues(&self, in_repo: &str, query: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        self.listed(in_repo, query, SEARCHED, "searching the issues")
    }

    fn query_guide(&self) -> QueryGuide {
        QueryGuide {
            language: "bd query".to_string(),
            guide: GUIDE.to_string(),
            open: OPEN.to_string(),
            // Beads has no word for whoever is asking, and Fleet's claims name a machine and a Job:
            // what is being worked is the nearest list.
            assigned_to_me: "status=in_progress".to_string(),
            fields: FIELD_NAMES.iter().map(|one| one.to_string()).collect(),
        }
    }
}

impl<R: Run> Backlog for BeadsTracker<R> {
    /// **Never adopts a remote**: which remote this machine's copy syncs with is set up once, by
    /// hand or a setup script, and a sync that chose one would choose it silently.
    fn sync(&self, in_repo: &str) -> Result<(), NotTracked> {
        let doing = "syncing the backlog";
        let synced = self.bd(in_repo, &["sync", "--no-adopt"], doing)?;
        match synced.status.success() {
            true => Ok(()),
            false => Err(NotTracked::of(doing, said(&synced))),
        }
    }

    fn mark(&self, in_repo: &str) -> Option<CopyMark> {
        mark_of(&main_checkout(Path::new(in_repo)).join(".beads"))
    }

    fn claim(&self, in_repo: &str, reference: &str, actor: &str) -> Result<(), NotTracked> {
        let doing = "claiming the issue";
        let claimed = self.bd(in_repo, &["update", reference.trim(), "--claim", "--actor", actor], doing)?;
        match claimed.status.success() {
            true => Ok(()),
            false => Err(NotTracked::of(doing, said(&claimed))),
        }
    }

    fn create_child(&self, in_repo: &str, parent: &str, title: &str) -> Result<String, NotTracked> {
        let doing = "filing a task under the issue";
        let made = self.bd(in_repo, &["create", "--title", title, "--parent", parent.trim(), "--silent"], doing)?;
        if !made.status.success() {
            return Err(NotTracked::of(doing, said(&made)));
        }
        let reference = last_line(&made);
        match reference.is_empty() || reference.contains(char::is_whitespace) {
            true => Err(NotTracked::of(doing, format!("it printed no issue id: {}", said(&made)))),
            false => Ok(reference),
        }
    }

    fn retitle(&self, in_repo: &str, reference: &str, title: &str) -> Result<(), NotTracked> {
        self.written(in_repo, &["update", reference.trim(), "--title", title], "renaming the issue")
    }

    fn put_state(&self, in_repo: &str, reference: &str, state: BeadState, reason: &str) -> Result<(), NotTracked> {
        let reference = reference.trim();
        match state {
            // `--force`: a task closed while the one before it is open, or a group whose children
            // are, is not for beads to refuse.
            BeadState::Closed => self.written(in_repo, &["close", reference, "--reason", reason, "--force"], "closing the issue"),
            _ => self.written(in_repo, &["update", reference, "--status", state.word()], "moving the issue"),
        }
    }

    fn block(&self, in_repo: &str, blocked: &str, blocker: &str) -> Result<(), NotTracked> {
        self.written(in_repo, &["dep", "add", blocked.trim(), blocker.trim()], "ordering the issues")
    }

    fn unblock(&self, in_repo: &str, blocked: &str, blocker: &str) -> Result<(), NotTracked> {
        self.written(in_repo, &["dep", "remove", blocked.trim(), blocker.trim()], "ordering the issues")
    }

    fn close_tree(&self, in_repo: &str, reference: &str, reason: &str) -> Result<(), NotTracked> {
        let doing = "closing the issue and its children";
        let reference = reference.trim();
        let mut open = Vec::new();
        self.open_under(in_repo, reference, DEPTH, &mut open, doing)?;
        // Deepest first, so nothing is closed over an open child.
        open.reverse();
        let mut args = vec!["close"];
        args.extend(open.iter().map(String::as_str));
        if let Some(own) = self.is_open(in_repo, reference, doing)?.then_some(reference) {
            args.push(own);
        }
        if args.len() == 1 {
            return Ok(());
        }
        args.extend(["--reason", reason, "--force"]);
        self.written(in_repo, &args, doing)
    }
}

impl<R: Run> TrackerWrites for BeadsTracker<R> {
    fn can(&self, write: &IssueWrite<'_>) -> Result<(), String> {
        bd_arguments("armada-1", write).map(drop)
    }

    fn write(&self, in_repo: &str, reference: &str, write: &IssueWrite<'_>) -> Result<(), NotTracked> {
        let doing = write.doing();
        let reference = reference.trim();
        if reference.is_empty() || reference.starts_with('-') {
            return Err(NotTracked::of(doing, format!("`{reference}` is not an issue id")));
        }
        let args = bd_arguments(reference, write).map_err(|why| NotTracked::of(doing, why))?;
        self.written(in_repo, &args.iter().map(String::as_str).collect::<Vec<_>>(), doing)
    }
}

/// What a close says it was for, since `bd` keeps one.
const CLOSED_BY_A_RULE: &str = "Closed by a rule";

/// The statuses `bd update --status` takes, beside `closed`, which is `bd close`.
const BD_STATUSES: [&str; 3] = ["open", "in_progress", "blocked"];

/// The `bd` arguments that make `write` to the issue `reference`, or why `bd` cannot.
///
/// **A close is forced past a blocker or an open child**, as [`Backlog::put_state`]'s is: the rule
/// is the owner's say that the thing is done.
pub(crate) fn bd_arguments(reference: &str, write: &IssueWrite<'_>) -> Result<Vec<String>, String> {
    let close = || ["close", reference, "--reason", CLOSED_BY_A_RULE, "--force"].map(String::from).to_vec();
    match write {
        IssueWrite::Move { status: "closed" } | IssueWrite::Close { .. } => Ok(close()),
        IssueWrite::Move { status } if BD_STATUSES.contains(status) => {
            Ok(["update", reference, "--status", status].map(String::from).to_vec())
        }
        IssueWrite::Move { status } => Err(format!("bd has no status called `{status}`; it has open, in_progress, blocked and closed")),
        IssueWrite::Assign { to } => match to.trim() {
            name if name.is_empty() || name.starts_with('-') => Err(format!("`{to}` is not a name bd takes for an assignee")),
            name => Ok(["update", reference, "--assignee", name].map(String::from).to_vec()),
        },
    }
}

/// How many generations of children a close looks down. A plan is group and task; this is room.
const DEPTH: usize = 4;

impl<R: Run> BeadsTracker<R> {
    /// A write: `bd` with `args`, and whether it exited 0.
    fn written(&self, in_repo: &str, args: &[&str], doing: &'static str) -> Result<(), NotTracked> {
        let ran = self.bd(in_repo, args, doing)?;
        match ran.status.success() {
            true => Ok(()),
            false => Err(NotTracked::of(doing, said(&ran))),
        }
    }

    /// Whether `reference` is not closed. A read that finds no such issue says it is not.
    fn is_open(&self, in_repo: &str, reference: &str, doing: &'static str) -> Result<bool, NotTracked> {
        match self.read(in_repo, &["show", reference], STATUS_FILTER, doing)? {
            Ok(text) => Ok(text.trim() != "closed"),
            Err(said) if said.contains("no issue found") => Ok(false),
            Err(said) => Err(refused(doing, said)),
        }
    }

    /// Every child of `reference` that is not closed, parents before their own children.
    fn open_under(&self, in_repo: &str, reference: &str, depth: usize, found: &mut Vec<String>, doing: &'static str) -> Result<(), NotTracked> {
        if depth == 0 {
            return Ok(());
        }
        let text = self.read(in_repo, &["children", reference], CHILDREN_FILTER, doing)?.map_err(|said| refused(doing, said))?;
        for line in text.lines() {
            let mut field = line.split('\t');
            let (id, status) = (field.next().unwrap_or_default(), field.next().unwrap_or_default());
            if id.is_empty() {
                continue;
            }
            if status != "closed" {
                found.push(id.to_string());
            }
            self.open_under(in_repo, id, depth - 1, found, doing)?;
        }
        Ok(())
    }
}

/// One issue's status.
const STATUS_FILTER: &str = r#"(if type == "array" then .[0] else . end) | (.status // "")"#;

/// A child's id and status, one `@tsv` line each.
const CHILDREN_FILTER: &str = r#"(if type == "array" then .[] else . end) | [(.id // ""), (.status // "")] | @tsv"#;

const FIELD_NAMES: [&str; 3] = ["Status", "Priority", "Type"];

fn fields(status: &str, priority: &str, kind: &str) -> Vec<IssueField> {
    let priority = (!priority.is_empty()).then(|| format!("P{priority}")).unwrap_or_default();
    FIELD_NAMES
        .iter()
        .zip([status.to_string(), priority, kind.to_string()])
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| IssueField { name: name.to_string(), values: vec![FromOutside::verbatim(value)] })
        .collect()
}

fn listed(joined: &str) -> Vec<FromOutside> {
    as_written(joined).split('\u{1f}').filter(|one| !one.is_empty()).map(FromOutside::verbatim).collect()
}

/// One line of a listing or a search. **A line with no id is not an issue.**
pub(crate) fn issue_of_line(line: &str) -> Option<OpenIssue> {
    let mut field = line.split('\t');
    let mut next = || field.next().unwrap_or_default();
    let reference = Some(next()).filter(|one| !one.is_empty())?.to_string();
    let (title, labels, assignee, updated_at, excerpt) = (next(), next(), next(), next(), next());
    let (author, status, priority, kind, url) = (next(), next(), next(), next(), next());
    Some(OpenIssue {
        reference,
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(as_written(url)),
        labels: listed(labels),
        assignees: listed(assignee),
        updated_at: updated_at.to_string(),
        excerpt: FromOutside::verbatim(as_written(excerpt)),
        linked_pull_request: false,
        author: FromOutside::verbatim(as_written(author)),
        closed: status == "closed",
        fields: fields(status, priority, kind),
    })
}

/// The one line [`READ_FILTER`] prints, as an issue.
pub(crate) fn read_of_line(line: &str) -> Option<ReadIssue> {
    let mut field = line.split('\t');
    let mut next = || field.next().unwrap_or_default();
    let reference = Some(next()).filter(|one| !one.is_empty())?.to_string();
    let (title, status, author, opened_at, labels) = (next(), next(), next(), next(), next());
    let (assignee, body, priority, kind, url) = (next(), next(), next(), next(), next());
    let mut read = ReadIssue::bare(
        reference,
        FromOutside::verbatim(as_written(title)),
        FromOutside::verbatim(as_written(url)),
        status == "closed",
    );
    read.author = FromOutside::verbatim(as_written(author));
    read.opened_at = opened_at.to_string();
    read.labels = listed(labels);
    read.assignees = listed(assignee);
    read.body = FromOutside::verbatim(as_written(body));
    read.fields = fields(status, priority, kind);
    Some(read)
}

/// What `bd` said where a read did not work. **No project is a setup, not a failure**: Bridge offers
/// the button from `needs_setup` and does not read the sentence.
fn refused(doing: &'static str, said: String) -> NotTracked {
    match said.contains("no beads project found") {
        true => NotTracked::unset(doing, "bd", said),
        false => NotTracked::of(doing, said),
    }
}

/// The checkout git keeps its common directory in: `root` itself, or the one a linked worktree at
/// `root` was cut from. `root` where neither can be read.
pub(crate) fn main_checkout(root: &Path) -> PathBuf {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return root.to_path_buf();
    }
    let linked = std::fs::read_to_string(&dot_git).ok().and_then(|text| {
        let gitdir = root.join(text.trim().strip_prefix("gitdir:")?.trim());
        let common = std::fs::read_to_string(gitdir.join("commondir")).ok()?;
        let common = gitdir.join(common.trim()).canonicalize().ok()?;
        (common.file_name()? == ".git").then(|| common.parent().map(Path::to_path_buf)).flatten()
    });
    linked.unwrap_or_else(|| root.to_path_buf())
}

/// Every Dolt store under `beads`, each by its `noms/manifest`: the root hash it names moves on a
/// write and a pull, and not on a read, where the files' times move on both. `None` where there is
/// none, as before the database is made.
pub(crate) fn mark_of(beads: &Path) -> Option<CopyMark> {
    let mut found = Vec::new();
    manifests(beads, 5, &mut found);
    found.sort();
    let read: Vec<String> = found
        .iter()
        .filter_map(|path| Some(format!("{}={}", path.display(), std::fs::read_to_string(path).ok()?.trim())))
        .collect();
    (!read.is_empty()).then(|| CopyMark::of(read.join("\n")))
}

fn manifests(folder: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
        if is_dir && depth > 0 {
            manifests(&path, depth - 1, found);
        } else if !is_dir && path.file_name().is_some_and(|name| name == "manifest") && folder.ends_with("noms") {
            found.push(path);
        }
    }
}
