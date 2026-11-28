//! The Operator's page as Fleet serves it: Your turn in the order he left it, the feed, and the digest Since
//! you last looked. `crates/ipc/operations/operator_page.toml`.
//!
//! **Everything is read off what Fleet keeps**: the queue and the page's state from the store, the pull
//! requests that merged, the holds that stand, the live Jobs and those waiting to start from the ledger.
//! Nothing here makes a call. The digest's `handled`, `learned` and `ruled` stay on the wire and are empty:
//! what the Operator does is in the feed, and no rule is kept.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::{
    item_read, OperatorComingUp, OperatorDigest, OperatorHandled, OperatorHold, OperatorLanded, OperatorLogFile, OperatorMain, OperatorPage, OperatorRunning, OperatorSlots, OperatorTurnItem, OperatorTurnKind,
    OperatorUrgency,
};
use ipc::operator_talk::OperatorOrderForm;
use ipc::{Instant, MainCiState, OperatorCardPicked, OperatorCardReturned, OperatorCardStep, OperatorRead, SessionId, SlotHolding};
use store::KeptTurn;

use super::feed::merged_since;
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
    /// `operator_page`: read now. **Off, Your turn is empty**: nothing is queued while the Operator is off.
    pub(crate) async fn operator_page_wire(self: &Arc<Self>) -> Result<OperatorPage, Refusal> {
        let on = self.operator_on();
        if on {
            self.red_turns_swept().await;
        }
        // A hold its Job has released is let go as the orders are read, so the page lists none that is over.
        self.operator_orders_wire().await;
        let now = self.now();
        let since = match self.store().lock().await.operator_page().map_err(|why| self.ledger_fault(why))?.looked_at {
            Some(at) => at,
            None => crate::clock::rfc3339_utc(crate::clock::local_day_began(now.epoch_millis().unwrap_or(0) - DAY).await),
        };
        let (turn, working, digest, kept, feed) = {
            let store = self.store().lock().await;
            let kept = store.operator_page().map_err(|why| self.ledger_fault(why))?;
            let rows = if on { store.operator_turn_open().map_err(|why| self.ledger_fault(why))? } else { Vec::new() };
            // A card he picked is worked on, off the front, until it is settled or comes back first in Your turn.
            let (mut picked, mut rows): (Vec<KeptTurn>, Vec<KeptTurn>) = rows.into_iter().partition(|row| row.picked_at.is_some() && row.returned_at.is_none());
            rows.sort_by_key(|row| row.returned_at.is_none());
            picked.sort_by(|a, b| b.picked_at.cmp(&a.picked_at).then(b.n.cmp(&a.n)));
            let resting = self.resting_jobs(&store);
            // A card put Later stays off the page until its time, and a resting Job's items stay off while it rests.
            let turn: Vec<OperatorTurnItem> =
                rows.iter().filter(|row| !resting.contains(&row.session)).filter_map(|row| self.turn_item(&store, row, now.as_str())).collect();
            // A card being worked on is never left out, so it is read with no Later.
            let working: Vec<OperatorTurnItem> =
                picked.into_iter().filter_map(|row| self.turn_item(&store, &KeptTurn { later_until: None, ..row }, now.as_str())).collect();
            let feed = if on { super::feed::feed_read(&store, &since) } else { Vec::new() };
            (grouped(turn), working, self.digest_read(&store, &since), kept, feed)
        };
        let acts = self.operator_acts_listed().await.map(|list| list.projects).unwrap_or_default();
        let operators = self.operator_settings_projects().await;
        // The first running Operator, for a Bridge that still draws one.
        let session_id = if on { operators.iter().find_map(|one| one.session_id.clone()) } else { None };
        // The thread the page draws while none runs: the newest session's, so the talk is never gone.
        let talk_session_id = match &session_id {
            Some(id) => Some(id.clone()),
            None => self.store().lock().await.newest_operator_hosting().ok().flatten().map(|hosting| SessionId::carried(&hosting.session_id)),
        };
        let mut digest = digest;
        digest.handled.log = self.journal_file_now().await;
        Ok(OperatorPage {
            on,
            top: kept.top,
            looked_at: kept.looked_at.as_deref().map(Instant::carried),
            on_it: None,
            turn,
            working,
            digest,
            mains: self.mains_read().await,
            slots: self.slots_read().await,
            feed,
            session_id,
            talk_session_id,
            operator_acts: acts,
            operators,
        })
    }

    /// The Jobs that are paused or snoozed: what they wait on is not his to answer until they wake.
    fn resting_jobs(&self, store: &store::Store) -> std::collections::BTreeSet<String> {
        let found = store.find_sessions(&store::SessionSearch { manifest_id: None, state: Some(store::SessionState::Live), text: None }).unwrap_or_default();
        found
            .iter()
            .filter(|session| store.hosting(&session.id).ok().flatten().is_some())
            .filter(|session| self.ledger_row(store, session).map_or(true, |record| super::raising::resting(&record)))
            .map(|session| session.id.clone())
            .collect()
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

    /// One row of Your turn as the wire has it. **A card put Later is left out until its time passes.**
    fn turn_item(&self, store: &store::Store, row: &KeptTurn, now: &str) -> Option<OperatorTurnItem> {
        let item = item_read(&row.body)?;
        if row.later_until.as_deref().is_some_and(|until| until > now) {
            return None;
        }
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
            later_until: None,
            then: Vec::new(),
            keeps: false,
            also_raised_by: ask.as_ref().map(|ask| ask.also_raised_by.clone()).unwrap_or_default(),
            group: None,
            follows: None,
            conflict: ask.as_ref().and_then(crate::session_host::job_asks::card_conflict),
            held: ask.as_ref().and_then(crate::session_host::job_asks::card_held),
            card: row.card.as_deref().and_then(|text| ipc::decode::<ipc::OperatorCard>("a card", text.as_bytes()).ok()).map(|card| card_picked(card, row)),
            operator: row.project.as_deref().map(|project| super::settings::operator_tag_in(store, project)),
        };
        Some(shown)
    }

    /// The digest, counted from `since`.
    fn digest_read(&self, store: &store::Store, since: &str) -> OperatorDigest {
        let title_of = |id: &str| store.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
        let landed = merged_since(store, since)
            .into_iter()
            .map(|one| OperatorLanded { session_id: SessionId::carried(&one.session_id), job: one.job, number: one.number, title: one.title, at: Instant::carried(&one.at) })
            .collect();
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
        OperatorDigest { landed, handled: OperatorHandled { acts: Vec::new(), log: None }, running, coming_up, learned: Vec::new(), holds, ruled: Vec::new() }
    }

    /// This week's journal, where anything is written in it.
    async fn journal_file_now(&self) -> Option<OperatorLogFile> {
        let at = crate::clock::local_reading(self.now().epoch_millis().unwrap_or(0)).await;
        let name = super::journal::week_file(&at);
        let path = self.journal_dir().join(&name);
        path.is_file().then(|| OperatorLogFile { week: name.trim_end_matches(".md").to_string(), path: path.to_string_lossy().to_string() })
    }
}

