//! What the person does with Your turn: a press on an item, Later, and the order he drags it into.
//! `crates/ipc/operations/operator_answer.toml`.
//!
//! **Yes does the pick**, through what Fleet already has: a suggestion's Do it, an option answered on
//! its Job's list, a red main's fix. **What Fleet cannot perform itself is left to Bridge**: the act of
//! a walk or a pull request goes back, and the item stands until its Job's list drops it.
//!
//! **A rule is kept only for what Fleet can do by itself**, a go-ahead and a red main, and it lives in the
//! store's orders beside the standing orders, in a form of its own that a list of orders never serves.
//! It applies only while the Operator is on, to an item as it arrives.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::{item_read, OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorLater, OperatorPage, OperatorRule, OperatorThen, OperatorTurnOrder};
use ipc::{AnswerWaiting, Instant, SessionId, WaitingAct, WaitingItem, WaitingOption, WaitingSource, WireError};
use store::{KeptOrder, KeptTurn};

use super::orders::OPERATOR_OFF;
use super::raising::{word, ToRaise};
use super::suggesting::suggestion_number;
use crate::daemon::Fleet;
use crate::main_ci::{MainChange, MainChanged};
use crate::main_fix::brief_of;

/// An item of a red main starts with this: `red:<repository>:<commit>`.
pub(super) const RED: &str = "red:";

const TURN_UNKNOWN: &str = "fleet.operator_turn_unknown";
const NOT_OFFERED: &str = "fleet.operator_answer_not_offered";
const NOT_DONE: &str = "fleet.operator_turn_not_done";
const TURN: &str = "turn:";

/// What a pick did, for the answer that carries it back.
#[derive(Debug, Default)]
pub(super) struct Done {
    pub(super) open: Option<WaitingAct>,
    pub(super) started: Option<String>,
    pub(super) filed: Option<String>,
    pub(super) handed_to: Option<String>,
}

/// Why a pick was not done.
pub(super) enum Fault {
    /// The answer is not one this item offers.
    NotOffered(String),
    /// It was offered and could not be carried out.
    NotDone(String),
}

/// An item as it is acted on: whose it is, the item, and what the Operator picked for it.
pub(super) struct Doing<'a> {
    pub(super) session: &'a str,
    pub(super) item: &'a WaitingItem,
    pub(super) pick: &'a str,
    /// The words of the rule that does it, where a rule does and he pressed nothing.
    pub(super) rule: Option<&'a str>,
}

/// The rule's pick for `what`, as the store keeps it: a go-ahead to file keeps its Then.
pub(super) fn pick_word(what: &str, then: Option<OperatorThen>, pick: &str) -> String {
    if what == "red_main" {
        if hands_red(pick) { HAND_RED } else { START_FIX }.to_string()
    } else if what.ends_with(":file") {
        word(&then.unwrap_or(OperatorThen::JustFile))
    } else {
        "do_it".to_string()
    }
}

/// A red main's rule picks: a Job started on it, or the live Job near it handed it.
const START_FIX: &str = "start_fix";
const HAND_RED: &str = "hand_to_job";
/// What a red's option to hand it starts with, then the Job's title.
const HAND_TO: &str = "Hand to ";

/// Whether `pick`, an option's label or a rule's word, hands the red to a live Job.
fn hands_red(pick: &str) -> bool {
    pick == HAND_RED || pick.starts_with(HAND_TO)
}

/// The Then a rule's pick names, where it names one.
fn then_of_pick(pick: &str) -> Option<OperatorThen> {
    [OperatorThen::JustFile, OperatorThen::FileAndStart, OperatorThen::FileAndHand].into_iter().find(|then| word(then) == pick)
}

