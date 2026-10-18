//! `record_plan`, `approve_plan` and `send_plan_changes`: a Session's plan, the person's approval of
//! it, and the changes he marks on it. `docs/concepts/session.md`, *The plan*.
//!
//! **The agent plans with the person in the thread, then records.** A recording is a new version and
//! is pending until he approves it; while pending it is one of the Session's calls. Nothing blocks the
//! agent meanwhile. His marks go to the agent as one message and change nothing: the plan changes
//! when the agent records it again. The rules are `plan_record`'s and `plan_marks`'s.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    ApprovePlan, Instant, RecordPlan, SendPlanChanges, SessionFact, SessionId, SessionRecord, SessionReport,
    SessionRow, WaitingActKind, WaitingItem, WaitingSource, WireError,
};
use store::KeptPlan;

use crate::daemon::Fleet;
use crate::plan_marks;
use crate::plan_record::recorded;
use crate::session_host::sketching::{sketch_attachment, TARGET_PREFIX};

const PLAN_REFUSED: &str = "fleet.plan_refused";
const PLAN_UNNAMED: &str = "fleet.plan_unnamed";
const PLAN_NONE: &str = "fleet.plan_none";
const PLAN_STALE: &str = "fleet.plan_version_stale";
const PLAN_MARK_REFUSED: &str = "fleet.plan_mark_refused";

/// The call a recorded plan makes of the person until he approves it: id `plan:<version>`.
pub(crate) fn plan_item(session: &str, version: u32, since: &str) -> WaitingItem {
    WaitingItem {
        id: format!("plan:{version}"),
        text: String::from("Plan to approve"),
        since: Instant::carried(since),
        source: WaitingSource::Plan,
        act: Some(ipc::WaitingAct { kind: WaitingActKind::Message, target: session.to_string() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
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
    pub(crate) async fn plan_recorded(
        &self,
        caller: Option<api::Caller>,
        record: RecordPlan,
    ) -> Result<SessionRecord, Refusal> {
        let checked = record.checked().map_err(|why| self.hosted_refusal(PLAN_REFUSED, &why))?;
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| record.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.hosted_refusal(PLAN_UNNAMED, "a plan needs a job to record it for, and this call places none")
            })?;
        let (session, before) = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(PLAN_UNNAMED, "a job Fleet knows"))?;
            (session, store.session_plan(&id).map_err(|why| self.ledger_fault(why))?)
        };
        // The scene is on the ledger once, as `show_sketch` keeps it, and the plan names it.
        let sketch = match checked.sketch {
            None => None,
            Some(sketch) => {
                let named = (format!("{TARGET_PREFIX}{}", sketch.title), sketch.title.clone());
                self.report_session(SessionReport {
                    harness: session.harness.clone(),
                    session_id: SessionId::carried(&id),
                    fact: SessionFact::Attached { attachment: sketch_attachment(sketch) },
                })
                .await?;
                Some(named)
            }
        };
        let now = self.now().as_str().to_string();
        let kept = recorded(before.as_ref(), &checked.groups, sketch, &now);
        let version = kept.version;
        self.store().lock().await.keep_session_plan(&id, &kept).map_err(|why| self.ledger_fault(why))?;
        if session.origin == "bridge" {
            self.row_put(&id, SessionRow::Plan { id: self.row_id(&id), at: self.instant(), version }).await;
        }
        self.published_hosted(&id).await
    }

    /// The plan `version` of `id` as held, or why there is none to act on.
    async fn plan_at(&self, id: &str, version: u32) -> Result<KeptPlan, Refusal> {
        let plan = {
            let store = self.store().lock().await;
            store.session(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| {
                self.hosted_refusal(PLAN_UNNAMED, "a job Fleet knows")
            })?;
            store.session_plan(id).map_err(|why| self.ledger_fault(why))?
        };
        let moved = |code: &'static str, why: String| {
            Refusal::IllegalMove(WireError::raised(code, why, self.run_id()))
        };
        match plan {
            None => Err(moved(PLAN_NONE, String::from("this job has recorded no plan"))),
            Some(plan) if plan.version != version => Err(moved(
                PLAN_STALE,
                format!("plan v{version} is not the current one: the agent has recorded v{} since", plan.version),
            )),
            Some(plan) => Ok(plan),
        }
    }

    pub(crate) async fn plan_approved(self: Arc<Self>, approve: ApprovePlan) -> Result<SessionRecord, Refusal> {
        let id = approve.session_id.as_str().trim().to_string();
        let plan = self.plan_at(&id, approve.version).await?;
        if plan.approved_at.is_none() {
            // Told before it is settled, so a send that fails leaves the plan to approve again.
            self.told_by_fleet(&id, &plan_marks::approved_message(&plan)).await?;
            let now = self.now().as_str().to_string();
            self.store()
                .lock()
                .await
                .approve_session_plan(&id, plan.version, &now)
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
    }

    pub(crate) async fn plan_changes_sent(self: Arc<Self>, sent: SendPlanChanges) -> Result<SessionRecord, Refusal> {
        let id = sent.session_id.as_str().trim().to_string();
        let plan = self.plan_at(&id, sent.version).await?;
        let said = plan_marks::checked(&plan, &sent.marks, sent.approve)
            .map_err(|why| self.hosted_refusal(PLAN_MARK_REFUSED, &why))?;
        let approving = sent.approve && plan.approved_at.is_none();
        let text = plan_marks::message(&plan, approving, &said);
        let row = SessionRow::PlanChanges {
            id: self.row_id(&id),
            at: self.instant(),
            version: plan.version,
            approved: approving,
            lines: said.into_iter().map(|one| one.line).collect(),
        };
        self.told_by_fleet_as(&id, &text, row).await?;
        if approving {
            let now = self.now().as_str().to_string();
            self.store()
                .lock()
                .await
                .approve_session_plan(&id, plan.version, &now)
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
    }
}
