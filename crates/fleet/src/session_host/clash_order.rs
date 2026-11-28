//! Which of two Jobs that write the same files lands first. **The two settle it between them** with `settle_clash`;
//! where they cannot, `raise_conflict` naming files puts the question on the raising Job's Needs you, Operator or no
//! Operator, and his choice is the order. Kept per pair, read on both Jobs' records. `docs/concepts/fleet.md`,
//! *Claims a Job holds*.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ClashFirst, ClashStanding, Instant, JobClash, Notice, SettleClash, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::{job_clash_standing as standing, KeptClash, KeptJobAsk};

use super::address_of;
use super::job_asks::title_or_address;
use super::waiting::SESSION_WAITING_EMPTY;
use crate::clashing::listed;
use crate::daemon::Fleet;

const CLASH_REFUSED: &str = "fleet.clash_refused";

/// What a clash's item id begins with, then the two addresses sorted.
pub(crate) const CLASH: &str = "clash:";

/// The pair's id: [`CLASH`], then the two addresses sorted.
fn clash_id(clash: &KeptClash) -> String {
    let mut addresses = clash.pair.clone().map(|one| address_of(&one));
    addresses.sort();
    format!("{CLASH}{}:{}", addresses[0], addresses[1])
}

/// The other of the pair, and its title, from `session`'s side.
fn other_of<'a>(clash: &'a KeptClash, session: &str) -> (&'a str, &'a str) {
    match clash.pair[0] == session {
        true => (&clash.pair[1], &clash.titles[1]),
        false => (&clash.pair[0], &clash.titles[0]),
    }
}

fn title_of<'a>(clash: &'a KeptClash, session: &str) -> &'a str {
    if clash.pair[0] == session { &clash.titles[0] } else { &clash.titles[1] }
}

fn named(clash: &KeptClash, session: &str) -> String {
    title_or_address(title_of(clash, session), session)
}

/// The clash as both Jobs' records carry it.
pub(crate) fn clash_wire(clash: &KeptClash) -> JobClash {
    let (other, other_title) = other_of(clash, &clash.recorded_by);
    let job = |session: &str, title: &str| ipc::OperatorAskJob { session_id: ipc::SessionId::carried(session), address: address_of(session), title: title.trim().to_string() };
    JobClash {
        id: clash_id(clash),
        jobs: vec![job(&clash.recorded_by, title_of(clash, &clash.recorded_by)), job(other, other_title)],
        files: clash.files.clone(),
        standing: match clash.standing.as_str() {
            standing::AGREED => ClashStanding::Agreed,
            standing::RULED => ClashStanding::Ruled,
            _ => ClashStanding::Waiting,
        },
        first: clash.first.as_deref().map(ipc::SessionId::carried),
        why: clash.why.clone(),
        at: clash.at.clone(),
    }
}

/// Whether the pair's clash stands for `ask`, a conflict naming files, on the Needs you list: one ask, one item. Its
/// clash was kept when it was raised or since, so it holds after his choice too; a conflict of wire types alone
/// keeps no clash and stands as `job_conflict`.
pub(crate) fn ordered_instead(clashes: &[KeptClash], ask: &KeptJobAsk) -> bool {
    let Some(other) = ask.other_session_id.as_deref().filter(|_| ask.kind == store::job_ask_kind::CONFLICT && !ask.files.is_empty()) else { return false };
    let pair = KeptClash::pair_of(&ask.session_id, other);
    clashes.iter().any(|clash| clash.pair == pair && clash.at >= ask.at)
}

/// The question on the raising Job's Needs you while its clash waits on him.
pub(crate) fn clash_items(clashes: &[KeptClash], session: &str) -> Vec<WaitingItem> {
    clashes.iter().filter(|clash| clash.standing == standing::WAITING && clash.recorded_by == session).map(|clash| clash_item(clash, session)).collect()
}

fn clash_item(clash: &KeptClash, session: &str) -> WaitingItem {
    let (other, _) = other_of(clash, session);
    let (mine, theirs) = (named(clash, session), named(clash, other));
    let id = clash_id(clash);
    let option = |label: &str, waits: &str| WaitingOption { label: label.to_string(), description: Some(format!("\"{waits}\" waits until it lands")), picture: None };
    WaitingItem {
        order: 0,
        why: String::new(),
        approves: false,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone(), on: None }),
        text: format!("Which lands first, \"{mine}\" or \"{theirs}\"?"),
        id,
        since: Instant::carried(&clash.at),
        source: WaitingSource::ClashOrder,
        options: vec![option(&mine, &theirs), option(&theirs, &mine)],
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: None,
        goal_met: None,
    }
}

/// What his choice tells one of the two, `to`.
fn ruling_told(clash: &KeptClash, to: &str, first: &str) -> String {
    let files = listed(&clash.files);
    let (other, _) = other_of(clash, to);
    let peer = format!("\"{}\" ({})", named(clash, other), address_of(other));
    match to == first {
        true => format!("He chose that you land first on {files}. {peer} waits until you land."),
        false => format!("He chose that {peer} lands first on {files}. Wait until it lands before you land yours."),
    }
}