/// A rule in Fleet's words.
pub(super) fn rule_said(what: &str, pick: &str) -> String {
    if what == "red_main" {
        return if pick == HAND_RED { "Hand a red main to the live Job near it" } else { "Start a fix when main goes red" }.to_string();
    }
    if what == "start:existing_job" {
        return "Hand a Job that repeats a live Job's work to that Job".to_string();
    }
    if what == super::answering::QUESTION {
        return "Answer questions you are sure of".to_string();
    }
    let (arrived, way) = what.split_once(':').unwrap_or((what, ""));
    let named = super::Arrived::from_word(arrived).map_or(arrived.to_string(), |arrived| arrived.named().to_string());
    match (way, then_of_pick(pick)) {
        ("file", Some(OperatorThen::FileAndStart)) => format!("File {named} as an issue and start a Job on it"),
        ("file", Some(OperatorThen::FileAndHand)) => format!("File {named} as an issue and hand it to a live Job on the same files"),
        ("file", _) => format!("File {named} as an issue"),
        ("new_job", _) => format!("Start a Job on {named}"),
        ("existing_job", _) => format!("Hand {named} to a live Job near it"),
        ("link", _) => format!("Link {named} to an open issue"),
        ("close", _) => "Close an open issue a merged pull request finished".to_string(),
        ("group", _) => format!("Group {named} with an open issue they repeat or belong under"),
        _ => format!("Do what I would for {named}"),
    }
}

fn turn_number(id: &str) -> Option<i64> {
    id.strip_prefix(TURN).and_then(|n| n.parse().ok())
}

