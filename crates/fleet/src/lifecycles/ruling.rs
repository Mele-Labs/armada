//! Running the rules a Job started with: what happens to it is handed to the engine, and what the
//! engine plans is kept.
//!
//! **Three things are heard.** An instance moved (every reader comes through
//! [`Fleet::lifecycle_seen`]), a step started or ended, and the Job did something (opened its
//! pull request, landed, ended). The last two are found by comparing two looks at the Job, from the
//! tap that already hears every Job's row change ([`looking`]).
//!
//! **Planned in one hold of the store, with the pairs that made the plan**, so a Fleet that stops
//! between hearing and doing neither loses an act nor plans it twice. Doing it is [`super::acting`]'s.
//!
//! **A Job with no copy is never planned for**: the check is a row, and no file is read for it.

use core_model::rule::{ActKind, Course, Engine, JobEvent, JobFacts, Occurrence, RanOnce, RuleId, Words};
use core_model::{InstanceRef, Moved, StepId, Standing};
use ipc::{SessionRecord, SessionState};
use store::{rule_act_standing as standing, KeptSeen, NewRuleAct, Store, WriteError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::copying::copy_of;
use super::holding::Transition;
use super::looking::{happened_between, same_look, Happened};
use super::saying::said;
use super::taking_on::{landed, opened_pull_request};
use crate::daemon::Fleet;

/// Whether this Fleet does an act of `kind` itself. **A kind it does not is planned and recorded,
/// and raises nothing**, so the owner is never asked to press what pressing cannot do.
pub fn carried_here(kind: ActKind) -> bool {
    matches!(kind, ActKind::StartWatch | ActKind::TellAgent | ActKind::StartJob | ActKind::Draft)
}

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

/// Hand `seen` to the Job's rules, and keep what they plan. The ids of the acts kept that are
/// waiting to be done or asked about. **Nothing for a Job that has no rules**, and nothing for a
/// rule that has seen this occurrence already.
pub(crate) fn plan_and_keep(
    store: &mut Store,
    session: &str,
    seen: &Occurrence,
    facts: &JobFacts,
    at: &str,
) -> Result<Vec<i64>, WriteError> {
    let Some(copy) = copy_of(store, session).filter(|copy| !copy.rules.is_empty()) else { return Ok(Vec::new()) };
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
    let acts: Vec<NewRuleAct<'_>> = planned
        .iter()
        .zip(&lines)
        .map(|(one, line)| NewRuleAct {
            rule_id: one.rule().as_str(),
            position: one.position() as u32,
            on: one.on().map(|on| (on.lifecycle().as_str(), on.key())),
            entered,
            course: course_wire(one.course()),
            state: match (carried_here(one.act().kind()), one.course()) {
                (false, _) => standing::NOT_CARRIED,
                (true, Course::Acts) => standing::PLANNED,
                (true, Course::Asks) => standing::ASKED,
            },
            text: line,
            said: "",
        })
        .collect();
    let ids = store.keep_rule_plan(session, &written, &newly, &acts, at)?;
    let waiting = ids.iter().zip(&acts).filter(|(_, act)| act.state != standing::NOT_CARRIED).map(|(id, _)| *id).collect();
    Ok(waiting)
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
        let Ok(copy) = self.job_copy_for(manifest) else { return };
        let at = self.now().as_str().to_string();
        let kept = {
            let mut store = self.store().lock().await;
            if !store.freeze_job_copy(session, &at, &copy).unwrap_or(false) {
                return;
            }
            let workflow = store.hosting(session).ok().flatten().and_then(|hosting| hosting.workflow);
            let facts = facts_from(&KeptSeen::default(), workflow.as_deref());
            let started = Occurrence::job(JobEvent::Started, "started");
            plan_and_keep(&mut store, session, &started, &facts, &at).unwrap_or_default()
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
            plan_and_keep(&mut store, session, seen, &facts, &at).unwrap_or_default()
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
                let Some(copy) = copy_of(&store, &session) else { continue };
                let (Ok(moves), Ok(facts)) = (store.lifecycle_moves_of(&session), facts_of(&store, &session)) else { continue };
                let mut kept = Vec::new();
                for one in moves {
                    let found = copy.lifecycles.iter().find(|lifecycle| lifecycle.id().as_str() == one.lifecycle);
                    let instance = found.and_then(|lifecycle| InstanceRef::of(lifecycle, &one.key).ok());
                    let moved = Standing::from_wire(&one.from).zip(Standing::from_wire(&one.to));
                    let Some((instance, (from, to))) = instance.zip(moved) else { continue };
                    let seen = Occurrence::entered(instance, Moved { from, to }, &format!("{}#{}", one.at, one.id));
                    kept.extend(plan_and_keep(&mut store, &session, &seen, &facts, &at).unwrap_or_default());
                }
                kept
            };
            self.rules_published(&session, &kept).await;
        }
    }

    /// The row changed: find what happened to the Job since the last look, and hand it to its rules.
    pub(crate) async fn rules_looked_at(&self, record: &SessionRecord) {
        let session = record.id.as_str();
        let step = record.manifest_id.as_ref().zip(record.workflow.as_deref()).zip(record.workflow_step).and_then(
            |((manifest, workflow), at)| {
                self.rail_steps(manifest.as_str(), workflow)?.get(at as usize).map(|one| one.id.clone())
            },
        );
        let now = KeptSeen {
            step,
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
            let facts = facts_from(&now, record.workflow.as_deref());
            let mut kept = Vec::new();
            let mut counted = before.events;
            for happened in happened_between(&before, &now) {
                counted += 1;
                let stamp = format!("look-{counted}");
                let seen = match happened {
                    Happened::StepEnded(step) => Occurrence::step_ended(StepId::new(step), &stamp),
                    Happened::StepStarted(step) => Occurrence::step_started(StepId::new(step), &stamp),
                    Happened::PullRequestOpened => Occurrence::job(JobEvent::PullRequestOpened, &stamp),
                    Happened::Landed => Occurrence::job(JobEvent::Landed, &stamp),
                    Happened::Ended => Occurrence::job(JobEvent::Ended, &stamp),
                };
                kept.extend(plan_and_keep(&mut store, session, &seen, &facts, &at).unwrap_or_default());
            }
            let _ = store.keep_job_seen(session, &KeptSeen { events: counted, ..now });
            kept
        };
        self.rules_published(session, &kept).await;
    }

    /// Draw the Job again where its rules planned something that waits, so an Asks item reaches
    /// Bridge, and wake whoever does what was planned.
    async fn rules_published(&self, session: &str, kept: &[i64]) {
        if !kept.is_empty() {
            let _ = self.published_hosted(session).await;
        }
    }
}
