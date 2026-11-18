//! Fleet's answers as the screen's view model. Words and places are carried over as Fleet gives them;
//! what is cut here is only what the screen draws, such as a hunk's lines out of its file's patch.

use std::collections::{HashMap, HashSet};

use ipc::{
    ChangeSetKind, MyReviewSaid, OverviewSeverity, PracticeVerdict, PullInbox, PullRequestCompare, PullRequestFilePatch, PullRequestNote, PullRequestPage, PullReviewList,
    PullReviewLists, ReviewResult, ReviewRun, ReviewRunKind, ReviewRunState, ReviewToolKind, ReviewTools,
};

use super::Row;
use crate::reviews::model::{self, ChangeSet, Critique, Flag, FlagKind, Hunk, InboxRow, Kept, Mine, Page, PageFile, Practice, Summary, Verdict, Viewed};
pub use crate::since_looked::when;
use crate::since_looked::{new_commits_of, seen_of, short};

/// Where one of Fleet's runs stands for the head in front.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Find {
    /// Nothing kept at this head: only a press starts it.
    #[default]
    NotRun,
    Running,
    Failed(String),
    Done,
}

/// Where the page's runs stand, beside what they found.
#[derive(Clone, Debug, Default)]
pub struct Finds {
    pub sets: Find,
    pub flags: Find,
    /// By change set id.
    pub practices: HashMap<String, Find>,
    /// A sentence for each tool run that failed or was refused.
    pub critique_failed: Vec<String>,
}

/// What a pull request's runs come to, for its page.
pub struct Mapped {
    pub summary: Summary,
    pub sets: Vec<ChangeSet>,
    pub flags: Vec<Flag>,
    pub critique: Critique,
    pub finds: Finds,
    /// What Summarize, Explain and Simplify answered at the head, those asked.
    pub answers: Vec<(model::Rewrite, model::Answer)>,
}

/// What the runs are read against: the heads, the runs and patches as Fleet answered, and the runs it refused.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    /// The head the page was read at, whole.
    pub head: Option<String>,
    /// The head he last left it at, whole.
    pub seen: Option<String>,
    pub gone: bool,
    pub runs: Vec<ReviewRun>,
    pub patches: Vec<PullRequestFilePatch>,
    /// Runs Fleet refused to start, by kind and name, with its reason.
    pub refused: Vec<(ReviewRunKind, String, String)>,
}

impl Facts {
    /// Whether some run of this kind and name is kept at another head, so a press must ask again for this one.
    pub fn kept_elsewhere(&self, kind: ReviewRunKind, name: &str) -> bool {
        self.runs.iter().any(|run| run.kind == kind && named(kind, &run.name, name) && Some(run.head.as_str()) != self.head.as_deref())
    }
}

/// `change_sets` and `overview` are kept under Fleet's own names whatever was asked.
fn named(kind: ReviewRunKind, kept: &str, asked: &str) -> bool {
    matches!(kind, ReviewRunKind::ChangeSets | ReviewRunKind::Overview) || kept == asked
}

/// How long since an instant, in its largest unit, as an inbox row says it.
fn age(at: &str) -> String {
    let Ok(at) = chrono::DateTime::parse_from_rfc3339(at) else { return String::new() };
    let minutes = (chrono::Utc::now() - at.with_timezone(&chrono::Utc)).num_minutes().max(0);
    match minutes {
        0 => "just now".into(),
        1..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    }
}

fn verdict_of(said: Option<MyReviewSaid>) -> Verdict {
    match said {
        Some(MyReviewSaid::Approved) => Verdict::Approved,
        Some(MyReviewSaid::ChangesRequested) => Verdict::ChangesRequested,
        Some(MyReviewSaid::Commented) => Verdict::Commented,
        None => Verdict::NotReviewed,
    }
}

/// The five lists as one inbox: a pull request on several lists is one row that names each, marked
/// reviewing where any of them says so. The rows kept, by number. Two repositories' pull requests of
/// one number keep the first.
pub fn inbox_of(lists: &PullReviewLists) -> (Vec<InboxRow>, Vec<(u64, Row)>) {
    let mut rows: Vec<InboxRow> = Vec::new();
    let mut kept: Vec<(u64, Row)> = Vec::new();
    let all = [(PullReviewList::Requested, &lists.requested), (PullReviewList::Team, &lists.team), (PullReviewList::Assigned, &lists.assigned), (PullReviewList::Mine, &lists.mine), (PullReviewList::Reviewing, &lists.reviewing)];
    for (list, on) in all {
        for row in on.iter() {
            if let Some(at) = kept.iter().position(|(number, _)| *number == row.number) {
                if kept[at].1.manifest == row.manifest_id.as_str() {
                    if !rows[at].lists.contains(&list) {
                        rows[at].lists.push(list);
                    }
                    rows[at].reviewing |= row.reviewing;
                }
                continue;
            }
            let changed = row.inbox == PullInbox::ChangedSinceYouLooked;
            rows.push(InboxRow {
                number: row.number,
                repository: row.repository.clone(),
                title: row.title.clone(),
                author: row.author.clone(),
                age: age(&row.updated_at),
                updated_at: row.updated_at.clone(),
                asked_at: row.asked_at.clone(),
                changed,
                new_commits: None,
                lists: vec![list],
                checks: row.checks,
                mine: verdict_of(row.my_review.as_ref().map(|mine| mine.said)),
                reviewing: row.reviewing || list == PullReviewList::Reviewing,
            });
            kept.push((
                row.number,
                Row {
                    manifest: row.manifest_id.as_str().to_string(),
                    repository: row.repository.clone(),
                    title: row.title.clone(),
                    branch: row.branch.clone(),
                    author: row.author.clone(),
                    holder: row.holder.clone(),
                    address: row.address.clone(),
                    draft: row.draft,
                },
            ));
        }
    }
    (rows, kept)
}

