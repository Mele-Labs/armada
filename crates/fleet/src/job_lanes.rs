//! A Job's lanes, worked out once for every client: `SessionRecord::lanes`.
//!
//! Pure. The input is what Fleet already holds of the Job (its `JobLifecycles`, the workflow it follows and
//! the step it is on, the rules of its repository, what the Jobs it started hold); the output is the
//! lanes and edges a screen draws, and the instances, acts and changes a panel opens from. This is a port
//! of what Bridge worked out in TypeScript, branch for branch and in the same order: `lanesOfJob` in
//! `packages/screens/src/session-lanes.ts` for `held`, and `jobLanes` in `LaneCanvas/build.ts` for
//! `lanes` and `rules`.
//!
//! **Departures, each because Fleet cannot or need not say it the way the TypeScript did.** A time is the
//! instant and not a clock, since Fleet knows no time zone. A rule edge carries what it waits for (`waits`)
//! and when it fired (`fired_at`) apart, where the TypeScript put either in one `meta`. What no real Fleet
//! ever sets is left off the wire: the mock's `triage`, a fired rule's `needs`, an instance's `assignee`
//! and `says`, a step's `endsOn`, and a rule's `ifs`, `does` and `scope`. `inFlight` is not here: the
//! record's `in_flight` is that list, read by Fleet from the store.

mod draw;
mod read;

use std::collections::HashMap;

use ipc::{AppliedRule, JobLanesHeld, JobLanesView, JobLifecycles, LaneChange, LaneCopy, LaneRule, LaneStep, StepVisit};

/// The workflow a Job follows and where it stands, as the rail reads it.
pub(crate) struct LaneWorkflow {
    pub name: String,
    /// Each step's id and label.
    pub steps: Vec<(String, String)>,
    /// The step the Job is on. Absent where Fleet names none.
    pub at: Option<u32>,
    /// Whether any step has begun, so a Job past its last step reads as done and not as not begun.
    pub began: bool,
    pub skipped: Vec<u32>,
    /// When each step began, so an act is placed on the step the Job was on when it was planned.
    pub visits: Vec<StepVisit>,
}

/// What the lanes are read from.
pub(crate) struct LaneInput<'a> {
    pub held: &'a JobLifecycles,
    /// Absent for a Job on no workflow, or with workflows off: its lanes have no top lane.
    pub workflow: Option<&'a LaneWorkflow>,
    /// The rules that apply to the Job's repository. **The rules the Job's copy holds win**
    /// (`JobLifecycles::rules`): they are the ones that run for it.
    pub rules: &'a [AppliedRule],
    /// What the Jobs this one started hold, by the id of each. A ticket handed to a Job is read from there.
    pub started: &'a HashMap<String, JobLifecycles>,
    /// The titles of the issues the Job holds and the Jobs it started do, by key.
    pub titles: &'a HashMap<String, String>,
    /// The agent is working on its step now.
    pub working: bool,
    /// `SessionRecord::workflow_waiting_on`.
    pub waiting: &'a [String],
}

/// The lanes a Job draws, or none where it holds no instance and follows no workflow with a step.
pub(crate) fn lanes_of_job(input: &LaneInput<'_>) -> Option<JobLanesView> {
    let held = input.held;
    let workflow = input.workflow;
    if held.instances.is_empty() && workflow.is_none_or(|one| one.steps.is_empty()) {
        return None;
    }
    let steps: Vec<LaneStep> = workflow
        .map(|one| {
            one.steps
                .iter()
                .enumerate()
                .map(|(col, (id, label))| LaneStep { id: id.clone(), label: label.clone(), col: col as u32 })
                .collect()
        })
        .unwrap_or_default();
    let applied: &[AppliedRule] = if held.rules.is_empty() { input.rules } else { &held.rules };
    let reads: Vec<read::RuleRead> = applied.iter().filter_map(|one| read::rule(one, &held.lifecycles)).collect();
    // A rule a Job event sets off is not drawn, and one the Job dropped is only the repository's now: a change names it from every rule either holds.
    let named: HashMap<&str, &str> =
        input.rules.iter().chain(held.rules.iter()).map(|one| (one.rule.id.as_str(), one.rule.name.as_str())).collect();
    let rules: Vec<LaneRule> = reads.iter().map(|one| one.rule.clone()).collect();
    let handed = read::handed_of(held);
    let given: Vec<usize> = handed.values().copied().collect();
    let lands_on: HashMap<String, String> =
        reads.iter().filter_map(|one| Some((one.rule.id.clone(), one.lands_on.clone()?))).collect();
    let visits: &[StepVisit] = workflow.map_or(&[], |one| one.visits.as_slice());
    let job = JobLanesHeld {
        copy: LaneCopy { steps, lifecycles: held.lifecycles.iter().map(read::lifecycle).collect(), rules },
        // A ticket handed away has no lane here: the card of the Job it went to carries it.
        instances: held
            .instances
            .iter()
            .enumerate()
            .filter(|(at, _)| !given.contains(at))
            .map(|(at, one)| {
                let ticket = handed.get(&at).map(|ticket| {
                    let ticket = &held.instances[*ticket];
                    read::carried(ticket, input.started.get(&one.key), held, input.titles.get(&ticket.key).map(String::as_str))
                });
                read::instance(one, &held.moves, ticket)
            })
            .collect(),
        fired: Vec::new(),
        changes: held
            .changes
            .iter()
            .map(|one| LaneChange {
                id: format!("{}:{}:{}", one.lane, one.item, one.at.as_str()),
                lane: one.lane.clone(),
                item: one.item.clone(),
                text: one.text.clone(),
                at: one.at.clone(),
                dropped: one.dropped,
                name: (one.lane == "rule").then(|| named.get(one.item.as_str()).map(|name| name.to_string())).flatten(),
            })
            .collect(),
        waiting: input.waiting.to_vec(),
    };
    let job = JobLanesHeld { fired: read::fired(&held.acts, &job.copy.rules, &lands_on, &job.copy.steps, visits), ..job };
    // `at` is -1 before a step starts and the step count once every one has ended; Fleet names no step in either case.
    let at = match workflow {
        Some(one) => one.at.map(|at| at as i32).unwrap_or(if one.began { job.copy.steps.len() as i32 } else { -1 }),
        None => -1,
    };
    let skipped = workflow.map(|one| one.skipped.clone()).unwrap_or_default();
    let working = input.working && workflow.is_some_and(|one| one.at.is_some());
    let name = workflow.map(|one| one.name.clone()).unwrap_or_default();
    let (lanes, edges) = draw::lanes(&job, &draw::Standing { workflow: &name, at, skipped: &skipped, working });
    Some(JobLanesView { workflow: name, at, working, skipped, job, lanes, rules: edges })
}
