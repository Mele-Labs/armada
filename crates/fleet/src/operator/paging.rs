//! The Operator's page as Fleet serves it: Your turn in the order he left it, and the digest Since you
//! last looked. `crates/ipc/operations/operator_page.toml`.
//!
//! **Everything is read off what Fleet keeps**: the queue and the page's state from the store, the pull
//! requests that merged, the Log's rows and the rules from what was already kept, the live Jobs and
//! those waiting to start from the ledger. Nothing here makes a call.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::{
    item_read, OperatorComingUp, OperatorDigest, OperatorHandled, OperatorHandledAct, OperatorHold, OperatorLanded, OperatorLearned, OperatorLogFile, OperatorPage, OperatorRule, OperatorRunning, OperatorTurnItem,
    OperatorTurnKind,
};
use ipc::operator_talk::OperatorOrderForm;
use ipc::{Instant, OperatorRead, SessionId};
use store::KeptTurn;

use super::boarding::{first_line, outcome_of};
use super::suggesting::suggestion_number;
use super::talking::{moved_since, TurnRead};
use super::turning::RED;
use crate::daemon::Fleet;

const DAY: i64 = 86_400_000;
/// How many of the Log's rows the digest reads before keeping those since he looked.
const HANDLED_READ: u32 = 60;

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
    /// Your turn as a message may be about it, newest decisions as he left them.
    pub(super) fn turn_read(&self, store: &store::Store) -> Vec<TurnRead> {
        store
            .operator_turn_open()
            .unwrap_or_default()
            .into_iter()
            .map(|row| TurnRead {
                id: format!("turn:{}", row.n),
                job: row.job,
                text: item_read(&row.body).map(|item| item.text).unwrap_or_default(),
                read: row.read_text.map(|read| OperatorRead { read, pick: row.pick.unwrap_or_default() }),
            })
            .collect()
    }

    /// `operator_page`: read now. **Off, Your turn is empty**: nothing is queued while the Operator is off.
    pub(crate) async fn operator_page_wire(self: &Arc<Self>) -> Result<OperatorPage, Refusal> {
        let on = self.operator_on();
        if on {
            self.red_turns_swept().await;
        }
        // A hold its Job has released is let go as the orders are read, so the page lists none that is over.
        self.operator_orders_wire().await;
        let now = self.now();
        let live = match self.operator_seat().operator().filter(|_| on) {
            Some(operator) => operator.live_jobs().await,
            None => Vec::new(),
        };
        let since = match self.store().lock().await.operator_page().map_err(|why| self.ledger_fault(why))?.looked_at {
            Some(at) => at,
            None => crate::clock::rfc3339_utc(crate::clock::local_day_began(now.epoch_millis().unwrap_or(0) - DAY).await),
        };
        let (turn, digest, kept) = {
            let store = self.store().lock().await;
            let kept = store.operator_page().map_err(|why| self.ledger_fault(why))?;
            let rows = if on { store.operator_turn_open().map_err(|why| self.ledger_fault(why))? } else { Vec::new() };
            let mut turn: Vec<(bool, OperatorTurnItem)> = rows.iter().filter_map(|row| self.turn_item(&store, row, &live, now.as_str())).collect();
            // Later puts an item after the rest, and the sort is stable, so each group keeps his order.
            turn.sort_by_key(|(later, _)| *later);
            (turn.into_iter().map(|(_, one)| one).collect::<Vec<_>>(), self.digest_read(&store, &since), kept)
        };
        let mut digest = digest;
        digest.handled.log = self.journal_file_now().await;
        Ok(OperatorPage {
            on,
            top: kept.top,
            looked_at: kept.looked_at.as_deref().map(Instant::carried),
            on_it: self.operator_seat().raising.on_it().filter(|_| on),
            turn,
            digest,
        })
    }

    /// One row of Your turn as the wire has it, and whether it is put Later at `now`.
    fn turn_item(&self, store: &store::Store, row: &KeptTurn, live: &[super::arriving::LiveJob], now: &str) -> Option<(bool, OperatorTurnItem)> {
        let item = item_read(&row.body)?;
        let later = row.later_until.as_deref().filter(|until| *until > now);
        let thens = suggestion_number(&row.item)
            .and_then(|n| store.operator_suggestion(n).ok().flatten())
            .map(|kept| super::thening::thens_of(&kept, live))
            .unwrap_or_default()
            .into_iter()
            .map(|mut offer| {
                offer.pick = row.then_pick.as_deref() == Some(super::raising::word(&offer.then).as_str());
                offer
            })
            .collect();
        let keeps = row.go_ahead && (row.item.starts_with(RED) || suggestion_number(&row.item).is_some());
        let shown = OperatorTurnItem {
            id: format!("turn:{}", row.n),
            kind: if row.go_ahead { OperatorTurnKind::GoAhead } else { OperatorTurnKind::Decision },
            session_id: Some(row.session.as_str()).filter(|id| !id.is_empty()).map(SessionId::carried),
            job: row.job.clone(),
            manifest_id: Some(row.manifest_id.clone()).filter(|id| !id.is_empty()),
            repository: self.board_repository(&row.manifest_id),
            item,
            read: row.read_text.clone().map(|read| OperatorRead { read, pick: row.pick.clone().unwrap_or_default() }),
            raised_at: Instant::carried(&row.raised_at),
            later_until: later.map(Instant::carried),
            then: thens,
            keeps,
        };
        Some((later.is_some(), shown))
    }

    /// The digest, counted from `since`.
    fn digest_read(&self, store: &store::Store, since: &str) -> OperatorDigest {
        let title_of = |id: &str| store.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
        let summary_of = |id: &str| store.done_summary(id).ok().flatten();
        let landed = moved_since(store, since)
            .1
            .into_iter()
            .map(|one| OperatorLanded { session_id: SessionId::carried(&one.session_id), job: one.job, number: one.number, title: one.title })
            .collect();
        let mut acts: Vec<OperatorHandledAct> = store
            .operator_rulings_signed(HANDLED_READ)
            .unwrap_or_default()
            .iter()
            .filter(|row| row.by != "human" && row.at.as_str() >= since)
            .map(|row| OperatorHandledAct { at: Instant::carried(&row.at), title: first_line(&row.subject), outcome: outcome_of(row, &title_of, &summary_of) })
            .collect();
        for row in store.operator_turn_settled_since(since).unwrap_or_default().iter().filter(|row| row.answer.as_deref() == Some("rule")) {
            let title = item_read(&row.body).map(|item| first_line(&item.text)).unwrap_or_default();
            acts.push(OperatorHandledAct { at: Instant::carried(row.settled_at.as_deref().unwrap_or(&row.raised_at)), title, outcome: format!("By a rule: {}", row.pick.as_deref().unwrap_or_default()) });
        }
        acts.sort_by(|a, b| b.at.as_str().cmp(a.at.as_str()));
        let running = store
            .hostings()
            .unwrap_or_default()
            .into_iter()
            .filter(|hosting| store.session(&hosting.session_id).ok().flatten().is_some_and(|session| session.state != store::SessionState::Ended))
            .map(|hosting| OperatorRunning { title: title_of(&hosting.session_id), repository: self.board_repository(&hosting.manifest_id), session_id: SessionId::carried(&hosting.session_id) })
            .collect();
        let coming_up = store
            .open_waits()
            .unwrap_or_default()
            .into_iter()
            .map(|wait| OperatorComingUp { title: title_of(&wait.session_id), session_id: SessionId::carried(&wait.session_id), needs: wait.needs })
            .collect();
        let learned = store
            .operator_orders()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|one| {
                let OperatorRule::PickRule { manifest_id, .. } = OperatorRule::read(&one.form)?;
                let repository = manifest_id.as_deref().and_then(|id| self.board_repository(id));
                Some(OperatorLearned { id: one.n, manifest_id, repository, said: one.said, at: Instant::carried(&one.at) })
            })
            .collect();
        let holds = store
            .operator_orders()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|one| {
                let OperatorOrderForm::HoldPaths { manifest_id, .. } = OperatorOrderForm::read(&one.form)?;
                let repository = self.board_repository(&manifest_id);
                Some(OperatorHold { id: one.n, manifest_id, repository, said: one.said, at: Instant::carried(&one.at) })
            })
            .collect();
        OperatorDigest { landed, handled: OperatorHandled { acts, log: None }, running, coming_up, learned, holds }
    }

    /// This week's journal, where anything is written in it.
    async fn journal_file_now(&self) -> Option<OperatorLogFile> {
        let at = crate::clock::local_reading(self.now().epoch_millis().unwrap_or(0)).await;
        let name = super::journal::week_file(&at);
        let path = self.journal_dir().join(&name);
        path.is_file().then(|| OperatorLogFile { week: name.trim_end_matches(".md").to_string(), path: path.to_string_lossy().to_string() })
    }
}