/// The lines a call sent, each trimmed, with the blank ones left out, and none twice.
fn paths(sent: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for one in sent.into_iter().map(|line| line.trim().trim_end_matches('/').to_string()).filter(|line| !line.is_empty()) {
        if !kept.contains(&one) {
            kept.push(one);
        }
    }
    kept
}

/// `before`'s files, then those of `more` it does not hold.
fn joined(before: Option<&KeptClash>, more: &[String]) -> Vec<String> {
    let mut files = before.map(|one| one.files.clone()).unwrap_or_default();
    files.extend(more.iter().filter(|one| !files.contains(one)).cloned().collect::<Vec<_>>());
    files
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
    /// `settle_clash`: the order two Jobs agreed, kept for the pair in place of what it held, its files joined.
    pub(crate) async fn clash_settled(self: &Arc<Self>, caller: Option<api::Caller>, settle: SettleClash) -> Result<JobClash, Refusal> {
        let (session, title, _) = self.asking_job(caller, settle.session_id.as_ref(), CLASH_REFUSED).await?;
        let refuse = |why: &str| self.hosted_refusal(CLASH_REFUSED, why);
        let with = settle.with.trim().to_string();
        let (other, other_title) = self.other_job_named(&session, &with, CLASH_REFUSED).await?;
        let files = paths(settle.files);
        if files.is_empty() {
            return Err(refuse("`files` is empty. Name the paths the two of you agreed on"));
        }
        let why = settle.why.trim();
        if why.is_empty() {
            return Err(refuse("`why` is empty. Say in one sentence why this order"));
        }
        let first = match settle.first {
            ClashFirst::Me => Some(session.clone()),
            ClashFirst::Them => Some(other.clone()),
            ClashFirst::BothGoOn => None,
        };
        let clash = {
            let mut store = self.store().lock().await;
            let before = store.job_clash(&session, &other).map_err(|why| self.ledger_fault(why))?;
            let clash = self.clash_kept(&session, &title, &other, &other_title, joined(before.as_ref(), &files), standing::AGREED, first, why);
            store.put_job_clash(&clash).map_err(|why| self.ledger_fault(why))?;
            clash
        };
        self.clash_published(&clash).await;
        Ok(clash_wire(&clash))
    }

    /// A `raise_conflict` naming files: the pair's clash waits on him, its files joined.
    pub(super) async fn clash_raised(&self, ask: &KeptJobAsk) -> Result<(), store::WriteError> {
        let Some(other) = ask.other_session_id.as_deref() else { return Ok(()) };
        let clash = {
            let mut store = self.store().lock().await;
            let before = store.job_clash(&ask.session_id, other)?;
            let files = joined(before.as_ref(), &paths(ask.files.clone()));
            let clash = self.clash_kept(&ask.session_id, &ask.job_title, other, ask.other_title.as_deref().unwrap_or_default(), files, standing::WAITING, None, &ask.breaks);
            store.put_job_clash(&clash)?;
            clash
        };
        self.clash_published(&clash).await;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn clash_kept(&self, by: &str, title: &str, other: &str, other_title: &str, files: Vec<String>, how: &str, first: Option<String>, why: &str) -> KeptClash {
        let pair = KeptClash::pair_of(by, other);
        let titles = match pair[0] == by {
            true => [title.to_string(), other_title.to_string()],
            false => [other_title.to_string(), title.to_string()],
        };
        KeptClash { pair, recorded_by: by.to_string(), titles, files, standing: how.to_string(), first, why: why.to_string(), at: self.now().as_str().to_string() }
    }

    /// Both Jobs' records carry the clash, so both are published.
    async fn clash_published(&self, clash: &KeptClash) {
        for session in &clash.pair {
            let _ = self.published_hosted(session).await;
        }
    }

    /// `answer_waiting` on a `clash_order` item: his choice is the pair's order, and both Jobs are told it.
    pub(crate) async fn clash_order_chosen(self: &Arc<Self>, session: &str, item: &str, choice: Option<u32>) -> Result<ipc::SessionRecord, Refusal> {
        let clash = {
            let store = self.store().lock().await;
            let held = store.job_clashes_of(session).map_err(|why| self.ledger_fault(why))?;
            // The item must be this Job's and still wait: an id from another Job's list settles nothing here.
            held.into_iter().find(|one| clash_id(one) == item && one.standing == standing::WAITING && one.recorded_by == session).ok_or_else(|| self.settled_already())?
        };
        let other = other_of(&clash, session).0.to_string();
        let first = match choice {
            Some(0) => session.to_string(),
            Some(1) => other.clone(),
            _ => return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "that is answered by choosing one of its buttons")),
        };
        let ruled = KeptClash { standing: standing::RULED.to_string(), first: Some(first.clone()), at: self.now().as_str().to_string(), ..clash };
        self.store().lock().await.put_job_clash(&ruled).map_err(|why| self.ledger_fault(why))?;
        for job in &ruled.pair {
            // A Job that has ended, or that muted the notice, is told nothing: his choice stands.
            let _ = self.told_by_fleet(job, &ruling_told(&ruled, job, &first), Notice::Claims).await;
        }
        let _ = self.published_hosted(&other).await;
        self.published_hosted(session).await
    }
}