/// The review tools that make findings, the first ticked until he picks.
pub fn tools_of(tools: &ReviewTools) -> Vec<model::ReviewTool> {
    tools
        .tools
        .iter()
        .filter(|tool| tool.kind == ReviewToolKind::Tool)
        .enumerate()
        .map(|(at, tool)| {
            // After another word it reads lower case, but for a name that opens on an acronym.
            let mut chars = tool.name.chars();
            let said = match (chars.next(), chars.next()) {
                (Some(first), Some(second)) if !second.is_uppercase() => first.to_lowercase().chain(tool.name.chars().skip(1)).collect(),
                _ => tool.name.clone(),
            };
            model::ReviewTool { id: tool.id.clone(), name: tool.name.clone(), said, first: at == 0 }
        })
        .collect()
}

/// A pull request's page, from its inbox row, Fleet's page, its patches, the change since he looked and his notes.
pub fn page_of(number: u64, row: &Row, detail: &PullRequestPage, patches: &[PullRequestFilePatch], compare: Option<&PullRequestCompare>, notes: &[PullRequestNote]) -> Page {
    let head = detail.head.clone().unwrap_or_default();
    let gone = matches!(compare, Some(PullRequestCompare::BaseGone { .. }));
    let since: HashMap<&str, Option<&str>> = match compare {
        Some(PullRequestCompare::Patches { files, .. }) => files.iter().map(|file| (file.path.as_str(), file.patch.as_deref())).collect(),
        _ => HashMap::new(),
    };
    let viewed: HashMap<&str, bool> = detail.viewed.iter().map(|file| (file.path.as_str(), file.still)).collect();
    let files = detail
        .files
        .iter()
        .map(|file| PageFile {
            path: file.path.clone(),
            status: file.status,
            additions: file.additions,
            deletions: file.deletions,
            viewed: match viewed.get(file.path.as_str()) {
                Some(true) => Viewed::Yes,
                Some(false) => Viewed::ChangedAfter,
                None => Viewed::No,
            },
            since: since.contains_key(file.path.as_str()),
            patch: patches.iter().find(|one| one.path == file.path).and_then(|one| one.patch.clone()).unwrap_or_default(),
            patch_since: since.get(file.path.as_str()).copied().flatten().map(str::to_string),
        })
        .collect();
    let seen = seen_of(detail, gone);
    let new_commits = new_commits_of(detail);
    let mine = match &detail.my_review {
        Some(review) => Mine {
            verdict: verdict_of(Some(review.said)),
            on: review.commit.as_deref().map(short),
            day: review.submitted_at.as_deref().and_then(when).map(|(day, _)| day),
            requested: None,
            commits: detail.since_my_review.and_then(|since| since.commits).unwrap_or(0) as u32,
            replies: detail.since_my_review.and_then(|since| since.replies).unwrap_or(0) as u32,
        },
        None => Mine { verdict: Verdict::NotReviewed, on: None, day: None, requested: None, commits: 0, replies: 0 },
    };
    let author = detail.author.clone().unwrap_or_else(|| row.author.clone());
    let body = detail.body.clone().unwrap_or_default();
    let why = if body.trim().is_empty() { format!("{} wrote no description", model::named(&author)) } else { body.clone() };
    Page {
        number,
        repository: row.repository.clone(),
        title: row.title.clone(),
        author,
        branch: row.branch.clone(),
        base: detail.base.clone(),
        head: short(&head),
        body,
        mine,
        seen,
        new_commits,
        files,
        replies: Vec::new(),
        checks: detail.checks.clone(),
        conversation: detail.conversation.clone(),
        threads: detail.threads.clone(),
        critique: Critique::None,
        summary: Summary::None,
        overview: model::Overview { why, flags: Vec::new(), sets: Vec::new(), read: false },
        holder: row.holder.as_ref().map(|holder| holder.title.clone()),
        notes: notes.iter().filter(|note| note.sent_to.is_none() && note.status == ipc::NoteStatus::Open).map(|note| format!("{}:{} {}", note.path, note.line, note.body)).collect(),
        kept: false,
        ended: None,
        reviewers: detail.reviewers.clone(),
        address: row.address.clone(),
        draft: row.draft,
    }
}

