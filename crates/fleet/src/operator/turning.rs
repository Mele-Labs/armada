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
use core_model::{ManifestId as Repository, Ulid};
use ipc::operator_page::{item_read, OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorLater, OperatorPage, OperatorRule, OperatorThen, OperatorTurnOrder, RuleScope};
use ipc::{AnswerWaiting, ApprovePlan, BeginSession, Instant, SessionId, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource, WireError};
use store::{KeptOrder, KeptTurn};

use super::held;
use super::orders::OPERATOR_OFF;
use super::raising::{word, ToRaise};
use super::vetting::{Work, APPROVE_PLAN, SEND_IT_BACK, SEND_PLAN_BACK, WALK_FIRST};
use super::suggesting::suggestion_number;
use crate::daemon::Fleet;
use crate::main_ci::{MainChange, MainChanged};
use crate::main_fix::brief_of;
use crate::project_files::NOT_MEMBER;

/// An item of a red main starts with this: `red:<repository>:<commit>`.
pub(super) const RED: &str = "red:";

const TURN_UNKNOWN: &str = "fleet.operator_turn_unknown";
const NOT_OFFERED: &str = "fleet.operator_answer_not_offered";
const NOT_DONE: &str = "fleet.operator_turn_not_done";
const LATER_PAST: &str = "fleet.operator_later_past";

/// How long after his Yes an Always here or an Always still keeps its rule.
const ALWAYS_AFTER_YES_MS: i64 = 60_000;
/// What a held Job's start item's id starts with, then the Job's id.
const HELD_START: &str = "start:";
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
    /// The Operator's read of it, which a send-back carries as its reason.
    pub(super) reason: &'a str,
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

