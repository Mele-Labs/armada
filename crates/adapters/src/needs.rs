//! What a branch says it needs on a path, as it was kept before the ledger: one
//! file per need under `armada-needs/` in the git common directory.
//! `.claude/decisions/2026-10-02-a-plan-leases-its-numbers.md`.
//!
//! **Fleet no longer writes these.** A need is a row on the session ledger
//! (`docs/capabilities/needs.md`) and `armada need` asks Fleet. What is left is
//! the one read Fleet makes of the files, at its first start, to carry them into
//! the ledger.
//!
//! A need is a path and what is needed there, in the declarer's words, keyed by
//! branch and path. First to declare goes first, and nothing expires by time.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

mod codec;
mod keys;

use keys::{git_common_dir, key};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declared {
    pub mine: Need,
    /// The branch had declared this path before; nothing was recorded.
    pub already: bool,
    /// The needs on the same path that declared first, in order.
    pub ahead: Vec<Need>,
}

/// The directory of needs for one repository.
#[derive(Clone, Debug)]
pub struct Needs {
    dir: PathBuf,
    /// Where git questions are asked: the repository, never one worktree.
    repo: PathBuf,
}

impl Needs {
    /// The needs of the repository `within` belongs to.
    pub fn of(within: &Path) -> Result<Needs, String> {
        let common = git_common_dir(within)?;
        Ok(Needs::at_common(&common))
    }

    pub fn at_common(common: &Path) -> Needs {
        Needs {
            dir: common.join("armada-needs"),
            repo: common.to_path_buf(),
        }
    }

    fn file(&self, branch: &str, path: &str) -> PathBuf {
        self.dir.join(format!("{}-{}.json", key(branch), key(path)))
    }

    /// Record `branch`'s need on `path`; the same branch and path again
    /// records nothing.
    pub fn declare(&self, branch: &str, path: &str, what: &str) -> Result<Declared, String> {
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
        Ok(Declared {
            mine,
            already,
            ahead,
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

    /// `branch` landed or was dropped: every need it held is spent or given
    /// back, which is the same removal.
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

    /// Every need on disk, in order, read and nothing else. **Unlike
    /// [`standing`](Needs::standing) it removes nothing**, which is what the
    /// conversion into Fleet's ledger needs: the files stay where they are.
    pub fn files(&self) -> Vec<Need> {
        self.read_all()
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

/// The order needs are served in: when each declared, then the branch.
pub fn order(need: &Need) -> (i64, String) {
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

/// A path as it is keyed: without a leading `./`.
pub fn clean_path(path: &str) -> String {
    path.trim_start_matches("./").to_string()
}

/// Whether `branch` exists in the repository at `repo`. Git that could not run
/// says nothing about it, so that reads as yes.
pub fn branch_exists(repo: &Path, branch: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .output()
        .map(|out| out.status.success())
        // Git that could not run says nothing about the branch.
        .unwrap_or(true)
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos() as i64)
        .unwrap_or(0)
}
