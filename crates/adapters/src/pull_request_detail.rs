//! What the forge shows on one pull request's page.
//!
//! One `pr view`, reduced by `--jq` to a tagged `@tsv` record per line, read as
//! `crate::under_review` reads its own: no document is parsed here, and a line
//! with a tag this build has no word for is dropped.
//!
//! The forge's words (`MERGEABLE`, `TIMED_OUT`, `RENAMED`) stop here. A word
//! Armada has no name for reads as the cautious answer.
//!
//! The review threads are a second call (`crate::pull_request_threads`), because the forge keeps
//! them out of `pr view`.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{
    CheckRow, CheckStanding, CommitLine, FileChange, Mergeability, Omitted, PageFile,
    PullRequestPageFacts, ReviewSaid, ReviewStanding, Reviewer, ReviewerKind, Said,
};

use crate::delivery::asked_lines;
use crate::under_review::as_written;

/// The most files one read keeps, from the first.
pub(crate) const MOST_FILES: usize = 300;
/// The most commits one read keeps, from the first.
pub(crate) const MOST_COMMITS: usize = 250;
/// The most checks one read keeps, from the first.
pub(crate) const MOST_CHECKS: usize = 500;
/// The most things said one read keeps, the latest: a thread is read from its end.
pub(crate) const MOST_SAID: usize = 200;

const FIELDS: &str = "baseRefName,author,createdAt,additions,deletions,mergeable,reviewDecision,\
                      latestReviews,reviewRequests,comments,reviews,statusCheckRollup,files,commits,headRefOid,body";

/// `// ""` on every field, so a `null` cannot stop jq mid-stream. Seconds are
/// worked out here because jq reads the timestamps; `try` makes one it cannot
/// read no figure. A draft review (`PENDING`) is its author's alone, and a
/// review with no body and no verdict says nothing.
const REDUCTION: &str = "\
    ([\"head\", (.baseRefName // \"\"), (.author.login // \"\"), (.createdAt // \"\"), \
      (.additions // 0), (.deletions // 0), (.mergeable // \"\"), (.reviewDecision // \"\"), \
      (.headRefOid // \"\"), (.body // \"\")]), \
    (.latestReviews // [] | .[] | select(.state == \"APPROVED\") | \
        [\"approver\", (.author.login // \"\")]), \
    (.latestReviews // [] | .[] | select(.state != \"PENDING\") | \
        [\"verdict\", (.author.login // \"\"), (.state // \"\")]), \
    (.reviewRequests // [] | .[] | \
        [\"requested\", (if .__typename == \"Team\" then \"team\" else \"person\" end), \
         (.login // .slug // .name // \"\")]), \
    (.comments // [] | .[] | \
        [\"comment\", (.author.login // \"\"), (.createdAt // \"\"), (.body // \"\")]), \
    (.reviews // [] | .[] | select(.state != \"PENDING\") | \
        select((.body // \"\") != \"\" or .state == \"APPROVED\" or .state == \"CHANGES_REQUESTED\") | \
        [\"review\", (.author.login // \"\"), (.submittedAt // \"\"), (.state // \"\"), (.body // \"\")]), \
    (.statusCheckRollup // [] | .[] | \
        [\"check\", (.name // .context // \"\"), (.status // \"\"), (.conclusion // .state // \"\"), \
         (if .status == \"COMPLETED\" and .startedAt and .completedAt \
          then (try ((.completedAt | fromdate) - (.startedAt | fromdate)) catch \"\") \
          else \"\" end), \
         (.detailsUrl // .targetUrl // \"\")]), \
    (.files // [] | .[] | \
        [\"file\", (.path // \"\"), (.additions // 0), (.deletions // 0), (.changeType // \"\")]), \
    (.commits // [] | .[] | \
        [\"commit\", (.oid // \"\"), (.messageHeadline // \"\"), (.authors[0].login // \"\"), \
         (.committedDate // .authoredDate // \"\")]) \
    | @tsv";

