//! What a Job needs first: every open ask gets its place and one sentence on why it stands there, so Bridge can
//! show one card at a time. `docs/concepts/session/waiting-on-the-person.md`, *What he needs first*;
//! `crates/ipc/operations/order_waiting.toml` is the contract of the act.
//!
//! **The Job's own order wins.** The asks it named with `order_waiting` come first, with its whys; the rest follow
//! in the fallback order with Fleet's. An id it named whose ask is settled is passed over.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{OrderWaiting, PartKind, SessionRecord, WaitingItem, WaitingSource, WAITING_ORDER_BLANK, WAITING_ORDER_UNKNOWN};
use store::KeptWaitingPlace;

use crate::daemon::Fleet;

/// **Fleet's whys, in one place**, so they are read against the armada-voice skill together: facts, no dashes.
pub(crate) const WHY_BLOCKS: &str = "The work waits on your answer";
pub(crate) const WHY_PAGE: &str = "A page to read";
pub(crate) const WHY_TRY: &str = "The work is ready to try";
pub(crate) const WHY_DONE: &str = "The work is done";
pub(crate) const WHY_PROTOTYPE_NEEDS_BUILD: &str = "The prototype needs this build";
pub(crate) const WHY_WALK_NEEDS_BUILD: &str = "The walk needs this build";

/// The fallback's bands, first to last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Band {
    /// A question, a vote, an approve: the work waits on it.
    Blocks,
    /// A page to read or approve.
    Page,
    /// A build to try.
    Try,
    /// The Job's done.
    Done,
}

/// The band of every source, exhaustive, so a new one is placed when it is added.
fn band_of(source: WaitingSource) -> Band {
    match source {
        WaitingSource::Document => Band::Page,
        WaitingSource::ReadyToTry => Band::Try,
        WaitingSource::Done => Band::Done,
        WaitingSource::Agent
        | WaitingSource::AskCard
        | WaitingSource::Walk
        | WaitingSource::Permission
        | WaitingSource::WindowAsk
        | WaitingSource::Draft
        | WaitingSource::Handoff
        | WaitingSource::Sketch
        | WaitingSource::FirstMessage
        | WaitingSource::Replied
        | WaitingSource::Plan
        | WaitingSource::SlotChoice
        | WaitingSource::Delivery
        | WaitingSource::WordsAddress
        | WaitingSource::FleetCall
        | WaitingSource::TaskDone
        | WaitingSource::WatchNotOurs
        | WaitingSource::HeldChild
        | WaitingSource::PreviewOffer
        | WaitingSource::RuleAct
        | WaitingSource::OperatorSuggestion
        | WaitingSource::BrowserAsk
        | WaitingSource::OperatorUndone
        | WaitingSource::Gate
        | WaitingSource::JobConflict
        | WaitingSource::ClashOrder
        | WaitingSource::JobHeld
        | WaitingSource::GoalMet => Band::Blocks,
    }
}

fn why_of(band: Band) -> &'static str {
    match band {
        Band::Blocks => WHY_BLOCKS,
        Band::Page => WHY_PAGE,
        Band::Try => WHY_TRY,
        Band::Done => WHY_DONE,
    }
}

/// What an ask needs of the build he would refresh to: a prototype drawn by the app, or a walk of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NeedsBuild {
    Prototype,
    Walk,
}

/// The fallback order of `items`, each with Fleet's why: bands first to last, the list's own order within one.
/// **A build to refresh comes first where a prototype or a walk on the list needs it**, the owner's own example.
fn fallback(items: Vec<WaitingItem>, needs_build: &dyn Fn(&WaitingItem) -> Option<NeedsBuild>) -> Vec<WaitingItem> {
    // Read only where there is a build to try: it decodes an artifact's parts.
    let to_try = items.iter().any(|one| band_of(one.source) == Band::Try);
    let needed = to_try.then(|| items.iter().filter_map(needs_build).min_by_key(|needs| *needs != NeedsBuild::Prototype)).flatten();
    let mut banded: Vec<(Band, WaitingItem)> = items.into_iter().map(|one| (band_of(one.source), one)).collect();
    banded.sort_by_key(|(band, _)| (!(needed.is_some() && *band == Band::Try), *band));
    banded
        .into_iter()
        .map(|(band, mut one)| {
            one.why = match (band, needed) {
                (Band::Try, Some(NeedsBuild::Prototype)) => WHY_PROTOTYPE_NEEDS_BUILD,
                (Band::Try, Some(NeedsBuild::Walk)) => WHY_WALK_NEEDS_BUILD,
                (band, _) => why_of(band),
            }
            .to_string();
            one
        })
        .collect()
}