/// What the latest Ask of these words at the page's head answered, or why Fleet refused it. None
/// where it was never asked. Fleet keeps the words lower-cased and single-spaced.
pub fn answer_of(facts: &Facts, question: &str) -> Option<model::Answer> {
    let words = |said: &str| said.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let question = words(question);
    let head = facts.head.as_deref().unwrap_or_default();
    let run = facts.runs.iter().filter(|run| run.kind == ReviewRunKind::Ask && run.head == head && words(&run.name) == question).max_by(|a, b| a.started_at.cmp(&b.started_at));
    match run {
        Some(run) => Some(match (&run.state, &run.result) {
            (ReviewRunState::Queued | ReviewRunState::Running, _) => model::Answer::Writing,
            (ReviewRunState::Done, Some(ReviewResult::Text { body })) => model::Answer::Written(body.clone()),
            (ReviewRunState::Failed, _) => model::Answer::Failed(run.failed.clone().unwrap_or_else(|| "The run ended without a result".into())),
            (ReviewRunState::Done, _) => model::Answer::Failed("The run ended without a result".into()),
        }),
        None => facts.refused.iter().find(|(kind, asked, _)| *kind == ReviewRunKind::Ask && words(asked) == question).map(|(_, _, why)| model::Answer::Failed(why.clone())),
    }
}

fn find_of(run: Option<&ReviewRun>, refused: Option<&str>) -> Find {
    match run {
        Some(run) => match run.state {
            ReviewRunState::Queued | ReviewRunState::Running => Find::Running,
            ReviewRunState::Failed => Find::Failed(run.failed.clone().unwrap_or_else(|| "The run ended without a result".into())),
            ReviewRunState::Done => Find::Done,
        },
        None => refused.map_or(Find::NotRun, |why| Find::Failed(why.to_string())),
    }
}

