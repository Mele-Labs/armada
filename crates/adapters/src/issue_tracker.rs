//! The tracker Fleet reads issues from and writes back on, for the one forge
//! this workspace already assumes elsewhere. #91, `docs/concepts/tracker.md`.
//!
//! **No parse**, `crate::main_ci`'s way: `--jq` reduces the listing to `@tsv`,
//! and a write's only answer is whether it worked.

use std::process::{Command, Output};

use adapter_traits::{FromOutside, IssueAddress, NotTracked, OpenIssue, ReadIssue, Tracker};

use crate::delivery::{said, FORGE};
use crate::issue_lookup::issue_reference;
use crate::under_review::as_written;

/// How many open issues one listing reads. A backlog past this is read newest
/// edit first, which is the order a person chooses from.
const LISTED: &str = "200";

/// How much of a body the list carries, in characters.
const EXCERPT: usize = 240;

/// Each issue as one line: number, title, address, labels and assignees joined
/// by `US`, last edit, the body's start, and whether a pull request closes it.
const OPEN_ISSUES: &str = "\
    .[] | [(.number | tostring), (.title // \"\"), (.url // \"\"), \
    ([.labels[]?.name] | join(\"\\u001f\")), ([.assignees[]?.login] | join(\"\\u001f\")), \
    (.updatedAt // \"\"), ((.body // \"\") | gsub(\"\\\\s+\"; \" \") | .[0:EXCERPT]), \
    LINKED] | @tsv";

/// One issue as a line: number, title, address and state.
const READ_ISSUE: &str = "[(.number | tostring), (.title // \"\"), (.url // \"\"), (.state // \"\")] | @tsv";

/// The field that says a pull request closes it. Newer than the rest of the
/// listing, so a CLI that does not know it is asked again without it.
const CLOSED_BY: &str = "closedByPullRequestsReferences";

/// Made on first use, so a repository needs no setup before a Job writes back.
const LABEL_COLOUR: &str = "ededed";
const LABEL_SAYS: &str = "Set by Armada while a Job works this issue";

pub struct IssueTracker;

impl Tracker for IssueTracker {
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked> {
        let doing = "listing the open issues";
        let run = listed(in_repo, true).map_err(|why| NotTracked::of(doing, why.to_string()))?;
        let run = match run.status.success() {
            true => run,
            false if said(&run).contains(CLOSED_BY) => {
                listed(in_repo, false).map_err(|why| NotTracked::of(doing, why.to_string()))?
            }
            false => return Err(NotTracked::of(doing, said(&run))),
        };
        if !run.status.success() {
            return Err(NotTracked::of(doing, said(&run)));
        }
        let mut issues: Vec<OpenIssue> = String::from_utf8_lossy(&run.stdout)
            .lines()
            .filter_map(issue_of_line)
            .collect();
        // RFC 3339 in one zone sorts as text.
        issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(issues)
    }

    fn read_issue(&self, in_repo: &str, number: u64) -> Result<Option<ReadIssue>, NotTracked> {
        let doing = "reading the issue";
        let number = number.to_string();
        let run = Command::new(FORGE)
            .args(["issue", "view", number.as_str(), "--json", "number,title,url,state"])
            .args(["--jq", READ_ISSUE])
            .current_dir(in_repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|why| NotTracked::of(doing, why.to_string()))?;
        if !run.status.success() {
            let refusal = said(&run);
            return match refusal.contains("Could not resolve") || refusal.contains("not found") {
                true => Ok(None),
                false => Err(NotTracked::of(doing, refusal)),
            };
        }
        Ok(String::from_utf8_lossy(&run.stdout)
            .lines()
            .find_map(read_issue_of_line))
    }

    fn comment(&self, issue: &IssueAddress, text: &str) -> Result<(), NotTracked> {
        let doing = "commenting on the issue";
        let url = addressed(issue, doing)?;
        done(
            forge(&["issue", "comment", url, "--body", text]),
            doing,
        )
    }

    fn relabel(
        &self,
        issue: &IssueAddress,
        add: &[&str],
        remove: &[&str],
    ) -> Result<(), NotTracked> {
        let doing = "moving the issue's labels";
        let url = addressed(issue, doing)?;
        let mut args = vec!["issue", "edit", url];
        for label in add {
            args.extend(["--add-label", label]);
        }
        for label in remove {
            args.extend(["--remove-label", label]);
        }
        if args.len() == 3 {
            return Ok(());
        }
        let first = forge(&args).map_err(|why| NotTracked::of(doing, why.to_string()))?;
        if first.status.success() {
            return Ok(());
        }
        // The usual first refusal: the repository has never had the label.
        // Made here and asked once more; a second refusal is the answer.
        let (owner, repo, _) = issue_reference(url).ok_or_else(|| NotTracked::of(doing, said(&first)))?;
        let repository = format!("{owner}/{repo}");
        for label in add {
            let _ = forge(&[
                "label",
                "create",
                label,
                "--repo",
                &repository,
                "--color",
                LABEL_COLOUR,
                "--description",
                LABEL_SAYS,
            ]);
        }
        done(forge(&args), doing)
    }

    fn closing_line(&self, issue: &IssueAddress) -> Option<String> {
        let (owner, repo, number) = issue_reference(issue.url())?;
        Some(format!("Closes {owner}/{repo}#{number}"))
    }
}

