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
    SessionRow, WaitingActKind, WaitingItem, WaitingSource, WireError, DOCUMENT_ASKED, DOCUMENT_SETTLED, DOCUMENT_SETTLED_AT,
};
use store::{Holder, KeptPlan};

use crate::daemon::Fleet;
use crate::plan_issues::name_the_tickets;
use crate::plan_marks;
use crate::plan_record::recorded;
use crate::rail;
use crate::rail_keeping::rail_read;
use crate::session_host::asking::answers_a_question;
use crate::session_issues::ISSUE;
use crate::session_host::document_answers::{in_front_of_him, is_document};
use crate::session_host::walking;
use crate::session_host::sketching::{sketch_attachment, TARGET_PREFIX};

const PLAN_REFUSED: &str = "fleet.plan_refused";
const PLAN_UNNAMED: &str = "fleet.plan_unnamed";
const PLAN_NONE: &str = "fleet.plan_none";
const PLAN_STALE: &str = "fleet.plan_version_stale";
const PLAN_MARK_REFUSED: &str = "fleet.plan_mark_refused";

/// The text of the call a recorded plan makes of the person.
pub(crate) const PLAN_TO_APPROVE: &str = "Plan to approve";

/// What the agent is told when he puts a plan away to keep talking.
pub(crate) const NOT_YET_TOLD: &str =
    "Not ready for a plan yet. Keep talking, and record no plan until I ask for one.";

