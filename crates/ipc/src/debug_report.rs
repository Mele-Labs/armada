//! The text of Copy debug report: names, versions and states, laid out to be pasted to whoever helps.
//!
//! **One pure function over gathered facts**, so Fleet and `armada report` print the same text from
//! what each could read. It holds no message text, prompt or file content: every value is a name, a
//! version or a state, and the home folder is written as `~`. The layout is the approved one,
//! `apps/desktop/src/renderer/src/mock/debug-report-fixtures.ts`.

use serde::{Deserialize, Serialize};

/// What `get_debug_report` answers: the whole report, ready to copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugReport {
    pub text: String,
}

/// Rows are indented two spaces and labels padded to the longest plus this many.
const GAP: usize = 3;

/// The words a value takes where a fact could not be read.
const NONE: &str = "none";

/// What the machine says of this install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    pub armada: String,
    pub runs_from: String,
    /// Whether `armada` is found on the `PATH` a Job is given.
    pub resolves: bool,
    /// What the report calls the agent CLI. The adapter that runs it knows its name.
    pub harness: String,
    /// `None` where it could not be read in time.
    pub harness_version: Option<String>,
}

/// Fleet's copy of the mod and the one the agent CLI has installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModFacts {
    pub copy_filled: bool,
    pub copy_version: Option<String>,
    pub installed_version: Option<String>,
}

/// Whether the repository has an `armada.yml`, and which of the four places keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryFacts {
    pub name: String,
    /// `present`, `absent`, or `not read` for a place that would be asked over the network.
    pub armada_yml: String,
    pub kept_in: String,
}

/// One live Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobFacts {
    pub title: String,
    pub id: String,
    pub hosted: bool,
    pub mod_version: Option<String>,
    pub door: String,
    /// What it came up without, in words; empty is nothing.
    pub missing: Vec<String>,
    /// Built-in tool names, then the door's, each group in the order to print. `None` where the Job's
    /// opening line has not been read.
    pub tools: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
}

/// What Fleet knew and the CLI cannot read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetFacts {
    pub protocol: String,
    pub mod_facts: ModFacts,
    pub repository: Option<RepositoryFacts>,
    pub jobs: Vec<JobFacts>,
}

/// One warning or refusal from Fleet's log, names only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trouble {
    /// `HH:MM:SS`, UTC.
    pub time: String,
    /// The whole instant, RFC3339, which the report does not print.
    pub at: String,
    /// `WARN` or `ERROR`.
    pub level: String,
    pub target: String,
    pub code: Option<String>,
}

/// What Fleet's log gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Log {
    /// The warnings and errors of the last day, oldest first.
    Read(Vec<Trouble>),
    /// The newest log file there is was last written this many days ago, so its lines are not
    /// recent and are not read.
    Old { days: u64 },
}

/// Everything the text is made from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub home: String,
    pub install: Install,
    /// `None` where Fleet did not answer.
    pub fleet: Option<FleetFacts>,
    /// `None` where there is no log file.
    pub trouble: Option<Log>,
    /// RFC3339, UTC, to the second.
    pub taken: String,
}

/// An RFC3339 instant to the second, as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn to_the_second(instant: &str) -> String {
    match instant.get(..19) {
        Some(whole) => format!("{whole}Z"),
        None => instant.to_string(),
    }
}

/// A block's rows aligned on the longest label; a value's further lines sit under its first.
fn block(heading: &str, rows: &[(String, String)]) -> String {
    let width = rows.iter().map(|(label, _)| label.chars().count()).max().unwrap_or(0) + GAP;
    let mut lines = vec![heading.to_string()];
    for (label, value) in rows {
        let mut parts = value.split('\n');
        lines.push(format!("  {label:<width$}{}", parts.next().unwrap_or("")).trim_end().to_string());
        lines.extend(parts.map(|part| format!("  {:width$}{part}", "")));
    }
    lines.join("\n")
}

fn row(label: &str, value: impl Into<String>) -> (String, String) {
    (label.to_string(), value.into())
}

