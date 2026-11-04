//! The Fleet side of a retro's timing row: the Job's kept time and the usual of its workflow, read
//! from the store in one short hold. What the row is made of is `session_time.rs`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::RecordSplit;

use super::session_time::TimingSources;
use crate::daemon::Fleet;

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
    /// The Job's time and the usual of its workflow, read in one short hold of the store. None where
    /// no time is kept for it, or its workflow has no name or no other Job to be usual from.
    pub(crate) async fn timing_of(&self, id: &str) -> Result<Option<TimingSources>, Refusal> {
        let (kept, usual) = {
            let store = self.store().lock().await;
            let kept = store.job_time(id).map_err(|why| self.ledger_fault(why))?;
            let usual = match &kept {
                Some(kept) if !kept.workflow.is_empty() => store.usual_splits().map_err(|why| self.ledger_fault(why))?,
                _ => Vec::new(),
            };
            (kept, usual)
        };
        let Some(kept) = kept else { return Ok(None) };
        let Some((_, usual)) = usual.into_iter().find(|(workflow, _)| *workflow == kept.workflow) else {
            return Ok(None);
        };
        let ms = |value: i64| u64::try_from(value).unwrap_or(0);
        Ok(Some(TimingSources {
            workflow: kept.workflow,
            jobs: usual.jobs,
            spent: RecordSplit {
                model: ms(kept.model_ms),
                tools: ms(kept.tools_ms),
                checks: ms(kept.checks_ms),
                subagents: ms(kept.subagents_ms),
                slot: ms(kept.slot_ms),
                you: ms(kept.you_ms),
                pull_request: ms(kept.pull_request_ms),
                stopped: ms(kept.stopped_ms),
                idle: ms(kept.idle_ms),
            },
            usual: RecordSplit {
                model: ms(usual.split.model),
                tools: ms(usual.split.tools),
                checks: ms(usual.split.checks),
                subagents: ms(usual.split.subagents),
                slot: ms(usual.split.slot),
                you: ms(usual.split.you),
                pull_request: ms(usual.split.pull_request),
                stopped: ms(usual.split.stopped),
                idle: ms(usual.split.idle),
            },
        }))
    }
}