/// What the runs kept at the page's head come to.
pub fn from_runs(page: &Page, facts: &Facts) -> Mapped {
    let head = facts.head.as_deref().unwrap_or_default();
    let at_head: Vec<&ReviewRun> = facts.runs.iter().filter(|run| run.head == head).collect();
    let latest = |kind: ReviewRunKind, name: &str, since: Option<&str>| -> Option<&ReviewRun> {
        at_head.iter().filter(|run| run.kind == kind && named(kind, &run.name, name) && run.since.as_deref() == since).max_by(|a, b| a.started_at.cmp(&b.started_at)).copied()
    };
    let refused = |kind: ReviewRunKind, name: &str| facts.refused.iter().find(|(was, asked, _)| *was == kind && named(kind, asked, name)).map(|(_, _, why)| why.as_str());

    let summary = match facts.seen.as_deref().and_then(|seen| latest(ReviewRunKind::Explanation, super::WHAT_CHANGED, Some(seen))) {
        Some(run) => match (&run.state, &run.result) {
            (ReviewRunState::Done, Some(ReviewResult::Text { body })) => Summary::Written(body.clone()),
            (ReviewRunState::Queued | ReviewRunState::Running, _) => Summary::Writing,
            _ => Summary::None,
        },
        None => Summary::None,
    };

    let sets_run = latest(ReviewRunKind::ChangeSets, "", None);
    let flags_run = latest(ReviewRunKind::Overview, "", None);
    let mut finds = Finds { sets: find_of(sets_run, refused(ReviewRunKind::ChangeSets, "")), flags: find_of(flags_run, refused(ReviewRunKind::Overview, "")), ..Finds::default() };

    let flags: Vec<Flag> = match flags_run.and_then(|run| run.result.as_ref().map(|result| (run, result))) {
        Some((run, ReviewResult::Overview { findings })) => findings
            .iter()
            .enumerate()
            .map(|(at, finding)| Flag {
                id: format!("overview@{}#{at}", run.started_at),
                kind: match finding.severity {
                    OverviewSeverity::Blocker => FlagKind::Blocker,
                    OverviewSeverity::Risk => FlagKind::Risk,
                    OverviewSeverity::Bug => FlagKind::PossibleBug,
                },
                set: finding.set.clone(),
                title: finding.title.clone(),
                said: finding.reason.clone(),
                path: finding.at.path.clone(),
                line: finding.at.end.max(finding.at.start),
                fix: finding.title.clone(),
                comment: finding.reason.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };

    let mut sets: Vec<ChangeSet> = Vec::new();
    if let Some(ReviewResult::ChangeSets { sets: found }) = sets_run.and_then(|run| run.result.as_ref()) {
        let short_repo = page.repository.rsplit('/').next().unwrap_or(&page.repository).to_string();
        for set in found {
            let practices_run = latest(ReviewRunKind::Practices, &set.id, None);
            finds.practices.insert(set.id.clone(), find_of(practices_run, refused(ReviewRunKind::Practices, &set.id)));
            let (kept_note, practices) = match practices_run.and_then(|run| run.result.as_ref()) {
                Some(ReviewResult::Practices { kind, area, read_at, sources, checks, .. }) => {
                    let area = if area.is_empty() { short_repo.clone() } else { area.clone() };
                    let note = if sources.is_empty() {
                        format!("No doc in {short_repo} states a practice for {} in {area}.", kind_said(*kind))
                    } else {
                        let docs: Vec<&str> = sources.iter().map(|source| source.path.as_str()).collect();
                        let day = when(read_at).map_or_else(String::new, |(day, _)| format!(" {}", model::on_day(&day)));
                        format!("Kept by Armada for every pull request that touches {area} in {short_repo}. Read from {}{day}, and read again when a doc changes.", docs.join(", "))
                    };
                    let practices = checks
                        .iter()
                        .map(|check| Practice {
                            said: check.item.clone(),
                            kept: match check.verdict {
                                PracticeVerdict::Met => Kept::Met,
                                PracticeVerdict::Missed => Kept::Missed,
                                PracticeVerdict::NotApplicable => Kept::DoesNotApply,
                            },
                        })
                        .collect();
                    (note, practices)
                }
                _ => (String::new(), Vec::new()),
            };
            let mut hunks = Vec::new();
            for at in &set.hunks {
                let Some(patch) = facts.patches.iter().find(|file| file.path == at.path).and_then(|file| file.patch.as_deref()).and_then(|patch| cut(patch, at.old_start, at.new_start)) else { continue };
                let reach = at.new_start..at.new_start + at.new_lines.max(1);
                let flag = flags.iter().find(|flag| flag.set == set.id && flag.path == at.path && reach.contains(&flag.line));
                let hot = flag.and_then(|flag| {
                    let drawn = crate::diff::file_diff_of(&at.path, Some(&patch), None);
                    let lines = drawn.files.into_iter().next().map(|file| file.lines).unwrap_or_default();
                    crate::diff::places(&lines).iter().position(|place| *place == Some((flag.line, false)))
                });
                hunks.push(Hunk { path: at.path.clone(), caption: String::new(), patch, hot, flag: flag.map(|flag| flag.id.clone()) });
            }
            let files = set.hunks.iter().map(|hunk| hunk.path.as_str()).collect::<HashSet<_>>().len() as u32;
            sets.push(ChangeSet { id: set.id.clone(), title: set.title.clone(), purpose: set.purpose.clone(), files, kind: kind_said(set.kind).into(), kept_note, practices, hunks });
        }
    }

    // The tools he asked at this head, each its latest run: running while any runs, their findings once all end.
    let mut tools: Vec<&ReviewRun> = Vec::new();
    for run in at_head.iter().filter(|run| run.kind == ReviewRunKind::Tool) {
        match tools.iter().position(|held| held.name == run.name) {
            Some(at) if tools[at].started_at < run.started_at => tools[at] = run,
            Some(_) => {}
            None => tools.push(run),
        }
    }
    let refused_tools: Vec<&(ReviewRunKind, String, String)> = facts.refused.iter().filter(|(kind, name, _)| *kind == ReviewRunKind::Tool && !tools.iter().any(|run| run.name == *name)).collect();
    finds.critique_failed = tools
        .iter()
        .filter(|run| run.state == ReviewRunState::Failed)
        .map(|run| format!("{} failed: {}", run.name, run.failed.clone().unwrap_or_default()))
        .chain(refused_tools.iter().map(|(_, name, why)| format!("{name} failed: {why}")))
        .collect();
    let critique = if tools.is_empty() {
        Critique::None
    } else {
        let asked = model::ReviewAsk { tools: tools.iter().map(|run| run.name.clone()).collect(), effort: model::Effort::default(), focus: String::new() };
        let range = page.range();
        if tools.iter().any(|run| matches!(run.state, ReviewRunState::Queued | ReviewRunState::Running)) {
            Critique::Running { range, asked }
        } else {
            let mut findings = Vec::new();
            for run in &tools {
                if let Some(ReviewResult::Findings { findings: found }) = &run.result {
                    findings.extend(found.iter().enumerate().map(|(at, finding)| model::Finding {
                        id: format!("{}@{}#{at}", run.name, run.started_at),
                        title: finding.title.clone(),
                        path: finding.at.path.clone(),
                        line: finding.at.end.max(finding.at.start),
                        comment: if finding.comment.trim().is_empty() { finding.body.clone() } else { finding.comment.clone() },
                    }));
                }
            }
            Critique::Finished { range, findings, asked }
        }
    };
    let answers = model::Rewrite::ALL.into_iter().filter_map(|rewrite| answer_of(facts, rewrite.question()).map(|answer| (rewrite, answer))).collect();
    Mapped { summary, sets, flags, critique, finds, answers }
}

/// The Overview's No problems found card: the change sets and the overview finished at this head,
/// nothing was found, and no tool's run failed. **A failed run never says nothing was found.**
pub fn clean_overview(finds: &Finds, flags: usize) -> bool {
    finds.sets == Find::Done && finds.flags == Find::Done && finds.critique_failed.is_empty() && flags == 0
}

/// Review comments' No problems found card: the tools finished and found nothing, and no run of the
/// review failed.
pub fn clean_critique(critique: &Critique, finds: &Finds) -> bool {
    matches!(critique, Critique::Finished { findings, .. } if findings.is_empty())
        && finds.critique_failed.is_empty()
        && !matches!(finds.sets, Find::Failed(_))
        && !matches!(finds.flags, Find::Failed(_))
}

/// What kind of change a set is, after "Practices for".
fn kind_said(kind: ChangeSetKind) -> &'static str {
    match kind {
        ChangeSetKind::Migration => "migrations",
        ChangeSetKind::Data => "data changes",
        ChangeSetKind::Logic => "logic",
        ChangeSetKind::Api => "API changes",
        ChangeSetKind::Ui => "UI",
        ChangeSetKind::Tests => "tests",
        ChangeSetKind::Config => "config",
        ChangeSetKind::Other => "other changes",
    }
}

/// A hunk header's old and new starts: `@@ -12,4 +12,6 @@`.
fn hunk_head(line: &str) -> Option<(u64, u64)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(' ')?;
    let start = |range: &str| range.split(',').next().and_then(|start| start.parse::<u64>().ok());
    Some((start(old)?, start(new)?))
}

