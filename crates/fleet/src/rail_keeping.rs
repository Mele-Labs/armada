//! Where the rail meets the store: what a row needs is read back under the lock the caller already
//! holds. `docs/concepts/session.md`, *The rail*. The rules are `rail`'s.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Ulid, WorkflowId};
use ipc::{AskedQuestion, DraftHow, Instant, QuestionAnswer, SessionRow, SessionVoice, WaitingSource};
use store::{Holder, KeptSession, Store, WriteError};

use crate::daemon::Fleet;
use crate::rail::{self, Extra, Facts, RailStep};
use crate::rail_ends::{self, Waiter};
use crate::rail_lifecycle::{self, Held, StepBits};
use crate::session_host::asking::{answers_said, ask_answered};
use crate::session_host::document_answers::approved_on_newest;
use crate::session_host::handoff_answers::{answered_later, ANSWERED_LATER};
use crate::session_host::sketching::sketch_approved;
use crate::session_host::walking::{replaced, walk_approval};

/// What a row's step is derived from.
#[derive(Default)]
pub(crate) struct RailReads {
    pub extra: Extra,
    /// The steps of the Job's workflow the rail was read against, which every index here is into. Empty where
    /// they did not resolve, and then the row draws no step.
    pub steps: Vec<RailStep>,
    /// The step the rail last kept the Job standing on.
    pub stood: Option<u32>,
    /// When each step began, as the rail kept it.
    pub visits: Vec<(u32, String)>,
    /// The steps the person skipped.
    pub skipped: Vec<u32>,
    /// The plan groups whose `you_approved` gate has closed, and every group's id: what a `you_approved` step reads.
    pub approved_groups: Vec<String>,
    pub plan_groups: Vec<String>,
    /// What the Job holds of its lifecycles, for a Job that follows a workflow and holds any.
    pub held: Held,
    /// The title of each Job it holds, by id, which `rail_lifecycle::named` calls one by.
    pub titles: std::collections::HashMap<String, String>,
    /// The lifecycles the Job froze when it started, for a Job that holds any. A Job that froze none
    /// is read against the ones that apply now.
    pub lifecycles: Option<Vec<core_model::Lifecycle>>,
    /// The Job's own copy of its workflow, once it changed a step of it. The rail reads this in place of the
    /// repository's.
    pub own: Option<store::KeptWorkflowCopy>,
}

impl RailReads {
    /// Whether the `you_approved` gate of the plan group whose id is `step` has closed, or any such gate where
    /// no group carries that id.
    pub(crate) fn approved(&self, step: &str) -> bool {
        match self.plan_groups.iter().any(|group| group == step) {
            true => self.approved_groups.iter().any(|group| group == step),
            false => !self.approved_groups.is_empty(),
        }
    }
}

