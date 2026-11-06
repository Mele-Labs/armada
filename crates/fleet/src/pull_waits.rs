//! Whether a Session's wait is about a pull request, so a merge or an auto-merge settles it. **Only
//! where the item's own `approve_pr` act names the pull request**: its words are never read.
//!
//! Only the agent's own items and the asks Fleet reads off a reply. A question card, a permission, a
//! walk is answered by its own press, never by a merge.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{WaitingActKind, WaitingItem, WaitingSource};

use crate::daemon::Fleet;

/// Whether `item` waits on the person to approve pull request `number`: its `approve_pr` act names it.
pub fn waits_on(item: &WaitingItem, number: u64) -> bool {
    if !matches!(item.source, WaitingSource::Agent | WaitingSource::Replied) {
        return false;
    }
    item.act
        .as_ref()
        .is_some_and(|act| act.kind == WaitingActKind::ApprovePr && number_of(&act.target) == Some(number))
}

/// `#2589`, `2589` or a pull request's address, as an `approve_pr` act's target is written.
pub fn number_of(target: &str) -> Option<u64> {
    let tail = target.trim().trim_end_matches('/').rsplit(['/', '#']).next()?;
    tail.parse().ok()
}

impl Fleet
{
    /// Take off a Session's own items, and the asks read off its reply, whose `approve_pr` act names pull request `number`, as [`Self::run_item_settled`] does for a
    /// command: the notice Fleet tells next says why, in Fleet's voice, and the turn it starts clears
    /// an ask read off a reply. The texts of the items taken off.
    pub async fn waits_taken_off(&self, session: &str, number: u64) -> Vec<String> {
        let record = {
            let mut store = self.store().lock().await;
            let Ok(Some(kept)) = store.session(session) else { return Vec::new() };
            let Ok(reads) = self.ledger_reads(&store, &kept) else { return Vec::new() };
            let taken: Vec<(String, String)> = self
                .ledger_waiting(&kept, reads)
                .into_iter()
                .filter(|item| waits_on(item, number))
                .map(|item| (item.id, item.text))
                .collect();
            if taken.is_empty() || taken.iter().any(|(id, _)| store.dismiss_waiting(session, id).is_err()) {
                return Vec::new();
            }
            let Ok(record) = self.ledger_row(&store, &kept) else { return Vec::new() };
            (record, taken)
        };
        let (row, taken) = record;
        self.publish(ipc::Event::SessionChanged(row));
        taken.into_iter().map(|(_, text)| text).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{Instant, WaitingAct};

    fn item(source: WaitingSource, text: &str, act: Option<(WaitingActKind, &str)>) -> WaitingItem {
        WaitingItem {
            id: "one".to_string(),
            text: text.to_string(),
            since: Instant::carried("2026-10-19T00:00:00Z"),
            source,
            act: act.map(|(kind, target)| WaitingAct { kind, target: target.to_string() }),
            options: Vec::new(),
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        }
    }

    #[test]
    fn an_approve_pr_act_names_the_pull_request_by_its_target_alone() {
        for target in ["#2589", "2589", "https://github.com/o/r/pull/2589"] {
            assert!(waits_on(&item(WaitingSource::Agent, "Review it", Some((WaitingActKind::ApprovePr, target))), 2589), "{target}");
        }
        assert!(!waits_on(&item(WaitingSource::Agent, "Merge #2589", Some((WaitingActKind::ApprovePr, "#2590"))), 2589));
    }

    #[test]
    fn words_never_name_it() {
        for text in ["Merge #2589 when ci is green", "Pull request 2589 is queued", "https://github.com/o/r/pull/2589"] {
            assert!(!waits_on(&item(WaitingSource::Agent, text, None), 2589), "{text}");
            assert!(!waits_on(&item(WaitingSource::Replied, text, Some((WaitingActKind::Run, "scripts/restart"))), 2589), "{text}");
        }
    }

    #[test]
    fn a_question_card_is_never_settled_by_a_merge() {
        for source in [WaitingSource::AskCard, WaitingSource::Permission, WaitingSource::Walk] {
            assert!(!waits_on(&item(source, "Merge?", Some((WaitingActKind::ApprovePr, "#2589"))), 2589));
        }
    }
}