/// One hunk of a file's patch, its header first, by the starts Fleet names it by.
fn cut(patch: &str, old_start: u64, new_start: u64) -> Option<String> {
    let mut out: Option<Vec<&str>> = None;
    for line in patch.lines() {
        if line.starts_with("@@") {
            if out.is_some() {
                break;
            }
            if hunk_head(line) == Some((old_start, new_start)) {
                out = Some(vec![line]);
            }
            continue;
        }
        if let Some(lines) = out.as_mut() {
            lines.push(line);
        }
    }
    out.map(|lines| lines.join("\n"))
}

/// What the inbox says for a refused read: Fleet's sentence, a rate limit with its time and, on a second line, when it reads again.
pub fn refused_words(error: &ipc::WireError) -> String {
    if error.code != ipc::REVIEW_LISTS_RATE_LIMITED {
        return error.message.clone();
    }
    let until = match error.fields.get("until") {
        Some(ipc::WireValue::Str(until)) => crate::settings::time::stamp_at(until),
        _ => None,
    };
    let title = until.map_or_else(|| error.message.clone(), |until| format!("{} until {until}", error.message));
    format!("{title}\n{RATE_LIMIT_LIFTS}")
}

/// Said under a rate limit's title: nothing for him to do.
pub const RATE_LIMIT_LIFTS: &str = "Reviews will read again by itself when the limit lifts.";

