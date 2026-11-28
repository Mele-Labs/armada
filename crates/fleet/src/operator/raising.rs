//! Your turn kept in step with what its Jobs still wait on. **Fleet detects**: a tap diffs each hosted Job's list
//! against the items of Your turn still standing, keyed by the Job and the item, and an item whose entry left the
//! list has left the queue. **Fleet queues nothing from a Job's list**: the Operator's session reads what waits on
//! Jobs with its tools and puts a card in front of him where it is his to decide. Fleet's own items, a red main
//! and a short disk, are queued where they arise.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_page::OperatorPageChanged;
use ipc::{Event, SessionRecord, WaitingItem, WaitingSource};
use store::KeptTurn;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;

/// What reads the queue against the lists, held by the seat.
#[derive(Default)]
pub(crate) struct Raising {
    /// Held while a list is read against the items standing, and while an item is kept, so an item is queued once.
    pub(super) turn: tokio::sync::Mutex<()>,
}

/// What only Fleet raises, each for one thing he does: an ask read off a reply beside one of these
/// says what it already asks.
const FLEETS_OWN: &[WaitingSource] = &[WaitingSource::HeldChild, WaitingSource::Plan, WaitingSource::Walk, WaitingSource::Draft, WaitingSource::Handoff, WaitingSource::Sketch, WaitingSource::Document];

/// A Job's items as they stand: **an ask read off a reply is left out while one of Fleet's own items stands**,
/// since the reply only repeats what that item asks.
pub(crate) fn without_echoes(items: Vec<WaitingItem>) -> Vec<WaitingItem> {
    let owned = items.iter().any(|one| FLEETS_OWN.contains(&one.source));
    items.into_iter().filter(|one| !(owned && one.source == WaitingSource::Replied)).collect()
}

/// Start keeping Your turn in step with the Jobs' lists. Every list is read against the items standing first.
/// Production starts it beside the ends' tap, and a test starts it itself.
pub async fn keep_the_raises<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<Box<SessionRecord>>();
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if record.hosted.is_some() {
                let _ = tell.send(Box::new(record.clone()));
            }
        }
    });
    fleet.raises_reconciled_all().await;
    tokio::spawn(async move {
        while let Some(record) = heard.recv().await {
            fleet.job_raises_reconciled(&record).await;
        }
    })
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The page is not what it was: Bridge reads it again.
    pub(crate) fn operator_page_changed(&self) {
        self.events().publish(Event::OperatorPageChanged(OperatorPageChanged {}));
    }

    /// One record of a hosted Job: its list against its items. **A paused Job's list is empty while it
    /// rests, and a snoozed Job raises nothing until its time, so either is left alone**; an ended
    /// Job's items have all left.
    async fn job_raises_reconciled(self: &Arc<Self>, record: &SessionRecord) {
        if resting(record) {
            return;
        }
        let items = if record.state == ipc::SessionState::Ended { Vec::new() } else { record.waiting_for.clone() };
        self.raises_reconciled(vec![(record.id.as_str().to_string(), items)], None).await;
    }

    /// Every hosted, live Job's list against every item standing. An item of a Job no longer live has left.
    pub(super) async fn raises_reconciled_all(self: &Arc<Self>) {
        let mut lists = Vec::new();
        let mut resting = BTreeSet::new();
        {
            let store = self.store().lock().await;
            let found = store.find_sessions(&store::SessionSearch { manifest_id: None, state: Some(store::SessionState::Live), text: None }).unwrap_or_default();
            for session in &found {
                if store.hosting(&session.id).ok().flatten().is_none() {
                    continue;
                }
                let Ok(record) = self.ledger_row(&store, session) else {
                    resting.insert(session.id.clone());
                    continue;
                };
                if self::resting(&record) {
                    resting.insert(session.id.clone());
                    continue;
                }
                lists.push((session.id.clone(), record.waiting_for));
            }
        }
        self.raises_reconciled(lists, Some(resting)).await;
    }

    /// Each `(session, items)` against the items standing of that session: one whose item left has left.
    /// With `all`, an item of any other session has left too, but those in `all`, which rest. **A red main's, a
    /// short disk's and a card's are their own to settle**, never read off a list.
    async fn raises_reconciled(self: &Arc<Self>, lists: Vec<(String, Vec<WaitingItem>)>, all: Option<BTreeSet<String>>) {
        let lists: Vec<_> = lists.into_iter().map(|(session, items)| (session, without_echoes(items))).collect();
        let _turn = self.operator_seat().raising.turn.lock().await;
        let present = self.store().lock().await.operator_turn_present().unwrap_or_default();
        let gone = |session: &str, item: &str| {
            if item.starts_with(super::turning::RED) || item.starts_with(super::disking::DISK_HOLD) || item.starts_with(super::cards::CARD) || item.starts_with(super::fleet_cards::FLEET_CARD) {
                return false;
            }
            match lists.iter().find(|(id, _)| id == session) {
                Some((_, items)) => !items.iter().any(|waiting| waiting.id == item),
                None => all.as_ref().is_some_and(|resting| !resting.contains(session)),
            }
        };
        let mut left = false;
        for one in present.iter().filter(|one| gone(&one.session, &one.item)) {
            left |= self.turn_left(one).await;
        }
        if left {
            self.operator_page_changed();
        }
    }

    /// An item of Your turn left its list, so its row has. The journal says so.
    pub(super) async fn turn_left(&self, one: &KeptTurn) -> bool {
        let left = self.store().lock().await.leave_operator_turn(one.n, self.now().as_str());
        match left {
            Ok(true) => {
                let text = ipc::operator_page::item_read(&one.body).map(|item| item.text).unwrap_or_default();
                self.operator_journaled(&format!("Settled: {}: {text}", one.job.as_deref().unwrap_or("My suggestion"))).await;
                true
            }
            Ok(false) => false,
            Err(why) => {
                self.operator_warned("an item of the person's Your turn was not marked settled", why);
                false
            }
        }
    }
}

/// A paused or snoozed Job, whose raises are left as they stand.
pub(super) fn resting(record: &SessionRecord) -> bool {
    record.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()) || record.snoozed_until.is_some()
}

#[cfg(test)]
mod tests {
    use ipc::SessionRecord;

    fn row(extra: &str) -> SessionRecord {
        let text = format!(
            r#"{{"id":"s1","harness":"h","origin":"bridge","cwd":"/r","state":"live","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{{}},"attachments":[]{extra}}}"#
        );
        ipc::decode("a session", text.as_bytes()).expect("a session")
    }

    #[test]
    fn a_snoozed_job_rests_and_its_raises_are_left_as_they_stand() {
        assert!(!super::resting(&row("")));
        assert!(super::resting(&row(r#","snoozed_until":"2026-10-08T05:00:00.000Z""#)));
    }
}
