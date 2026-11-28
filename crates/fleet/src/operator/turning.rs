//! What the person does with Your turn: a press on an item, Later, and the order he drags it into.
//! `crates/ipc/operations/operator_answer.toml`.
//!
//! **A press on a card** settles it and tells the Operator's session which option. **A press on a Fleet-own
//! item** does its pick through what Fleet already has: a red main's fix, a short disk's drop, a held Job's
//! start, or an option answered on its Job's list. **What Fleet cannot perform itself is left to Bridge**: the
//! act of a walk or a pull request goes back, and the item stands until its Job's list drops it.
//!
//! Always here and Always are not offered: what the Operator does unasked is the project's `operator_acts`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::{item_read, OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorLater, OperatorPage, OperatorTurnOrder};
use ipc::{AnswerWaiting, BeginSession, Instant, SessionId, WaitingAct, WaitingItem, WaitingOption, WaitingSource, WireError};
use store::KeptTurn;

use super::held;
use super::orders::OPERATOR_OFF;
use crate::daemon::Fleet;
use crate::main_ci::{MainChange, MainChanged};
use crate::main_fix::brief_of;

/// An item of a red main starts with this: `red:<repository>:<commit>`.
pub(super) const RED: &str = "red:";

const TURN_UNKNOWN: &str = "fleet.operator_turn_unknown";
const NOT_OFFERED: &str = "fleet.operator_answer_not_offered";
const NOT_DONE: &str = "fleet.operator_turn_not_done";
const LATER_PAST: &str = "fleet.operator_later_past";

/// What a held Job's start item's id starts with, then the Job's id.
const HELD_START: &str = "start:";
const TURN: &str = "turn:";
/// The one option a red main's item offers.
const START_FIX: &str = "Start a fix";
/// A pick that puts the item last for the local day.
const LATER: &str = "Later";

/// What a pick did, for the answer that carries it back.
#[derive(Debug, Default)]
pub(super) struct Done {
    pub(super) open: Option<WaitingAct>,
    pub(super) started: Option<String>,
}

/// Why a pick was not done.
pub(super) enum Fault {
    /// The answer is not one this item offers.
    NotOffered(String),
    /// It was offered and could not be carried out.
    NotDone(String),
}

/// An item as it is acted on: whose it is, the item, and what was picked for it.
pub(super) struct Doing<'a> {
    pub(super) session: &'a str,
    pub(super) item: &'a WaitingItem,
    pub(super) pick: &'a str,
}

/// An RFC 3339 instant in milliseconds since the epoch, its zone read: `Z`, or `+HH:MM` and `-HH:MM`
/// as a zone Bridge sends from his clock. `None` where it does not read.
pub(crate) fn utc_millis(text: &str) -> Option<i64> {
    let at = core_model::Timestamp::from_rfc3339(text).epoch_millis()?;
    let zone = text.get(19..)?.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match zone.as_bytes() {
        b"Z" | b"z" => 0,
        [sign @ (b'+' | b'-'), ..] if zone.len() == 6 && zone.as_bytes()[3] == b':' => {
            let minutes = zone[1..3].parse::<i64>().ok()? * 60 + zone[4..6].parse::<i64>().ok()?;
            if *sign == b'+' { minutes } else { -minutes }
        }
        _ => return None,
    };
    Some(at - offset * 60_000)
}

pub(super) fn turn_number(id: &str) -> Option<i64> {
    id.strip_prefix(TURN).and_then(|n| n.parse().ok())
}

