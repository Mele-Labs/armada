//! The engine: what an occurrence sets off, given what the Job holds and the rules that apply.
//!
//! **Pure.** It reads a value in and returns values out; it writes nothing, asks nothing and
//! starts nothing. What comes back is a plan, and doing it, or raising an Asks item for it, is
//! Fleet's.
//!
//! **A rule is judged once for each occurrence, when it first sees it.** The key goes into
//! [`RanOnce`] whether or not the rule's conditions held and whether or not its autonomy lets it
//! plan anything, so a replay after a restart plans nothing a first reading did not, and a rule
//! turned on later does not reach back for a transition it was off for.

use alloc::vec::Vec;

use crate::lifecycle::{HeldInstances, InstanceRef, LifecycleId, LifecycleInstance, Standing};

use super::act::Act;
use super::autonomy::{AutonomyDefaults, Course};
use super::declared::Rule;
use super::failure::OnFailure;
use super::ids::RuleId;
use super::occurrence::{Occurrence, OccurrenceKey, RanOnce};
use super::select::{Among, Condition, JobCondition, JobFacts, Quantity, Target};

/// One act a rule plans, to one instance where it is done to instances.
///
/// **The course has no Off.** An act that is off is not planned, so nothing downstream can hold
/// one and forget to look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedAct {
    rule: RuleId,
    occurrence: OccurrenceKey,
    position: usize,
    act: Act,
    on: Option<InstanceRef>,
    course: Course,
    on_failure: OnFailure,
}

impl PlannedAct {
    pub fn rule(&self) -> &RuleId {
        &self.rule
    }

    /// The occurrence that set it off.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// Where the act stands in the rule's Do, from the first.
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn act(&self) -> &Act {
        &self.act
    }

    /// The instance it is done to. None for an act that is done to none.
    pub fn on(&self) -> Option<&InstanceRef> {
        self.on.as_ref()
    }

    /// Done at once, or asked about first.
    pub fn course(&self) -> Course {
        self.course
    }

    /// What becomes of it should it fail.
    pub fn on_failure(&self) -> OnFailure {
        self.on_failure
    }
}

/// The rules that apply, and the defaults an act's autonomy falls back on.
#[derive(Clone, Copy, Debug)]
pub struct Engine<'a> {
    rules: &'a [Rule],
    defaults: &'a AutonomyDefaults,
}

impl<'a> Engine<'a> {
    /// **No rules plan nothing.** A Job with none saved, or one that started before any existed,
    /// gets an empty plan for everything that happens to it.
    pub fn new(rules: &'a [Rule], defaults: &'a AutonomyDefaults) -> Engine<'a> {
        Engine { rules, defaults }
    }

    /// What `seen` sets off, in the order the rules and their acts are written. `held` is what
    /// the Job holds *now*: a condition reads where things stand and not where they stood.
    pub fn plan(&self, seen: &Occurrence, held: &HeldInstances, job: &JobFacts, ran: &mut RanOnce) -> Vec<PlannedAct> {
        let key = seen.key();
        let reading = Reading { entered: seen.entered_instance(), held, job };
        let mut planned = Vec::new();
        for rule in self.rules.iter().filter(|rule| seen.is(rule.when())) {
            if !ran.note(rule.id(), &key) || !rule.conditions().iter().all(|condition| reading.holds(condition)) {
                continue;
            }
            for (position, act) in rule.does().iter().enumerate() {
                let Some(course) = self.defaults.resolved(act.kind(), rule.autonomy()).course() else { continue };
                for on in reading.done_to(act) {
                    planned.push(PlannedAct {
                        rule: rule.id().clone(),
                        occurrence: key.clone(),
                        position,
                        act: act.clone(),
                        on,
                        course,
                        on_failure: rule.on_failure(),
                    });
                }
            }
        }
        planned
    }
}

/// What a rule can see: the instance that entered, what the Job holds and what it shows of itself.
struct Reading<'a> {
    entered: Option<&'a InstanceRef>,
    held: &'a HeldInstances,
    job: &'a JobFacts,
}

impl<'a> Reading<'a> {
    fn among(&self, among: &Among) -> Vec<&'a LifecycleInstance> {
        let of = |lifecycle: &LifecycleId, found: Vec<&'a LifecycleInstance>| -> Vec<&'a LifecycleInstance> {
            found.into_iter().filter(|one| one.subject().lifecycle() == lifecycle).collect()
        };
        match (among, self.entered) {
            (Among::Entered, Some(entered)) => self.held.get(entered).into_iter().collect(),
            (Among::LinkedFrom(lifecycle), Some(entered)) => of(lifecycle, self.held.linked(entered)),
            (Among::LinkingTo(lifecycle), Some(entered)) => of(lifecycle, self.held.linking(entered)),
            (Among::Held(lifecycle), _) => of(lifecycle, self.held.all().iter().collect()),
            (_, None) => Vec::new(),
        }
    }

    fn holds(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Instances { among, quantity, in_state } => {
                let chosen = self.among(among);
                let standing = Standing::In(in_state.clone());
                let inside = chosen.iter().filter(|one| one.standing() == &standing).count();
                match quantity {
                    Quantity::Every => !chosen.is_empty() && inside == chosen.len(),
                    Quantity::Any => inside > 0,
                    Quantity::No => inside == 0,
                }
            }
            Condition::Job(JobCondition::Workflow(workflow)) => self.job.workflow.as_ref() == Some(workflow),
            Condition::Job(JobCondition::AtStep(step)) => self.job.step.as_ref() == Some(step),
            Condition::Job(JobCondition::PullRequestOpened) => self.job.pull_request_opened,
        }
    }

    /// What `act` is done to, one entry each: the instances its target selects, or the single
    /// nothing of an act with no target.
    fn done_to(&self, act: &Act) -> Vec<Option<InstanceRef>> {
        let Some(Target { among, in_state }) = act.target() else { return alloc::vec![None] };
        let wanted = in_state.clone().map(Standing::In);
        self.among(among)
            .into_iter()
            .filter(|one| wanted.as_ref().is_none_or(|standing| one.standing() == standing))
            .map(|one| Some(one.subject().clone()))
            .collect()
    }
}