/// Read what the rail needs beyond the record. **The commit, the Check run, the thread and the plan's
/// walk are looked for only where the Session follows a workflow**, since nothing else reads them. `steps` are the
/// ones the Job's workflow has now (empty where it follows none), since the rail is kept by step id.
pub(crate) fn rail_read(store: &Store, session: &KeptSession, follows: bool, steps: Vec<RailStep>) -> Result<RailReads, WriteError> {
    let rail = store.session_rail(&session.id, &rail::step_ids(&steps))?;
    let mut held = Held::default();
    let mut lifecycles = None;
    let extra = if follows {
        if store.is_lifecycle_job(&session.id)? {
            // The rail reads a state by its id and never by a tracker's word, so the carried four are enough.
            lifecycles = crate::lifecycles::copying::frozen_lifecycles(store, &session.id, core_model::Lifecycle::built_in());
            held = Held { instances: store.lifecycle_instances_of(&session.id)?, moves: store.lifecycle_moves_of(&session.id)? };
        }
        let rows: Vec<SessionRow> = store
            .session_rows(&session.id)?
            .iter()
            .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
            .collect();
        let plan = store.session_plan(&session.id)?;
        let plan_approved_at = plan.as_ref().and_then(|plan| plan.approved_at.as_deref()).map(Instant::carried);
        let dismissed = store.dismissed_waiting(&session.id)?;
        let walk_approved = walk_approved_at(&rows, plan_approved_at.as_ref(), &dismissed);
        let pushed_since_walk = match walk_approved {
            Some(approved) => store.session_work(&session.id, approved.as_str())?.first_push.is_some(),
            None => false,
        };
        Extra {
            walk_approved: walk_approved.is_some(),
            pushed_since_walk,
            committed: store.session_work(&session.id, &session.started_at)?.first_commit.is_some(),
            checked: store.session_has_run_a_check(&session.id)?,
            plan_expects_walk: plan.is_some_and(|plan| plan.seen || plan.walk.is_some()),
            messaged: messaged_on(&rows, &rail.visits),
            landed: store.attachments_of(&Holder::session(&session.id))?.iter().any(|one| one.kind == "pr" && crate::build_record::merged(one)),
            document_approved: store
                .attachments_of(&Holder::session(&session.id))?
                .iter()
                .any(|one| one.kind == "artifact" && approved_on_newest(&one.detail)),
            ..thread_facts(&rows, plan_approved_at.as_ref())
        }
    } else {
        Extra::default()
    };
    let titles = crate::lifecycles::waking::titles_of(store, &held);
    let own = if follows { store.job_workflow_copy(&session.id)? } else { None };
    let (approved_groups, plan_groups) = match follows {
        true => (
            store.session_gates(&session.id)?.into_iter().filter(|gate| gate.gate == "you_approved").map(|gate| gate.group_id).collect(),
            store.session_plan(&session.id)?.map(|plan| plan.groups.iter().map(|group| group.id.clone()).collect()).unwrap_or_default(),
        ),
        false => Default::default(),
    };
    Ok(RailReads { extra, steps, stood: rail.step, visits: rail.visits, skipped: rail.skipped, approved_groups, plan_groups, held, titles, lifecycles, own })
}

/// The steps he wrote on after they began, by the latest visit to each. **Only `You`**: Fleet's own
/// notices and other Jobs' words are not his.
pub(crate) fn messaged_on(rows: &[SessionRow], visits: &[(u32, String)]) -> StepBits {
    let wrote = |since: &str| {
        rows.iter().any(|row| matches!(row, SessionRow::Message { from: SessionVoice::You, at, .. } if at.as_str() >= since))
    };
    StepBits::of(visits.iter().enumerate().filter_map(|(n, (step, since))| {
        let latest = !visits[n + 1..].iter().any(|(later, _)| later == step);
        (latest && wrote(since)).then_some(*step as usize)
    }))
}

/// What the thread shows: the person wrote after the agent's first turn (a message or a tool call),
/// the Job showed a window and he approved one (since the plan's approval, at `plan_approved_at`, and
/// at all), he approved a sketch or a draft, and he picked an idea. **Only `You` is the person**: Fleet's own
/// notices and other Sessions' words are not a reply.
pub(crate) fn thread_facts(rows: &[SessionRow], plan_approved_at: Option<&Instant>) -> Extra {
    let spoke = rows.iter().position(|row| match row {
        SessionRow::Message { from, .. } => *from == SessionVoice::Agent,
        SessionRow::Tool { .. } => true,
        _ => false,
    });
    let replied = spoke.is_some_and(|spoke| {
        rows[spoke..].iter().any(|row| matches!(row, SessionRow::Message { from: SessionVoice::You, .. }))
    });
    let windows: Vec<&str> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Window { url, .. } => Some(url.as_str()),
            _ => None,
        })
        .collect();
    let approvals = approvals_of(rows);
    let draft_approved = rows.iter().any(|row| {
        matches!(row, SessionRow::Draft { settled: Some(settled), .. } if matches!(settled.how, DraftHow::Approved | DraftHow::Fixed))
    });
    Extra {
        replied,
        walk_shown: !windows.is_empty(),
        walk_approved: walk_approved_at(rows, plan_approved_at, &[]).is_some(),
        any_walk_approved: !approvals.is_empty(),
        sketch_approved: sketch_approved(rows).is_some(),
        draft_approved,
        idea_picked: idea_picked(rows),
        ..Extra::default()
    }
}