/// A pick's option on `item`, as its index.
fn choice_of(item: &WaitingItem, pick: &str) -> Option<u32> {
    let label = pick.trim();
    item.options.iter().position(|option: &WaitingOption| !label.is_empty() && option.label == label).and_then(|at| u32::try_from(at).ok())
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
    pub(super) fn turn_unknown(&self, id: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(TURN_UNKNOWN, format!("{id} is not an item waiting in Your turn"), self.run_id()))
    }

    pub(super) async fn turn_open(&self, id: &str) -> Result<KeptTurn, Refusal> {
        let n = turn_number(id).ok_or_else(|| self.turn_unknown(id))?;
        let row = self.store().lock().await.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?;
        row.filter(|row| row.settled_at.is_none()).ok_or_else(|| self.turn_unknown(id))
    }

    /// Do `doing`'s pick. **Nothing is done that the item does not offer.**
    pub(super) async fn turn_performed(self: &Arc<Self>, doing: &Doing<'_>) -> Result<Done, Fault> {
        let id = doing.item.id.as_str();
        if let Some(rest) = id.strip_prefix(RED) {
            let (manifest, commit) = rest.split_once(':').ok_or_else(|| Fault::NotDone("the red main's id does not read".to_string()))?;
            let job = self.operator_started_fix(manifest, commit).await.map_err(Fault::NotDone)?;
            return Ok(Done { started: Some(job), ..Done::default() });
        }
        if let Some((manifest, number)) = super::reviewing::parts_of(id) {
            return match doing.pick {
                super::reviewing::RUN => self.review_run_pressed(manifest, number).await.map(|()| Done::default()).map_err(Fault::NotDone),
                super::reviewing::NOT_NOW => Ok(Done::default()),
                other => Err(Fault::NotOffered(format!("{other} is not a pick this item offers"))),
            };
        }
        if let Some(manifest) = id.strip_prefix(super::disking::DISK_HOLD) {
            return match doing.pick {
                super::disking::DROP_THEM => self.disk_dropped(manifest).await.map(|()| Done::default()).map_err(Fault::NotDone),
                super::disking::LEAVE_THEM => Ok(Done::default()),
                other => Err(Fault::NotOffered(format!("{other} is not a pick this item offers"))),
            };
        }
        // A held Job's start is pressed as its card's Start, with the landing the card picked. Where the
        // card still asks how the work lands, it is opened for him to answer.
        if let (WaitingSource::HeldChild, Some(child)) = (doing.item.source, id.strip_prefix(HELD_START)) {
            let delivery = self.card_picked(child).await;
            if self.session_begun(BeginSession { session_id: SessionId::carried(child), delivery, message: None }).await.is_ok() {
                return Ok(Done { started: Some(child.to_string()), ..Done::default() });
            }
        }
        if let Some(choice) = choice_of(doing.item, doing.pick) {
            let said = AnswerWaiting { session_id: SessionId::carried(doing.session), item_id: id.to_string(), choice: Some(choice), text: None, mode: None, fixed: None, extras: Default::default() };
            return match Arc::clone(self).settle_waiting(said).await {
                Ok(_) => Ok(Done::default()),
                Err(refused) => Err(Fault::NotDone(refused.error().message.clone())),
            };
        }
        match &doing.item.act {
            Some(act) => Ok(Done { open: Some(act.clone()), ..Done::default() }),
            None => Err(Fault::NotOffered("this item has no pick Fleet can do and nothing for Bridge to open".to_string())),
        }
    }

    /// `operator_answer`: one press on an item of Your turn.
    pub(crate) async fn operator_answered(self: &Arc<Self>, press: OperatorAnswerPress) -> Result<OperatorAnswered, Refusal> {
        if !self.operator_on() {
            return Err(self.hosted_refusal(OPERATOR_OFF, "the Operator is off"));
        }
        let row = self.turn_open(&press.id).await?;
        if row.card.is_some() {
            return self.card_answered(&row, &press).await;
        }
        if matches!(press.answer, OperatorAnswer::AlwaysHere | OperatorAnswer::Always) {
            return Err(self.hosted_refusal(NOT_OFFERED, "Always here and Always are not offered: what the Operator does unasked is the project's `operator_acts`"));
        }
        let item = item_read(&row.body).ok_or_else(|| self.ledger_unnamed("an item that reads"))?;
        // The option he named, where it is one the item offers: absent takes the pick the item was queued with.
        let named = match press.option.as_deref().map(str::trim).filter(|label| !label.is_empty()) {
            Some(label) if item.options.iter().any(|option| option.label == label) => Some(label.to_string()),
            Some(label) => return Err(self.hosted_refusal(NOT_OFFERED, &format!("{label} is not an option this item offers"))),
            None => None,
        };
        // Yes on a pick of Later is Later: what it waits on is his to do away from the page.
        if press.answer == OperatorAnswer::Yes && named.is_none() && row.pick.as_deref() == Some(LATER) {
            let until = self.next_day_began(&self.now()).await;
            self.turn_put_later(row.n, &until).await?;
            let page = self.operator_page_wire().await?;
            return Ok(OperatorAnswered { open: None, started: None, filed: None, handed_to: None, page });
        }
        let answer_word = if press.answer == OperatorAnswer::Yes { "yes" } else { "no" };
        let mut done = Done::default();
        if press.answer == OperatorAnswer::Yes {
            let doing = Doing { session: &row.session, item: &item, pick: named.as_deref().or(row.pick.as_deref()).unwrap_or_default() };
            done = self.turn_performed(&doing).await.map_err(|fault| match fault {
                Fault::NotOffered(why) => self.hosted_refusal(NOT_OFFERED, &why),
                Fault::NotDone(why) => self.hosted_refusal(NOT_DONE, &why),
            })?;
        }
        if done.open.is_some() {
            if let Err(why) = self.store().lock().await.answer_operator_turn(row.n, answer_word) {
                self.operator_warned("a press on an item of Your turn was not kept", why);
            }
            self.operator_journaled(&format!("Answered {answer_word}: {}: {}. Left to him to open.", row.job.as_deref().unwrap_or("Fleet"), item.text)).await;
        } else {
            self.settled_by_press(&row, answer_word, &item, &done).await;
        }
        self.operator_page_changed();
        let page = self.operator_page_wire().await?;
        Ok(OperatorAnswered { open: done.open, started: done.started.as_deref().map(SessionId::carried), filed: None, handed_to: None, page })
    }

    /// One press on a card: the option he named, else the Operator's pick, settles it and tells the Operator.
    /// A card is answered by an option alone, so No and the Always presses are not offered on it.
    async fn card_answered(self: &Arc<Self>, row: &KeptTurn, press: &OperatorAnswerPress) -> Result<OperatorAnswered, Refusal> {
        if press.answer != OperatorAnswer::Yes {
            return Err(self.hosted_refusal(NOT_OFFERED, "a card is answered with one of its options, or put Later"));
        }
        self.card_pressed(row, press.option.as_deref().map(str::trim).filter(|label| !label.is_empty())).await?;
        let page = self.operator_page_wire().await?;
        Ok(OperatorAnswered { open: None, started: None, filed: None, handed_to: None, page })
    }

    /// Item `row` settled by his press, and the journal says what he did.
    async fn settled_by_press(&self, row: &KeptTurn, answer: &str, item: &WaitingItem, done: &Done) {
        let at = self.now();
        if let Err(why) = self.store().lock().await.settle_operator_turn(row.n, at.as_str(), Some(answer)) {
            self.operator_warned("a press on an item of Your turn was not kept", why);
        }
        let mut entry = format!("Answered {answer}: {}: {}", row.job.as_deref().unwrap_or("Fleet"), item.text);
        if done.started.is_some() {
            entry.push_str(". Started a Job");
        }
        self.operator_journaled(&entry).await;
    }

    /// `operator_turn_order`: the open items he named go first, in his order.
    pub(crate) async fn operator_turn_ordered(self: &Arc<Self>, order: OperatorTurnOrder) -> Result<OperatorPage, Refusal> {
        let ids: Vec<i64> = order.ids.iter().filter_map(|id| turn_number(id)).collect();
        self.store().lock().await.order_operator_turn(&ids).map_err(|why| self.ledger_fault(why))?;
        self.operator_page_changed();
        self.operator_page_wire().await
    }

    /// A Later that ran out since the last look puts its item back, which no event says, so the page
    /// is announced changed then. Called on each tick; the store is read only when the soonest is due.
    pub(crate) async fn laters_run_out(self: &Arc<Self>) {
        let now = self.now().as_str().to_string();
        let soonest = held(&self.operator_seat().next_later).clone();
        if let Some(None) = &soonest {
            return;
        }
        if let Some(Some(until)) = &soonest {
            if until.as_str() > now.as_str() {
                return;
            }
        }
        let next = self.store().lock().await.operator_turn_open().unwrap_or_default().into_iter().filter_map(|row| row.later_until).filter(|until| until.as_str() > now.as_str()).min();
        *held(&self.operator_seat().next_later) = Some(next);
        // The first look only reads what stands: Bridge reads the page whole when it comes.
        if soonest.is_some() {
            self.operator_page_changed();
        }
    }

    /// `operator_later`: the item goes last until `until`, or until the local day ends. **Refused with
    /// nothing changed** where `until` has passed or does not read as a time.
    pub(crate) async fn operator_turn_later(self: &Arc<Self>, later: OperatorLater) -> Result<OperatorPage, Refusal> {
        let row = self.turn_open(&later.id).await?;
        let now = self.now();
        let until = match &later.until {
            None => self.next_day_began(&now).await,
            Some(until) => {
                let millis = utc_millis(until.as_str()).ok_or_else(|| self.hosted_refusal(LATER_PAST, "That time could not be read. Pick another."))?;
                if millis <= now.epoch_millis().unwrap_or(0) {
                    return Err(self.hosted_refusal(LATER_PAST, "That time has passed. Pick a later one."));
                }
                crate::clock::rfc3339_utc(millis)
            }
        };
        self.turn_put_later(row.n, &until).await?;
        if row.card.is_some() {
            self.card_sent_later(&row, &until).await;
        }
        self.operator_page_wire().await
    }

    /// Row `n` last until `until`, and the page told now and again when it runs out.
    pub(super) async fn turn_put_later(self: &Arc<Self>, n: i64, until: &str) -> Result<(), Refusal> {
        self.store().lock().await.later_operator_turn(n, until).map_err(|why| self.ledger_fault(why))?;
        if let Some(next) = held(&self.operator_seat().next_later).as_mut() {
            if next.as_deref().is_none_or(|soonest| until < soonest) {
                *next = Some(until.to_string());
            }
        }
        self.operator_page_changed();
        Ok(())
    }

    /// A red main nobody is on joins Your turn, with the pick Start a fix, and one that is fixed or
    /// green leaves it. Called with what main's CI just did. **Main turning red or green moves the
    /// head's pills, so the page is announced changed here, once, whether or not an item came of it.**
    pub(crate) async fn red_offered(self: &Arc<Self>, changes: &[MainChanged]) {
        if !changes.is_empty() {
            self.operator_page_changed();
        }
        for changed in changes {
            match changed.change {
                MainChange::WentRed | MainChange::BackToRed => self.red_item_offered(&changed.now).await,
                MainChange::WentGreen => self.red_turns_swept().await,
            }
        }
    }

    /// Queued once for each red commit, however many times it is offered.
    pub(crate) async fn red_item_offered(self: &Arc<Self>, main: &store::MainCi) {
        if !self.operator_on() || main.held() || self.taking(main).await.is_some() {
            return;
        }
        let Some(served) = self.repositories().served().into_iter().find(|served| served.root() == main.repository) else { return };
        let manifest = served.manifest().id().as_str().to_string();
        let commit = main.red_commit();
        let facts = brief_of(main).lines().next().unwrap_or_default().to_string();
        let id = format!("{RED}{manifest}:{commit}");
        let item = WaitingItem {
            order: 0,
            why: String::new(),
            approves: false,
            id,
            text: format!("Main is red at {}. {facts}", commit.get(..7).unwrap_or(commit)).trim().to_string(),
            since: Instant::carried(self.now().as_str()),
            source: WaitingSource::OperatorSuggestion,
            act: None,
            options: vec![WaitingOption { label: START_FIX.to_string(), description: None, picture: None }],
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
            artifact: None,
            goal_met: None,
        };
        let raising = &self.operator_seat().raising;
        let _turn = raising.turn.lock().await;
        let kept = {
            let mut store = self.store().lock().await;
            if store.operator_turn_present().unwrap_or_default().iter().any(|row| row.item == item.id) {
                return;
            }
            let row = KeptTurn {
                manifest_id: manifest,
                go_ahead: true,
                body: ipc::operator_page::item_kept(&item),
                pick: Some(START_FIX.to_string()),
                raised_at: self.now().as_str().to_string(),
                item: item.id.clone(),
                ..KeptTurn::default()
            };
            store.add_operator_turn(&row)
        };
        match kept {
            Ok(_) => {
                self.operator_journaled(&format!("Queued: {}", item.text)).await;
                self.operator_page_changed();
            }
            Err(why) => self.operator_warned("a red main's item was not kept", why),
        }
    }

    /// Every red main still in Your turn that is green now, or has a Job on it, has left it.
    pub(crate) async fn red_turns_swept(self: &Arc<Self>) {
        let present = self.store().lock().await.operator_turn_present().unwrap_or_default();
        let mut left = false;
        for row in present.iter().filter(|row| row.item.starts_with(RED)) {
            let (manifest, commit) = row.item.strip_prefix(RED).and_then(|rest| rest.split_once(':')).unwrap_or_default();
            if !self.red_is_standing(manifest, commit).await {
                left |= self.turn_left(row).await;
            }
        }
        if left {
            self.operator_page_changed();
        }
    }
}
