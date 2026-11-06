//! Every comment on a pull request, filed in the scratch folder of the Job that holds it, and one
//! line to that Job when a human reviewer said something new. The agent reads the file only if it
//! decides to. The decision record `2026-11-03-pull-request-comments-reach-the-job.md`.
//!
//! **A comment is told once, and again only once it is edited**: the store keeps each comment's
//! edit time as told (`pull_comments_told`). A bot's comment and the owner's own are filed and
//! kept without a word, so they never speak later. A Job with nothing kept for a pull request is
//! taking it on, and hears one line for all that is there.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, FollowedPull, PullComment, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::my_pulls::Mine;
use crate::pull_owners::Owner;
use crate::repositories::Served;

/// The file's name in the scratch folder.
pub fn file_of(number: u64) -> String {
    format!("comments-{number}.md")
}

/// The whole file, oldest first. **One heading a comment, its fields always in this order**, so
/// `grep -A` on an id reads that comment: then its link, then its body quoted, so a `##` it wrote
/// is never read as a heading.
pub fn filed(number: u64, comments: &[&PullComment]) -> String {
    let mut out = format!(
        "# Comments on #{number}, oldest first\n\
         Each is `## id · author · bot · path:line · resolved · written · edited`, the fields it has, \
         then its link, then its body.\n"
    );
    for comment in comments {
        let mut heading = vec![comment.id.clone(), comment.author.clone()];
        if comment.bot {
            heading.push("bot".to_string());
        }
        if let Some(file) = &comment.file {
            heading.push(match file.line {
                Some(line) => format!("{}:{line}", file.path),
                None => file.path.clone(),
            });
            if file.resolved {
                heading.push("resolved".to_string());
            }
        }
        heading.push(format!("written {}", comment.at));
        if let Some(edited) = &comment.edited_at {
            heading.push(format!("edited {edited}"));
        }
        out.push_str(&format!("\n## {}\n{}\n", heading.join(" · "), comment.url));
        for line in comment.body.lines() {
            out.push_str(&format!("> {line}\n"));
        }
    }
    out
}

/// "a", "a and b", "a, b and c".
fn named(logins: &[&str]) -> String {
    match logins {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Each login once, in the order it first wrote.
fn authors<'a>(comments: &[&'a PullComment]) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for comment in comments {
        if !out.contains(&comment.author.as_str()) {
            out.push(&comment.author);
        }
    }
    out
}

fn counted(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => one.to_string(),
        _ => format!("{count} {many}"),
    }
}

/// What a reviewer said since the last notice, in one line: `new` they wrote, `edited` they changed.
fn heard_line(number: u64, new: &[&PullComment], edited: &[&PullComment], file: &Path) -> String {
    let mut parts = Vec::new();
    if !new.is_empty() {
        let what = counted(new.len(), "1 new comment", "new comments");
        parts.push(format!("{what} from {}", named(&authors(new))));
    }
    if !edited.is_empty() {
        let what = counted(edited.len(), "a comment", "comments");
        parts.push(format!("{} edited {what}", named(&authors(edited))));
    }
    format!("#{number}: {}, in {}", parts.join("; "), file.display())
}

/// A pull request a Job takes on with `count` comments already on it.
fn taken_on_line(number: u64, count: usize, file: &Path) -> String {
    let what = counted(count, "1 comment", "comments");
    format!("#{number} already has {what}, in {}", file.display())
}

impl Fleet
{
    /// One pull request of the person's own, its comments filed for each Job that holds it.
    /// **Asked of the forge only where a Job holds it**, one call; a forge that will not answer
    /// leaves the file and the store as they were.
    pub async fn comments_told(self: &Arc<Self>, served: &Served, pull: &FollowedPull, mine: &Mine) {
        let owners = self.owners_of(served, pull.number, pull.branch.as_written(), false).await;
        if owners.is_empty() {
            return;
        }
        let (root, number) = (served.root().to_string(), pull.number);
        let Some(comments) = self
            .forge_asked(&root, move |vcs: &dyn adapter_traits::DynVcs, root: &str| vcs.pull_request_comments(root, number))
            .await
        else {
            return;
        };
        let mut ordered: Vec<&PullComment> = comments.iter().collect();
        ordered.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| a.id.cmp(&b.id)));
        let text = filed(number, &ordered);
        // The owner's own is the pull request's author's, and any login that is the person's.
        let author = pull.author.as_ref().map(|it| it.as_written());
        let owners_own = |login: &str| {
            author.is_some_and(|it| it.eq_ignore_ascii_case(login))
                || mine.logins().iter().any(|it| it.eq_ignore_ascii_case(login))
        };
        let reviewer = |comment: &&PullComment| !comment.bot && !owners_own(&comment.author);
        for owner in owners {
            let Owner::Session(id) = &owner;
            let file = self.scratch_of(id).join(file_of(number));
            let unchanged = std::fs::read_to_string(&file).ok().as_deref() == Some(text.as_str());
            if (!ordered.is_empty() || file.exists()) && !unchanged {
                let _ = std::fs::write(&file, &text);
            }
            let recipient = owner.key();
            let Ok(kept) = self.store().lock().await.pull_comments_told(&root, number, &recipient) else {
                continue;
            };
            let (new, edited): (Vec<&PullComment>, Vec<&PullComment>) = ordered
                .iter()
                .copied()
                .filter(|comment| kept.get(&comment.id) != Some(&comment.edited_at))
                .partition(|comment| !kept.contains_key(&comment.id));
            // Nothing said yet: kept as read, so the first comment after is new and not a take-on.
            let mut told: Vec<(&str, Option<&str>)> =
                new.iter().chain(&edited).map(|it| (it.id.as_str(), it.edited_at.as_deref())).collect();
            if kept.is_empty() && ordered.is_empty() {
                told.push(("", None));
            }
            if told.is_empty() {
                continue;
            }
            let heard_new: Vec<&PullComment> = new.iter().copied().filter(&reviewer).collect();
            let heard_edited: Vec<&PullComment> = edited.iter().copied().filter(&reviewer).collect();
            let delivered = if heard_new.is_empty() && heard_edited.is_empty() {
                true
            } else {
                let line = match kept.is_empty() {
                    true => taken_on_line(number, ordered.len(), &file),
                    false => heard_line(number, &heard_new, &heard_edited, &file),
                };
                matches!(self.told_by_fleet(id, &line, ipc::Notice::PrComments).await, Ok(true))
            };
            if delivered {
                let now = self.now();
                let _ = self.store().lock().await.keep_pull_comments_told(&root, number, &recipient, &told, &now);
            }
        }
    }
}
