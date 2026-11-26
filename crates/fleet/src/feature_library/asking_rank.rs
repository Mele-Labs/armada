//! Rank with the librarian: both might-lists put in the librarian's order, each row with a short why, in one model call.
//! `docs/concepts/feature-catalog.md`, *Ranking the issues he might pick up*.
//!
//! **Held in Fleet's memory for each project, never written down.** `list_ranked_issues` puts the rows the ranking
//! names first; Reset, or Undo of the ask that ranked, lets go of it.

use ipc::{Instant, RankedIssue, RankedIssues, RankedPull};

use super::asking_reply::one_line;

/// The longest why kept, in characters.
const MOST_WHY: usize = 80;
/// The most rows of each list the prompt names.
const MOST_ROWS: usize = 40;

/// One project's ranking: each row it named, in its order, with its why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ranking {
    pub(crate) at: Instant,
    /// The repository it was listed from, absent for the project's own tracker, and its reference.
    issues: Vec<(Option<String>, String, String)>,
    /// The repository and the number.
    pulls: Vec<(String, u64, String)>,
}

/// The key the prompt gives the issue at `at` in the list, and the pull request.
fn issue_key(at: usize) -> String {
    format!("I{}", at + 1)
}

fn pull_key(at: usize) -> String {
    format!("P{}", at + 1)
}

fn features_of(features: &[ipc::MatchedFeature]) -> String {
    if features.is_empty() {
        return String::new();
    }
    format!(" [features: {}]", features.iter().map(|one| one.title.as_str()).collect::<Vec<_>>().join(", "))
}

/// What the model is asked: the rules, both lists by key, and the features his recent Jobs worked on with their counts.
pub(super) fn question(lists: &RankedIssues, worked: &[(String, u32)]) -> String {
    let mut text = String::from(
        "Rank the issues a software project's owner might pick up and the pull requests of other people he might be \
         interested in, the most worth his time first. Weigh the features he worked on lately.\n\n\
         Answer one line a row, best first, and nothing else:\n\
         <key> <why: at most 80 characters, facts only, no dashes>\n\
         Rank issues and pull requests together. Leave out a row not worth his time.\n",
    );
    if !worked.is_empty() {
        text.push_str(&format!("\nFeatures his Jobs worked on in the last {} days:\n", lists.days));
        for (title, jobs) in worked {
            text.push_str(&format!("{title} ({jobs} Jobs)\n"));
        }
    }
    if !lists.might_pick_up.is_empty() {
        text.push_str("\nIssues:\n");
        for (at, row) in lists.might_pick_up.iter().enumerate().take(MOST_ROWS) {
            text.push_str(&format!("{} {} {}{}\n", issue_key(at), row.issue.reference, one_line(&row.issue.title, 120), features_of(&row.features)));
        }
    }
    if !lists.might_be_interested.is_empty() {
        text.push_str("\nPull requests:\n");
        for (at, row) in lists.might_be_interested.iter().enumerate().take(MOST_ROWS) {
            text.push_str(&format!("{} #{} {} by {}{}\n", pull_key(at), row.number, one_line(&row.title, 120), row.author, features_of(&row.features)));
        }
    }
    text
}

/// The model's lines as a ranking of `lists`, or `None` where no line named a row.
pub(super) fn read(reply: &str, lists: &RankedIssues, at: Instant) -> Option<Ranking> {
    let mut issues = Vec::new();
    let mut pulls = Vec::new();
    for raw in reply.lines() {
        let raw = raw.trim().trim_start_matches(['-', '*', '#']).trim();
        let (key, why) = raw.split_once(char::is_whitespace).map_or((raw, ""), |(key, why)| (key, why.trim()));
        let key = key.trim_end_matches([':', '.', ')']).to_ascii_uppercase();
        let number = |prefix: char| key.strip_prefix(prefix).and_then(|n| n.parse::<usize>().ok()).and_then(|n| n.checked_sub(1)).filter(|n| *n < MOST_ROWS);
        let why = one_line(why.trim_start_matches([':', '-']).trim(), MOST_WHY);
        if let Some(row) = number('I').and_then(|n| lists.might_pick_up.get(n)) {
            let named = (row.manifest_id.as_ref().map(|one| one.as_str().to_string()), row.issue.reference.clone());
            if !issues.iter().any(|(manifest, reference, _)| (manifest, reference) == (&named.0, &named.1)) {
                issues.push((named.0, named.1, why));
            }
        } else if let Some(row) = number('P').and_then(|n| lists.might_be_interested.get(n)) {
            let named = (row.manifest_id.as_str().to_string(), row.number);
            if !pulls.iter().any(|(manifest, number, _)| (manifest, number) == (&named.0, &named.1)) {
                pulls.push((named.0, named.1, why));
            }
        }
    }
    (!issues.is_empty() || !pulls.is_empty()).then_some(Ranking { at, issues, pulls })
}

/// `issues` and `pulls` in `ranking`'s order: the rows it named first with their why, the rest after as they were.
pub(crate) fn apply(ranking: &Ranking, issues: &mut Vec<RankedIssue>, pulls: &mut Vec<RankedPull>) {
    let mut rest = std::mem::take(issues);
    for (manifest, reference, why) in &ranking.issues {
        if let Some(at) = rest.iter().position(|row| row.manifest_id.as_ref().map(|one| one.as_str()) == manifest.as_deref() && row.issue.reference == *reference) {
            let mut row = rest.remove(at);
            row.why = Some(why.clone()).filter(|why| !why.is_empty());
            issues.push(row);
        }
    }
    issues.extend(rest);
    let mut rest = std::mem::take(pulls);
    for (manifest, number, why) in &ranking.pulls {
        if let Some(at) = rest.iter().position(|row| row.manifest_id.as_str() == manifest && row.number == *number) {
            let mut row = rest.remove(at);
            row.why = Some(why.clone()).filter(|why| !why.is_empty());
            pulls.push(row);
        }
    }
    pulls.extend(rest);
}
