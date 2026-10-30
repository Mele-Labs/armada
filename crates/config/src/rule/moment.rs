//! The When and the If of a rule file, and the two ideas they share with an act's target: which
//! instances, and in which state.

use core_model::rule::{Among, Condition, JobCondition, JobEvent, Quantity, Target, When, Words};
use core_model::{LifecycleId, StateId, StepId};
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::lifecycle::id_of;
use crate::yaml::{self, Table};

use super::{the_one, yes};

const WHEN_KEYS: &[&str] = &["step_starts", "step_ends", "job", "enters"];
const WHEN_WANTED: &str = "one of `step_starts`, `step_ends`, `job` or `enters`";
const ENTERS_KEYS: &[&str] = &["lifecycle", "state"];
const JOB_EVENTS: &[&str] = &["started", "pull_request_opened", "landed", "ended"];

const CONDITION_KEYS: &[&str] = &["every", "any", "no", "job"];
const CONDITION_WANTED: &str = "one of `every`, `any`, `no` or `job`";
const JOB_FACTS: &[&str] = &["workflow", "step", "pull_request_opened"];
const JOB_FACT_WANTED: &str = "one of `workflow`, `step` or `pull_request_opened`";

const AMONG_KEYS: &[&str] = &["entered", "linked_from", "linking_to", "held"];
const AMONG_WANTED: &str = "one of `entered`, `linked_from`, `linking_to` or `held`";
const INSTANCE_KEYS: &[&str] = &["entered", "linked_from", "linking_to", "held", "in"];

pub(super) fn when_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<When> {
    let mut table = Table::open(at, value, out)?;
    let one = the_one(&mut table, at, WHEN_KEYS, WHEN_WANTED, out);
    table.close(WHEN_KEYS, out);
    let (key, value) = one?;
    let at = format!("{at}.{key}");
    match key {
        "step_starts" => yaml::text(&at, value, out).map(|step| When::StepStarts(StepId::new(step))),
        "step_ends" => yaml::text(&at, value, out).map(|step| When::StepEnds(StepId::new(step))),
        "job" => {
            let event = yaml::text(&at, value, out)?;
            match JobEvent::from_wire(&event) {
                Some(event) => Some(When::Job(event)),
                None => {
                    out.push(Refusal::new(at, Fault::NotInTheSchema { value: event, legal: JOB_EVENTS }));
                    None
                }
            }
        }
        _ => {
            let mut inner = Table::open(&at, value, out)?;
            let lifecycle = inner
                .required("lifecycle", out)
                .and_then(|value| id_of(&inner.at("lifecycle"), value, LifecycleId::new, out));
            let state =
                inner.required("state", out).and_then(|value| id_of(&inner.at("state"), value, StateId::new, out));
            inner.close(ENTERS_KEYS, out);
            Some(When::Enters { lifecycle: lifecycle?, state: state? })
        }
    }
}

/// Which instances, from the one `among` key a table holds. **A key that names a lifecycle takes
/// its id; `entered` takes `true`**, since it names no lifecycle and is the instance itself.
fn among_of(table: &mut Table<'_>, at: &str, out: &mut Vec<Refusal>) -> Option<Among> {
    let (key, value) = the_one(table, at, AMONG_KEYS, AMONG_WANTED, out)?;
    let at = table.at(key);
    if key == "entered" {
        return yes(&at, value, out).map(|()| Among::Entered);
    }
    let lifecycle = id_of(&at, value, LifecycleId::new, out)?;
    Some(match key {
        "linked_from" => Among::LinkedFrom(lifecycle),
        "linking_to" => Among::LinkingTo(lifecycle),
        _ => Among::Held(lifecycle),
    })
}

/// What an act is done to: the `among` key, and `in` for a state to narrow it to.
pub(super) fn target_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Target> {
    let mut table = Table::open(at, value, out)?;
    let among = among_of(&mut table, at, out);
    let in_state = match table.optional("in") {
        None => Some(None),
        Some(value) => id_of(&table.at("in"), value, StateId::new, out).map(Some),
    };
    table.close(INSTANCE_KEYS, out);
    Some(Target { among: among?, in_state: in_state? })
}

pub(super) fn condition_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Condition> {
    let mut table = Table::open(at, value, out)?;
    let one = the_one(&mut table, at, CONDITION_KEYS, CONDITION_WANTED, out);
    table.close(CONDITION_KEYS, out);
    let (key, value) = one?;
    let at = format!("{at}.{key}");
    if key == "job" {
        return job_condition_of(&at, value, out).map(Condition::Job);
    }
    let quantity = Quantity::from_wire(key)?;
    let mut inner = Table::open(&at, value, out)?;
    let among = among_of(&mut inner, &at, out);
    let in_state = inner.required("in", out).and_then(|value| id_of(&inner.at("in"), value, StateId::new, out));
    inner.close(INSTANCE_KEYS, out);
    Some(Condition::Instances { among: among?, quantity, in_state: in_state? })
}

fn job_condition_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<JobCondition> {
    let mut table = Table::open(at, value, out)?;
    let one = the_one(&mut table, at, JOB_FACTS, JOB_FACT_WANTED, out);
    table.close(JOB_FACTS, out);
    let (key, value) = one?;
    let at = format!("{at}.{key}");
    match key {
        "workflow" => yaml::text(&at, value, out).and_then(|written| Words::new(&written).ok()).map(JobCondition::Workflow),
        "step" => yaml::text(&at, value, out).map(|step| JobCondition::AtStep(StepId::new(step))),
        _ => yes(&at, value, out).map(|()| JobCondition::PullRequestOpened),
    }
}
