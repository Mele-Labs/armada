//! `armada worktree`'s three forms: `lease <branch>`, `release [<path>]` and
//! `--status`, the merge line's flag for the same question.

use std::fmt;
use std::path::PathBuf;

use super::{Fault, Verb, WORKTREE};

/// Which of `armada worktree`'s forms was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorktreeAct {
    /// `lease <branch>`: a warm slot on a new branch cut from the base.
    Lease { branch: String },
    /// `release [<path>]`: give a slot back, the one standing in by default.
    Release { path: Option<PathBuf> },
    /// `--status`: every slot, and who holds it since when.
    Status,
}

/// The three forms, as `armada help` prints them.
pub(super) fn usage(out: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (shape, what) in [
        (
            "lease <branch>  ",
            "a warm slot on a new branch; prints its path",
        ),
        ("release [<path>]", "give it back, once clean and landed"),
        (
            "--status        ",
            "every slot, who holds it, and since when",
        ),
    ] {
        writeln!(out, "  armada {WORKTREE} {shape}    {what}")?;
    }
    Ok(())
}

pub(super) fn read(rest: &[String], faults: &mut Vec<Fault>) -> Option<Verb> {
    let mut status = false;
    let mut positional = Vec::new();
    for arg in rest {
        match arg.as_str() {
            "--status" => status = true,
            flag if flag.starts_with('-') => faults.push(Fault::NoSuchFlag {
                given: arg.clone(),
                allowed: vec!["--status".to_string()],
            }),
            _ => positional.push(arg.clone()),
        }
    }
    if status {
        if let Some(extra) = positional.first() {
            faults.push(Fault::WorktreeActUnknown {
                given: extra.clone(),
            });
        }
        return Some(Verb::Worktree(WorktreeAct::Status));
    }
    let act = match positional.first().map(String::as_str) {
        Some("lease") => match positional.get(1) {
            Some(branch) => WorktreeAct::Lease {
                branch: branch.clone(),
            },
            None => {
                faults.push(Fault::NoBranch);
                return None;
            }
        },
        Some("release") => WorktreeAct::Release {
            path: positional.get(1).map(PathBuf::from),
        },
        other => {
            faults.push(Fault::WorktreeActUnknown {
                given: other.unwrap_or("").to_string(),
            });
            return None;
        }
    };
    if let Some(extra) = positional.get(2..).filter(|extra| !extra.is_empty()) {
        faults.push(Fault::TooMany {
            verb: format!("{WORKTREE} {}", positional[0]),
            extra: extra.to_vec(),
        });
    }
    Some(Verb::Worktree(act))
}
