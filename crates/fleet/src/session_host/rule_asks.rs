//! An act a lifecycle rule planned that is the owner's to say first, on its Job's Needs you list.
//! Fleet raises it and nobody else does: the agent never sees it, and answering it tells the agent
//! nothing.
//!
//! **Derived on every read from the kept act**, so a restart keeps it. `Do it` hands the act back to
//! the worker that does acts ([`crate::lifecycles::acting`]), `Skip` records it skipped, and either
//! leaves the item with nothing to derive.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Instant, SessionRecord, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::{rule_act_standing as standing, KeptRuleAct};

use super::waiting::SESSION_WAITING_EMPTY;
use crate::daemon::Fleet;

/// The id of the item for one act: `rule_act:<row id>`.
pub(crate) const RULE_ACT: &str = "rule_act:";

/// Choice 0 carries the act out, or tries a failed one again.
pub(crate) const DO_IT: u32 = 0;
/// Choice 1 records the act skipped.
pub(crate) const SKIP: u32 = 1;

/// The items for the acts waiting on the owner, in the order they were planned.
pub(crate) fn rule_act_items(acts: &[KeptRuleAct]) -> Vec<WaitingItem> {
    acts.iter().filter_map(rule_act_item).collect()
}

fn rule_act_item(act: &KeptRuleAct) -> Option<WaitingItem> {
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
    let (text, first) = match act.state.as_str() {
        standing::ASKED => (act.text.clone(), "Do it"),
        standing::STUCK => (act.said.clone(), "Try again"),
        _ => return None,
    };
    let id = format!("{RULE_ACT}{}", act.id);
    Some(WaitingItem {
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone() }),
        id,
        text,
        since: Instant::carried(&act.raised_at),
        source: WaitingSource::RuleAct,
        options: vec![option(first), option("Skip")],
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    })
}

fn row_of(item: &str) -> Option<i64> {
    item.strip_prefix(RULE_ACT)?.parse().ok()
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
    /// `answer_waiting` on a `rule_act` item. **The act is settled in the store first**, and a second
    /// answer finds it settled: the first one wins and the other is a 409.
    pub(crate) async fn rule_act_chosen(&self, session: &str, item: &str, choice: Option<u32>) -> Result<SessionRecord, Refusal> {
        let go = match choice {
            Some(DO_IT) => true,
            Some(SKIP) => false,
            _ => {
                return Err(self.hosted_refusal(
                    SESSION_WAITING_EMPTY,
                    "an act a rule planned is answered by choosing Do it, or Try again, or Skip",
                ))
            }
        };
        let row = row_of(item).ok_or_else(|| self.settled_already())?;
        let at = self.now().as_str().to_string();
        let moved = {
            let mut store = self.store().lock().await;
            // The row must be this Job's: an id from another Job's list settles nothing here.
            let mine = store.rule_act(row).map_err(|why| self.ledger_fault(why))?.is_some_and(|one| one.session_id == session);
            match (mine, go) {
                (false, _) => false,
                (true, true) => {
                    store.move_rule_act(row, &[standing::ASKED], standing::PLANNED, "", &at).map_err(|why| self.ledger_fault(why))?
                        || store.restart_rule_act(row, standing::PLANNED).map_err(|why| self.ledger_fault(why))?
                }
                (true, false) => store
                    .move_rule_act(row, &[standing::ASKED, standing::STUCK], standing::SKIPPED, "skipped", &at)
                    .map_err(|why| self.ledger_fault(why))?,
            }
        };
        if !moved {
            return Err(self.settled_already());
        }
        if go {
            self.rule_acts().wake();
        }
        self.published_hosted(session).await
    }

    /// Dismissing the item is Skip.
    pub(crate) async fn rule_act_skipped(&self, session: &str, item: &str) -> Result<(), Refusal> {
        self.rule_act_chosen(session, item, Some(SKIP)).await.map(|_| ())
    }
}
