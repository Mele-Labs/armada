//! What happened to one slot: each lease and release, and each checkout inside it.
//!
//! **Leases are written by the pool, in whichever process took them.** `armada worktree lease` is a
//! command of its own and never reaches Fleet, so a record Fleet polled would miss a lease taken and
//! given back between two reads, and every one taken while Fleet was down. The pool already holds the
//! slot's lock when it writes the lease record or clears it, so the line goes down there and no
//! caller has to remember to. It sits beside the record as `slot-<n>.history`, one tab-separated line
//! a turn, and keeps [`KEPT_DAYS`] days: the file is cut on the next write, as the timeline is.
//!
//! **Checkouts are read, not written.** The slot's own `HEAD` reflog has each `git switch` and
//! `git checkout` with its instant, and is read as a file rather than by running git.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::record::Holder;
use super::{holder_of, session_of};

/// How long a slot's history is kept, and read back.
pub const KEPT_DAYS: i64 = 30;

const DAY_MS: i64 = 86_400_000;

/// One lease or release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    /// Milliseconds since the epoch.
    pub at_ms: i64,
    /// A lease, else a release.
    pub leased: bool,
    pub holder: Holder,
    /// The agent Session the lease was written as taken for, where the record named one.
    pub session: Option<String>,
    /// The branch the slot was leased onto. Absent on a release.
    pub branch: Option<String>,
}

/// One `git switch` or `git checkout` the slot's reflog holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkout {
    pub at_ms: i64,
    /// A branch, or the commit a detached head stood at. Absent where the reflog names none.
    pub from: Option<String>,
    /// A branch, or the commit it was detached at.
    pub to: String,
    /// The commit `HEAD` stood at once it moved.
    pub sha: String,
}

pub(super) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0)
}

fn journal_path(slot: &Path) -> PathBuf {
    let mut path = slot.as_os_str().to_owned();
    path.push(".history");
    PathBuf::from(path)
}

impl Turn {
    fn written(&self) -> String {
        let said = |text: &Option<String>| text.as_deref().unwrap_or("-").replace(['\t', '\n'], " ");
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.at_ms,
            if self.leased { "leased" } else { "released" },
            self.holder.written(),
            said(&self.session),
            said(&self.branch)
        )
    }

    fn read(line: &str) -> Option<Turn> {
        let mut fields = line.split('\t');
        let at_ms = fields.next()?.parse().ok()?;
        let leased = match fields.next()? {
            "leased" => true,
            "released" => false,
            _ => return None,
        };
        let holder = Holder::read(fields.next()?)?;
        let named = |field: Option<&str>| field.filter(|text| *text != "-").map(str::to_string);
        Some(Turn {
            at_ms,
            leased,
            holder,
            session: named(fields.next()),
            branch: named(fields.next()),
        })
    }
}

/// Add `turn` to the history of the slot at `slot`, and cut what is older than [`KEPT_DAYS`].
/// **Never fails the lease or release it describes**: a history that cannot be written is a gap
/// in a record, and the slot is still held or free as it was.
pub(super) fn note(slot: &Path, turn: &Turn) {
    let path = journal_path(slot);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let oldest = turn.at_ms - KEPT_DAYS * DAY_MS;
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| Turn::read(line).is_some_and(|turn| turn.at_ms >= oldest))
        .collect();
    let line = turn.written();
    if kept.len() == text.lines().count() {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = file.write_all(format!("{line}\n").as_bytes());
        }
        return;
    }
    let mut cut = kept.join("\n");
    if !cut.is_empty() {
        cut.push('\n');
    }
    cut.push_str(&line);
    cut.push('\n');
    let scratch = path.with_extension("history.cut");
    if std::fs::write(&scratch, cut).is_ok() {
        let _ = std::fs::rename(&scratch, &path);
    }
}

/// Every lease and release of the slot at `slot` since `since_ms`, oldest first. **A lease taken
/// without the agent's session in its record** reads as the record's now, where the same process
/// still holds the slot: Fleet writes the session onto a live holder's record after the lease.
pub fn turns_of(slot: &Path, since_ms: i64) -> Vec<Turn> {
    let text = std::fs::read_to_string(journal_path(slot)).unwrap_or_default();
    let mut turns: Vec<Turn> = text
        .lines()
        .filter_map(Turn::read)
        .filter(|turn| turn.at_ms >= since_ms)
        .collect();
    if let (Some(held), Some(session)) = (holder_of(slot), session_of(slot)) {
        for turn in turns.iter_mut().filter(|turn| turn.session.is_none() && turn.holder == held) {
            turn.session = Some(session.clone());
        }
    }
    turns
}

/// Every checkout in the `HEAD` reflog of the worktree at `slot` since `since_ms`, oldest first.
/// Empty where the slot is no worktree or its reflog is gone.
pub fn checkouts_of(slot: &Path, since_ms: i64) -> Vec<Checkout> {
    let Some(reflog) = git_dir_of(slot).map(|dir| dir.join("logs").join("HEAD")) else {
        return Vec::new();
    };
    match std::fs::read(reflog) {
        Ok(bytes) => checkouts_in(&String::from_utf8_lossy(&bytes), since_ms),
        Err(_) => Vec::new(),
    }
}

/// Where a linked worktree's own git directory is: what its `.git` file points at, or the
/// directory itself where it is one.
fn git_dir_of(slot: &Path) -> Option<PathBuf> {
    let dot_git = slot.join(".git");
    match std::fs::read_to_string(&dot_git) {
        Ok(said) => Some(slot.join(said.trim().strip_prefix("gitdir: ")?)),
        Err(_) if dot_git.is_dir() => Some(dot_git),
        Err(_) => None,
    }
}

/// The checkouts a reflog's text holds since `since_ms`, in the order it lists them.
///
/// A line is `<old> <new> <name> <<email>> <epoch> <zone>\t<message>`. The message of a checkout
/// is `checkout: moving from <from> to <to>`; a refname has no space, so the first ` to ` splits it.
/// Every other line, a commit or a reset or a merge, is not one and is left out.
pub fn checkouts_in(reflog: &str, since_ms: i64) -> Vec<Checkout> {
    reflog
        .lines()
        .filter_map(|line| {
            let (head, message) = line.split_once('\t')?;
            let (from, to) = message
                .strip_prefix("checkout: moving from ")?
                .split_once(" to ")?;
            let mut stamped = head.split_whitespace().rev();
            let _zone = stamped.next()?;
            let at_ms = stamped.next()?.parse::<i64>().ok()?.checked_mul(1000)?;
            let sha = head.split_whitespace().nth(1)?;
            (at_ms >= since_ms && !to.is_empty()).then(|| Checkout {
                at_ms,
                from: Some(from.to_string()).filter(|from| !from.is_empty()),
                to: to.to_string(),
                sha: sha.to_string(),
            })
        })
        .collect()
}
