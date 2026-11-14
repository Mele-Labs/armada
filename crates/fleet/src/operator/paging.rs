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
    item_read, OperatorComingUp, OperatorDigest, OperatorFollows, OperatorHandled, OperatorHandledAct, OperatorHold, OperatorLanded, OperatorLearned, OperatorLogFile, OperatorMain, OperatorPage, OperatorRuled, OperatorRule, OperatorRunning,
    OperatorSlots, OperatorTurnItem, OperatorTurnKind, OperatorUrgency,
};
use ipc::operator_talk::OperatorOrderForm;
use ipc::{Instant, MainCiState, OperatorRead, SessionId, SlotHolding};
use store::KeptTurn;

use super::boarding::{first_line, outcome_of};
use super::suggesting::suggestion_number;
use super::talking::{moved_since, TurnRead};
use super::turning::RED;
use crate::daemon::Fleet;

const DAY: i64 = 86_400_000;

/// How soon `row` wants him: a red main and a short disk are urgent, else its kind. No stopped Job is queued today, so none reads urgent.
fn urgency_of(row: &KeptTurn) -> OperatorUrgency {
    if row.item.starts_with(RED) || row.item.starts_with(super::disking::DISK_HOLD) {
        OperatorUrgency::Urgent
    } else if row.go_ahead {
        OperatorUrgency::GoAhead
    } else {
        OperatorUrgency::Decision
    }
}
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
            (grouped(turn), self.digest_read(&store, &since), kept)
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
            mains: self.mains_read().await,
            slots: self.slots_read().await,
        })
    }

    /// Each served repository's main, as `main_ci` last kept it: the row `red_item_offered` and the
    /// merge line read, so no call is made.
    async fn mains_read(&self) -> Vec<OperatorMain> {
        let store = self.store().lock().await;
        self.repositories()
            .served()
            .iter()
            .map(|served| {
                let manifest_id = served.manifest().id().as_str().to_string();
                let main = store.main_ci(served.root()).ok().flatten();
                // A red stays red until a green, so a fix still running does not clear it.
                let red = main.as_ref().is_some_and(|it| it.red_at.is_some() || it.state == store::MainState::Red);
                OperatorMain {
                    repository: self.board_repository(&manifest_id).unwrap_or_else(|| manifest_id.clone()),
                    manifest_id,
                    state: main.as_ref().map(|it| match (red, it.state) {
                        (true, _) => MainCiState::Red,
                        (false, store::MainState::Green) => MainCiState::Green,
                        (false, store::MainState::Running) => MainCiState::Running,
                        (false, store::MainState::NothingRan | store::MainState::Red) => MainCiState::NothingRan,
                    }),
                    commit: main.as_ref().map(|it| if red { it.red_commit() } else { it.decided() }.to_string()),
                    since: main.as_ref().filter(|_| red).and_then(|it| it.red_at.as_ref()).map(|at| Instant::carried(at.as_str())),
                }
            })
            .collect()
    }

    /// The slots in use, from the list Bridge's Cleanup reads: kept in memory where a keeper holds it.
    async fn slots_read(&self) -> Option<OperatorSlots> {
        let held = self.worktrees_answer().await.ok()?;
        let used = held.slots.iter().filter(|slot| matches!(slot.held, SlotHolding::Busy | SlotHolding::Session { .. } | SlotHolding::HostedSession { .. } | SlotHolding::Stranded { .. })).count();
        Some(OperatorSlots { used: used as u32, total: held.slots.len() as u32 }).filter(|slots| slots.total > 0)
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
        let keeps = row.go_ahead && (row.item.starts_with(RED) || suggestion_number(&row.item).is_some() || super::answering::what_of(&item).is_some());
        let ask = store.latest_job_ask(&row.item).ok().flatten().filter(|ask| ask.session_id == row.session);
        let shown = OperatorTurnItem {
            id: format!("turn:{}", row.n),
            kind: if row.go_ahead { OperatorTurnKind::GoAhead } else { OperatorTurnKind::Decision },
            urgency: urgency_of(row),
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
            also_raised_by: match &ask {
                Some(ask) => ask.also_raised_by.clone(),
                None => suggestion_number(&row.item)
                    .and_then(|n| store.operator_suggestion(n).ok().flatten())
                    .map(|kept| super::suggesting::folded_into(&kept, &store.operator_suggestions_of(&kept.session_id).unwrap_or_default()))
                    .unwrap_or_default(),
            },
            group: None,
            follows: row.follows.as_deref().and_then(|text| ipc::decode::<ipc::operator_page::OperatorFollows>("a citation", text.as_bytes()).ok()),
            conflict: ask.as_ref().and_then(crate::session_host::job_asks::card_conflict),
            held: ask.as_ref().and_then(crate::session_host::job_asks::card_held),
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
            .map(|one| OperatorLanded { session_id: SessionId::carried(&one.session_id), job: one.job, number: one.number, title: one.title, at: Instant::carried(&one.at) })
            .collect();
        let mut acts: Vec<OperatorHandledAct> = store
            .operator_rulings_signed(HANDLED_READ)
            .unwrap_or_default()
            .iter()
            .filter(|row| row.by != "human" && row.at.as_str() >= since)
            .map(|row| OperatorHandledAct { at: Instant::carried(&row.at), title: first_line(&row.subject), outcome: outcome_of(row, &title_of, &summary_of) })
            .collect();
        for row in store.operator_turn_settled_since(since).unwrap_or_default().iter().filter(|row| matches!(row.answer.as_deref(), Some("rule" | super::vetting::SETTING))) {
            let title = item_read(&row.body).map(|item| first_line(&item.text)).unwrap_or_default();
            let by = if row.answer.as_deref() == Some(super::vetting::SETTING) { "By a setting" } else { "By a rule" };
            acts.push(OperatorHandledAct { at: Instant::carried(row.settled_at.as_deref().unwrap_or(&row.raised_at)), title, outcome: format!("{by}: {}", row.pick.as_deref().unwrap_or_default()) });
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
                let OperatorRule::PickRule { scope, .. } = OperatorRule::read(&one.form)?;
                let repository = self.scope_named(&scope);
                Some(OperatorLearned { id: one.n, scope, repository, said: one.said, at: Instant::carried(&one.at) })
            })
            .collect();
        let holds = store
            .operator_orders()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|one| {
                let OperatorOrderForm::HoldPaths { manifest_id, paths, job, until } = OperatorOrderForm::read(&one.form)?;
                let repository = self.board_repository(&manifest_id);
                Some(OperatorHold { id: one.n, manifest_id, repository, said: one.said, paths, job: Some(job), until: Some(until), at: Instant::carried(&one.at) })
            })
            .collect();
        // The last seven days, whatever he last looked at: it is what he may take back.
        let week_ago = self.now().epoch_millis().map(|now| crate::clock::rfc3339_utc(now - 7 * DAY)).unwrap_or_default();
        let ruled = store
            .operator_turn_settled_since(&week_ago)
            .unwrap_or_default()
            .iter()
            .filter(|row| row.answer.as_deref() == Some("precedent"))
            .filter_map(|row| {
                let follows = row.follows.as_deref().and_then(|text| ipc::decode::<OperatorFollows>("a citation", text.as_bytes()).ok())?;
                Some(OperatorRuled {
                    id: format!("turn:{}", row.n),
                    session_id: SessionId::carried(&row.session),
                    job: row.job.clone(),
                    question: item_read(&row.body).map(|item| first_line(&item.text)).unwrap_or_default(),
                    pick: row.pick.clone().unwrap_or_default(),
                    follows,
                    at: Instant::carried(row.settled_at.as_deref().unwrap_or(&row.raised_at)),
                    undone_at: row.undone_at.as_deref().map(Instant::carried),
                    undo: row.undone_at.is_none() && store.session(&row.session).ok().flatten().is_some_and(|session| session.state != store::SessionState::Ended),
                })
            })
            .collect();
        OperatorDigest { landed, handled: OperatorHandled { acts, log: None }, running, coming_up, learned, holds, ruled }
    }

    /// This week's journal, where anything is written in it.
    async fn journal_file_now(&self) -> Option<OperatorLogFile> {
        let at = crate::clock::local_reading(self.now().epoch_millis().unwrap_or(0)).await;
        let name = super::journal::week_file(&at);
        let path = self.journal_dir().join(&name);
        path.is_file().then(|| OperatorLogFile { week: name.trim_end_matches(".md").to_string(), path: path.to_string_lossy().to_string() })
    }
}

