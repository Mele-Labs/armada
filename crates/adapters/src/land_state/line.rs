//! The merge line as Fleet reads it: the queue in place order with each
//! branch's outcome, the newest few that landed, and those sent back lately.
//!
//! **Read-only, and it never starts a runner.** `armada land --status` takes up
//! a turn a killed runner left, which is right for an agent standing in the
//! clone and wrong for a daemon reading every repository it serves.
//! Nothing here creates the state directory either: [`StateDir::existing`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use super::codec::{self, ReadStateError};
use super::dir::{git_common_dir, StateDir};
use super::outcome::{read_outcome, Outcome, OutcomeState};
use super::queue::{queued, QueueEntry, QueuedError};
use crate::reading_in::FORGE_HOST;

/// How many branches that left the line are read back, newest first, as one
/// list: what a Bridge before protocol 23.1 draws.
///
/// **A bound, because the outcomes are never pruned**: every branch ever landed
/// keeps one, and this clone held 310 on 2 Oct 2026. Three is what the panel
/// was drawn and approved with.
pub const OFF: usize = 3;

/// How many landed branches are read back, newest first: *Recently landed*.
pub const LANDED: usize = 3;

/// How long a red, a conflict or a stop is read back for: *Sent back*.
///
/// **A bound by age rather than by count**, because a branch sent back is work
/// somebody still owes, and three newer reds should not hide a fourth. Three
/// days carries one over a weekend. The age is the outcome file's own write,
/// so a branch sent back again starts again.
pub const SENT_BACK_FOR: Duration = Duration::from_secs(3 * 24 * 60 * 60);

/// Where one repository's line is, resolved once: two `git` calls a
/// repository, not two a read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    common: PathBuf,
    /// `https://<forge>/<owner>/<repo>/pull/`, where `origin` is on the forge.
    pulls: Option<String>,
}

impl Located {
    /// A pull request's address, where `origin` names one on the forge.
    pub fn pull_request(&self, number: u64) -> Option<String> {
        self.pulls.as_ref().map(|pulls| format!("{pulls}{number}"))
    }
}

/// One repository's line. `None` from [`read`] is no line at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// In place order, each with its outcome where the runner has said one.
    pub waiting: Vec<(QueueEntry, Option<Outcome>)>,
    /// Ended outcomes of branches no longer in line, newest first, at most [`OFF`].
    pub off: Vec<Outcome>,
    /// Landed and no longer in line, newest first, at most [`LANDED`].
    pub landed: Vec<Outcome>,
    /// Red, conflict or stopped, no longer in line, written within
    /// [`SENT_BACK_FOR`], newest first.
    ///
    /// **A branch that has since landed is not here without a rule saying so**:
    /// a branch has one outcome file, and its landing overwrote the red.
    pub sent_back: Vec<Outcome>,
}

/// `repo`'s common git directory and forge address. `None` where `git` cannot
/// answer, which is a repository with no line to read.
pub fn locate(repo: &Path) -> Option<Located> {
    let common = git_common_dir(repo).ok()?;
    let origin = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string());
    Some(Located {
        common,
        pulls: origin.as_deref().and_then(pulls_of),
    })
}

/// Read the line as of `now`. `Ok(None)` where `armada land` has never run in
/// this clone.
pub fn read(at: &Located, now: SystemTime) -> Result<Option<Line>, ReadLineError> {
    let Some(state) = StateDir::existing(&at.common) else {
        return Ok(None);
    };
    let entries = if state.queue_dir().is_dir() {
        queued(&state).map_err(ReadLineError::Queue)?
    } else {
        Vec::new()
    };
    let mut waiting = Vec::with_capacity(entries.len());
    for entry in entries {
        let outcome = read_outcome(&state, &entry.branch).map_err(ReadLineError::Outcome)?;
        waiting.push((entry, outcome));
    }
    let since = now
        .checked_sub(SENT_BACK_FOR)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let left = left(&state, &waiting, since)?;
    Ok(Some(Line {
        waiting,
        off: left.off,
        landed: left.landed,
        sent_back: left.sent_back,
    }))
}

/// The outcomes of branches no longer in line, three ways.
#[derive(Default)]
struct Left {
    off: Vec<Outcome>,
    landed: Vec<Outcome>,
    sent_back: Vec<Outcome>,
}

