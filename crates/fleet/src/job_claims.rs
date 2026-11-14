//! What each live Job holds, claimed in its project's store so a Job on another Mac or a teammate's
//! knows. `docs/concepts/fleet.md`, *Claims a Job holds*.
//!
//! **Fleet works out what a Job holds; the agent reports nothing.** Its tracker item is each open
//! issue row on its ledger, its files are its plan's paths and each file Fleet saw it write
//! (`crate::clashing`), and its pull request is each open one it holds. A pass sends the store only
//! the difference from what the store already answered, so **there is no log of calls to replay**:
//! while the store is down nothing grows but the difference, and it goes out when the store answers.
//!
//! **A held item makes the Job wait; a held file only tells it.** A beads assignee who is a person
//! holds the item too. The wait is a line on the Job, and ends when Fleet takes the claim, which
//! wakes the Job with a word to carry on. The owner's rulings of 13 Nov 2026,
//! `.claude/decisions/2026-11-13-claims-store.md`.
//!
//! **Memory only.** After a restart the next pass claims everything again, and the store answers a
//! claim this holder already has as its own.

mod exchange;
mod pass;
mod wanting;
mod words;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use adapter_traits::{ClaimKind, Claimant};

#[cfg(test)]
pub(crate) use exchange::key_of;
pub(crate) use exchange::{exchanged, Answered, Asked};

/// How often a pass sends what changed. A new write waits at most this long to be claimed.
pub(crate) const PASS_EVERY: Duration = Duration::from_secs(60);

/// How often a live Job's claims are renewed, in one call, and a claim held elsewhere is asked
/// for again. The store drops a claim 30 minutes after its holder's last renewal.
pub(crate) const RENEW_EVERY: Duration = Duration::from_secs(5 * 60);

/// The most store calls one Job makes in a pass, so a Job that wrote many files at once stays under
/// the store's 120 calls a minute for the device. The rest go in the next pass.
pub(crate) const CALLS_A_PASS: usize = 40;

/// One thing a Job claims, by the name this Mac knows it by: an issue's reference, a path, or a
/// pull request's number. The store sees only a hash of it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Wanted {
    pub kind: ClaimKind,
    pub named: String,
}

impl Wanted {
    pub(crate) fn item(named: &str) -> Wanted {
        Wanted { kind: ClaimKind::Item, named: named.to_string() }
    }

    pub(crate) fn file(named: &str) -> Wanted {
        Wanted { kind: ClaimKind::File, named: named.to_string() }
    }

    pub(crate) fn pull_request(number: u64) -> Wanted {
        Wanted { kind: ClaimKind::PullRequest, named: number.to_string() }
    }
}

/// Who an item is held by, while a Job waits on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WaitedOn {
    /// Another Job, on this Mac or another, holds its claim.
    Job(Claimant),
    /// A person is its assignee in beads.
    Person(String),
}

/// A Job waiting on an item, and since when (RFC 3339).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wait {
    pub on: WaitedOn,
    pub since: String,
}

/// What one live Job holds, as far as this Mac knows.
#[derive(Clone, Debug, Default)]
pub(crate) struct JobHeld {
    /// What the store answered as this Job's.
    pub sent: BTreeSet<Wanted>,
    /// What another holder has, as the store answered. Asked for again at each renewal.
    pub elsewhere: BTreeMap<Wanted, Claimant>,
    /// The items it waits on, by reference.
    pub waits: BTreeMap<String, Wait>,
    /// The last pass had something to send and the store did not take it.
    pub unsent: bool,
    /// When its claims were last renewed, in epoch milliseconds.
    pub renewed_at: Option<i64>,
    /// Files whose claims ended with a pull request that merged or closed, not claimed again.
    pub files_ended: BTreeSet<String>,
    /// The pull requests whose end was already read for its files.
    pub pulls_ended: BTreeSet<u64>,
}

impl JobHeld {
    /// Whether its renewal is due at `now`.
    pub(crate) fn renewal_due(&self, now: i64) -> bool {
        self.renewed_at.is_none_or(|at| now.saturating_sub(at) >= RENEW_EVERY.as_millis() as i64)
    }
}