/// The issue's canonical address, refused where it is not one this forge
/// names — so nothing that is not an issue's address reaches the CLI.
fn addressed<'a>(issue: &'a IssueAddress, doing: &'static str) -> Result<&'a str, NotTracked> {
    match issue_reference(issue.url()) {
        Some(_) if !issue.url().starts_with('-') => Ok(issue.url()),
        _ => Err(NotTracked::of(
            doing,
            format!("`{}` is not an issue's address", issue.url()),
        )),
    }
}

fn listed(in_repo: &str, with_links: bool) -> Result<Output, std::io::Error> {
    let fields = match with_links {
        true => format!("number,title,url,labels,assignees,updatedAt,body,{CLOSED_BY}"),
        false => "number,title,url,labels,assignees,updatedAt,body".to_string(),
    };
    let linked = match with_links {
        true => format!("(if ((.{CLOSED_BY} // []) | length) > 0 then \"1\" else \"0\" end)"),
        false => "\"0\"".to_string(),
    };
    let jq = OPEN_ISSUES
        .replace("EXCERPT", &EXCERPT.to_string())
        .replace("LINKED", &linked);
    Command::new(FORGE)
        .args(["issue", "list", "--state", "open", "--limit", LISTED, "--json"])
        .args([fields.as_str(), "--jq", jq.as_str()])
        .current_dir(in_repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
}

/// A write by address: no repository directory is needed, so none is named.
fn forge(args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(FORGE)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
}

fn done(run: Result<Output, std::io::Error>, doing: &'static str) -> Result<(), NotTracked> {
    let run = run.map_err(|why| NotTracked::of(doing, why.to_string()))?;
    match run.status.success() {
        true => Ok(()),
        false => Err(NotTracked::of(doing, said(&run))),
    }
}

/// One `@tsv` line of [`OPEN_ISSUES`]. A line with no number is not an issue.
pub(crate) fn issue_of_line(line: &str) -> Option<OpenIssue> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let title = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let labels = field.next().unwrap_or_default();
    let assignees = field.next().unwrap_or_default();
    let updated_at = field.next().unwrap_or_default();
    let excerpt = field.next().unwrap_or_default();
    let linked = field.next().unwrap_or_default();
    let listed = |joined: &str| -> Vec<FromOutside> {
        as_written(joined)
            .split('\u{1f}')
            .filter(|one| !one.is_empty())
            .map(FromOutside::verbatim)
            .collect()
    };
    Some(OpenIssue {
        number,
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(as_written(url)),
        labels: listed(labels),
        assignees: listed(assignees),
        updated_at: updated_at.to_string(),
        excerpt: FromOutside::verbatim(as_written(excerpt)),
        linked_pull_request: linked == "1",
    })
}

/// One `@tsv` line of [`READ_ISSUE`]. **An address under `/pull/` is a pull request's**, which the
/// forge's issue view answers for a pull request's number, and is not an issue here.
pub(crate) fn read_issue_of_line(line: &str) -> Option<ReadIssue> {
    let mut field = line.split('\t');
    let number = field.next()?.parse().ok()?;
    let title = field.next().unwrap_or_default();
    let url = field.next().unwrap_or_default();
    let state = field.next().unwrap_or_default();
    if url.contains("/pull/") {
        return None;
    }
    Some(ReadIssue {
        number,
        title: FromOutside::verbatim(as_written(title)),
        url: FromOutside::verbatim(as_written(url)),
        closed: !state.eq_ignore_ascii_case("open"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_issue_reads_as_closed_and_an_open_one_as_open() {
        let closed = read_issue_of_line("12\tOld\thttps://forge.example/o/r/issues/12\tCLOSED")
            .expect("an issue");
        assert!(closed.closed);
        assert_eq!(closed.title.as_written(), "Old");
        let open = read_issue_of_line("13\tNew\thttps://forge.example/o/r/issues/13\tOPEN")
            .expect("an issue");
        assert!(!open.closed);
    }

    #[test]
    fn a_pull_requests_address_is_not_an_issue() {
        assert_eq!(
            read_issue_of_line("14\tA change\thttps://forge.example/o/r/pull/14\tMERGED"),
            None
        );
        assert_eq!(read_issue_of_line(""), None);
    }

    #[test]
    fn a_line_reads_into_an_issue_with_its_labels_apart() {
        let line = "91\tTicket system link\thttps://github.com/o/r/issues/91\t\
                    idea\u{1f}armada: in progress\tnick\t2026-10-11T19:15:42Z\tThe idea\\tis\t1";
        let issue = issue_of_line(line).expect("an issue");
        assert_eq!(issue.number, 91);
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

    #[test]
    fn the_closing_line_names_the_repository_so_it_closes_across_repositories() {
        let issue = IssueAddress::at("r#9", "https://github.com/o/r/issues/9");
        assert_eq!(
            IssueTracker.closing_line(&issue).as_deref(),
            Some("Closes o/r#9")
        );
        let not = IssueAddress::at("r#9", "https://example.com/elsewhere");
        assert_eq!(IssueTracker.closing_line(&not), None);
    }

    #[test]
    fn a_write_to_something_that_is_not_an_issue_is_refused_before_the_cli() {
        let not = IssueAddress::at("x", "--repo=evil");
        let refused = IssueTracker.comment(&not, "hello").expect_err("refused");
        assert!(refused.said.contains("is not an issue's address"));
    }
}
