//! Taking a precedent ruling back: the question the Operator answered by following something he said is his again.
//! `crates/ipc/operations/operator_undo_ruling.toml`.
//!
//! **Fleet puts it back.** The ruling's row is marked, the Job is told the answer is withdrawn, and the Job's Needs
//! you list holds an `operator_undone` item with the question and its options, derived on every read from the
//! marked rows. His answer to it is an answer to an agent's item. The Operator's review never answers one: it
//! answers `AskCard` and `Agent` items only.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::operator_page::{item_read, OperatorPage, OperatorUndoRuling};
use ipc::{Instant, WaitingAct, WaitingActKind, WaitingItem, WaitingSource};
use store::KeptTurn;

use super::turning::turn_number;
use crate::daemon::Fleet;

/// The id of the item for one ruling taken back: `undone:<n>`, `<n>` being the ruling's row.
pub(crate) const UNDONE: &str = "undone:";

const ALREADY_UNDONE: &str = "fleet.operator_ruling_undone";
const JOB_ENDED: &str = "fleet.operator_ruling_ended";

/// The rows of a Job's Needs you list for the rulings taken back: the question as it was asked, with its options.
pub(crate) fn undone_items(taken_back: &[KeptTurn]) -> impl Iterator<Item = WaitingItem> + '_ {
    taken_back.iter().filter_map(|row| {
        let asked = item_read(&row.body)?;
        let id = format!("{UNDONE}{}", row.n);
        Some(WaitingItem {
            approves: false,
            act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone() }),
            id,
            text: asked.text,
            since: Instant::carried(row.undone_at.as_deref()?),
            source: WaitingSource::OperatorUndone,
            options: asked.options,
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        })
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
    /// `operator_undo_ruling`: the question goes back to him and the Job is told to wait for his answer. **Refused
    /// with nothing changed** where the row is no precedent ruling, it is taken back already, or its Job has ended.
    pub(crate) async fn operator_ruling_undone(self: &Arc<Self>, undo: OperatorUndoRuling) -> Result<OperatorPage, Refusal> {
        let n = turn_number(&undo.id).ok_or_else(|| self.turn_unknown(&undo.id))?;
        let (row, live) = {
            let store = self.store().lock().await;
            let row = store.operator_turn_item(n).map_err(|why| self.ledger_fault(why))?.filter(|row| row.answer.as_deref() == Some("precedent"));
            let live = match &row {
                Some(row) => store.session(&row.session).map_err(|why| self.ledger_fault(why))?.is_some_and(|session| session.state != store::SessionState::Ended),
                None => false,
            };
            (row, live)
        };
        let row = row.ok_or_else(|| self.turn_unknown(&undo.id))?;
        if row.undone_at.is_some() {
            return Err(self.hosted_refusal(ALREADY_UNDONE, "that ruling is taken back already"));
        }
        if !live {
            return Err(self.hosted_refusal(JOB_ENDED, "that Job has ended, so the question cannot be given back to it"));
        }
        let asked = item_read(&row.body).ok_or_else(|| self.ledger_unnamed("an item that reads"))?;
        let marked = self.store().lock().await.undo_operator_turn(n, self.now().as_str()).map_err(|why| self.ledger_fault(why))?;
        if !marked {
            return Err(self.hosted_refusal(ALREADY_UNDONE, "that ruling is taken back already"));
        }
        let question = asked.text.trim();
        let _ = self
            .told_by_fleet(&row.session, &format!("The Operator's answer to \"{question}\" is withdrawn. The person will answer it; wait for his answer."), ipc::Notice::Operator)
            .await;
        self.operator_journaled(&format!("Undone: {}: {question}", row.job.as_deref().unwrap_or("A Job"))).await;
        // The Job's list now holds the question, so its record is read again beside the page.
        let record = {
            let store = self.store().lock().await;
            store.session(&row.session).map_err(|why| self.ledger_fault(why))?.map(|session| self.ledger_row(&store, &session)).transpose()?
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
        self.operator_page_changed();
        self.operator_page_wire().await
    }
}