/// The ended outcomes of branches not in line, newest first. Newest by the
/// file's own write, which every outcome makes whole through a rename.
///
/// **Decoded only until every list is full**: past `since`, with [`OFF`] and
/// [`LANDED`] read, the rest of the folder is never opened.
fn left(
    state: &StateDir,
    waiting: &[(QueueEntry, Option<Outcome>)],
    since: SystemTime,
) -> Result<Left, ReadLineError> {
    let folder = state.outcomes_dir();
    let listing = match std::fs::read_dir(&folder) {
        Ok(listing) => listing,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(Left::default()),
        Err(cause) => {
            return Err(ReadLineError::Outcomes {
                path: folder,
                cause,
            })
        }
    };
    let mut files: Vec<(SystemTime, PathBuf)> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter_map(|path| Some((path.metadata().ok()?.modified().ok()?, path)))
        .collect();
    files.sort_by_key(|file| std::cmp::Reverse(file.0));

    let mut left = Left::default();
    for (written, path) in files {
        let past = written < since;
        if past && left.off.len() == OFF && left.landed.len() == LANDED {
            break;
        }
        // One file that will not decode does not hide the rest: `queued`'s rule.
        let Ok(Some(outcome)) = codec::read::<Outcome>("outcome", &path) else {
            continue;
        };
        let in_line = waiting
            .iter()
            .any(|(entry, _)| entry.branch == outcome.branch);
        let landed = outcome.state == OutcomeState::Landed;
        let sent_back = matches!(
            outcome.state,
            OutcomeState::Red | OutcomeState::Conflict | OutcomeState::Stopped
        );
        if in_line || !(landed || sent_back) {
            continue;
        }
        if left.off.len() < OFF {
            left.off.push(outcome.clone());
        }
        if landed && left.landed.len() < LANDED {
            left.landed.push(outcome);
        } else if sent_back && !past {
            left.sent_back.push(outcome);
        }
    }
    Ok(left)
}

/// `https://<forge>/<owner>/<repo>/pull/` from an `origin` on the forge,
/// spelled any of the three ways `git` takes one.
fn pulls_of(origin: &str) -> Option<String> {
    let scp = format!("{}:", FORGE_HOST.trim_end_matches('/'));
    let path = match origin.split_once(&scp) {
        Some((_, path)) if !origin.contains("://") => path,
        _ => origin.split_once(FORGE_HOST)?.1,
    };
    let path = path.trim_end_matches('/').trim_end_matches(".git");
    let (owner, repo) = path.split_once('/')?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some(format!("https://{FORGE_HOST}{owner}/{repo}/pull/"))
}

/// Why a line that exists could not be read.
#[derive(Debug)]
pub enum ReadLineError {
    Queue(QueuedError),
    Outcome(ReadStateError),
    Outcomes {
        path: PathBuf,
        cause: std::io::Error,
    },
}

impl std::fmt::Display for ReadLineError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadLineError::Queue(why) => write!(out, "{why}"),
            ReadLineError::Outcome(why) => write!(out, "{why}"),
            ReadLineError::Outcomes { path, .. } => {
                write!(out, "{} could not be listed", path.display())
            }
        }
    }
}

impl std::error::Error for ReadLineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadLineError::Queue(why) => Some(why),
            ReadLineError::Outcome(why) => Some(why),
            ReadLineError::Outcomes { cause, .. } => Some(cause),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pulls_of;

    #[test]
    fn an_origin_on_the_forge_names_its_pull_requests_whichever_way_it_is_spelled() {
        let want = Some(format!("https://{}o/r/pull/", super::FORGE_HOST));
        let host = super::FORGE_HOST.trim_end_matches('/');
        assert_eq!(pulls_of(&format!("git@{host}:o/r.git")), want);
        assert_eq!(pulls_of(&format!("https://{host}/o/r.git")), want);
        assert_eq!(pulls_of(&format!("https://{host}/o/r")), want);
        assert_eq!(pulls_of(&format!("ssh://git@{host}/o/r.git")), want);
    }

    #[test]
    fn an_origin_elsewhere_names_none() {
        assert_eq!(pulls_of("https://git.example/o/r.git"), None);
        assert_eq!(pulls_of("/tmp/elsewhere"), None);
    }
}
