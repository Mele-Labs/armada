//! Autopilot's apparatus: a backlog as the tracker lists it, the workflows a
//! picker chooses from, and a model that answers what the test wrote.
//!
//! The model is `FakeJudge`, which renders a shell rather than calling a
//! vendor, so the reading and the picking go through Fleet's own runner.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Model, OpenIssue};
use config::ResolvedWorkflow;
use core_model::{JobStatus, Timestamp, WorkflowId};
use fleet::autopilot::{Asking, Standing};
use fleet::JudgeBudget;
use testkit::{FakeJudge, FakeTracker};

/// What the owner types at 11pm.
pub const SENTENCE: &str = "work through the open issues that don't need design decisions, \
                            no more than two at a time, until 7am, $30 max";

/// 11pm on the owner's clock, four hours behind UTC.
pub const LOCAL_NOW: &str = "2026-10-11T23:00:00-04:00";
pub const NOW: &str = "2026-10-12T03:00:00Z";

/// What the sentence is read as.
pub const READ_AS: &str = "instruction: work through the open issues that don't need design decisions\n\
                           at_once: 2\n\
                           until: 2026-10-12T07:00:00-04:00\n\
                           spend: 30\n\
                           jobs: none";

pub fn now() -> Timestamp {
    Timestamp::from_rfc3339(NOW)
}

/// A model that answers each call with the next of `replies`.
pub fn asking(replies: &[&str]) -> (Asking, Arc<FakeJudge>) {
    let model = Arc::new(FakeJudge::answering_in_turn(replies));
    let asking = Asking {
        client: model.clone(),
        budget: JudgeBudget::of(Duration::from_secs(20)),
        model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
    };
    (asking, model)
}

/// The backlog: a flaky test, a wrong label, a design call, one a pull
/// request already closes, and one a Job is already on.
pub fn the_backlog() -> FakeTracker {
    let mut closed_by_a_pull = FakeTracker::issue(14, "Rename the settings key");
    closed_by_a_pull.linked_pull_request = true;
    FakeTracker::listing(vec![
        FakeTracker::issue(11, "The reader test sleeps 50ms and still flakes"),
        FakeTracker::issue(12, "The Board labels a killed Job as failed"),
        FakeTracker::issue(13, "Should Helm keep one conversation per repository?"),
        closed_by_a_pull,
        FakeTracker::issue(15, "The merge line drops a queued entry on restart"),
    ])
}

/// What the tracker lists, as the picker is handed it.
pub fn listed(tracker: &FakeTracker) -> Vec<OpenIssue> {
    use adapter_traits::Tracker;
    tracker.open_issues("/repos/storefront").expect("the fake lists")
}

pub fn url_of(number: u64) -> String {
    format!("https://forge.invalid/o/r/issues/{number}")
}

fn named(id: &str) -> ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: {id}\nname: {id}\nsteps:\n  - id: only\n    \
             label: \"Do it\"\n    evidence: {{submitted: {{type: diff}}}}\n    \
             delivers: true\n    advance_gate: auto\n"
        ),
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = config::Manifest::parse(
        std::path::Path::new("fixture-armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the fixture manifest parses");
    ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

/// Three workflows, because choosing right from one proves nothing.
pub fn the_workflows() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    ["bug", "feature", "plan-first"]
        .into_iter()
        .map(named)
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect()
}

/// One of the Autopilot's Jobs, as room is counted over it.
pub fn standing(status: JobStatus, waiting_on_a_person: bool, ever_ran: bool) -> Standing {
    Standing {
        status,
        waiting_on_a_person,
        ever_ran,
    }
}