/// Ask the forge. `None` is its silence: no tool, nobody signed in, no such
/// pull request, or an answer with no head record.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> Option<PullRequestPageFacts> {
    let mut page = folded(&asked_lines(in_repo, pull_request, FIELDS, REDUCTION)?)?;
    // **A page without its threads is not a page**: a review of a diff that hides the
    // conversations on it would be answered wrongly, so their silence is the page's.
    page.threads = crate::pull_request_threads::read(in_repo, pull_request)?;
    // A name is garnish: a profile the forge will not give leaves the reviewer bare.
    with_profiles(&mut page.reviewers, |login| profile_of(in_repo, login));
    // The same: a team the forge will not list leaves its people ungrouped.
    with_teams(&mut page.reviewers, |slug| members_of(in_repo, slug));
    Some(page)
}

/// What a person's profile shows: a full name, if they set one, and the address.
pub(crate) struct Profile {
    pub name: Option<String>,
    pub url: Option<String>,
}

/// One `users/<login>` read. `None` is any failure or an empty answer; an empty
/// name is a person with no profile name, whose address still stands.
fn profile_of(in_repo: &str, login: &str) -> Option<Profile> {
    let run = crate::delivery::read_in(
        in_repo,
        crate::delivery::FORGE,
        &["api", &format!("users/{login}"), "--jq", "[.name, .html_url] | @tsv"],
    )
    .ok()?;
    if !run.status.success() {
        return None;
    }
    profile_from(&String::from_utf8_lossy(&run.stdout))
}

/// The `@tsv` line [`profile_of`] asks for, as a [`Profile`]. A null name prints
/// as an empty field.
pub(crate) fn profile_from(line: &str) -> Option<Profile> {
    let line = line.lines().next()?;
    let mut field = line.split('\t').map(as_written);
    Profile::of(field.next(), field.next())
}

impl Profile {
    /// A name or an address that is blank is none; a profile with neither is none.
    pub(crate) fn of(name: Option<String>, url: Option<String>) -> Option<Profile> {
        let present = |it: Option<String>| it.filter(|it| !it.trim().is_empty());
        let profile = Profile { name: present(name), url: present(url) };
        (profile.name.is_some() || profile.url.is_some()).then_some(profile)
    }
}

/// Put each person's profile on them, one `look` per distinct login. A team is
/// never looked up.
pub(crate) fn with_profiles(
    reviewers: &mut [Reviewer],
    mut look: impl FnMut(&str) -> Option<Profile>,
) {
    let mut seen: BTreeMap<String, Option<Profile>> = BTreeMap::new();
    for who in reviewers.iter_mut().filter(|it| it.kind == ReviewerKind::Person) {
        let profile = seen.entry(who.login.clone()).or_insert_with(|| look(&who.login));
        if let Some(profile) = profile {
            who.name = profile.name.clone();
            who.url = profile.url.clone();
        }
    }
}

/// One `orgs/<owner>/teams/<slug>/members` read, the owner being the repository's. `None` is any
/// failure, a token without the scope included.
fn members_of(in_repo: &str, slug: &str) -> Option<Vec<String>> {
    let run = crate::delivery::read_in(
        in_repo,
        crate::delivery::FORGE,
        &["api", &format!("orgs/{{owner}}/teams/{slug}/members"), "--paginate", "--jq", ".[].login"],
    )
    .ok()?;
    run.status
        .success()
        .then(|| String::from_utf8_lossy(&run.stdout).lines().map(str::to_owned).collect())
}

/// Put each person under the first requested team that lists them, one `members` read per
/// requested team. A team nobody could read lists nobody.
pub(crate) fn with_teams(
    reviewers: &mut [Reviewer],
    mut members: impl FnMut(&str) -> Option<Vec<String>>,
) {
    let teams: Vec<String> = reviewers
        .iter()
        .filter(|it| it.kind == ReviewerKind::Team)
        .map(|it| it.login.clone())
        .collect();
    for slug in teams {
        let Some(members) = members(&slug) else { continue };
        for who in reviewers.iter_mut().filter(|it| it.kind == ReviewerKind::Person && it.team.is_none()) {
            if members.contains(&who.login) {
                who.team = Some(slug.clone());
            }
        }
    }
}

