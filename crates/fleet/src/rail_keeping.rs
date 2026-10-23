//! Where the rail and the judges meet the store: what a row needs is read back under the lock the
//! caller already holds, and a kept concern is drawn as the waiting item the wire carries.
//! `docs/concepts/session.md`, *The rail*, *The judges*. The rules are `rail`'s.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Ulid, WorkflowId};
use ipc::{
    ConcernFrom, ConcernHow, ConcernSettled, Instant, JudgeConcern, JudgeReading, WaitingAct, WaitingActKind,
    WaitingCite, WaitingItem, WaitingSource,
};
use store::{KeptConcern, KeptReading, KeptSession, Store, WriteError};

use crate::daemon::Fleet;
use crate::rail::{self, Extra, Facts, RailStep};

/// The prefix of a concern's item id: `judge:<n>`.
pub(crate) const CONCERN_PREFIX: &str = "judge:";

/// What a row's step and judge reading are derived from.
#[derive(Default)]
pub(crate) struct RailReads {
    pub extra: Extra,
    pub reading: Option<KeptReading>,
}

/// Read what the rail needs beyond the record. **The commit and Check run are looked for only where the
/// Session follows a workflow**, since nothing else reads them.
pub(crate) fn rail_read(store: &Store, session: &KeptSession, follows: bool) -> Result<RailReads, WriteError> {
    let extra = if follows {
        Extra {
            committed: store.session_work(&session.id, &session.started_at)?.first_commit.is_some(),
            checked: store.session_has_run_a_check(&session.id)?,
        }
    } else {
        Extra::default()
    };
    Ok(RailReads { extra, reading: store.session_judge_reading(&session.id)? })
}

pub(crate) fn reading_wire(kept: &KeptReading) -> JudgeReading {
    JudgeReading { at: Instant::carried(&kept.at), concerns: kept.concerns }
}

/// A concern as the Session's waiting item. Waiting, or settled when the person has settled it.
pub(crate) fn concern_item(session: &str, kept: &KeptConcern) -> WaitingItem {
    WaitingItem {
        id: format!("{CONCERN_PREFIX}{}", kept.n),
        text: kept.text.clone(),
        since: Instant::carried(&kept.since),
        source: WaitingSource::Judge,
        act: Some(WaitingAct { kind: WaitingActKind::Message, target: session.to_string() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        cites: kept
            .cites
            .iter()
            .filter_map(|one| one.path.as_ref().map(|path| WaitingCite { path: path.clone(), line: Some(one.from_line) }))
            .collect(),
        concern: Some(JudgeConcern {
            from: match &kept.from_step {
                Some((index, label)) => ConcernFrom::Step { index: *index, label: label.clone() },
                None => ConcernFrom::Tile,
            },
            settled: kept.settled.as_ref().map(|(how, at)| ConcernSettled {
                how: if how == "sent" { ConcernHow::Sent } else { ConcernHow::Dismissed },
                at: Instant::carried(at),
            }),
            expected: kept.expected.clone(),
            produced: kept.produced.clone(),
            consequence: kept.consequence.clone(),
        }),
        confirmed: false,
    }
}

/// The number in a concern's item id, if it is one.
pub(crate) fn concern_number(item_id: &str) -> Option<u32> {
    item_id.strip_prefix(CONCERN_PREFIX)?.parse().ok()
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
    /// The steps of the workflow `workflow` names in the repository `manifest`, as the rail reads them.
    /// Absent where the repository is not served or no longer holds it.
    pub(crate) fn rail_steps(&self, manifest: &str, workflow: &str) -> Option<Vec<RailStep>> {
        let served = self.repositories().serving(manifest)?;
        let held = served.workflows();
        let found = held.get(&WorkflowId::carried(Ulid::carried(workflow)))?;
        Some(rail::steps_of(found))
    }

    /// Draw what the rail and the judges add to a hosted Session's row: the step it is on, whether a
    /// judge is reading, and the last reading. **A Session that ended has no step.**
    pub(crate) fn rail_drawn(&self, session: &KeptSession, reads: &RailReads, record: &mut ipc::SessionRecord) {
        if record.hosted.is_none() {
            return;
        }
        record.judge = reads.reading.as_ref().map(reading_wire);
        if session.state != store::SessionState::Live {
            return;
        }
        record.judging = self.hosts().of(&session.id).state().judging > 0;
        if !self.workflows_on() {
            return;
        }
        let (Some(manifest), Some(workflow)) = (session.manifest_id.as_deref(), record.workflow.as_deref()) else {
            return;
        };
        let Some(steps) = self.rail_steps(manifest, workflow) else {
            return;
        };
        let facts = Facts::of(record, reads.extra);
        record.workflow_step = rail::current(&steps, &facts).map(|at| at as u32);
    }
}
