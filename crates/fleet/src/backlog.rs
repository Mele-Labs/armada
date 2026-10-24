//! Keeping this machine's copy of a backlog in step with the others', and claiming an issue in it.
//! `docs/concepts/tracker.md`, *Beads*.
//!
//! **Fleet syncs, because nothing else will.** A write lands in this machine's copy and stays
//! there until something syncs it (spike 38), and an agent writes with the tracker's own command
//! line, so Fleet is never told. It reads the copy's [`CopyMark`] every [`LOOK_EVERY`] and syncs
//! when it moved, and syncs every [`SYNC_EVERY`] whatever, so a machine that writes nothing still
//! hears the others. **One sync of a repository at a time**: the caller holds its [`Keeping`]
//! behind one lock, and the claim syncs through the same one.
//!
//! **A claim is sync, claim, sync, read back.** Only the copy after a sync names the winner. The
//! first sync is not in the spike's three steps: without it a machine whose copy had not yet heard
//! of another's claim takes the issue anyway, and the later write wins, so a claim already
//! answered yes on the other machine would be taken from under it.

use std::time::Duration;

use adapter_traits::{Backlog, CopyMark, NotTracked, Tracker};

/// How often each copy's mark is read for a write: a file read, no process.
pub const LOOK_EVERY: Duration = Duration::from_secs(2);

/// The longest a copy goes without a sync, so another machine's write is seen within about a
/// minute: its own sync, then this one.
pub const SYNC_EVERY: Duration = Duration::from_secs(30);

/// What Fleet holds of one repository's copy between looks. `now` throughout is time since any
/// fixed moment, the same one for every call.
#[derive(Debug, Default)]
pub struct Keeping {
    synced_at: Option<Duration>,
    /// The copy's mark as the last sync left it.
    mark: Option<CopyMark>,
}

/// What one look did.
#[derive(Debug)]
pub enum Looked {
    /// Nothing wrote, and the timer is not due.
    Waited,
    /// `moved` where the copy holds something it did not before: a list read from it is old.
    Synced { moved: bool },
    /// The sync failed. The timer tries again, and a write already seen is not tried before it.
    Failed(NotTracked),
}

/// Why a claim did not take the issue.
#[derive(Debug, PartialEq, Eq)]
pub enum ClaimLost {
    /// The copy, synced, names someone else.
    Held { holder: String },
    /// Whether it holds is not known: the tracker refused or a sync failed. Claiming again is safe.
    Unsettled(NotTracked),
}

impl Keeping {
    /// Sync where the copy was written since the last sync, or the last is [`SYNC_EVERY`] old.
    pub fn look(&mut self, backlog: &dyn Backlog, root: &str, now: Duration) -> Looked {
        let written = backlog.mark(root) != self.mark;
        let due = self.synced_at.is_none_or(|at| now.saturating_sub(at) >= SYNC_EVERY);
        if !written && !due {
            return Looked::Waited;
        }
        match self.sync(backlog, root, now) {
            Ok(moved) => Looked::Synced { moved },
            Err(why) => Looked::Failed(why),
        }
    }

    /// Sync now. `true` where the copy moved since the last sync, by its own writes or the sync's.
    pub fn sync(&mut self, backlog: &dyn Backlog, root: &str, now: Duration) -> Result<bool, NotTracked> {
        let synced = backlog.sync(root);
        let mark = backlog.mark(root);
        let moved = mark != self.mark;
        self.synced_at = Some(now);
        self.mark = mark;
        synced.map(|()| moved)
    }
}

/// Claim `reference` for `actor`: sync, claim, sync, and read back who the copy says holds it.
pub fn claim_settled(
    keeping: &mut Keeping,
    backlog: &dyn Backlog,
    tracker: &dyn Tracker,
    root: &str,
    reference: &str,
    actor: &str,
    now: Duration,
) -> Result<(), ClaimLost> {
    keeping.sync(backlog, root, now).map_err(ClaimLost::Unsettled)?;
    let claimed = backlog.claim(root, reference, actor);
    if claimed.is_ok() {
        keeping.sync(backlog, root, now).map_err(ClaimLost::Unsettled)?;
    }
    let holders: Vec<String> = match tracker.read_issue(root, reference) {
        Ok(Some(read)) => read.assignees.iter().map(|one| one.as_written().trim().to_string()).collect(),
        Ok(None) => Vec::new(),
        Err(why) => return Err(ClaimLost::Unsettled(claimed.err().unwrap_or(why))),
    };
    match (holders.iter().any(|one| one == actor), holders.into_iter().find(|one| !one.is_empty())) {
        (true, _) => Ok(()),
        (false, Some(holder)) => Err(ClaimLost::Held { holder }),
        (false, None) => Err(ClaimLost::Unsettled(claimed.err().unwrap_or_else(|| {
            NotTracked::of("claiming the issue", format!("the claim on {reference} was not read back"))
        }))),
    }
}

/// An issue a Job claimed through Fleet, and who as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimHeld {
    pub session: String,
    pub issue: String,
    pub actor: String,
}

/// A claim the copy, synced, gave to someone else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimLoss {
    pub claim: ClaimHeld,
    pub holder: String,
}

/// Which of `held` the copy now names someone else for. **Two claims whose syncs overlap are both
/// answered yes**, and the later write wins at the next sync (spike 38): this is where the other
/// Job learns it. An issue the copy names nobody for, or cannot be read, is no loss.
pub fn claims_lost(tracker: &dyn Tracker, root: &str, held: &[ClaimHeld]) -> Vec<ClaimLoss> {
    held.iter()
        .filter_map(|claim| {
            let read = tracker.read_issue(root, &claim.issue).ok().flatten()?;
            let holders: Vec<String> =
                read.assignees.iter().map(|one| one.as_written().trim().to_string()).filter(|one| !one.is_empty()).collect();
            let holder = holders.first().filter(|_| !holders.contains(&claim.actor))?;
            Some(ClaimLoss { claim: claim.clone(), holder: holder.clone() })
        })
        .collect()
}