/// The records the reduction printed, as one value. Split from [`read`] so a
/// test hands it what jq prints and needs no forge.
pub(crate) fn folded(lines: &[String]) -> Option<PullRequestPageFacts> {
    let mut parts = PageParts::default();
    for line in lines {
        let mut field = line.split('\t').map(as_written);
        let tag = field.next().unwrap_or_default();
        let mut next = || field.next().unwrap_or_default();
        match tag.as_str() {
            "head" => {
                parts.head = Some(Head {
                    base: next(),
                    author: next(),
                    created_at: next(),
                    additions: number(&next()),
                    deletions: number(&next()),
                    mergeable: mergeability(&next()),
                    review: review_standing(&next()),
                    commit: next(),
                    body: next(),
                });
            }
            "approver" => {
                parts.approvers.insert(next());
            }
            "verdict" => {
                let (login, state) = (next(), next());
                parts.verdicts.insert(login, review_said(&state));
            }
            "requested" => {
                let (kind, name) = (next(), next());
                let kind = if kind == "team" { ReviewerKind::Team } else { ReviewerKind::Person };
                if !name.is_empty() {
                    parts.requested.push((kind, name));
                }
            }
            "comment" => parts.said.push(Said::Comment {
                author: next(),
                at: next(),
                body: next(),
            }),
            "review" => {
                let (author, at, state, body) = (next(), next(), next(), next());
                parts.said.push(Said::Review {
                    author,
                    at,
                    verdict: review_said(&state),
                    body,
                });
            }
            "check" => {
                let (name, status, conclusion, seconds, address) =
                    (next(), next(), next(), next(), next());
                parts.checks.push(CheckRow {
                    standing: check_standing(&status, &conclusion),
                    seconds: seconds
                        .parse::<i64>()
                        .ok()
                        .and_then(|it| u64::try_from(it).ok()),
                    address: (!address.is_empty()).then_some(address),
                    name,
                });
            }
            "file" => {
                let (path, additions, deletions, change) = (next(), next(), next(), next());
                parts.files.push(PageFile {
                    path,
                    additions: number(&additions),
                    deletions: number(&deletions),
                    change: file_change(&change),
                });
            }
            "commit" => {
                let (oid, headline, author, at) = (next(), next(), next(), next());
                parts.commits.push(CommitLine {
                    sha: oid.chars().take(7).collect(),
                    headline,
                    author: (!author.is_empty()).then_some(author),
                    at: (!at.is_empty()).then_some(at),
                });
            }
            _ => {}
        }
    }
    parts.facts()
}

/// One read of the page before its order and its caps, however the forge was asked: the `gh`
/// records here, or GitHub's API in `crate::api_forge`.
#[derive(Default)]
pub(crate) struct PageParts {
    pub head: Option<Head>,
    pub approvers: BTreeSet<String>,
    pub verdicts: BTreeMap<String, ReviewSaid>,
    pub requested: Vec<(ReviewerKind, String)>,
    pub said: Vec<Said>,
    pub checks: Vec<CheckRow>,
    pub files: Vec<PageFile>,
    pub commits: Vec<CommitLine>,
    /// What the forge holds past what was read, which the caps add to.
    pub unread: Omitted,
}

impl PageParts {
    /// `None` without a head record naming a base.
    pub(crate) fn facts(self) -> Option<PullRequestPageFacts> {
        let PageParts { head, approvers, verdicts, requested, mut said, mut checks, mut files, mut commits, unread } = self;
        let head = head.filter(|head| !head.base.is_empty())?;
        // Comments and reviews arrive as two lists; the sort is stable.
        said.sort_by(|a, b| a.at().cmp(b.at()));
        let reviewers = reviewers_of(requested, verdicts);
        let cut = Omitted {
            conversation: said.len().saturating_sub(MOST_SAID),
            files: files.len().saturating_sub(MOST_FILES),
            commits: commits.len().saturating_sub(MOST_COMMITS),
            checks: checks.len().saturating_sub(MOST_CHECKS),
        };
        said.drain(..cut.conversation);
        files.truncate(MOST_FILES);
        commits.truncate(MOST_COMMITS);
        checks.truncate(MOST_CHECKS);
        Some(PullRequestPageFacts {
            base: head.base,
            author: (!head.author.is_empty()).then_some(head.author),
            created_at: (!head.created_at.is_empty()).then_some(head.created_at),
            additions: head.additions,
            deletions: head.deletions,
            mergeable: head.mergeable,
            review: head.review,
            approvals: approvers.len() as u64,
            conversation: said,
            checks,
            files,
            commits,
            omitted: Omitted {
                conversation: cut.conversation + unread.conversation,
                files: cut.files + unread.files,
                commits: cut.commits + unread.commits,
                checks: cut.checks + unread.checks,
            },
            head: (!head.commit.is_empty()).then_some(head.commit),
            body: (!head.body.trim().is_empty()).then_some(head.body),
            threads: Vec::new(),
            reviewers,
        })
    }
}

