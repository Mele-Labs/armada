//! `armada worktree`'s forms: `lease <branch>`, `release [<path>]`, the four
//! that change the pool on this machine — `add`, `remove`, `close` and `open`
//! — and `--status`, the merge line's flag for the same question.

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
    /// `add`: one more slot on this machine, made by the next lease.
    Add,
    /// `remove <n>`: the slot gone, refused unless it is free or not made.
    Remove { slot: usize },
    /// `close <n>`: no lease takes it until it is reopened.
    Close { slot: usize },
    /// `open <n>`.
    Open { slot: usize },
}

/// The forms, as `armada help` prints them.
pub(super) fn usage(out: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (shape, what) in [
        (
            "lease <branch>  ",
            "a warm slot on a new branch; prints its path",
        ),
        ("release [<path>]", "give it back, once clean and landed"),
        (
            "add             ",
            "one more slot on this machine; the next lease makes it",
        ),
        ("remove <n>      ", "slot n gone, if it is free or not made"),
        (
            "close <n>       ",
            "no lease takes slot n until it is opened",
        ),
        ("open <n>        ", "lease slot n again"),
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
        Some("add") => WorktreeAct::Add,
        Some(form @ ("remove" | "close" | "open")) => {
            let Some(slot) = positional
                .get(1)
                .map(|n| n.trim_start_matches("slot-"))
                .and_then(|n| n.parse().ok())
            else {
                faults.push(Fault::NoSlot {
                    form: form.to_string(),
                });
                return None;
            };
            match form {
                "remove" => WorktreeAct::Remove { slot },
                "close" => WorktreeAct::Close { slot },
                _ => WorktreeAct::Open { slot },
            }
        }
        other => {
            faults.push(Fault::WorktreeActUnknown {
                given: other.unwrap_or("").to_string(),
            });
            return None;
        }
    };
    let takes = if act == WorktreeAct::Add { 1 } else { 2 };
    if let Some(extra) = positional.get(takes..).filter(|extra| !extra.is_empty()) {
        faults.push(Fault::TooMany {
            verb: format!("{WORKTREE} {}", positional[0]),
            extra: extra.to_vec(),
        });
    }
    Some(Verb::Worktree(act))
}
