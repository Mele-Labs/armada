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
    ApprovePlan, Instant, PlanHold, PlanHoldWhy, RecordPlan, SendPlanChanges, SessionFact, SessionId, SessionRecord, SessionReport,
    SessionRow, WaitingActKind, WaitingItem, WaitingSource, WireError,
};
use store::KeptPlan;

use crate::daemon::Fleet;
use crate::plan_marks;
use crate::plan_record::recorded;
use crate::session_host::asking::answers_a_question;
use crate::session_host::walking;
use crate::session_host::sketching::{sketch_attachment, TARGET_PREFIX};

const PLAN_REFUSED: &str = "fleet.plan_refused";
const PLAN_UNNAMED: &str = "fleet.plan_unnamed";
const PLAN_NONE: &str = "fleet.plan_none";
const PLAN_STALE: &str = "fleet.plan_version_stale";
const PLAN_MARK_REFUSED: &str = "fleet.plan_mark_refused";

/// The text of the call a recorded plan makes of the person. `plan_asks.rs` reads an agent's item
/// against it.
pub(crate) const PLAN_TO_APPROVE: &str = "Plan to approve";

/// What the agent is told when he puts a plan away to keep talking.
pub(crate) const NOT_YET_TOLD: &str =
    "Not ready for a plan yet. Keep talking, and record no plan until I ask for one.";

/// The call a recorded plan makes of the person until he approves it: id `plan:<version>`.
pub(crate) fn plan_item(session: &str, version: u32, since: &str) -> WaitingItem {
    WaitingItem {
        id: format!("plan:{version}"),
        text: String::from(PLAN_TO_APPROVE),
        since: Instant::carried(since),
        source: WaitingSource::Plan,
        act: Some(ipc::WaitingAct { kind: WaitingActKind::Message, target: session.to_string() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    }
}

/// What holds a pending plan back, and when it can be answered once nothing does.
pub(crate) struct Holds {
    pub held_by: Vec<PlanHold>,
    /// The latest of the recording, when the windows that count were last shown or made ready, and
    /// his Approves of them.
    pub since: Instant,
}

/// The windows that hold a pending plan back. **Only a window he can see or still owes holds it.**
/// One a newer window of its page replaced, or whose walk item he dismissed, holds nothing. Every
/// other window he has not approved since it was last shown holds it. **An Approve stands until the
/// page is shown again**: a file write marks every window changed, and the Job goes on writing after
/// he approves, so a window he approved holds nothing however it was marked since. One he has not
/// approved and was changed waits to be shown again.
/// A walk the plan names holds it until he approves it. **A window he answered with words, not an
/// Approve, before the plan was recorded holds nothing**: the plan is the Job's answer to them, and
/// showing the window again after the recording makes it hold again. His answer to one of the Job's
/// questions is not such words: it answers the question, not the walk.
pub(crate) fn plan_holds(
    recorded: &str,
    walk: Option<&str>,
    attachments: &[ipc::Attachment],
    rows: &[SessionRow],
    dismissed: &[String],
) -> Holds {
    let newest = rows.iter().rev().find_map(|row| match row {
        SessionRow::Window { url, .. } => Some(url.as_str()),
        _ => None,
    });
    let mut since = Instant::carried(recorded);
    let mut held_by = Vec::new();
    for window in walking::walk_windows(attachments) {
        let url = window.target.as_str();
        if dismissed.contains(&format!("walk:{url}")) || walking::replaced(rows, url) || answered(rows, url, recorded) {
            continue;
        }
        let on_screen = newest == Some(url);
        let changed = walking::is_changed(window);
        let approved = walking::walk_approval(rows, url);
        let why = if approved.is_some() {
            None
        } else if on_screen && walking::is_working(window) {
            Some(PlanHoldWhy::Changing)
        } else if changed {
            Some(PlanHoldWhy::Changed)
        } else {
            Some(PlanHoldWhy::Unapproved)
        };
        if let Some(why) = why {
            held_by.push(PlanHold { url: url.to_string(), why });
            continue;
        }
        if !changed {
            since = since.max(window.changed_at.clone());
        }
        if let Some(shown) = walking::walk_shown_at(rows, url) {
            since = since.max(shown.clone());
        }
        if let Some(approved) = approved {
            since = since.max(approved.clone());
        }
    }
    if let Some(walk) = walk {
        match walking::walk_approval(rows, walk) {
            Some(approved) => since = since.max(approved.clone()),
            None if held_by.iter().any(|one| one.url == walk) => {}
            None => held_by.push(PlanHold { url: walk.to_string(), why: PlanHoldWhy::Walk }),
        }
    }
    Holds { held_by, since }
}

/// Whether he wrote to the Job, other than an Approve or an answer to one of its questions, after
/// `url` was last shown and before the plan was `recorded`.
fn answered(rows: &[SessionRow], url: &str, recorded: &str) -> bool {
    let Some(shown) = walking::walk_shown_at(rows, url) else {
        return false;
    };
    rows.iter().any(|row| {
        matches!(row, SessionRow::Message { id, at, from: ipc::SessionVoice::You, text, .. }
            if at > shown && at.as_str() < recorded && walking::approved_address(text).is_none()
                && !answers_a_question(id))
    })
}

/// The call a pending plan makes, if it makes one yet. **It is put to him when it can be answered**,
/// once nothing in [`plan_holds`] holds it back. It begins then, so it calls even after he viewed the
/// Job.
pub(crate) fn plan_called(session: &str, version: u32, holds: &Holds) -> Option<WaitingItem> {
    holds.held_by.is_empty().then(|| plan_item(session, version, holds.since.as_str()))
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
        seen: bool,
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
        let kept = recorded(before.as_ref(), &checked.groups, sketch, checked.walk, seen, checked.summary, &now);
        let version = kept.version;
        self.store().lock().await.keep_session_plan(&id, &kept).map_err(|why| self.ledger_fault(why))?;
        // The issues the groups work, held as the person's tags are: a second recording is a no-op.
        if let Some(manifest) = session.manifest_id.as_deref() {
            let named = checked
                .groups
                .iter()
                .filter_map(|group| group.issue.as_deref().and_then(ipc::IssueReference::read))
                .map(|issue| (issue, Default::default()))
                .collect();
            self.issues_held(&id, manifest, named).await;
        }
        if session.origin == "bridge" {
            self.row_put(&id, SessionRow::Plan { id: self.row_id(&id), at: self.instant(), version }).await;
        }
        self.published_hosted(&id).await
    }

    /// Put the plan version the call `item` names away. A call for a version that is not the one held
    /// anymore changes nothing.
    pub(crate) async fn plan_set_aside(&self, id: &str, item: &str) -> Result<(), Refusal> {
        let Some(version) = item.strip_prefix("plan:").and_then(|version| version.parse::<u32>().ok()) else {
            return Ok(());
        };
        let now = self.now().as_str().to_string();
        self.store()
            .lock()
            .await
            .set_aside_session_plan(id, version, &now)
            .map_err(|why| self.ledger_fault(why))?;
        Ok(())
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
            self.told_by_fleet(&id, &plan_marks::approved_message(&plan), ipc::Notice::PlanApproved).await?;
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