/// The asked first, in the forge's order, each with the verdict they have already
/// given; then those who reviewed and are asked no longer, by login.
fn reviewers_of(
    requested: Vec<(ReviewerKind, String)>,
    mut verdicts: BTreeMap<String, ReviewSaid>,
) -> Vec<Reviewer> {
    let mut reviewers: Vec<Reviewer> = Vec::new();
    for (kind, login) in requested {
        if reviewers.iter().any(|it| it.kind == kind && it.login == login) {
            continue;
        }
        let verdict = match kind {
            ReviewerKind::Person => verdicts.remove(&login),
            ReviewerKind::Team => None,
        };
        reviewers.push(Reviewer { login, kind, verdict, name: None, url: None, team: None });
    }
    reviewers.extend(verdicts.into_iter().map(|(login, verdict)| Reviewer {
        login,
        kind: ReviewerKind::Person,
        verdict: Some(verdict),
        name: None,
        url: None,
        team: None,
    }));
    reviewers
}

pub(crate) struct Head {
    pub base: String,
    pub author: String,
    pub created_at: String,
    pub additions: u64,
    pub deletions: u64,
    pub mergeable: Mergeability,
    pub review: ReviewStanding,
    pub commit: String,
    pub body: String,
}

fn number(said: &str) -> u64 {
    said.trim().parse().unwrap_or(0)
}

pub(crate) fn mergeability(said: &str) -> Mergeability {
    match said {
        "MERGEABLE" => Mergeability::Mergeable,
        "CONFLICTING" => Mergeability::Conflicting,
        _ => Mergeability::Unknown,
    }
}

pub(crate) fn review_standing(said: &str) -> ReviewStanding {
    match said {
        "APPROVED" => ReviewStanding::Approved,
        "CHANGES_REQUESTED" => ReviewStanding::ChangesRequested,
        "REVIEW_REQUIRED" => ReviewStanding::Required,
        _ => ReviewStanding::None,
    }
}

pub(crate) fn review_said(state: &str) -> ReviewSaid {
    match state {
        "APPROVED" => ReviewSaid::Approved,
        "CHANGES_REQUESTED" => ReviewSaid::ChangesRequested,
        _ => ReviewSaid::Commented,
    }
}

/// A check run has `status` and, once over, `conclusion`; a status context has
/// only `state`, which the reduction puts in the conclusion's place. A finished
/// check with a conclusion nobody named did not pass.
pub(crate) fn check_standing(status: &str, conclusion: &str) -> CheckStanding {
    match conclusion {
        "SUCCESS" => CheckStanding::Passed,
        "NEUTRAL" | "SKIPPED" => CheckStanding::Skipped,
        "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED"
        | "STARTUP_FAILURE" | "STALE" => CheckStanding::Failed,
        "" | "PENDING" | "EXPECTED" => CheckStanding::Running,
        _ if status == "COMPLETED" => CheckStanding::Failed,
        _ => CheckStanding::Running,
    }
}

/// `COPIED` and `CHANGED` read as modified: the page has no fifth word.
pub(crate) fn file_change(said: &str) -> FileChange {
    match said {
        "ADDED" => FileChange::Added,
        "DELETED" => FileChange::Removed,
        "RENAMED" => FileChange::Renamed,
        _ => FileChange::Modified,
    }
}
