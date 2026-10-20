//! Whether an agent's waiting item repeats the plan Fleet already lists, read by the language model.
//! `docs/spikes/walk-item-classifier.md` has the readers measured.
//!
//! **A recorded plan reaches his list only through `record_plan`**, as Fleet's own "Plan to approve"
//! item, which settles at his approval. An agent's "Approve plan v4" beside it is a second call for
//! the same thing, and the rules have no phrase for it: it is worded a dozen ways. The model reads an
//! item against Fleet's own item text, so the same figure serves whatever the plan is called.
//!
//! **Only while a plan is pending, and only once the model is loaded.** With no plan waiting nothing
//! is embedded, and until the model loads the list is accepted as it was before this reader.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, NotEmbedded, TextModel, Vcs, WorkProduct};
use api::Refusal;

use crate::daemon::Fleet;
use crate::session_host::planning::PLAN_TO_APPROVE;

const SESSION_WAITING_PLAN: &str = "fleet.session_waiting_plan";

/// Whether this reader is in force, and so reads with the model. It is: on the scored items it
/// refused nothing the rules did not and caught the plan item they cannot.
pub(crate) const IN_FORCE: bool = true;

/// The cosine at and above which an item reads as the plan's own: the lowest at which none of the
/// development items that were not about the plan was called one, as if a plan had been pending.
pub(crate) const REPEATS_AT: f32 = 0.712;

/// What the model reads of an item: its id as words, then its text, on one line.
pub(crate) fn item_text(id: &str, text: &str) -> String {
    let id = id.replace(['-', '_'], " ");
    let said = if id.trim().is_empty() { text.to_string() } else { format!("{}: {text}", id.trim()) };
    said.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The cosine of each item to Fleet's plan item, in order, from one batch with the plan item first.
pub(crate) fn scores(model: &dyn TextModel, items: &[(String, String)]) -> Result<Vec<f32>, NotEmbedded> {
    let said: Vec<String> = items.iter().map(|(id, text)| item_text(id, text)).collect();
    let texts: Vec<&str> = std::iter::once(PLAN_TO_APPROVE).chain(said.iter().map(String::as_str)).collect();
    let vectors = model.embed(&texts)?;
    let (plan, rest) = vectors.split_first().ok_or_else(|| NotEmbedded(String::from("no vectors came back")))?;
    Ok(rest.iter().map(|item| plan.iter().zip(item).map(|(a, b)| a * b).sum()).collect())
}

/// What the agent reads when its list is refused for an item that repeats the plan.
pub(crate) fn plan_repeat_refusal(id: &str, version: u32) -> String {
    format!(
        "Refused: item \"{id}\" repeats the plan you recorded (version {version}). Fleet lists it for the \
         owner as \"{PLAN_TO_APPROVE}\" until he approves it or sends changes. Call waiting_for again \
         without the item."
    )
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
    /// The refusal of a list one of whose items repeats the session's pending plan, else `None`. The
    /// model runs on a blocking thread, once for the whole list. A read that fails accepts the list.
    pub(crate) async fn plan_repeated(&self, session: &str, items: Vec<(String, String)>) -> Result<(), Refusal> {
        let Some(model) = self.language_model().model().filter(|_| IN_FORCE) else {
            return Ok(());
        };
        let pending = self
            .store()
            .lock()
            .await
            .session_plan(session)
            .map_err(|why| self.ledger_fault(why))?
            .filter(|plan| plan.approved_at.is_none());
        let Some(plan) = pending.filter(|_| !items.is_empty()) else {
            return Ok(());
        };
        let read = {
            let items = items.clone();
            let model: Arc<dyn TextModel> = model;
            tokio::task::spawn_blocking(move || scores(&*model, &items)).await
        };
        match read {
            Ok(Ok(scores)) => match scores.iter().position(|score| *score >= REPEATS_AT) {
                Some(at) => Err(self.hosted_refusal(SESSION_WAITING_PLAN, &plan_repeat_refusal(&items[at].0, plan.version))),
                None => Ok(()),
            },
            _ => Ok(()),
        }
    }
}