/// Your turn with each Job's asks together. **Where a Job has two or more waiting they share its id as
/// `group`**, and stand at the place of the first of them in his order, in the order the Job raised
/// them, so naming any one of them in an order moves them all. Those put Later group among themselves.
fn grouped(turn: Vec<(bool, OperatorTurnItem)>) -> Vec<OperatorTurnItem> {
    let key = |(later, one): &(bool, OperatorTurnItem)| one.session_id.as_ref().map(|id| (*later, id.as_str().to_string()));
    let mut counts: std::collections::BTreeMap<(bool, String), usize> = std::collections::BTreeMap::new();
    for one in &turn {
        if let Some(key) = key(one) {
            *counts.entry(key).or_default() += 1;
        }
    }
    let mut placed = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(turn.len());
    for one in &turn {
        match key(one).filter(|key| counts[key] > 1) {
            None => out.push(one.1.clone()),
            Some(group) if placed.insert(group.clone()) => {
                let mut asks: Vec<OperatorTurnItem> = turn.iter().filter(|other| key(other).as_ref() == Some(&group)).map(|other| other.1.clone()).collect();
                asks.sort_by(|a, b| a.raised_at.as_str().cmp(b.raised_at.as_str()));
                out.extend(asks.into_iter().map(|ask| OperatorTurnItem { group: Some(group.1.clone()), ..ask }));
            }
            Some(_) => {}
        }
    }
    out
}

