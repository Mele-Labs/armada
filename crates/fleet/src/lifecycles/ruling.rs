//! Running the rules a Job started with: what happens to it is handed to the engine, and what the
//! engine plans is kept.
//!
//! **Four things are heard.** An instance moved (every reader comes through
//! [`Fleet::lifecycle_seen`]), a gate of the Job's plan closed, a step started or ended, and the Job
//! did something (opened its pull request, landed, ended). The Job's own events are found by
//! comparing two looks at it, from the tap that already hears every Job's row change ([`looking`]).
//! **A step moment is a gate row's**: a group's stretch ends when its gate closes and the next one
//! starts, and the first starts when the plan is first recorded.
//!
//! **Planned in one hold of the store, with the pairs that made the plan**, so a Fleet that stops
//! between hearing and doing neither loses an act nor plans it twice. Doing it is [`super::acting`]'s.
//!
//! **A Job with no copy is never planned for**: the check is a row, and no file is read for it.

use core_model::rule::{ActKind, Course, Engine, Gate, JobEvent, JobFacts, Occurrence, RanOnce, RuleId, Words};
use core_model::{InstanceRef, Lifecycle, Moved, StepId, Standing};
use ipc::{SessionRecord, SessionState};
use store::{rule_act_standing as standing, KeptGate, KeptPlanGroup, KeptSeen, NewRuleAct, Store, WriteError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::copying::copy_of;
use super::holding::Transition;
use super::looking::{happened_between, same_look, Happened};
use super::saying::said;
use super::taking_on::{landed, opened_pull_request};
use super::writing::Writes;
use crate::daemon::Fleet;

/// What the Job shows of itself, from the look at it and the workflow it follows.
fn facts_from(look: &KeptSeen, workflow: Option<&str>) -> JobFacts {
    JobFacts {
        workflow: workflow.and_then(|slug| Words::new(slug).ok()),
        step: look.step.as_deref().map(StepId::new),
        pull_request_opened: look.pull_request_opened,
    }
}

/// The facts of the Job as the last look left them.
fn facts_of(store: &Store, session: &str) -> Result<JobFacts, WriteError> {
    let look = store.job_seen(session)?.unwrap_or_default();
    let workflow = store.hosting(session)?.and_then(|hosting| hosting.workflow);
    Ok(facts_from(&look, workflow.as_deref()))
}

fn course_wire(course: Course) -> &'static str {
    match course {
        Course::Acts => "acts",
        Course::Asks => "asks",
    }
}

/// Hand `seen` to the Job's rules, and keep what they plan. The ids of the acts kept. **Nothing for
/// a Job that has no rules**, and nothing for a rule that has seen this occurrence already.
///
/// **An act the Job's repository cannot make is kept as not carried, with the reason**, and raises
/// nothing, so the owner is never asked to press what pressing cannot do. `writes` says which.
pub(crate) fn plan_and_keep(
    store: &mut Store,
    session: &str,
    seen: &Occurrence,
    facts: &JobFacts,
    at: &str,
    writes: &Writes,
    built_in: Vec<Lifecycle>,
) -> Result<Vec<i64>, WriteError> {
    let Some(copy) = copy_of(store, session, built_in).filter(|copy| !copy.rules.is_empty()) else { return Ok(Vec::new()) };
    let held = super::holding::rebuilt(&copy.lifecycles, &store.lifecycle_instances_of(session)?);
    let key = seen.key();
    let written = key.to_string();
    let before: RanOnce = store
        .rules_that_ran(session, &written)?
        .iter()
        .filter_map(|rule| RuleId::new(rule).ok())
        .map(|rule| (rule, key.clone()))
        .collect();
    let mut ran = before.clone();
    let planned = Engine::new(&copy.rules, &copy.defaults).plan(seen, &held, facts, &mut ran);
    let newly: Vec<&str> = ran.iter().filter(|(rule, _)| !before.has(rule, &key)).map(|(rule, _)| rule.as_str()).collect();
    if newly.is_empty() {
        return Ok(Vec::new());
    }

    let entered = seen.entered_instance().map(|one| (one.lifecycle().as_str(), one.key()));
    let lines: Vec<String> = planned.iter().map(|one| said(one.act(), one.on(), &copy.lifecycles)).collect();
    let carried: Vec<Result<(), String>> = planned
        .iter()
        .map(|one| writes.carries(one.act(), one.on().map(|on| (on.lifecycle().as_str(), on.key())), &copy.lifecycles))
        .collect();
    let acts: Vec<NewRuleAct<'_>> = planned
        .iter()
        .zip(&lines)
        .zip(&carried)
        .map(|((one, line), carried)| NewRuleAct {
            rule_id: one.rule().as_str(),
            position: one.position() as u32,
            on: one.on().map(|on| (on.lifecycle().as_str(), on.key())),
            entered,
            course: course_wire(one.course()),
            state: match (carried, one.course()) {
                (Err(_), _) => standing::NOT_CARRIED,
                // A draft waits for his press whatever the rule says, so proposing it is the ask.
                (Ok(()), _) if matches!(one.act().kind(), ActKind::Draft) => standing::PLANNED,
                (Ok(()), Course::Acts) => standing::PLANNED,
                (Ok(()), Course::Asks) => standing::ASKED,
            },
            text: line,
            // Why it is not carried, the one sentence the act keeps.
            said: carried.as_ref().err().map_or("", String::as_str),
        })
        .collect();
    // A not carried act is among them: it settled as it was kept, and the Job is drawn again for it.
    store.keep_rule_plan(session, &written, &newly, &acts, at)
}