/// Every live Job's claims, and when the last pass ran.
#[derive(Debug, Default)]
pub(crate) struct Kept {
    pub jobs: BTreeMap<String, JobHeld>,
    /// The epoch milliseconds the last pass began.
    pub passed_at: Option<i64>,
    /// The assignees each beads item had when last read, by repository and reference.
    pub assignees: BTreeMap<(String, String), Vec<String>>,
}

/// The fields a Job's record carries: each item it waits on, and whether its claims are unsent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Shown {
    pub waits: Vec<(String, Wait)>,
    pub unsent: bool,
}

/// The Jobs' claims, behind one lock that is never held across an await.
#[derive(Debug, Default)]
pub(crate) struct JobClaims(std::sync::Mutex<Kept>);

impl JobClaims {
    pub(crate) fn with<T>(&self, work: impl FnOnce(&mut Kept) -> T) -> T {
        let mut kept = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        work(&mut kept)
    }

    /// What `session`'s record shows of its claims.
    pub(crate) fn shown(&self, session: &str) -> Shown {
        self.with(|kept| {
            kept.jobs
                .get(session)
                .map(|held| Shown { waits: held.waits.iter().map(|(item, wait)| (item.clone(), wait.clone())).collect(), unsent: held.unsent })
                .unwrap_or_default()
        })
    }
}

impl Shown {
    /// The record's wait lines, each with its holder, Mac and title, and what ends it.
    pub(crate) fn wire(&self) -> Vec<ipc::ClaimWait> {
        self.waits
            .iter()
            .map(|(item, wait)| {
                let since = ipc::Instant::from(&core_model::Timestamp::from_rfc3339(wait.since.clone()));
                let known = |text: &str| Some(text.trim().to_string()).filter(|text| !text.is_empty());
                match &wait.on {
                    WaitedOn::Job(by) => ipc::ClaimWait {
                        item: item.clone(),
                        holder: by.name.clone(),
                        machine: known(&by.holder.machine),
                        title: known(&by.title),
                        since,
                        ends: ipc::ClaimWaitEnds::ClaimEnds,
                    },
                    WaitedOn::Person(person) => {
                        ipc::ClaimWait { item: item.clone(), holder: person.clone(), machine: None, title: None, since, ends: ipc::ClaimWaitEnds::Unassigned }
                    }
                }
            })
            .collect()
    }
}

/// Whether a beads assignee is a person: a Job claims as `<Mac>/<job>`, a person by a bare name.
pub(crate) fn is_person(assignee: &str) -> bool {
    let assignee = assignee.trim();
    !assignee.is_empty() && !assignee.contains('/')
}

/// Whether no person is assigned `issue`, so a Job may take it. Read of beads only.
pub(crate) fn no_person_holds(issue: &adapter_traits::OpenIssue) -> bool {
    !issue.assignees.iter().any(|one| is_person(one.as_written()))
}

/// The person an item's assignees name, other than `own`, the name this Job claimed it as.
pub(crate) fn person_holding<'a>(assignees: &'a [String], own: Option<&str>) -> Option<&'a String> {
    assignees.iter().find(|one| is_person(one) && Some(one.trim()) != own.map(str::trim))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_is_a_person_and_a_jobs_claim_is_not() {
        assert!(is_person("nick"));
        assert!(!is_person("work-mac/5aaa472e"));
        assert!(!is_person("  "));
        let assignees = vec!["work-mac/5aaa472e".to_string(), "nick".to_string()];
        assert_eq!(person_holding(&assignees, None).map(String::as_str), Some("nick"));
        assert_eq!(person_holding(&assignees, Some("nick")), None);
    }

    #[test]
    fn a_renewal_is_due_five_minutes_after_the_last() {
        let mut held = JobHeld::default();
        assert!(held.renewal_due(0));
        held.renewed_at = Some(1_000);
        assert!(!held.renewal_due(1_000 + 299_999));
        assert!(held.renewal_due(1_000 + 300_000));
    }
}