/// `items` in the order the Job set, then the rest in the fallback order, each with its place from 1 and its why.
pub(crate) fn placed(
    items: Vec<WaitingItem>,
    said: &[KeptWaitingPlace],
    needs_build: &dyn Fn(&WaitingItem) -> Option<NeedsBuild>,
) -> Vec<WaitingItem> {
    let (mut named, rest): (Vec<WaitingItem>, Vec<WaitingItem>) =
        items.into_iter().partition(|one| said.iter().any(|place| place.item == one.id));
    named.sort_by_key(|one| said.iter().position(|place| place.item == one.id));
    for one in &mut named {
        one.why = said.iter().find(|place| place.item == one.id).map(|place| place.why.clone()).unwrap_or_default();
    }
    named.extend(fallback(rest, needs_build));
    for (at, one) in named.iter_mut().enumerate() {
        one.order = u32::try_from(at + 1).unwrap_or(u32::MAX);
    }
    named
}

/// What `item` needs of the build: a walk, or an asking artifact that holds a prototype part.
pub(crate) fn needs_build_of(item: &WaitingItem, attachments: &[ipc::Attachment]) -> Option<NeedsBuild> {
    match item.source {
        WaitingSource::Walk | WaitingSource::WindowAsk => Some(NeedsBuild::Walk),
        _ => {
            let artifact = item.artifact.as_deref()?;
            let row = attachments.iter().find(|one| super::artifact_parts::is_sketch(one) && one.target == artifact)?;
            super::artifact_parts::stored(&row.detail)
                .iter()
                .any(|part| part.content.kind() == PartKind::Prototype)
                .then_some(NeedsBuild::Prototype)
        }
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
    /// `order_waiting`: refused whole, naming each id, where any is not an open ask of the Job now.
    pub(crate) async fn waiting_ordered(&self, order: OrderWaiting) -> Result<SessionRecord, Refusal> {
        let id = self.waiting_session(api::asking(), order.session_id.as_ref())?;
        let mut kept: Vec<KeptWaitingPlace> = Vec::new();
        for one in &order.items {
            let (item, why) = (one.id.trim(), one.why.trim());
            if item.is_empty() || why.is_empty() {
                return Err(self.hosted_refusal(WAITING_ORDER_BLANK, "every item needs the id of an ask and one sentence on why it stands there"));
            }
            if kept.iter().any(|placed| placed.item == item) {
                return Err(self.hosted_refusal(WAITING_ORDER_BLANK, &format!("{item} is named twice: name each ask once")));
            }
            kept.push(KeptWaitingPlace { item: item.to_string(), why: why.to_string() });
        }
        let open = self.waiting_now(&id).await?;
        let unknown: Vec<&str> = kept.iter().map(|one| one.item.as_str()).filter(|item| !open.iter().any(|one| one.id == *item)).collect();
        if !unknown.is_empty() {
            let asks: Vec<&str> = open.iter().map(|one| one.id.as_str()).collect();
            let asks = if asks.is_empty() { String::from("none") } else { asks.join(", ") };
            return Err(self.hosted_refusal(WAITING_ORDER_UNKNOWN, &format!("{} is not an open ask of this job. Its open asks are {asks}", unknown.join(", "))));
        }
        let record = {
            let mut store = self.store().lock().await;
            store.order_waiting(&id, &kept).map_err(|why| self.ledger_fault(why))?;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(super::waiting::SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }
}