/// A pick's option on `item`, as its index.
fn choice_of(item: &WaitingItem, pick: &str) -> Option<u32> {
    let label = super::raising::pick_of(pick, item);
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
    fn turn_unknown(&self, id: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(TURN_UNKNOWN, format!("{id} is not an item waiting in Your turn"), self.run_id()))
    }

    async fn turn_open(&self, id: &str) -> Result<KeptTurn, Refusal> {
        let n = turn_number(id).ok_or_else(|| self.turn_unknown(id))?;
        let row = self.store().lock().await.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?;
        row.filter(|row| row.settled_at.is_none()).ok_or_else(|| self.turn_unknown(id))
    }

    /// What `what` is for an item, as a rule names it: a go-ahead's kind of arrival and way, or a red main.
    pub(super) async fn rule_what(&self, item: &str) -> Option<String> {
        if item.starts_with(RED) {
            return Some("red_main".to_string());
        }
        let n = suggestion_number(item)?;
        let kept = self.store().lock().await.operator_suggestion(n).ok().flatten()?;
        Some(format!("{}:{}", kept.arrived, kept.way))
    }

    /// Every rule kept, with its row's number.
    pub(super) async fn rules_kept(&self) -> Vec<(i64, OperatorRule)> {
        let kept = self.store().lock().await.operator_orders().unwrap_or_default();
        kept.iter().filter_map(|one| Some((one.n, OperatorRule::read(&one.form)?))).collect()
    }

    /// Do `doing`'s pick. **Nothing is done that the item does not offer.**
    pub(super) async fn turn_performed(self: &Arc<Self>, doing: &Doing<'_>, then: Option<OperatorThen>) -> Result<Done, Fault> {
        let id = doing.item.id.as_str();
        let offered = self.thens_offered(doing.item).await;
        let hand_to = match then {
            Some(then) => match offered.iter().find(|offer| offer.then == then) {
                Some(offer) => offer.hand_to.as_ref().map(|job| job.session_id.as_str().to_string()),
                None => return Err(Fault::NotOffered(format!("{} is not a Then this item offers", word(&then)))),
            },
            None => None,
        };
        if let Some(rest) = id.strip_prefix(RED) {
            let (manifest, commit) = rest.split_once(':').ok_or_else(|| Fault::NotDone("the red main's id does not read".to_string()))?;
            if hands_red(doing.pick) {
                let (served, main) = self.red_still(manifest, commit).await.ok_or_else(|| Fault::NotDone("main is no longer red there, or a Job is on it already".to_string()))?;
                let (job, _) = self.red_near(&served, &main).await.ok_or_else(|| Fault::NotDone("no live Job is near that red now".to_string()))?;
                self.operator_gave_red(manifest, commit, &job).await.map_err(Fault::NotDone)?;
                self.red_handed_logged(manifest, commit, &job, doing.rule).await;
                return Ok(Done { handed_to: Some(job), ..Done::default() });
            }
            let job = self.operator_started_fix(manifest, commit).await.map_err(Fault::NotDone)?;
            return Ok(Done { started: Some(job), ..Done::default() });
        }
        if let Some(n) = suggestion_number(id) {
            return self.suggestion_settled(doing.session, n, true, then.map(|then| (then, hand_to)), doing.rule).await.map_err(|refused| Fault::NotDone(refused.error().message.clone()));
        }
        if let Some(choice) = choice_of(doing.item, doing.pick) {
            let said = AnswerWaiting { session_id: SessionId::carried(doing.session), item_id: id.to_string(), choice: Some(choice), text: None, mode: None, fixed: None };
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

    /// Items just arrived, each done by a rule where one covers it, while the Operator is on. Answers
    /// whether any was, and the rest, which go to review: **a rule that cannot be carried out leaves its
    /// item to the person**.
    pub(super) async fn rules_applied(self: &Arc<Self>, offered: Vec<ToRaise>) -> (bool, Vec<ToRaise>) {
        let rules = if self.operator_on() { self.rules_kept().await } else { Vec::new() };
        if rules.is_empty() {
            return (false, offered);
        }
        let (mut ruled, mut rest) = (false, Vec::new());
        for one in offered {
            let what = self.rule_what(&one.item.id).await;
            // A rule for this repository wins over one for every repository.
            let found = what.as_ref().and_then(|what| {
                let of = |scoped: bool| rules.iter().find_map(|(_, OperatorRule::PickRule { manifest_id, what: kept, pick })| (kept == what && manifest_id.is_some() == scoped && (!scoped || manifest_id.as_deref() == Some(one.manifest_id.as_str()))).then(|| pick.clone()));
                of(true).or_else(|| of(false))
            });
            let (Some(what), Some(pick)) = (what, found) else {
                rest.push(one);
                continue;
            };
            let then = then_of_pick(&pick);
            let said = rule_said(&what, &pick);
            let doing = Doing { session: &one.session, item: &one.item, pick: &pick, rule: Some(&said) };
            match self.turn_performed(&doing, then).await {
                Ok(_) => {
                    let at = self.now();
                    let row = KeptTurn {
                        session: one.session.clone(),
                        item: one.item.id.clone(),
                        manifest_id: one.manifest_id.clone(),
                        job: one.job.clone(),
                        go_ahead: true,
                        body: ipc::operator_page::item_kept(&one.item),
                        pick: Some(said.clone()),
                        raised_at: at.as_str().to_string(),
                        answer: Some("rule".to_string()),
                        settled_at: Some(at.as_str().to_string()),
                        ..KeptTurn::default()
                    };
                    if let Err(why) = self.store().lock().await.add_operator_turn(&row) {
                        self.operator_warned("an item a rule did was not kept", why);
                    }
                    self.operator_journaled(&format!("By a rule, {said}: {}", one.item.text)).await;
                    ruled = true;
                }
                Err(Fault::NotOffered(why) | Fault::NotDone(why)) => {
                    self.operator_warned("a rule could not be carried out, so its item is left to the person", why);
                    rest.push(one);
                    continue;
                }
            }
            self.raise_released(&one);
        }
        (ruled, rest)
    }

    /// `operator_answer`: one press on an item of Your turn.
    pub(crate) async fn operator_answered(self: &Arc<Self>, press: OperatorAnswerPress) -> Result<OperatorAnswered, Refusal> {
        if !self.operator_on() {
            return Err(self.hosted_refusal(OPERATOR_OFF, "the Operator is off"));
        }
        let row = self.turn_open(&press.id).await?;
        let item = item_read(&row.body).ok_or_else(|| self.ledger_unnamed("an item that reads"))?;
        let not_offered = |why: &str| self.hosted_refusal(NOT_OFFERED, why);
        // The option he named, where it is one the item offers: absent takes the Operator's pick.
        let named = match press.option.as_deref().map(str::trim).filter(|label| !label.is_empty()) {
            Some(label) if item.options.iter().any(|option| option.label == label) => Some(label.to_string()),
            Some(label) => return Err(not_offered(&format!("{label} is not an option this item offers"))),
            None => None,
        };
        // On the Operator's own go-ahead the second option is the refusal: Keep both, or Skip.
        let refuses = named.is_some() && suggestion_number(&row.item).is_some() && item.options.get(1).map(|option| option.label.as_str()) == named.as_deref();
        if refuses && !matches!(press.answer, OperatorAnswer::Yes | OperatorAnswer::No) {
            return Err(not_offered("an Always is for doing the act, not for leaving it"));
        }
        let answer = if refuses { OperatorAnswer::No } else { press.answer };
        let what = self.rule_what(&row.item).await.or_else(|| super::answering::what_of(&item).map(str::to_string));
        let keeps = row.go_ahead && what.is_some();
        let answer_word = match answer {
            OperatorAnswer::Yes => "yes",
            OperatorAnswer::AlwaysHere => "always_here",
            OperatorAnswer::Always => "always",
            OperatorAnswer::No => "no",
        };
        let mut done = Done::default();
        if answer == OperatorAnswer::No {
            if let Some(n) = suggestion_number(&row.item) {
                self.suggestion_settled(&row.session, n, false, None, None).await?;
            }
            self.settled_by_press(&row, answer_word, &item, &done).await;
        } else {
            if answer != OperatorAnswer::Yes && !keeps {
                return Err(not_offered("Always here and Always are offered only on what Fleet can do by itself"));
            }
            if answer == OperatorAnswer::AlwaysHere && row.manifest_id.is_empty() {
                return Err(not_offered("this item is in no repository, so Always here has none to hold in"));
            }
            let then = press.then.or_else(|| then_of_pick(row.then_pick.as_deref().unwrap_or_default())).or_else(|| {
                // Where it offers Thens and he and the Operator named none: just file it.
                what.as_deref().filter(|what| what.ends_with(":file")).map(|_| OperatorThen::JustFile)
            });
            let doing = Doing { session: &row.session, item: &item, pick: named.as_deref().or(row.pick.as_deref()).unwrap_or_default(), rule: None };
            done = self.turn_performed(&doing, then).await.map_err(|fault| match fault {
                Fault::NotOffered(why) => self.hosted_refusal(NOT_OFFERED, &why),
                Fault::NotDone(why) => self.hosted_refusal(NOT_DONE, &why),
            })?;
            if let (true, Some(what)) = (answer != OperatorAnswer::Yes, &what) {
                let manifest_id = (answer == OperatorAnswer::AlwaysHere).then(|| row.manifest_id.clone());
                self.rule_kept(OperatorRule::PickRule { manifest_id, what: what.clone(), pick: pick_word(what, then, doing.pick) }).await?;
            }
            if done.open.is_some() {
                if let Err(why) = self.store().lock().await.answer_operator_turn(row.n, answer_word) {
                    self.operator_warned("a press on an item of Your turn was not kept", why);
                }
                self.operator_journaled(&format!("Answered {answer_word}: {}: {}. Left to him to open.", row.job.as_deref().unwrap_or("My suggestion"), item.text)).await;
            } else {
                self.settled_by_press(&row, answer_word, &item, &done).await;
            }
        }
        self.operator_page_changed();
        let page = self.operator_page_wire().await?;
        Ok(OperatorAnswered { open: done.open, started: done.started.as_deref().map(SessionId::carried), filed: done.filed, handed_to: done.handed_to.as_deref().map(SessionId::carried), page })
    }

    /// Item `row` settled by his press, and the journal says what he did.
    async fn settled_by_press(&self, row: &KeptTurn, answer: &str, item: &WaitingItem, done: &Done) {
        let at = self.now();
        if let Err(why) = self.store().lock().await.settle_operator_turn(row.n, at.as_str(), Some(answer)) {
            self.operator_warned("a press on an item of Your turn was not kept", why);
        }
        let mut entry = format!("Answered {answer}: {}: {}", row.job.as_deref().unwrap_or("My suggestion"), item.text);
        if let Some(filed) = &done.filed {
            entry.push_str(&format!(". Filed {filed}"));
        }
        if done.started.is_some() {
            entry.push_str(". Started a Job");
        }
        self.operator_journaled(&entry).await;
    }

    /// Keep `rule`, in place of one for the same thing in the same place.
    async fn rule_kept(&self, rule: OperatorRule) -> Result<(), Refusal> {
        let OperatorRule::PickRule { manifest_id, what, pick } = &rule;
        let at = self.now();
        let mut store = self.store().lock().await;
        for (n, kept) in self.rules_of(&store) {
            let OperatorRule::PickRule { manifest_id: there, what: kept_what, .. } = &kept;
            if kept_what == what && there == manifest_id {
                store.remove_operator_order(n, at.as_str()).map_err(|why| self.ledger_fault(why))?;
            }
        }
        let one = KeptOrder { n: 0, form: rule.kept(), said: rule_said(what, pick), until: None, at: at.as_str().to_string() };
        store.add_operator_order(&one).map(|_| ()).map_err(|why| self.ledger_fault(why))
    }

    fn rules_of(&self, store: &store::Store) -> Vec<(i64, OperatorRule)> {
        store.operator_orders().unwrap_or_default().iter().filter_map(|one| Some((one.n, OperatorRule::read(&one.form)?))).collect()
    }

    /// `operator_turn_order`: the open items he named go first, in his order.
    pub(crate) async fn operator_turn_ordered(self: &Arc<Self>, order: OperatorTurnOrder) -> Result<OperatorPage, Refusal> {
        let ids: Vec<i64> = order.ids.iter().filter_map(|id| turn_number(id)).collect();
        self.store().lock().await.order_operator_turn(&ids).map_err(|why| self.ledger_fault(why))?;
        self.operator_page_changed();
        self.operator_page_wire().await
    }

    /// `operator_later`: the item goes last until the local day ends.
    pub(crate) async fn operator_turn_later(self: &Arc<Self>, later: OperatorLater) -> Result<OperatorPage, Refusal> {
        let row = self.turn_open(&later.id).await?;
        let until = self.next_day_began(&self.now()).await;
        self.store().lock().await.later_operator_turn(row.n, &until).map_err(|why| self.ledger_fault(why))?;
        self.operator_page_changed();
        self.operator_page_wire().await
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

    /// Queued once for each red commit, however many times it is offered, and done by a rule where one covers it.
    pub(crate) async fn red_item_offered(self: &Arc<Self>, main: &store::MainCi) {
        if !self.operator_on() || main.held() || self.taking(main).await.is_some() {
            return;
        }
        let Some(served) = self.repositories().served().into_iter().find(|served| served.root() == main.repository) else { return };
        let manifest = served.manifest().id().as_str().to_string();
        let commit = main.red_commit();
        let facts = brief_of(main).lines().next().unwrap_or_default().to_string();
        let id = format!("{RED}{manifest}:{commit}");
        // Asked once for each red commit, so the forge is not asked for the Job near it again where it stands.
        if self.store().lock().await.operator_turn_present().unwrap_or_default().iter().any(|row| row.item == id) {
            return;
        }
        let mut options = vec![WaitingOption { label: "Start a fix".to_string(), description: None }];
        if let Some((_, title)) = self.red_near(&served, main).await {
            options.push(WaitingOption { label: format!("{HAND_TO}{title}"), description: None });
        }
        let item = WaitingItem {
            approves: false,
            id,
            text: format!("Main is red at {}. {facts}", commit.get(..7).unwrap_or(commit)).trim().to_string(),
            since: Instant::carried(self.now().as_str()),
            source: WaitingSource::OperatorSuggestion,
            act: None,
            options,
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        };
        let one = ToRaise { session: String::new(), job: None, manifest_id: manifest, go_ahead: true, item, thens: Vec::new() };
        let raising = &self.operator_seat().raising;
        let _turn = raising.turn.lock().await;
        let standing = self.store().lock().await.operator_turn_present().unwrap_or_default().iter().any(|row| row.item == one.item.id);
        if standing || !self.raise_marked(&one) {
            return;
        }
        let (ruled, rest) = self.rules_applied(vec![one]).await;
        if ruled {
            self.operator_page_changed();
        }
        self.review_queued(rest);
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