/// Whether the group names a gate, as the plan keeps it: absent and `none` are the same.
fn gated(group: &KeptPlanGroup) -> bool {
    group.gate.as_deref().is_some_and(|word| word != "none")
}

/// The groups from the one after `after` (the first, where there is none) up to and including the
/// next group with a gate, or to the end of the plan where no more is gated.
fn stretch_after(groups: &[KeptPlanGroup], after: Option<usize>) -> impl Iterator<Item = &KeptPlanGroup> {
    let from = after.map_or(0, |at| at + 1);
    let to = groups.iter().skip(from).position(gated).map_or(groups.len(), |at| from + at + 1);
    groups.get(from..to).unwrap_or_default().iter()
}

/// What one closed gate makes happen to the Job, in order: its stretch's steps end, the gate's kind
/// closes, the next stretch's steps start. **Nothing for a group the plan no longer has.** **A walk gate
/// closed again** (a new walk reopened it and he approved that) says only that the gate closed: its
/// steps ended and the next stretch started at the first close, and are not ended or started twice.
pub(crate) fn gate_moments(groups: &[KeptPlanGroup], row: &KeptGate) -> Vec<Occurrence> {
    let Some(at) = groups.iter().position(|group| group.id == row.group_id) else { return Vec::new() };
    let again = row.gate == store::WALK_APPROVED && row.version > 0;
    let previous = groups[..at].iter().rposition(gated);
    let stamp = |kind: &str, group: &str| format!("gate:{}:{}:{}:{kind}:{group}", row.group_id, row.gate, row.version);
    let mut moments: Vec<Occurrence> = stretch_after(&groups[..=at], previous)
        .filter(|_| !again)
        .map(|group| Occurrence::step_ended(StepId::new(&group.id), &stamp("ends", &group.id)))
        .collect();
    if let Some(gate) = Gate::from_wire(&row.gate) {
        moments.push(Occurrence::gate_closed(gate, &stamp("closes", &row.group_id)));
    }
    moments.extend(
        stretch_after(groups, Some(at))
            .filter(|_| !again)
            .map(|group| Occurrence::step_started(StepId::new(&group.id), &stamp("starts", &group.id))),
    );
    moments
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
    /// Freeze the lifecycles and rules that apply to the Job's repository, and tell its rules the
    /// Job started. **Nothing here fails the start**: a Job whose copy could not be kept has none,
    /// and runs as a Job from before rules did.
    pub(crate) async fn job_copy_frozen(&self, session: &str, manifest: &str) {
        let project = self.project_of_job(session).await;
        let Ok(copy) = self.job_copy_for(manifest, project.as_deref()) else { return };
        let at = self.now().as_str().to_string();
        let writes = self.writes_in(Some(manifest), project.as_deref());
        let kept = {
            let mut store = self.store().lock().await;
            if !store.freeze_job_copy(session, &at, &copy).unwrap_or(false) {
                return;
            }
            let workflow = store.hosting(session).ok().flatten().and_then(|hosting| hosting.workflow);
            let facts = facts_from(&KeptSeen::default(), workflow.as_deref());
            let started = Occurrence::job(JobEvent::Started, "started");
            let built_in = self.built_in_lifecycles_in(Some(manifest), project.as_deref());
            plan_and_keep(&mut store, session, &started, &facts, &at, &writes, built_in).unwrap_or_default()
        };
        self.rules_published(session, &kept).await;
    }

    /// An instance of the Job moved: hand its rules the move a reader just made.
    pub(crate) async fn rules_heard_move(&self, session: &str, transition: &Transition, move_id: i64) {
        let stamp = format!("{}#{move_id}", transition.at);
        let seen = Occurrence::entered(transition.instance.clone(), transition.moved.clone(), &stamp);
        self.rules_heard(session, &seen).await;
    }

    /// Hand `seen` to the Job's rules, with the Job as the last look left it.
    pub(crate) async fn rules_heard(&self, session: &str, seen: &Occurrence) {
        let at = self.now().as_str().to_string();
        let kept = {
            let mut store = self.store().lock().await;
            if !store.has_job_copy(session).unwrap_or(false) {
                return;
            }
            let Ok(facts) = facts_of(&store, session) else { return };
            let writes = self.writes_for_job(&store, session);
            let built_in = self.built_in_lifecycles_of_job(&store, session);
            plan_and_keep(&mut store, session, seen, &facts, &at, &writes, built_in).unwrap_or_default()
        };
        self.rules_published(session, &kept).await;
    }

    /// Hand every live Job's rules the moves its instances made that they have not seen. **For a
    /// Fleet that stopped between keeping a move and hearing it**: a move a rule has seen is found
    /// in what it ran for and plans nothing, so this is safe to run on every start.
    pub(crate) async fn rules_caught_up(&self) {
        let at = self.now().as_str().to_string();
        let jobs = self.store().lock().await.live_job_copies().unwrap_or_default();
        for session in jobs {
            let kept = {
                let mut store = self.store().lock().await;
                let Some(copy) = copy_of(&store, &session, self.built_in_lifecycles_of_job(&store, &session)) else { continue };
                let (Ok(moves), Ok(facts)) = (store.lifecycle_moves_of(&session), facts_of(&store, &session)) else { continue };
                let writes = self.writes_for_job(&store, &session);
                let mut kept = Vec::new();
                for one in moves {
                    let found = copy.lifecycles.iter().find(|lifecycle| lifecycle.id().as_str() == one.lifecycle);
                    let instance = found.and_then(|lifecycle| InstanceRef::of(lifecycle, &one.key).ok());
                    let moved = Standing::from_wire(&one.from).zip(Standing::from_wire(&one.to));
                    let Some((instance, (from, to))) = instance.zip(moved) else { continue };
                    let seen = Occurrence::entered(instance, Moved { from, to }, &format!("{}#{}", one.at, one.id));
                    let built_in = self.built_in_lifecycles_of_job(&store, &session);
                    kept.extend(plan_and_keep(&mut store, &session, &seen, &facts, &at, &writes, built_in).unwrap_or_default());
                }
                kept
            };
            self.rules_published(&session, &kept).await;
        }
    }

    /// The row changed: find what happened to the Job since the last look, and hand it to its rules.
    pub(crate) async fn rules_looked_at(&self, record: &SessionRecord) {
        let session = record.id.as_str();
        // The Job's own steps, so a step it added or moved is the one its rules read.
        let step = match (record.manifest_id.as_ref(), record.workflow.as_deref(), record.workflow_step) {
            (Some(manifest), Some(workflow), Some(at)) => self
                .job_rail_steps(session, manifest.as_str(), workflow)
                .await
                .and_then(|steps| steps.get(at as usize).map(|one| one.id.clone())),
            _ => None,
        };
        let now = KeptSeen {
            step: None,
            pull_request_opened: opened_pull_request(&record.attachments),
            landed: landed(&record.attachments),
            ended: record.state == SessionState::Ended,
            events: 0,
        };
        let at = self.now().as_str().to_string();
        let kept = {
            let mut store = self.store().lock().await;
            if !store.has_job_copy(session).unwrap_or(false) {
                return;
            }
            let before = store.job_seen(session).ok().flatten().unwrap_or_default();
            if same_look(&before, &now) {
                return;
            }
            // The step is read for an `if` and is never kept or compared: a look is kept without it.
            let facts = facts_from(&KeptSeen { step, ..now.clone() }, record.workflow.as_deref());
            let writes = self.writes_for_job(&store, session);
            let mut kept = Vec::new();
            let mut counted = before.events;
            for happened in happened_between(&before, &now) {
                counted += 1;
                let stamp = format!("look-{counted}");
                let seen = match happened {
                    Happened::PullRequestOpened => Occurrence::job(JobEvent::PullRequestOpened, &stamp),
                    Happened::Landed => Occurrence::job(JobEvent::Landed, &stamp),
                    Happened::Ended => Occurrence::job(JobEvent::Ended, &stamp),
                };
                let built_in = self.built_in_lifecycles_of_job(&store, session);
                kept.extend(plan_and_keep(&mut store, session, &seen, &facts, &at, &writes, built_in).unwrap_or_default());
            }
            let _ = store.keep_job_seen(session, &KeptSeen { events: counted, ..now });
            kept
        };
        self.rules_published(session, &kept).await;
    }

    /// Gates of the Job's plan closed just now (`closed` are rows written now, and none else). For each,
    /// in order: every group of the stretch it ends has its step end, the gate's kind closes, and every
    /// group of the next stretch has its step start. **A stretch is the groups from just after the
    /// previous gated group to this one**, so a group with no gate runs on into the next gated one.
    ///
    /// Each stamp names the row it came from, so a second hearing of one close plans nothing.
    pub(crate) async fn rules_gates_closed(&self, session: &str, closed: &[KeptGate]) {
        let at = self.now().as_str().to_string();
        let kept = {
            let mut store = self.store().lock().await;
            if !store.has_job_copy(session).unwrap_or(false) {
                return;
            }
            let Some(plan) = store.session_plan(session).ok().flatten() else { return };
            let seen: Vec<Occurrence> = closed.iter().flat_map(|row| gate_moments(&plan.groups, row)).collect();
            self.rules_planned(&mut store, session, &seen, &at)
        };
        self.rules_published(session, &kept).await;
    }

    /// The first version of a plan was recorded: the first stretch starts.
    pub(crate) async fn rules_plan_first_recorded(&self, session: &str, groups: &[KeptPlanGroup]) {
        let at = self.now().as_str().to_string();
        let kept = {
            let mut store = self.store().lock().await;
            if !store.has_job_copy(session).unwrap_or(false) {
                return;
            }
            let seen: Vec<Occurrence> = stretch_after(groups, None)
                .map(|group| Occurrence::step_started(StepId::new(&group.id), &format!("plan:first:{}", group.id)))
                .collect();
            self.rules_planned(&mut store, session, &seen, &at)
        };
        self.rules_published(session, &kept).await;
    }

    /// Hand each of `seen` to the Job's rules, in one hold of the store. The ids of the acts kept.
    fn rules_planned(&self, store: &mut Store, session: &str, seen: &[Occurrence], at: &str) -> Vec<i64> {
        let Ok(facts) = facts_of(store, session) else { return Vec::new() };
        let writes = self.writes_for_job(store, session);
        let mut kept = Vec::new();
        for one in seen {
            let built_in = self.built_in_lifecycles_of_job(store, session);
            kept.extend(plan_and_keep(store, session, one, &facts, at, &writes, built_in).unwrap_or_default());
        }
        kept
    }

    /// Draw the Job again where its rules kept something, so an Asks item reaches Bridge and the
    /// Workflow view shows an act at once, and wake whoever does what was planned.
    async fn rules_published(&self, session: &str, kept: &[i64]) {
        if !kept.is_empty() {
            self.rule_acts().wake();
            let _ = self.published_hosted(session).await;
            self.job_lifecycles_published(session).await;
        }
    }
}