/// When the person approved each window the Job showed, of those he approved.
fn approvals_of(rows: &[SessionRow]) -> Vec<&Instant> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Window { url, .. } => walk_approval(rows, url),
            _ => None,
        })
        .collect()
}

/// His last Approve of the windows at or after the plan's approval, at `plan_approved_at`, once every
/// window shown since stands approved: the one that passes a `walks` step. `None` before the plan is
/// approved, and while one waits. **A window a newer one of its page replaced, or one taken back
/// (`dismissed`), waits on nothing.**
fn walk_approved_at<'a>(rows: &'a [SessionRow], plan_approved_at: Option<&Instant>, dismissed: &[String]) -> Option<&'a Instant> {
    let plan = plan_approved_at?;
    let waiting = rows.iter().any(|row| {
        matches!(row, SessionRow::Window { url, at, .. }
            if at >= plan && !replaced(rows, url) && !dismissed.contains(&format!("walk:{url}")) && walk_approval(rows, url).is_none())
    });
    if waiting {
        return None;
    }
    approvals_of(rows).into_iter().filter(|at| *at >= plan).max()
}

/// Whether the person picked an option the Job's question gave a preview, on its card or later from
/// Needs you. Words of his own pick nothing.
fn idea_picked(rows: &[SessionRow]) -> bool {
    let picked = |questions: &[AskedQuestion], answers: Vec<QuestionAnswer>| {
        answers.iter().any(|answer| {
            questions.iter().filter(|asked| asked.question == answer.question).any(|asked| {
                asked.options.iter().any(|option| !option.preview.is_empty() && answer.chosen.contains(&option.label))
            })
        })
    };
    let asked_on = |ask_id: &str| {
        rows.iter().find_map(|row| match row {
            SessionRow::Ask { id, ask, .. } if id == ask_id => Some(ask.questions.as_slice()),
            _ => None,
        })
    };
    let mut latest: &[AskedQuestion] = &[];
    rows.iter().any(|row| match row {
        SessionRow::Ask { ask, .. } => {
            if !ask.questions.is_empty() {
                latest = &ask.questions;
            }
            false
        }
        SessionRow::Message { id, from: SessionVoice::You, text, .. } => match ask_answered(id).and_then(asked_on) {
            Some(questions) => picked(questions, answers_said(questions, text)),
            None => text.starts_with(ANSWERED_LATER) && picked(latest, answered_later(latest, text)),
        },
        _ => false,
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
    /// The steps of the workflow `workflow` names in the repository `manifest`, as the rail reads them.
    /// Absent where the repository is not served or no longer holds it.
    pub(crate) fn rail_steps(&self, manifest: &str, workflow: &str) -> Option<Vec<RailStep>> {
        let served = self.repositories().serving(manifest)?;
        let held = served.workflows();
        let found = held.get(&WorkflowId::carried(Ulid::carried(workflow)))?;
        Some(rail::steps_of(found))
    }

    /// **Keep by id every rail still kept by index**, for each Job whose workflow resolves now, so a workflow saved
    /// afterwards does not move it. A rail whose workflow does not resolve stays as it is and still reads as indexes.
    pub(crate) async fn rails_kept_by_id(&self) {
        let Ok(sessions) = self.store().lock().await.rails_kept_by_index() else { return };
        for id in sessions {
            let hosting = self.store().lock().await.hosting(&id).ok().flatten();
            let Some(hosting) = hosting else { continue };
            let Some(workflow) = hosting.workflow.as_deref() else { continue };
            let Some(steps) = self.job_rail_steps(&id, &hosting.manifest_id, workflow).await else { continue };
            let ids = rail::step_ids(&steps);
            let mut store = self.store().lock().await;
            if let Ok(kept) = store.session_rail(&id, &ids) {
                if let Err(why) = store.keep_session_rail(&id, &kept, &ids) {
                    self.session_warned(&id, "the rail was not kept by step id", why);
                }
            }
        }
    }

    /// Draw what the rail adds to a hosted Session's row: the step it is on. **A Session that ended has no step.** A step that ends
    /// on his reply waits on him from `reply_owed_since`, the turn's end, where nothing else does.
    pub(crate) fn rail_drawn(
        &self,
        session: &KeptSession,
        reads: &RailReads,
        reply_owed_since: Option<&str>,
        record: &mut ipc::SessionRecord,
    ) {
        if record.hosted.is_none() {
            return;
        }
        record.workflow_visits =
            reads.visits.iter().map(|(step, since)| ipc::StepVisit { step: *step, since: Instant::carried(since) }).collect();
        record.skipped_steps = reads.skipped.clone();
        let own = match (session.manifest_id.as_deref(), record.workflow.as_deref(), reads.own.as_ref()) {
            (Some(manifest), Some(workflow), Some(own)) if own.workflow_id == workflow => {
                self.own_resolved(manifest, own).map(|found| (manifest.to_string(), found))
            }
            _ => None,
        };
        record.workflow_copy = own.as_ref().and_then(|(manifest, found)| {
            let served = self.repositories().serving(manifest)?;
            Some(crate::wire::workflow_summary(found, served.manifest().id(), &served.files()))
        });
        if session.state != store::SessionState::Live {
            return;
        }
        let (Some(manifest), Some(workflow)) = (session.manifest_id.as_deref(), record.workflow.as_deref()) else {
            return;
        };
        let project = record.hosted.as_ref().and_then(|hosted| hosted.project.as_deref());
        if !self.workflow_on(manifest, project, workflow) {
            return;
        }
        // The steps the rail was read against, so the indexes it kept are into these.
        let steps = &reads.steps;
        if steps.is_empty() {
            return;
        }
        let mut facts = Facts::of(record, reads.extra);
        facts.stood = reads.stood.map(|at| at as usize);
        facts.visited = StepBits::of(reads.visits.iter().map(|(at, _)| *at as usize));
        let mut lifecycles = Vec::new();
        if steps.iter().any(RailStep::has_lifecycle_end) {
            lifecycles = reads.lifecycles.clone().unwrap_or_else(|| self.lifecycles_that_apply(manifest));
            (facts.lifecycle_held, facts.lifecycle_met) = rail_lifecycle::bits(steps, &lifecycles, &reads.held);
        }
        facts.you_approved = StepBits::of(
            (0..steps.len()).filter(|at| steps[*at].waits_on_approval() && reads.approved(&steps[*at].id)),
        );
        let at = rail::current(steps, &facts);
        record.workflow_step = at.map(|at| at as u32);
        let name = |one: &store::KeptInstance| rail_lifecycle::named(one, reads.titles.get(&one.key).map(String::as_str));
        record.workflow_waiting_on = at.map(|at| rail_lifecycle::waiting(&steps[at], &lifecycles, &reads.held, &name)).unwrap_or_default();
        let waits_on_reply = at.is_some_and(|at| {
            rail_ends::ends_of(steps, at).into_iter().any(|end| end.waits_on() == Waiter::ThePerson(WaitingSource::Replied))
        });
        let turn_over = matches!(record.standing, Some(ipc::SessionStanding::Idle | ipc::SessionStanding::Replied));
        if let (true, true, true, Some(ended), Some(at)) = (waits_on_reply, turn_over, record.waiting_for.is_empty(), reply_owed_since, at) {
            record.waiting_for.push(crate::session_host::waiting::step_reply_item(&steps[at].label, ended));
        }
    }
}