/// The call a recorded plan makes of the person until he approves it: id `plan:<version>`.
pub(crate) fn plan_item(session: &str, version: u32, since: &str) -> WaitingItem {
    WaitingItem {
        approves: false,
        id: format!("plan:{version}"),
        text: String::from(PLAN_TO_APPROVE),
        since: Instant::carried(since),
        source: WaitingSource::Plan,
        act: Some(ipc::WaitingAct { kind: WaitingActKind::Message, target: session.to_string() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
    }
}

/// What holds a pending plan back, and when it can be answered once nothing does.
pub(crate) struct Holds {
    pub held_by: Vec<PlanHold>,
    /// The latest of the recording, when the windows that count were last shown or made ready, and
    /// his Approves of them.
    pub since: Instant,
}

/// The windows and pages that hold a pending plan back. **Only a window or page he can see or still owes
/// holds it.** One a newer window of its page replaced, or whose walk item he dismissed, holds nothing,
/// and neither does a page he dismissed or the Job deleted (`in_front_of_him`), the test its card makes. Every
/// other window he has not approved since it was last shown holds it. **An Approve stands until the
/// page is shown again**: a file write marks every window changed, and the Job goes on writing after
/// he approves, so a window he approved holds nothing however it was marked since. One he has not
/// approved and was changed waits to be shown again.
/// A walk the plan names holds it until he approves it, **unless it was shown and is no longer in front
/// of him**: dismissed, withdrawn or replaced as a window's is. One never shown still holds, the Job owes it.
/// **A window he answered with words, not an Approve, before the plan was recorded holds nothing**: the plan is the Job's answer to them, and
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
    // A page put to him holds the plan too: his answer to the page is the one approval the Job asks, and it settles the plan
    // (`document_answered`), so the plan is never a second card beside it.
    for page in attachments.iter().filter(|one| one.kind == "artifact" && is_document(&one.detail)) {
        if page.detail.get(DOCUMENT_ASKED).map(String::as_str) != Some("true") {
            continue;
        }
        match page.detail.get(DOCUMENT_SETTLED_AT) {
            Some(settled) if page.detail.contains_key(DOCUMENT_SETTLED) => since = since.max(Instant::carried(settled)),
            _ if in_front_of_him(page, dismissed) => held_by.push(PlanHold { url: page.target.clone(), why: PlanHoldWhy::Unapproved }),
            _ => {}
        }
    }
    if let Some(walk) = walk {
        match walking::walk_approval(rows, walk) {
            Some(approved) => since = since.max(approved.clone()),
            None if held_by.iter().any(|one| one.url == walk) => {}
            // Shown and then dismissed or withdrawn, or replaced by a newer window of its page: no card of his holds it.
            None if walking::walk_shown_at(rows, walk).is_some() && !walk_in_front_of_him(rows, walk, dismissed) => {}
            None => held_by.push(PlanHold { url: walk.to_string(), why: PlanHoldWhy::Walk }),
        }
    }
    Holds { held_by, since }
}

/// Whether the window `walk` names, shown, is still one he can see or owes: its walk item not dismissed (a withdrawal
/// dismisses it) and no newer window of its page replaced it. Closing it is not leaving it, as the walk item stays.
fn walk_in_front_of_him(rows: &[SessionRow], walk: &str, dismissed: &[String]) -> bool {
    let window = walking::window_of(rows, walk).unwrap_or(walk);
    !dismissed.contains(&format!("walk:{window}")) && !walking::replaced(rows, window)
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

/// When a plan that stands on an approved card counts as approved. **Where the step lists a walk, from
/// when the first walk he approved went out**, so that walk and any shown with it are the ones `walks`
/// reads and he is not asked for another. Else `now`.
fn stands_since(rows: &[SessionRow], lists_walk: bool, now: &str) -> String {
    let first = rows.iter().filter_map(|row| match row {
        SessionRow::Window { url, at, .. } if lists_walk && walking::walk_approval(rows, url).is_some() => Some(at.as_str()),
        _ => None,
    });
    first.min().unwrap_or(now).to_string()
}

/// The call a pending plan makes, if it makes one yet. **It is put to him when it can be answered**,
/// once nothing in [`plan_holds`] holds it back. It begins then, so it calls even after he viewed the
/// Job.
pub(crate) fn plan_called(session: &str, version: u32, holds: &Holds) -> Option<WaitingItem> {
    holds.held_by.is_empty().then(|| plan_item(session, version, holds.since.as_str()))
}

/// The Job's thread, as `rail_read` reads it.
fn decoded_rows(store: &store::Store, id: &str) -> Vec<SessionRow> {
    store
        .session_rows(id)
        .unwrap_or_default()
        .iter()
        .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
        .collect()
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
        groups_seen: &[bool],
    ) -> Result<SessionRecord, Refusal> {
        let mut checked = record.checked().map_err(|why| self.hosted_refusal(PLAN_REFUSED, &why))?;
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
                let sketch = self.sketch_styled(&id, sketch, "record_plan").await?;
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
        // A group the agent named no issue for takes the one ticket of the Job's its words name.
        let tickets: Vec<String> = self
            .store()
            .lock()
            .await
            .attachments_of(&Holder::session(&id))
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .filter(|row| row.kind == ISSUE && row.state != store::AttachmentState::GivenBack)
            .map(|row| row.target)
            .collect();
        name_the_tickets(&mut checked.groups, &tickets);
        let now = self.now().as_str().to_string();
        let mut kept = recorded(before.as_ref(), &checked.groups, sketch, checked.walk, seen, checked.summary, &now);
        // A group the person would see makes a plan he would see.
        for (group, seen) in kept.groups.iter_mut().zip(groups_seen) {
            group.seen = *seen;
        }
        kept.seen |= kept.groups.iter().any(|group| group.seen);
        let version = kept.version;
        // A version recorded after one he approved stands on that approval unless the agent asks again
        // (owner, 8 Nov 2026): no card, no waiting item, and no plan gate reopens.
        let approved_before = before.as_ref().filter(|one| one.approved_at.is_some());
        if let Some(one) = approved_before.filter(|_| !record.ask_again) {
            kept.approved_at = one.approved_at.clone();
            kept.asked_version = one.asked_version;
        }
        self.store().lock().await.keep_session_plan(&id, &kept).map_err(|why| self.ledger_fault(why))?;
        if before.is_none() {
            self.rules_plan_first_recorded(&id, &kept.groups).await;
        }
        // A version after one he approved, with `ask_again`, comes once the plan step is passed: it is his to approve again.
        let late = approved_before.is_some();
        if kept.approved_at.is_none() {
            if let Some(since) = self.plan_stands_since(&id, late).await {
                self.store().lock().await.approve_session_plan(&id, version, &since).map_err(|why| self.ledger_fault(why))?;
            }
        }
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

    /// The rail's steps for `id`, where it follows a workflow that is on.
    async fn plan_steps(&self, id: &str) -> Option<Vec<rail::RailStep>> {
        let (_, hosting) = self.session_and_hosting(id).await.ok()?;
        let workflow = hosting.workflow.as_deref().filter(|workflow| self.workflow_on(&hosting.manifest_id, workflow))?;
        self.job_rail_steps(id, &hosting.manifest_id, workflow).await
    }

    /// When a plan recorded now counts as approved, where it stands with no card. `None` where he is asked.
    /// **A `late` one**, recorded after he approved a version, stands only where its step never asks: what
    /// the step listed was approved for the plan before it (owner, 4 Nov 2026).
    async fn plan_stands_since(&self, id: &str, late: bool) -> Option<String> {
        let steps = self.plan_steps(id).await?;
        if late && !steps.iter().find(|step| step.records_plan).is_some_and(|step| step.plan_unasked) {
            return None;
        }
        let store = self.store().lock().await;
        let session = store.session(id).ok().flatten()?;
        let extra = rail_read(&store, &session, true).ok()?.extra;
        rail::plan_stands(&steps, extra)
            .then(|| stands_since(&decoded_rows(&store, id), rail::walk_settles_plan(&steps), self.now().as_str()))
    }

    /// His Approve of the window at `address`, about to be sent: a plan held behind that window alone
    /// stands with it, where the step lists a walk, rather than coming up as a second card. **Before the
    /// send**, so what he is owed never shows the plan for a moment.
    pub(crate) async fn plan_settled_by_walk(self: &Arc<Self>, id: &str, address: &str) -> Result<(), Refusal> {
        if !self.plan_steps(id).await.is_some_and(|steps| rail::walk_settles_plan(&steps)) {
            return Ok(());
        }
        let (plan, since) = {
            let store = self.store().lock().await;
            let session = store.session(id).map_err(|why| self.ledger_fault(why))?;
            let plan = store.session_plan(id).map_err(|why| self.ledger_fault(why))?;
            let (Some(session), Some(plan)) = (session, plan.filter(|plan| plan.approved_at.is_none() && plan.set_aside_at.is_none())) else {
                return Ok(());
            };
            let rows = decoded_rows(&store, id);
            let dismissed = store.dismissed_waiting(id).map_err(|why| self.ledger_fault(why))?;
            let attachments = self.ledger_row(&store, &session)?.attachments;
            let holds = plan_holds(&plan.recorded_at, plan.walk.as_deref(), &attachments, &rows, &dismissed);
            let window = walking::window_of(&rows, address).unwrap_or(address);
            if holds.held_by.is_empty() || holds.held_by.iter().any(|one| one.url != window) {
                return Ok(());
            }
            let shown = walking::walk_shown_at(&rows, window).map_or("", Instant::as_str);
            let since = stands_since(&rows, true, shown);
            let since = if since.as_str() < shown { since } else { shown.to_string() };
            (plan, since)
        };
        self.told_by_fleet(id, &plan_marks::approved_message(&plan), ipc::Notice::PlanApproved).await?;
        self.store().lock().await.approve_session_plan(id, plan.version, &since).map_err(|why| self.ledger_fault(why))?;
        Ok(())
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
