//! Where the rail and the judges meet the store: what a row needs is read back under the lock the
//! caller already holds, and a kept concern is drawn as the waiting item the wire carries.
//! `docs/concepts/session.md`, *The rail*, *The judges*. The rules are `rail`'s.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Ulid, WorkflowId};
use ipc::{
    AskedQuestion, ConcernFrom, ConcernHow, ConcernSettled, DraftHow, Instant, JudgeConcern, JudgeReading, QuestionAnswer,
    SessionRow, SessionVoice, WaitingAct, WaitingActKind, WaitingCite, WaitingItem, WaitingSource,
};
use store::{Holder, KeptConcern, KeptReading, KeptSession, KeptWorkflowNudge, Store, WriteError};

use crate::daemon::Fleet;
use crate::rail::{self, Extra, Facts, RailStep};
use crate::rail_ends::{self, Waiter};
use crate::rail_lifecycle::{self, Held, StepBits};
use crate::session_host::asking::{answers_said, ask_answered};
use crate::session_host::document_answers::approved_on_newest;
use crate::session_host::handoff_answers::{answered_later, ANSWERED_LATER};
use crate::session_host::sketching::sketch_approved;
use crate::session_host::walking::walk_approval;

/// The prefix of a concern's item id: `judge:<n>`.
pub(crate) const CONCERN_PREFIX: &str = "judge:";

/// How many nudges a step's list holds.
const RECENT_NUDGES: u32 = 20;

