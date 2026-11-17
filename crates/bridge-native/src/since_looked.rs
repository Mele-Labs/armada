//! What a person last left a pull request at, and what came after, said the way both screens that
//! read "since you looked" say it: Reviews for a reviewer, and the Job's pull request panel for its
//! author. Fleet keeps the head (`PullRequestPage::seen`); nothing here asks it.

use ipc::{PullRequestCommit, PullRequestPage};

use crate::reviews::model::Seen;

pub fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// The day and time of an instant, as a person says them: `today`, `yesterday`, `Tuesday`, `4 Nov`.
pub fn when(at: &str) -> Option<(String, String)> {
    let at = chrono::DateTime::parse_from_rfc3339(at)
        .ok()?
        .with_timezone(&chrono::Local);
    let days = (chrono::Local::now().date_naive() - at.date_naive()).num_days();
    let day = match days {
        ..=0 => "today".to_string(),
        1 => "yesterday".to_string(),
        2..=6 => at.format("%A").to_string(),
        _ => at.format("%-d %b").to_string(),
    };
    Some((day, at.format("%H:%M").to_string()))
}

/// A commit's time as the What changed list says it: `Fri 08:58`, or `4 Nov` past a week.
pub fn stamp(at: &str) -> String {
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(at) else {
        return at.to_string();
    };
    let at = parsed.with_timezone(&chrono::Local);
    if (chrono::Local::now().date_naive() - at.date_naive()).num_days() < 7 {
        at.format("%a %H:%M").to_string()
    } else {
        at.format("%-d %b").to_string()
    }
}

/// Where he last looked: the day, the commits `old → new`, and whether `old` is gone after a force push.
pub fn seen_of(detail: &PullRequestPage, gone: bool) -> Option<Seen> {
    let head = detail.head.clone().unwrap_or_default();
    detail.seen.as_ref().map(|seen| {
        let (day, time) = when(&seen.at).unwrap_or_default();
        Seen {
            day,
            time,
            old: short(&seen.head),
            new: short(&head),
            gone,
        }
    })
}

/// The commits after the one he left it at, newest first; all of them where his is no longer there.
pub fn new_commits_of(detail: &PullRequestPage) -> Vec<PullRequestCommit> {
    let head = detail.head.clone().unwrap_or_default();
    match &detail.seen {
        Some(seen) if seen.head != head => {
            let after = detail
                .commits
                .iter()
                .position(|commit| !commit.sha.is_empty() && seen.head.starts_with(&commit.sha))
                .map_or(0, |at| at + 1);
            detail.commits[after..]
                .iter()
                .rev()
                .cloned()
                .map(|commit| PullRequestCommit {
                    at: commit.at.as_deref().map(stamp),
                    ..commit
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(sha: &str, by: &str) -> PullRequestCommit {
        PullRequestCommit {
            sha: sha.into(),
            headline: format!("Commit {sha}"),
            author: Some(by.into()),
            at: None,
        }
    }

    fn page(seen: Option<&str>, head: &str, commits: Vec<PullRequestCommit>) -> PullRequestPage {
        PullRequestPage {
            head: Some(head.into()),
            commits,
            seen: seen.map(|head| ipc::PullSeen {
                head: head.into(),
                at: "2026-11-08T10:30:00Z".into(),
            }),
            ..PullRequestPage::default()
        }
    }

    #[test]
    fn the_commits_after_his_seen_head_come_newest_first() {
        let page = page(
            Some("bbbbbbb1111111111111111111111111111111"),
            "dddddddffffffffffffffffffffffffffffffff",
            vec![
                commit("aaaaaaa", "mira"),
                commit("bbbbbbb", "mira"),
                commit("ccccccc", "devon"),
                commit("ddddddd", "mira"),
            ],
        );
        let shas: Vec<String> = new_commits_of(&page)
            .into_iter()
            .map(|one| one.sha)
            .collect();
        assert_eq!(shas, ["ddddddd", "ccccccc"]);
    }

    #[test]
    fn every_commit_is_new_where_his_seen_head_is_not_among_them() {
        let page = page(
            Some("9999999000000000000000000000000000000000"),
            "dddddddffffffffffffffffffffffffffffffff",
            vec![commit("aaaaaaa", "mira"), commit("ddddddd", "mira")],
        );
        assert_eq!(new_commits_of(&page).len(), 2);
    }

    #[test]
    fn nothing_is_new_where_he_never_looked_or_the_head_has_not_moved() {
        assert!(new_commits_of(&page(None, "ddddddd", vec![commit("ddddddd", "mira")])).is_empty());
        assert!(new_commits_of(&page(
            Some("ddddddd"),
            "ddddddd",
            vec![commit("ddddddd", "mira")]
        ))
        .is_empty());
        assert!(seen_of(&page(None, "ddddddd", vec![]), false).is_none());
    }

    #[test]
    fn his_seen_head_and_the_head_are_said_short() {
        let seen = seen_of(
            &page(
                Some("bbbbbbb1111111111111111111111111111111"),
                "dddddddffffffffffffffffffffffffffffffff",
                vec![],
            ),
            true,
        )
        .expect("he looked");
        assert_eq!(
            (seen.old.as_str(), seen.new.as_str(), seen.gone),
            ("bbbbbbb", "ddddddd", true)
        );
    }
}