fn or_none(version: &Option<String>) -> String {
    version.clone().unwrap_or_else(|| NONE.to_string())
}

/// Built-in names on a line, then the door's on the next, as the report prints them.
fn names(list: &Option<Vec<String>>) -> String {
    let Some(list) = list else { return "not read".to_string() };
    if list.is_empty() {
        return NONE.to_string();
    }
    let (door, built_in): (Vec<&String>, Vec<&String>) = list.iter().partition(|name| name.starts_with("mcp__"));
    let join = |group: Vec<&String>| group.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ");
    [join(built_in), join(door)].into_iter().filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n")
}

fn install(install: &Install) -> String {
    block(
        "Install",
        &[
            row("armada", &install.armada),
            row("runs from", &install.runs_from),
            row("armada resolves", if install.resolves { "yes" } else { "no" }),
            row(&install.harness, install.harness_version.clone().unwrap_or_else(|| "not found".to_string())),
        ],
    )
}

fn job(job: &JobFacts) -> String {
    let missing = if job.missing.is_empty() { "nothing".to_string() } else { job.missing.join(", ") };
    block(
        &format!("Job  {}", job.title),
        &[
            row("id", &job.id),
            row("runs", if job.hosted { "hosted" } else { "terminal" }),
            row("mod", or_none(&job.mod_version)),
            row("door", &job.door),
            row("missing", missing),
            row("tools", names(&job.tools)),
            row("skills", names(&job.skills)),
        ],
    )
}

fn trouble(log: &Log) -> String {
    let list = match log {
        Log::Read(list) => list,
        Log::Old { days } => return block("Recent trouble", &[row("log", format!("last written {days} days ago, not read"))]),
    };
    if list.is_empty() {
        return block("Recent trouble", &[row(NONE, "")]);
    }
    let rows: Vec<(String, String)> = list
        .iter()
        .map(|one| {
            let value = match &one.code {
                Some(code) => format!("{:<4}  {:<11}{code}", one.level, one.target),
                None => format!("{:<4}  {}", one.level, one.target),
            };
            row(&one.time, value)
        })
        .collect();
    block("Recent trouble", &rows)
}

/// `path` with the home folder written as `~`, at a folder boundary.
fn tilde(text: &str, home: &str) -> String {
    let home = home.trim_end_matches('/');
    if home.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(home) {
        let after = &rest[at + home.len()..];
        out.push_str(&rest[..at]);
        // The home ends where the next character could not continue a folder's name.
        if after.chars().next().map_or(true, |next| !(next.is_alphanumeric() || matches!(next, '_' | '-' | '.'))) {
            out.push('~');
        } else {
            out.push_str(home);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// The report. Blocks are separated by a blank line and the last line says when it was taken.
pub fn render(facts: &Facts) -> String {
    let mut blocks = vec!["Armada debug report".to_string(), install(&facts.install)];
    match &facts.fleet {
        Some(fleet) => {
            blocks.push(block(
                "Mod",
                &[
                    row("fleet copy", if fleet.mod_facts.copy_filled { "filled" } else { "empty" }),
                    row("fleet copy version", or_none(&fleet.mod_facts.copy_version)),
                    row("plugin installed", fleet.mod_facts.installed_version.clone().unwrap_or_else(|| "not installed".to_string())),
                ],
            ));
            if let Some(repository) = &fleet.repository {
                blocks.push(block(
                    &format!("This repository  {}", repository.name),
                    &[row("armada.yml", &repository.armada_yml), row("kept in", &repository.kept_in)],
                ));
            }
            blocks.extend(fleet.jobs.iter().map(job));
        }
        None => blocks.push("Fleet   did not answer".to_string()),
    }
    if let Some(log) = &facts.trouble {
        blocks.push(trouble(log));
    }
    let taken = format!("taken {}", facts.taken);
    blocks.push(match &facts.fleet {
        Some(fleet) => format!("fleet protocol {}  {taken}", fleet.protocol),
        None => taken,
    });
    tilde(&blocks.join("\n\n"), &facts.home)
}