/// An RFC 3339 instant in milliseconds since the epoch, its zone read: `Z`, or `+HH:MM` and `-HH:MM`
/// as a zone Bridge sends from his clock. `None` where it does not read.
fn utc_millis(text: &str) -> Option<i64> {
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

fn turn_number(id: &str) -> Option<i64> {
    id.strip_prefix(TURN).and_then(|n| n.parse().ok())
}

/// A pick's option on `item`, as its index.
fn choice_of(item: &WaitingItem, pick: &str) -> Option<u32> {
    let label = super::raising::pick_of(pick, item);
    item.options.iter().position(|option: &WaitingOption| !label.is_empty() && option.label == label).and_then(|at| u32::try_from(at).ok())
}

/// The pick of the rule that covers `what` for an item in `repository` (empty where it is in none), a
/// repository of the projects `projects`. **A rule for the repository beats one for a project, which beats
/// one for everywhere**, and an item in no repository meets only the last.
pub(crate) fn rule_pick(rules: &[(i64, OperatorRule)], what: &str, repository: &str, projects: &[String]) -> Option<String> {
    let of = |covers: &dyn Fn(&RuleScope) -> bool| rules.iter().find_map(|(_, OperatorRule::PickRule { scope, what: kept, pick })| (kept == what && covers(scope)).then(|| pick.clone()));
    of(&|scope| matches!(scope, RuleScope::Repository { manifest_id } if !repository.is_empty() && manifest_id == repository))
        .or_else(|| of(&|scope| matches!(scope, RuleScope::Project { project } if projects.contains(project))))
        .or_else(|| of(&|scope| matches!(scope, RuleScope::Everywhere)))
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
            return Box::pin(self.suggestion_settled(doing.session, n, true, then.map(|then| (then, hand_to)), doing.rule)).await.map_err(|refused| Fault::NotDone(refused.error().message.clone()));
        }
        // A held Job's start is pressed as its card's Start, with the landing the card picked. Where the
        // card still asks how the work lands, it is opened for him to answer.
        if let (WaitingSource::HeldChild, Some(child)) = (doing.item.source, id.strip_prefix(HELD_START)) {
            let delivery = self.card_picked(child).await;
            if self.session_begun(BeginSession { session_id: SessionId::carried(child), delivery, message: None }).await.is_ok() {
                return Ok(Done { started: Some(child.to_string()), ..Done::default() });
            }
        }
        if let Some(done) = self.work_done(doing).await? {
            return Ok(done);
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

    /// A review's pick on a Job's work, done as its label says: the plan approved, or sent back with the
    /// review's read as the reason; the pull request sent back the same way; the Job's walk opened, or the
    /// Job asked for one where none is shown. `None` where the pick is none of those, which the item's
    /// own act does.
    async fn work_done(self: &Arc<Self>, doing: &Doing<'_>) -> Result<Option<Done>, Fault> {
        let not_done = |refused: Refusal| Fault::NotDone(refused.error().message.clone());
        let told = |text: String| async move { self.told_by_fleet(doing.session, &text, ipc::Notice::Operator).await.map_err(not_done) };
        let work = Work::of(doing.item);
        let plan = (work == Some(Work::Plan)).then(|| doing.item.id.strip_prefix("plan:").and_then(|version| version.parse::<u32>().ok())).flatten();
        let pull = work == Some(Work::PullRequest);
        match (doing.pick, plan) {
            (APPROVE_PLAN, Some(version)) => {
                Arc::clone(self).plan_approved(ApprovePlan { session_id: SessionId::carried(doing.session), version }).await.map_err(not_done)?;
            }
            (SEND_PLAN_BACK, Some(version)) => {
                told(format!("The person sends plan v{version} back, on the Operator's review: {}\nRecord the plan again with that settled.", doing.reason)).await?;
            }
            (SEND_IT_BACK, None) if pull => {
                told(format!("The person sends the pull request back, on the Operator's review: {}\nFix it on the branch and push.", doing.reason)).await?;
            }
            (WALK_FIRST, None) if pull => match self.walk_shown(doing.session).await {
                Some(walk) => return Ok(Some(Done { open: Some(walk), ..Done::default() })),
                None => {
                    told("The person walks this before the pull request: build the walk and show it with show_window.".to_string()).await?;
                }
            },
            _ => return Ok(None),
        }
        Ok(Some(Done::default()))
    }

    /// The walk `session` has waiting on him, as its item opens it.
    async fn walk_shown(&self, session: &str) -> Option<WaitingAct> {
        let store = self.store().lock().await;
        let kept = store.session(session).ok().flatten()?;
        let record = self.ledger_row(&store, &kept).ok()?;
        record.waiting_for.into_iter().filter_map(|item| item.act).find(|act| act.kind == WaitingActKind::Walk)
    }

    /// The ids of the projects that hold `repository`, which a rule kept for a project meets. None for an
    /// item in no repository.
    pub(super) fn rule_projects(&self, repository: &str) -> Vec<String> {
        match repository {
            "" => Vec::new(),
            repository => self.projects_held().holding(&Repository::carried(Ulid::carried(repository))).into_iter().map(|project| project.id().as_str().to_string()).collect(),
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
            let held = self.rule_projects(&one.manifest_id);
            let found = what.as_ref().and_then(|what| rule_pick(&rules, what, &one.manifest_id, &held));
            let (Some(what), Some(pick)) = (what, found) else {
                rest.push(one);
                continue;
            };
            let then = then_of_pick(&pick);
            let said = rule_said(&what, &pick);
            let doing = Doing { session: &one.session, item: &one.item, pick: &pick, reason: "", rule: Some(&said) };
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
        let row = match self.turn_open(&press.id).await {
            Ok(row) => row,
            Err(refused) if matches!(press.answer, OperatorAnswer::AlwaysHere | OperatorAnswer::Always) => return Box::pin(self.always_after_yes(&press, refused)).await,
            Err(refused) => return Err(refused),
        };
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
        // Yes on a pick of Later is Later: what it waits on is his to do away from the page.
        if answer == OperatorAnswer::Yes && named.is_none() && row.pick.as_deref() == Some(super::raising::LATER) {
            let until = self.next_day_began(&self.now()).await;
            self.turn_put_later(row.n, &until).await?;
            let page = self.operator_page_wire().await?;
            return Ok(OperatorAnswered { open: None, started: None, filed: None, handed_to: None, page });
        }
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
            let scope = self.rule_scope(&row, answer, press.project.as_deref()).await?;
            let then = then_pressed(&press, &row, what.as_deref());
            let doing = Doing { session: &row.session, item: &item, pick: named.as_deref().or(row.pick.as_deref()).unwrap_or_default(), reason: row.read_text.as_deref().unwrap_or_default(), rule: None };
            done = self.turn_performed(&doing, then).await.map_err(|fault| match fault {
                Fault::NotOffered(why) => self.hosted_refusal(NOT_OFFERED, &why),
                Fault::NotDone(why) => self.hosted_refusal(NOT_DONE, &why),
            })?;
            if let (true, Some(what)) = (answer != OperatorAnswer::Yes, &what) {
                self.rule_kept(OperatorRule::PickRule { scope, what: what.clone(), pick: pick_word(what, then, doing.pick) }).await?;
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

    /// Where an Always here or an Always on `row` holds: the project the page is filtered to, else the
    /// item's repository; everywhere for an Always.
    async fn rule_scope(&self, row: &KeptTurn, answer: OperatorAnswer, project: Option<&str>) -> Result<RuleScope, Refusal> {
        Ok(match (answer, project) {
            (OperatorAnswer::AlwaysHere, Some(project)) => {
                let held = self.project_repositories(project).await?;
                if !held.iter().any(|one| one.as_str() == row.manifest_id) {
                    return Err(Refusal::Unacceptable(self.project_refusal(NOT_MEMBER, format!("this item's repository is not a member of the project {project}, so Always here cannot hold in it"))));
                }
                RuleScope::Project { project: project.to_string() }
            }
            (OperatorAnswer::AlwaysHere, None) if row.manifest_id.is_empty() => {
                return Err(self.hosted_refusal(NOT_OFFERED, "this item is in no repository, so Always here has none to hold in"));
            }
            (OperatorAnswer::AlwaysHere, None) => RuleScope::Repository { manifest_id: row.manifest_id.clone() },
            _ => RuleScope::Everywhere,
        })
    }

    /// Always here or Always pressed **after his Yes, within a minute: only the rule is kept**, since
    /// the act is done. A row settled any other way, or longer ago, is refused as `refused` says.
    async fn always_after_yes(self: &Arc<Self>, press: &OperatorAnswerPress, refused: Refusal) -> Result<OperatorAnswered, Refusal> {
        let Some(n) = turn_number(&press.id) else { return Err(refused) };
        let row = self.store().lock().await.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?;
        let now = self.now().epoch_millis().unwrap_or(i64::MAX);
        let just = |at: &str| core_model::Timestamp::from_rfc3339(at).epoch_millis().is_some_and(|at| now - at <= ALWAYS_AFTER_YES_MS);
        let Some(row) = row.filter(|row| row.answer.as_deref() == Some("yes") && row.settled_at.as_deref().is_some_and(just)) else {
            return Err(refused);
        };
        let item = item_read(&row.body).ok_or_else(|| self.ledger_unnamed("an item that reads"))?;
        let what = self.rule_what(&row.item).await.or_else(|| super::answering::what_of(&item).map(str::to_string));
        let Some(what) = what.filter(|_| row.go_ahead) else {
            return Err(self.hosted_refusal(NOT_OFFERED, "Always here and Always are offered only on what Fleet can do by itself"));
        };
        let scope = self.rule_scope(&row, press.answer, press.project.as_deref()).await?;
        let then = then_pressed(press, &row, Some(&what));
        let named = press.option.as_deref().map(str::trim).filter(|label| item.options.iter().any(|option| option.label == *label));
        let pick = pick_word(&what, then, named.or(row.pick.as_deref()).unwrap_or_default());
        self.rule_kept(OperatorRule::PickRule { scope, what: what.clone(), pick: pick.clone() }).await?;
        let word = if press.answer == OperatorAnswer::Always { "always" } else { "always here" };
        self.operator_journaled(&format!("Kept {word} after a yes: {}", rule_said(&what, &pick))).await;
        self.operator_page_changed();
        let page = self.operator_page_wire().await?;
        Ok(OperatorAnswered { open: None, started: None, filed: None, handed_to: None, page })
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
        let OperatorRule::PickRule { scope, what, pick } = &rule;
        let at = self.now();
        let mut store = self.store().lock().await;
        for (n, kept) in self.rules_of(&store) {
            let OperatorRule::PickRule { scope: there, what: kept_what, .. } = &kept;
            if kept_what == what && there == scope {
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
        self.operator_page_wire().await
    }

    /// Row `n` last until `until`, and the page told now and again when it runs out.
    async fn turn_put_later(self: &Arc<Self>, n: i64, until: &str) -> Result<(), Refusal> {
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
        let mut options = vec![WaitingOption { label: "Start a fix".to_string(), description: None, picture: None }];
        if let Some((_, title)) = self.red_near(&served, main).await {
            options.push(WaitingOption { label: format!("{HAND_TO}{title}"), description: None, picture: None });
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
        let one = ToRaise { session: String::new(), job: None, manifest_id: manifest, go_ahead: true, item, thens: Vec::new(), vet: None };
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

/// The Then of a press: the one he named, else the Operator's pick, else just file where it files.
fn then_pressed(press: &OperatorAnswerPress, row: &KeptTurn, what: Option<&str>) -> Option<OperatorThen> {
    press.then.or_else(|| then_of_pick(row.then_pick.as_deref().unwrap_or_default())).or_else(|| what.filter(|what| what.ends_with(":file")).map(|_| OperatorThen::JustFile))
}