/// The banner over rows kept from an earlier read while the forge is rate-limiting us.
pub fn kept_banner(lists: &PullReviewLists) -> Option<String> {
    let limit = lists.rate_limited.as_ref()?;
    let until = limit.until.as_ref().and_then(|until| crate::settings::time::stamp_at(until.as_str()));
    let from = crate::settings::time::stamp_at(&lists.fetched_at)?;
    // A Fleet that sent no sentence still gets a plain one.
    let said = if limit.said.is_empty() { "The forge is limiting us" } else { limit.said.as_str() };
    Some(match until {
        Some(until) => format!("{said} until {until}. These are from {from}."),
        None => format!("{said}. These are from {from}."),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: &str = "The forge is limiting us";
    const NO_TOKEN: &str = "The forge has no token for this machine. Sign in, then open Reviews again.";

    fn limited(until: Option<&str>) -> ipc::WireError {
        let mut error = ipc::WireError::raised(ipc::REVIEW_LISTS_RATE_LIMITED, LIMIT, ipc::RunId::carried("run-1"));
        if let Some(until) = until {
            error.fields.insert("until".into(), ipc::WireValue::Str(until.into()));
        }
        error
    }

    #[test]
    fn a_rate_limit_with_a_time_says_it_and_when_reading_resumes() {
        let at = "2026-11-17T14:02:00.000Z";
        let time = crate::settings::time::stamp_at(at).unwrap();
        assert_eq!(refused_words(&limited(Some(at))), format!("{LIMIT} until {time}\nReviews will read again by itself when the limit lifts."));
    }

    #[test]
    fn a_rate_limit_with_no_time_says_only_that() {
        assert_eq!(refused_words(&limited(None)), format!("{LIMIT}\nReviews will read again by itself when the limit lifts."));
    }

    #[test]
    fn no_token_and_no_answer_say_fleets_own_sentence() {
        let none = ipc::WireError::raised("fleet.review_lists_unreadable", NO_TOKEN, ipc::RunId::carried("run-1"));
        assert_eq!(refused_words(&none), NO_TOKEN);
    }

    #[test]
    fn kept_rows_under_a_rate_limit_say_both_times() {
        let mut lists = lists(Vec::new(), Vec::new());
        assert_eq!(kept_banner(&lists), None);
        let until = "2026-11-17T14:02:00.000Z";
        lists.fetched_at = "2026-11-17T13:40:00.000Z".into();
        lists.rate_limited = Some(ipc::PullReviewRateLimited { until: Some(ipc::Instant::carried(until)), said: LIMIT.into() });
        let (a, b) = (crate::settings::time::stamp_at(until).unwrap(), crate::settings::time::stamp_at("2026-11-17T13:40:00.000Z").unwrap());
        assert_eq!(kept_banner(&lists), Some(format!("{LIMIT} until {a}. These are from {b}.")));
        lists.rate_limited = Some(ipc::PullReviewRateLimited { until: None, said: LIMIT.into() });
        assert_eq!(kept_banner(&lists), Some(format!("{LIMIT}. These are from {b}.")));
        lists.rate_limited = Some(ipc::PullReviewRateLimited { until: None, said: String::new() });
        assert_eq!(kept_banner(&lists), Some(format!("{LIMIT}. These are from {b}.")));
    }

    #[test]
    fn a_hunk_is_cut_from_its_file_by_its_starts() {
        let patch = "@@ -1,2 +1,3 @@\n a\n+b\n c\n@@ -10,1 +11,2 @@\n x\n+y";
        assert_eq!(cut(patch, 10, 11).as_deref(), Some("@@ -10,1 +11,2 @@\n x\n+y"));
        assert_eq!(cut(patch, 1, 1).as_deref(), Some("@@ -1,2 +1,3 @@\n a\n+b\n c"));
        assert_eq!(cut(patch, 5, 5), None);
    }

    #[test]
    fn a_hunk_header_it_cannot_read_matches_no_hunk() {
        assert_eq!(hunk_head("@@ -12,4 +12,6 @@ fn x()"), Some((12, 12)));
        assert_eq!(hunk_head("@@ -7 +9 @@"), Some((7, 9)));
        assert_eq!(hunk_head("@@ -x,4 +12,6 @@"), None);
        assert_eq!(hunk_head("@@ -12,4 @@"), None);
        assert_eq!(hunk_head("not a header"), None);
        let patch = "@@ -x,1 +1,1 @@\n a\n@@ -3,1 +3,1 @@\n b";
        assert_eq!(cut(patch, 3, 3).as_deref(), Some("@@ -3,1 +3,1 @@\n b"));
        assert_eq!(cut(patch, 1, 1), None);
    }

    #[test]
    fn a_cut_hunk_stops_before_the_next_header() {
        let patch = "@@ -1,1 +1,1 @@\n a\n@@ -5,1 +5,1 @@\n b";
        assert_eq!(cut(patch, 1, 1).as_deref(), Some("@@ -1,1 +1,1 @@\n a"));
    }

    #[test]
    fn his_review_says_what_he_said_and_nothing_where_he_has_none() {
        assert_eq!(verdict_of(Some(MyReviewSaid::Approved)), Verdict::Approved);
        assert_eq!(verdict_of(Some(MyReviewSaid::ChangesRequested)), Verdict::ChangesRequested);
        assert_eq!(verdict_of(Some(MyReviewSaid::Commented)), Verdict::Commented);
        assert_eq!(verdict_of(None), Verdict::NotReviewed);
    }

    #[test]
    fn a_time_that_cannot_be_read_shows_no_age_and_its_own_words_for_a_stamp() {
        assert_eq!(age("yesterday-ish"), "");
        assert_eq!(crate::since_looked::stamp("yesterday-ish"), "yesterday-ish");
        assert_eq!(when("yesterday-ish"), None);
    }

    #[test]
    fn an_age_is_said_in_its_largest_unit() {
        let ago = |minutes: i64| (chrono::Utc::now() - chrono::Duration::minutes(minutes) - chrono::Duration::seconds(5)).to_rfc3339();
        assert_eq!(age(&ago(0)), "just now");
        assert_eq!(age(&ago(5)), "5m");
        assert_eq!(age(&ago(180)), "3h");
        assert_eq!(age(&ago(3 * 1440)), "3d");
    }

    fn pull(number: u64, inbox: PullInbox, said: Option<MyReviewSaid>) -> ipc::PullReviewRow {
        ipc::PullReviewRow {
            manifest_id: ipc::ManifestId::carried("ledger-api"),
            repository: "harbor/ledger-api".into(),
            number,
            title: format!("Pull {number}"),
            branch: "mira/fix".into(),
            address: String::new(),
            draft: false,
            checks: ipc::PullReviewChecks::Passed,
            review: ipc::PullRequestReviewState::None,
            head: "abc1234".into(),
            author: "mira".into(),
            updated_at: "not a time".into(),
            asked_at: None,
            team: None,
            holder: None,
            seen: None,
            my_review: said.map(|said| ipc::PullMyReview { said, commit: None, submitted_at: None }),
            inbox,
            reviewing: false,
        }
    }

    fn lists(requested: Vec<ipc::PullReviewRow>, mine: Vec<ipc::PullReviewRow>) -> PullReviewLists {
        PullReviewLists { me: "nick".into(), fetched_at: String::new(), requested, team: Vec::new(), assigned: Vec::new(), mine, reviewing: Vec::new(), silent: Vec::new(), rate_limited: None }
    }

    #[test]
    fn fleets_changed_state_and_his_review_reach_the_row() {
        let (rows, _) = inbox_of(&lists(
            vec![
                pull(1, PullInbox::NeedsFirstLook, None),
                pull(2, PullInbox::ChangedSinceYouLooked, Some(MyReviewSaid::Commented)),
                pull(3, PullInbox::WaitingOnAuthor, Some(MyReviewSaid::ChangesRequested)),
                pull(4, PullInbox::Done, Some(MyReviewSaid::Approved)),
            ],
            Vec::new(),
        ));
        let says: Vec<(u64, bool, Verdict)> = rows.iter().map(|row| (row.number, row.changed, row.mine)).collect();
        assert_eq!(says, [(1, false, Verdict::NotReviewed), (2, true, Verdict::Commented), (3, false, Verdict::ChangesRequested), (4, false, Verdict::Approved)]);
        // Each is on his requested list, so each lands in the first group.
        assert!(rows.iter().all(|row| row.group() == model::Group::Requested));
    }

    #[test]
    fn a_pull_request_on_two_lists_is_one_row_that_names_both() {
        let (rows, kept) = inbox_of(&lists(vec![pull(7, PullInbox::NeedsFirstLook, None)], vec![pull(7, PullInbox::NeedsFirstLook, None)]));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lists, [PullReviewList::Requested, PullReviewList::Mine]);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn the_fifth_list_is_read_and_reviewing_on_any_list_marks_the_row() {
        let ran = |number: u64| ipc::PullReviewRow { reviewing: true, ..pull(number, PullInbox::NeedsFirstLook, None) };
        let mut read = lists(vec![pull(7, PullInbox::NeedsFirstLook, None)], vec![ran(7), ran(8)]);
        read.reviewing = vec![ran(9)];
        let (rows, kept) = inbox_of(&read);
        let says: Vec<(u64, &[PullReviewList], bool)> = rows.iter().map(|row| (row.number, row.lists.as_slice(), row.reviewing)).collect();
        assert_eq!(
            says,
            [(7, &[PullReviewList::Requested, PullReviewList::Mine][..], true), (8, &[PullReviewList::Mine][..], true), (9, &[PullReviewList::Reviewing][..], true)]
        );
        assert_eq!(kept.len(), 3, "a pull request on the fifth list alone is kept, so its page reads");
        // Still asked of him, #7 stays under Review requested; his own #8 and the fifth list's #9 are Reviewed with Armada.
        let groups: Vec<model::Group> = rows.iter().map(|row| row.group()).collect();
        assert_eq!(groups, [model::Group::Requested, model::Group::Reviewing, model::Group::Reviewing]);
    }

    fn page_with(viewed: Vec<ipc::PullViewedFile>) -> Page {
        let detail = PullRequestPage {
            head: Some("abcdef1234567".into()),
            files: ["a.rs", "b.rs", "c.rs"].map(|path| ipc::PullRequestFile { path: path.into(), additions: 1, deletions: 0, status: ipc::PullRequestFileStatus::Modified }).to_vec(),
            viewed,
            ..PullRequestPage::default()
        };
        page_of(1, &row(), &detail, &[], None, &[])
    }

    fn row() -> Row {
        Row {
            manifest: "ledger-api".into(),
            repository: "harbor/ledger-api".into(),
            title: "t".into(),
            branch: "b".into(),
            author: "mira".into(),
            holder: None,
            address: "https://example.com/harbor/ledger-api/pull/1".into(),
            draft: true,
        }
    }

    #[test]
    fn a_page_carries_its_reviewers_its_address_and_whether_it_is_a_draft() {
        let tobias = ipc::PullRequestReviewer { login: "tobias".into(), kind: ipc::PullRequestReviewerKind::Person, verdict: None, name: None, url: None, team: None };
        let detail = PullRequestPage { reviewers: vec![tobias.clone()], ..PullRequestPage::default() };
        let page = page_of(1, &row(), &detail, &[], None, &[]);
        assert_eq!((page.reviewers, page.address.as_str(), page.draft), (vec![tobias], "https://example.com/harbor/ledger-api/pull/1", true));
    }

    fn ask(name: &str, head: &str, at: &str, state: ReviewRunState, body: Option<&str>) -> ReviewRun {
        ReviewRun {
            manifest_id: ipc::ManifestId::carried("ledger-api"),
            number: 1,
            kind: ReviewRunKind::Ask,
            name: name.into(),
            head: head.into(),
            since: None,
            state,
            started_at: at.into(),
            finished_at: None,
            failed: (state == ReviewRunState::Failed).then(|| "The bay stopped".into()),
            result: body.map(|body| ReviewResult::Text { body: body.into() }),
            cost_micros: None,
        }
    }

    #[test]
    fn an_ask_answers_its_latest_run_at_the_head_whatever_the_case_of_its_words() {
        let question = "Summarize this pull request's description";
        let kept = "summarize this pull request's description";
        let facts = |runs: Vec<ReviewRun>, refused: Vec<(ReviewRunKind, String, String)>| Facts { head: Some("h2".into()), runs, refused, ..Facts::default() };
        let older = ask(kept, "h1", "2026-11-17T09:00:00Z", ReviewRunState::Done, Some("Old head"));
        let first = ask(kept, "h2", "2026-11-17T10:00:00Z", ReviewRunState::Done, Some("First"));
        let again = ask(kept, "h2", "2026-11-17T11:00:00Z", ReviewRunState::Running, None);
        assert_eq!(answer_of(&facts(vec![older.clone()], Vec::new()), question), None, "a run at another head is not this commit's");
        assert_eq!(answer_of(&facts(vec![older, first.clone()], Vec::new()), question), Some(model::Answer::Written("First".into())));
        assert_eq!(answer_of(&facts(vec![first, again], Vec::new()), question), Some(model::Answer::Writing));
        let failed = ask(kept, "h2", "2026-11-17T10:00:00Z", ReviewRunState::Failed, None);
        assert_eq!(answer_of(&facts(vec![failed], Vec::new()), question), Some(model::Answer::Failed("The bay stopped".into())));
        let refused = vec![(ReviewRunKind::Ask, question.to_string(), "Fleet is busy".to_string())];
        assert_eq!(answer_of(&facts(Vec::new(), refused), question), Some(model::Answer::Failed("Fleet is busy".into())));
    }

    #[test]
    fn no_problems_found_only_after_every_run_finished_and_none_failed() {
        let run = |kind: ReviewRunKind, name: &str, state: ReviewRunState, result: Option<ReviewResult>| ReviewRun { kind, result, ..ask(name, "h2", "2026-11-17T10:00:00Z", state, None) };
        let sets = |state| run(ReviewRunKind::ChangeSets, "change_sets", state, (state == ReviewRunState::Done).then(|| ReviewResult::ChangeSets { sets: Vec::new() }));
        let overview = |state| run(ReviewRunKind::Overview, "overview", state, (state == ReviewRunState::Done).then(|| ReviewResult::Overview { findings: Vec::new() }));
        let tool = |state| run(ReviewRunKind::Tool, "bugs", state, (state == ReviewRunState::Done).then(|| ReviewResult::Findings { findings: Vec::new() }));
        let page = page_with(Vec::new());
        let clean = |runs: Vec<ReviewRun>| {
            let mapped = from_runs(&page, &Facts { head: Some("h2".into()), runs, ..Facts::default() });
            (clean_overview(&mapped.finds, mapped.flags.len()), clean_critique(&mapped.critique, &mapped.finds))
        };
        use ReviewRunState::{Done, Failed, Running};
        assert_eq!(clean(vec![sets(Done), overview(Done), tool(Done)]), (true, true), "finished with nothing found");
        assert_eq!(clean(vec![sets(Done), overview(Done), tool(Failed)]), (false, false), "a failed tool is not a clean review");
        assert_eq!(clean(vec![sets(Done), overview(Failed), tool(Done)]), (false, false), "a failed overview is not a clean review either");
        assert_eq!(clean(vec![sets(Failed)]), (false, false));
        assert_eq!(clean(vec![sets(Done), overview(Running), tool(Running)]), (false, false), "not before the runs end");
        assert_eq!(clean(Vec::new()), (false, false), "nor before any ran");
    }

    #[test]
    fn a_file_viewed_at_an_older_head_is_unviewed_once_its_blob_changes() {
        let viewed = |path: &str, still: bool| ipc::PullViewedFile { path: path.into(), commit: "old1234".into(), viewed_at: "2026-11-01T10:00:00Z".into(), still };
        let page = page_with(vec![viewed("a.rs", true), viewed("b.rs", false)]);
        let says: Vec<(&str, Viewed)> = page.files.iter().map(|file| (file.path.as_str(), file.viewed)).collect();
        // Fleet's `still: false` clears the box; the file he never viewed is plain not viewed.
        assert_eq!(says, [("a.rs", Viewed::Yes), ("b.rs", Viewed::ChangedAfter), ("c.rs", Viewed::No)]);
    }
}
