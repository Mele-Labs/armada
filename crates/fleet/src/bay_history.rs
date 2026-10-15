//! What happened to one slot, for the Cockpit's Bay pane — `get_slot_history` in
//! `crates/ipc/operations/`. `docs/practices/protocol.md`, *A bay's history*.
//!
//! **Read off disk at the asking, and nothing is kept here.** The pool writes a line for every lease and
//! release in whichever process made it (`adapters::leasing::turns_of`), and the slot's own `HEAD` reflog
//! has every checkout (`checkouts_of`). Fleet's part is naming the holders: a lease is written as a process
//! or as an id, and only Fleet knows whether an id is a Job or a hosted Session and what it is called.

use std::path::PathBuf;

use adapters::leasing::{checkouts_of, turns_of, Holder, Turn, KEPT_DAYS};
use ipc::{BayHistory, BayHistoryAsk, BayHistoryEntry, BayHolder, BayHolderKind, Instant};

use crate::clock::rfc3339_utc;

const DAY_MS: i64 = 86_400_000;

/// An entry and the instant it is ordered by.
type Dated = (i64, BayHistoryEntry);

fn at(millis: i64) -> Instant {
    Instant::carried(rfc3339_utc(millis))
}

impl<H, V, W> crate::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_slot_history`: the slot's leases, releases and checkouts of the last [`KEPT_DAYS`] days,
    /// newest first, cut to [`ipc::MOST_BAY_HISTORY`]. **A slot with no history answers none**, and a
    /// repository Fleet does not serve is refused.
    pub(crate) async fn slot_history(&self, ask: BayHistoryAsk) -> Result<BayHistory, api::Refusal> {
        let served = self.served_named(Some(&ask.manifest_id))?;
        let path = PathBuf::from(adapter_traits::slot_path(served.root(), ask.slot));
        let now = self.now().epoch_millis().unwrap_or(0);
        let since = now - KEPT_DAYS * DAY_MS;
        let (turns, checkouts) = tokio::task::spawn_blocking(move || (turns_of(&path, since), checkouts_of(&path, since)))
            .await
            .unwrap_or_default();

        let mut dated: Vec<Dated> = Vec::with_capacity(turns.len() + checkouts.len());
        for turn in turns {
            let at_ms = turn.at_ms;
            dated.push((at_ms, self.turn_entry(turn).await));
        }
        dated.extend(checkouts.into_iter().map(|one| {
            (
                one.at_ms,
                BayHistoryEntry::Checkout { at: at(one.at_ms), from: one.from, to: one.to, sha: one.sha },
            )
        }));
        Ok(history(ask, dated))
    }

    async fn turn_entry(&self, turn: Turn) -> BayHistoryEntry {
        let holder = self.bay_holder(&turn).await;
        let when = at(turn.at_ms);
        match turn.leased {
            true => BayHistoryEntry::Leased { at: when, holder, branch: turn.branch },
            false => BayHistoryEntry::Released { at: when, holder },
        }
    }

    /// Who a written holder is. **A hosted Session's lease is written as a Job of its own id**, so an id
    /// no Job has is a Session; a repair's holder is `<job>-repair-…` and is shown as its Job; and a
    /// process is an `agent`, named by the Session its lease was taken for where the record has one.
    async fn bay_holder(&self, turn: &Turn) -> BayHolder {
        match &turn.holder {
            Holder::Job(id) => {
                let job_named = |id: &str| {
                    let job = core_model::JobId::carried(core_model::Ulid::carried(id));
                    self.name_of(&job).map(|handle| (id.to_string(), handle))
                };
                let job = job_named(id)
                    .or_else(|| crate::repair_recovery::job_of_holder(id).and_then(job_named));
                match job {
                    Some((id, handle)) => BayHolder { kind: BayHolderKind::Job, id, title: Some(handle) },
                    None => BayHolder {
                        kind: BayHolderKind::Session,
                        title: self.session_in_ledger(id).await.and_then(|one| one.title),
                        id: id.clone(),
                    },
                }
            }
            Holder::Process { pid, .. } => match &turn.session {
                Some(session) => BayHolder {
                    kind: BayHolderKind::Agent,
                    title: self.session_in_ledger(session).await.and_then(|one| one.title),
                    id: session.clone(),
                },
                None => BayHolder { kind: BayHolderKind::Agent, id: pid.to_string(), title: None },
            },
        }
    }
}

/// The entries in order, newest first, and whether the limit left older ones out.
fn history(ask: BayHistoryAsk, mut dated: Vec<Dated>) -> BayHistory {
    dated.sort_by_key(|(at_ms, _)| std::cmp::Reverse(*at_ms));
    let cut = dated.len() > ipc::MOST_BAY_HISTORY;
    dated.truncate(ipc::MOST_BAY_HISTORY);
    BayHistory {
        manifest_id: ask.manifest_id,
        slot: ask.slot,
        entries: dated.into_iter().map(|(_, entry)| entry).collect(),
        cut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask() -> BayHistoryAsk {
        BayHistoryAsk { manifest_id: ipc::ManifestId::carried("armada"), slot: 10 }
    }

    fn checkout(at_ms: i64) -> Dated {
        (at_ms, BayHistoryEntry::Checkout { at: at(at_ms), from: None, to: "main".into(), sha: "a".repeat(40) })
    }

    #[test]
    fn entries_are_newest_first_and_a_full_list_is_not_cut() {
        let full: Vec<Dated> = (0..ipc::MOST_BAY_HISTORY as i64).map(|n| checkout(n * 1000)).collect();
        let read = history(ask(), full);
        assert!(!read.cut, "exactly the limit is not a cut");
        assert_eq!(read.entries.len(), ipc::MOST_BAY_HISTORY);
        let BayHistoryEntry::Checkout { at: first, .. } = &read.entries[0] else { panic!("a checkout") };
        assert_eq!(first.as_str(), "1970-01-01T00:04:59.000Z");
    }

    #[test]
    fn past_the_limit_the_oldest_are_left_out_and_it_says_so() {
        let over: Vec<Dated> = (0..=ipc::MOST_BAY_HISTORY as i64).map(|n| checkout(n * 1000)).collect();
        let read = history(ask(), over);
        assert!(read.cut);
        assert_eq!(read.entries.len(), ipc::MOST_BAY_HISTORY);
        let BayHistoryEntry::Checkout { at: last, .. } = read.entries.last().unwrap() else { panic!("a checkout") };
        assert_eq!(last.as_str(), "1970-01-01T00:00:01.000Z", "the entry at 0 is the one cut");
    }
}