/// What a row's step and judge reading are derived from.
#[derive(Default)]
pub(crate) struct RailReads {
    pub extra: Extra,
    pub reading: Option<KeptReading>,
    /// When each step began, as the rail kept it.
    pub visits: Vec<(u32, String)>,
    /// The steps the person skipped.
    pub skipped: Vec<u32>,
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

/// Read what the rail needs beyond the record. **The commit, the Check run, the thread and the plan's
/// walk are looked for only where the Session follows a workflow**, since nothing else reads them.
pub(crate) fn rail_read(store: &Store, session: &KeptSession, follows: bool) -> Result<RailReads, WriteError> {
    let rail = store.session_rail(&session.id)?;
    let mut held = Held::default();
    let mut lifecycles = None;
    let extra = if follows {
        if store.is_lifecycle_job(&session.id)? {
            lifecycles = crate::lifecycles::copying::frozen_lifecycles(store, &session.id);
            held = Held { instances: store.lifecycle_instances_of(&session.id)?, moves: store.lifecycle_moves_of(&session.id)? };
        }
        let rows: Vec<SessionRow> = store
            .session_rows(&session.id)?
            .iter()
            .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
            .collect();
        let plan = store.session_plan(&session.id)?;
        let plan_approved_at = plan.as_ref().and_then(|plan| plan.approved_at.as_deref()).map(Instant::carried);
        Extra {
            committed: store.session_work(&session.id, &session.started_at)?.first_commit.is_some(),
            checked: store.session_has_run_a_check(&session.id)?,
            plan_expects_walk: plan.is_some_and(|plan| plan.seen || plan.walk.is_some()),
            messaged: messaged_on(&rows, &rail.visits),
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
    Ok(RailReads { extra, reading: store.session_judge_reading(&session.id)?, visits: rail.visits, skipped: rail.skipped, held, titles, lifecycles, own })
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
    let approvals: Vec<&Instant> = windows.iter().filter_map(|url| walk_approval(rows, url)).collect();
    let draft_approved = rows.iter().any(|row| {
        matches!(row, SessionRow::Draft { settled: Some(settled), .. } if matches!(settled.how, DraftHow::Approved | DraftHow::Fixed))
    });
    Extra {
        replied,
        walk_shown: !windows.is_empty(),
        walk_approved: plan_approved_at.is_some_and(|plan| approvals.iter().any(|at| *at >= plan)),
        any_walk_approved: !approvals.is_empty(),
        sketch_approved: sketch_approved(rows).is_some(),
        draft_approved,
        idea_picked: idea_picked(rows),
        ..Extra::default()
    }
}

/// Whether the person picked an option the Job's question gave a preview, on its card or later from
/// Needs you. Words of his own pick nothing, and Fleet's own questions are not the Job's ideas.
fn idea_picked(rows: &[SessionRow]) -> bool {
    let picked = |questions: &[AskedQuestion], answers: Vec<QuestionAnswer>| {
        answers.iter().any(|answer| {
            questions.iter().filter(|asked| asked.question == answer.question && !fleet_asked(asked)).any(|asked| {
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

/// A question Fleet put on the Job's card itself, the handoff's, which offers no ideas.
fn fleet_asked(asked: &AskedQuestion) -> bool {
    asked.question == ipc::HANDOFF_REMIND_QUESTION || asked.options.iter().any(|option| option.label == ipc::HANDOFF_YES)
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

    /// Keep a nudge against the step it was sent for, so the workflow's page can list it. A fault is
    /// the page's loss and not the Job's, so it is said and nothing more. **The place the definition
    /// came from, where the nudge was new**: a Job drifts from a step in one way once.
    pub(crate) async fn nudge_kept(
        &self,
        manifest: &str,
        workflow: &str,
        step: &RailStep,
        session: &str,
        skip: rail::Skip,
    ) -> Option<&'static str> {
        let source = self.repositories().serving(manifest).and_then(|served| {
            served.workflows().get(&WorkflowId::carried(Ulid::carried(workflow))).map(|found| found.source().as_wire())
        })?;
        let at = self.now().as_str().to_string();
        let kept = self.store().lock().await.keep_workflow_nudge(workflow, source, &step.id, session, skip.fact(), &at);
        match kept {
            Ok(new) => new.then_some(source),
            Err(why) => {
                self.session_warned(session, "the nudge was not kept against its step", why);
                None
            }
        }
    }

    /// The newest nudges sent for one step of a workflow, newest first, those marked not a drift
    /// included (they do not count). For the workflow's page.
    pub(crate) async fn step_nudges(&self, workflow: &str, source: &str, step: &str) -> Vec<KeptWorkflowNudge> {
        self.store().lock().await.workflow_nudges(workflow, source, step, RECENT_NUDGES).unwrap_or_default()
    }

    /// Mark a nudge not a drift, so it stops counting. Whether there was one.
    pub(crate) async fn nudge_not_a_drift(&self, id: i64) -> bool {
        let at = self.now().as_str().to_string();
        self.store().lock().await.dismiss_workflow_nudge(id, &at).unwrap_or(false)
    }

    /// Draw what the rail and the judges add to a hosted Session's row: the step it is on, whether a
    /// judge is reading, and the last reading. **A Session that ended has no step.** A step that ends
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
        record.judge = reads.reading.as_ref().map(reading_wire);
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
        record.judging = self.hosts().of(&session.id).state().judging > 0;
        let (Some(manifest), Some(workflow)) = (session.manifest_id.as_deref(), record.workflow.as_deref()) else {
            return;
        };
        if !self.workflow_on(manifest, workflow) {
            return;
        }
        let steps = match &own {
            Some((_, found)) => rail::steps_of(found),
            None => match self.rail_steps(manifest, workflow) {
                Some(steps) => steps,
                None => return,
            },
        };
        let mut facts = Facts::of(record, reads.extra);
        let mut lifecycles = Vec::new();
        if steps.iter().any(RailStep::has_lifecycle_end) {
            lifecycles = reads.lifecycles.clone().unwrap_or_else(|| self.lifecycles_that_apply(manifest));
            (facts.lifecycle_held, facts.lifecycle_met) = rail_lifecycle::bits(&steps, &lifecycles, &reads.held);
        }
        let at = rail::current(&steps, &facts);
        record.workflow_step = at.map(|at| at as u32);
        let name = |one: &store::KeptInstance| rail_lifecycle::named(one, reads.titles.get(&one.key).map(String::as_str));
        record.workflow_waiting_on = at.map(|at| rail_lifecycle::waiting(&steps[at], &lifecycles, &reads.held, &name)).unwrap_or_default();
        let waits_on_reply = at.is_some_and(|at| {
            rail_ends::ends_of(&steps, at).into_iter().any(|end| end.waits_on() == Waiter::ThePerson(WaitingSource::Replied))
        });
        let turn_over = matches!(record.standing, Some(ipc::SessionStanding::Idle | ipc::SessionStanding::Replied));
        if let (true, true, true, Some(ended), Some(at)) = (waits_on_reply, turn_over, record.waiting_for.is_empty(), reply_owed_since, at) {
            record.waiting_for.push(crate::session_host::waiting::step_reply_item(&steps[at].label, ended));
        }
    }
}