/// `card` with what its row keeps of his press: the pick, the Operator's steps and why it came back.
fn card_picked(card: ipc::OperatorCard, row: &KeptTurn) -> ipc::OperatorCard {
    let picked = row.picked.clone().zip(row.picked_at.as_deref()).map(|(option, at)| OperatorCardPicked { option, at: Instant::carried(at) });
    let steps = row.steps.as_deref().and_then(|text| ipc::decode::<Vec<OperatorCardStep>>("a card's steps", text.as_bytes()).ok()).unwrap_or_default();
    let returned = row.returned_why.clone().zip(row.returned_at.as_deref()).map(|(why, at)| OperatorCardReturned { why, at: Instant::carried(at) });
    ipc::OperatorCard { picked, steps, returned, ..card }
}

/// Your turn with each Job's asks together. **Where a Job has two or more waiting they share its id as
/// `group`**, and stand at the place of the first of them in his order, in the order the Job raised
/// them, so naming any one of them in an order moves them all.
fn grouped(turn: Vec<OperatorTurnItem>) -> Vec<OperatorTurnItem> {
    let key = |one: &OperatorTurnItem| one.session_id.as_ref().map(|id| id.as_str().to_string());
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for one in &turn {
        if let Some(key) = key(one) {
            *counts.entry(key).or_default() += 1;
        }
    }
    let mut placed = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(turn.len());
    for one in &turn {
        match key(one).filter(|key| counts[key] > 1) {
            None => out.push(one.clone()),
            Some(group) if placed.insert(group.clone()) => {
                let mut asks: Vec<OperatorTurnItem> = turn.iter().filter(|other| key(other).as_ref() == Some(&group)).cloned().collect();
                asks.sort_by(|a, b| a.raised_at.as_str().cmp(b.raised_at.as_str()));
                out.extend(asks.into_iter().map(|ask| OperatorTurnItem { group: Some(group.clone()), ..ask }));
            }
            Some(_) => {}
        }
    }
    out
}
