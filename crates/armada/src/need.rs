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
//! `armada-land/`, so every worktree of one clone sees the same line. A need is
//! spent when its branch lands, and given back when its branch no longer exists
//! locally or a person runs `--release`. **Nothing expires by time**: whether a
//! stalled need should is open, so a person gives a stalled one back.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::land::git::{best_effort, checked};
use crate::land::{codec, key, StateDir};

/// One branch's claim on a path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Need {
    pub branch: String,
    pub path: String,
    /// What is needed there, as the declarer said it.
    pub what: String,
    /// What the branch took (`V95`, `23.5`), once it has said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<String>,
    /// When it declared, in nanoseconds: the order.
    pub place: i64,
}

impl Need {
    /// `branch: what`, and what it took where it said.
    pub fn describe(&self) -> String {
        match &self.took {
            Some(took) => format!("{}: {}, took {took}", self.branch, self.what),
            None => format!("{}: {}, took nothing yet", self.branch, self.what),
        }
    }
}

/// What declaring came to.
pub struct Declared {
    pub mine: Need,
    pub already: bool,
    pub ahead: Vec<Need>,
    /// Set where the branch already changes the path while others are ahead on
    /// it: whatever number it took there may have to move.
    pub late: bool,
}

/// The directory of needs for one repository.
pub struct Needs {
    dir: PathBuf,
    /// Where git questions are asked: the repository, never one worktree.
    repo: PathBuf,
}

impl Needs {
    /// The needs of the repository `within` belongs to.
    pub fn of(within: &Path) -> Result<Needs, String> {
        let out = checked(
            within,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .map_err(|why| why.to_string())?;
        let common = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
        Ok(Needs::at_common(&common))
    }

    pub fn at_common(common: &Path) -> Needs {
        Needs {
            dir: common.join("armada-needs"),
            repo: common.to_path_buf(),
        }
    }

    /// The needs beside the merge line's own state.
    pub fn beside(state: &StateDir) -> Needs {
        Needs::at_common(state.path().parent().unwrap_or(state.path()))
    }

    fn file(&self, branch: &str, path: &str) -> PathBuf {
        self.dir.join(format!("{}-{}.json", key(branch), key(path)))
    }

    /// Record `branch`'s need on `path`; the same branch and path again
    /// records nothing. `cwd` is the checkout, for what the branch has changed.
    pub fn declare(
        &self,
        cwd: &Path,
        branch: &str,
        path: &str,
        what: &str,
    ) -> Result<Declared, String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|why| format!("{}: {why}", self.dir.display()))?;
        let path = clean_path(path);
        let file = self.file(branch, &path);
        let held: Option<Need> = codec::read("need", &file).map_err(|why| why.to_string())?;
        let already = held.is_some();
        let mine = match held {
            Some(held) => held,
            None => {
                let mine = Need {
                    branch: branch.to_string(),
                    path: path.clone(),
                    what: what.to_string(),
                    took: None,
                    place: now(),
                };
                codec::write(&file, &mine).map_err(|why| why.to_string())?;
                mine
            }
        };
        let ahead = self.ahead_of(&mine);
        let late = !ahead.is_empty() && changes(cwd, &path);
        Ok(Declared {
            mine,
            already,
            ahead,
            late,
        })
    }

    /// Record what `branch` took on `path`.
    pub fn took(&self, branch: &str, path: &str, value: &str) -> Result<Need, String> {
        let path = clean_path(path);
        let file = self.file(branch, &path);
        let held: Option<Need> = codec::read("need", &file).map_err(|why| why.to_string())?;
        let Some(mut need) = held else {
            return Err(format!(
                "{branch} has no need on {path} — `armada need {path} \"<what>\"` first, \
                 then say what it took"
            ));
        };
        need.took = Some(value.to_string());
        codec::write(&file, &need).map_err(|why| why.to_string())?;
        Ok(need)
    }

    /// Give `branch`'s need on `path` back. `false` where it had none.
    pub fn release(&self, branch: &str, path: &str) -> bool {
        std::fs::remove_file(self.file(branch, &clean_path(path))).is_ok()
    }

    /// `branch` landed: every need it held is spent.
    pub fn spend(&self, branch: &str) {
        for need in self.read_all() {
            if need.branch == branch {
                let _ = std::fs::remove_file(self.file(&need.branch, &need.path));
            }
        }
    }

    /// Every need still standing, in order. A need whose branch no longer
    /// exists locally is given back here, so nobody waits behind a branch
    /// that was deleted.
    pub fn standing(&self) -> Vec<Need> {
        let mut all = self.read_all();
        all.retain(|need| {
            let gone = !branch_exists(&self.repo, &need.branch);
            if gone {
                let _ = std::fs::remove_file(self.file(&need.branch, &need.path));
            }
            !gone
        });
        all
    }

    /// The needs on the same path that declared before `need`.
    pub fn ahead_of(&self, need: &Need) -> Vec<Need> {
        self.standing()
            .into_iter()
            .filter(|other| other.path == need.path && order(other) < order(need))
            .collect()
    }

    /// What `branch` waits behind: each of its needs with the ones ahead of it.
    pub fn behind(&self, branch: &str) -> Vec<(Need, Vec<Need>)> {
        let standing = self.standing();
        standing
            .iter()
            .filter(|need| need.branch == branch)
            .map(|need| {
                let ahead = standing
                    .iter()
                    .filter(|other| other.path == need.path && order(other) < order(need))
                    .cloned()
                    .collect();
                (need.clone(), ahead)
            })
            .filter(|(_, ahead): &(Need, Vec<Need>)| !ahead.is_empty())
            .collect()
    }

    fn read_all(&self) -> Vec<Need> {
        let Ok(listing) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut found: Vec<Need> = listing
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .filter_map(|path| codec::read::<Need>("need", &path).ok().flatten())
            .collect();
        found.sort_by_key(order);
        found
    }
}

fn order(need: &Need) -> (i64, String) {
    (need.place, need.branch.clone())
}

/// What a held branch says it waits behind, one clause per path.
pub fn waiting_text(behind: &[(Need, Vec<Need>)]) -> String {
    let clauses: Vec<String> = behind
        .iter()
        .map(|(mine, ahead)| {
            let who: Vec<String> = ahead.iter().map(Need::describe).collect();
            format!("{} on {} ({})", mine.what, mine.path, who.join("; "))
        })
        .collect();
    format!("waiting behind {}", clauses.join("; "))
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
    needs.sort_by(|a, b| a.path.cmp(&b.path).then(order(a).cmp(&order(b))));
    needs
}

fn clean_path(path: &str) -> String {
    path.trim_start_matches("./").to_string()
}

fn branch_exists(repo: &Path, branch: &str) -> bool {
    best_effort(
        repo,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .map(|out| out.status.success())
    // Git that could not run says nothing about the branch.
    .unwrap_or(true)
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

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos() as i64)
        .unwrap_or(0)
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
            Ok(declared_text(&needs.declare(cwd, &branch, &path, &what)?))
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
