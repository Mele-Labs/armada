//! `armada need`: a branch says what it needs on a path, and is told who is
//! ahead of it there. The decision is `decisions/2026-10-02-a-plan-leases-its-numbers.md`;
//! this is the half for agents working outside Fleet.
//!
//! **A need is a path and what is needed there, in the declarer's words**, and
//! the repository declares no kinds up front: the file is the resource. First
//! to declare goes first. `armada land` holds a branch until every need ahead
//! of it on the same path is spent or given back ([`crate::land::hold`]).
//!
//! **State is one file per need under the git common directory**, beside
//! `armada-land/`, so every worktree of one clone sees the same line. The files
//! and their order are `adapters::needs`, which Fleet reads and writes for its
//! Jobs too, so a session and a Job see one order. A need is spent when its
//! branch lands, and given back when its branch no longer exists locally or a
//! person runs `--release`. **Nothing expires by time**: whether a stalled need
//! should is open, so a person gives a stalled one back.

use std::path::Path;

pub use adapters::needs::{waiting_text, Need, Needs};

use crate::land::git::checked;
use crate::land::StateDir;

/// What declaring came to.
pub struct Declared {
    pub mine: Need,
    pub already: bool,
    pub ahead: Vec<Need>,
    /// Set where the branch already changes the path while others are ahead on
    /// it: whatever number it took there may have to move.
    pub late: bool,
}

/// Record `branch`'s need on `path`, and say whether the checkout at `cwd`
/// already changes it. The same branch and path again records nothing.
pub fn declare(
    needs: &Needs,
    cwd: &Path,
    branch: &str,
    path: &str,
    what: &str,
) -> Result<Declared, String> {
    let done = needs.declare(branch, path, what)?;
    let late = !done.ahead.is_empty() && changes(cwd, &done.mine.path);
    Ok(Declared {
        mine: done.mine,
        already: done.already,
        ahead: done.ahead,
        late,
    })
}

/// What `armada need` prints after declaring.
pub fn declared_text(done: &Declared) -> String {
    let mine = &done.mine;
    let mut out = String::new();
    let verb = if done.already {
        "already recorded"
    } else {
        "recorded"
    };
    out.push_str(&format!(
        "need {verb}: {} on {}: {}\n",
        mine.branch, mine.path, mine.what
    ));
    if done.ahead.is_empty() {
        out.push_str(&format!("nothing is ahead of you on {}\n", mine.path));
        return out;
    }
    out.push_str(&format!("ahead of you on {}:\n", mine.path));
    for ahead in &done.ahead {
        out.push_str(&format!("  {}\n", ahead.describe()));
    }
    out.push_str(
        "pick the value after theirs, then `armada need --took <path> \"<value>\"`. \
         `armada land` holds this branch until they have landed or been given back.\n",
    );
    if done.late {
        out.push_str(&format!(
            "this branch already changes {}: if it took a number there before declaring, \
             search comments and docs for the old number and change every mention.\n",
            mine.path
        ));
    }
    out
}

/// `armada need --status`.
pub fn status_text(needs: &[Need]) -> String {
    if needs.is_empty() {
        return "no needs standing\n".to_string();
    }
    let mut out = String::new();
    let mut at: Option<&str> = None;
    let mut n = 0;
    for need in needs {
        if at != Some(need.path.as_str()) {
            at = Some(need.path.as_str());
            n = 0;
            out.push_str(&format!("{}\n", need.path));
        }
        n += 1;
        out.push_str(&format!("  {n}. {}\n", need.describe()));
    }
    out
}

/// Standing needs, grouped by path and in order within one.
pub fn by_path(mut needs: Vec<Need>) -> Vec<Need> {
    needs.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(adapters::needs::order(a).cmp(&adapters::needs::order(b)))
    });
    needs
}

/// Whether the checkout at `cwd` differs from the base at `path`, committed or
/// not. Unanswerable reads as no.
fn changes(cwd: &Path, path: &str) -> bool {
    let env = crate::land::Env::read();
    let base = format!("{}/{}", env.remote, env.base);
    let Ok(point) = crate::land::merge_base(cwd, "HEAD", &base) else {
        return false;
    };
    checked(cwd, &["diff", "--name-only", &point, "--", path])
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false)
}

/// Run one form of `armada need` from the checkout at `cwd`; what it says.
pub fn run(cwd: &Path, act: crate::cli::NeedAct) -> Result<String, String> {
    use crate::cli::NeedAct;
    let needs = Needs::of(cwd)?;
    if let NeedAct::Status = act {
        return Ok(status_text(&by_path(needs.standing())));
    }
    let branch = crate::land::current_branch(cwd)
        .ok_or("no branch is checked out here, and a need belongs to a branch")?;
    match act {
        NeedAct::Declare { path, what } => {
            Ok(declared_text(&declare(&needs, cwd, &branch, &path, &what)?))
        }
        NeedAct::Took { path, value } => {
            let need = needs.took(&branch, &path, &value)?;
            Ok(format!(
                "recorded: {} took {value} on {}\n",
                need.branch, need.path
            ))
        }
        NeedAct::Release { path } => {
            let said = match needs.release(&branch, &path) {
                true => format!("gave back {branch}'s need on {path}\n"),
                false => format!("{branch} had no need on {path}, so nothing was given back\n"),
            };
            nudge_the_line(cwd);
            Ok(said)
        }
        NeedAct::Status => unreachable!("answered above"),
    }
}

/// A branch held behind the need just given back is not waiting on a runner
/// that has already ended: start one where the line is not empty.
fn nudge_the_line(cwd: &Path) {
    let Ok(state) = StateDir::resolve(cwd) else {
        return;
    };
    let Ok(line) = crate::land::queued(&state) else {
        return;
    };
    if line.is_empty() {
        return;
    }
    if let (Ok(exe), Ok(common)) = (std::env::current_exe(), crate::land::common_git_dir(cwd)) {
        let _ = crate::land::runner::ensure_runner(&exe, &state, &common);
    }
}
