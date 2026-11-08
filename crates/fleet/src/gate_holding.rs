//! What a gate does to the agent and to the person. `docs/concepts/session/the-plan.md`, *Gates*.
//!
//! **A gate closing is told to the agent once, in one line**, from the one funnel
//! ([`Fleet::gates_closed`]) every close goes through, and only for a row that was written then.
//! **`all_done` is held while a gate is open**, not refused, and Fleet finishes the Job when the last
//! gate closes. **The person is asked only for a gate nothing in the Job can close**, and only while
//! nothing in it runs.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{AllDone, ApprovePlan, Instant, Notice, SessionId, SessionRecord, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::{KeptAttachment, KeptGate, KeptPlan, Store, CHECKS_GREEN, PLAN_APPROVED};

use crate::daemon::Fleet;

/// The id of the item for the group whose gate is open: `gate:<group id>`.
const ITEM: &str = "gate:";

/// Choice 0 and choice 1 of the item, in this order.
const APPROVE: u32 = 0;
const SKIP: u32 = 1;

/// Whether the Job's plan has a gate still open.
pub(crate) fn gate_open(store: &Store, id: &str) -> bool {
    store.session_plan(id).ok().flatten().is_some_and(|plan| open_in(&plan))
}

fn open_in(plan: &KeptPlan) -> bool {
    plan.groups.iter().any(|group| group.gate.is_some() && group.closed.is_none())
}

/// What a gate waits on, as the item says it, for a gate the person can close.
fn phrase(gate: &str) -> Option<&'static str> {
    Some(match gate {
        "plan_approved" => "your approval of the plan",
        "walk_approved" => "your approval of every walk shown",
        "sketch_approved" => "your approval of a sketch",
        "document_approved" => "your approval of a page",
        CHECKS_GREEN => "Checks at a pushed commit",
        "merged" => "the pull request's merge",
        _ => return None,
    })
}

/// The Needs you item for a held `all_done`: the first open gate, when the person is the one who can
/// close it. **Derived on every read and never kept**, so it goes when the gate closes or the hold clears.
/// A merge waits on its pull request while one is open, and Checks while a run has begun: those close
/// themselves.
pub(crate) fn item_of(plan: &KeptPlan, held: &[KeptAttachment], session: &str, nothing_runs: bool) -> Option<WaitingItem> {
    let hold = plan.held.as_ref().filter(|_| nothing_runs)?;
    let group = plan.groups.iter().find(|group| group.gate.is_some() && group.closed.is_none())?;
    let gate = group.gate.as_deref()?;
    let yours = match gate {
        "merged" => !held.iter().any(|one| one.kind == "pr" && one.state == store::AttachmentState::Standing && !crate::build_record::merged(one)),
        CHECKS_GREEN => group.checks.is_none(),
        _ => true,
    };
    let waits_on = phrase(gate).filter(|_| yours)?;
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
    Some(WaitingItem {
        id: format!("{ITEM}{}", group.id),
        text: format!("\"{}\" waits on {waits_on}", group.title),
        since: Instant::carried(&hold.since),
        source: WaitingSource::Gate,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: session.to_string() }),
        options: vec![option("Approve"), option("Skip this group")],
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
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
    /// **The one funnel for a close.** Each of `closed` is a row written just now, so the agent is told
    /// once for each. Then, where a call is held and no gate is open, the hold is cleared and the done is
    /// run, as the agent's own call would have been; a refusal of it is told to the agent.
    pub(crate) async fn gates_closed(self: &Arc<Self>, id: &str, closed: &[KeptGate]) {
        if closed.is_empty() {
            return;
        }
        let titles: Vec<String> = {
            let store = self.store().lock().await;
            let plan = store.session_plan(id).ok().flatten();
            closed
                .iter()
                .map(|one| {
                    plan.as_ref()
                        .and_then(|plan| plan.groups.iter().find(|group| group.id == one.group_id))
                        .map(|group| group.title.clone())
                        .unwrap_or_else(|| one.group_id.clone())
                })
                .collect()
        };
        for (gate, title) in closed.iter().zip(titles) {
            let text = format!("Gate closed on \"{title}\": {}.", gate.fact.trim_end_matches('.'));
            let _ = self.told_news_by_fleet(id, &text, Notice::GateClosed).await;
        }
        // The hold is cleared as it is read, so it is taken once, and its summary goes with the call.
        let finishing = {
            let mut store = self.store().lock().await;
            match store.session_gate_hold(id).ok().flatten() {
                Some(hold) if !gate_open(&store, id) && store.clear_session_gate_hold(id).unwrap_or(false) => Some(hold.summary),
                _ => None,
            }
        };
        if let Some(summary) = finishing {
            self.held_done_taken(id, summary).await;
        }
    }

    /// Run the `all_done` the agent said while a gate held it, with the summary it gave then.
    async fn held_done_taken(self: &Arc<Self>, id: &str, summary: Option<String>) {
        let done = AllDone { session_id: Some(SessionId::carried(id)), summary };
        if let Err(refusal) = self.say_done(None, done).await {
            let why = refusal.error().message.trim_start_matches("all_done refused. ").to_string();
            let _ = self.told_by_fleet(id, &format!("all_done could not finish: {why}"), Notice::GateHold).await;
        }
    }

    /// The person pressed `Approve` or `Skip this group` on the item for a group's gate.
    pub(crate) async fn gate_chosen(self: &Arc<Self>, id: &str, item: &str, choice: u32) -> Result<SessionRecord, api::Refusal> {
        let group_id = item.strip_prefix(ITEM).unwrap_or_default();
        let plan = self.store().lock().await.session_plan(id).ok().flatten();
        let Some((plan, group)) = plan.and_then(|plan| {
            let group = plan.groups.iter().find(|group| group.id == group_id && group.closed.is_none())?.clone();
            Some((plan, group))
        }) else {
            return Err(self.settled_already());
        };
        let Some(gate) = group.gate.clone() else { return Err(self.settled_already()) };
        if choice == APPROVE && gate == PLAN_APPROVED {
            Arc::clone(self).plan_approved(ApprovePlan { session_id: SessionId::carried(id), version: plan.version }).await?;
            self.gates_kept(id).await;
            return self.published_hosted(id).await;
        }
        let skipped = choice == SKIP;
        let kept = KeptGate {
            group_id: group.id.clone(),
            gate: gate.clone(),
            version: if gate == PLAN_APPROVED { plan.version } else { 0 },
            closed_at: self.timeline().now(),
            fact: if skipped { "You skipped this group" } else { "You approved it in Needs you" }.to_string(),
            source: format!("needs-you:{}", group.id),
            commit: None,
            already_red: Vec::new(),
            skipped,
        };
        let wrote = {
            let mut store = self.store().lock().await;
            let wrote = store.close_session_gate(id, &kept).map_err(|why| self.ledger_fault(why))?;
            if wrote && gate == CHECKS_GREEN {
                let _ = store.clear_session_gate_checks(id);
            }
            wrote
        };
        let _ = self.published_hosted(id).await;
        if wrote {
            self.gates_closed(id, &[kept]).await;
        }
        // Read again: the close may have finished the Job.
        self.published_hosted(id).await
    }
}
