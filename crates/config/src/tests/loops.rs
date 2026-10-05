//! `verdict_routing` and `iteration_cap`: the back edge and its bound.
//!
//! **`verdict_routing` is the only declaration of a back edge.** There is no
//! workflow-level label to compare it with, so a step that routes needs only
//! name an earlier step, and the bound on how often is the step's own cap or
//! [`DEFAULT_ITERATION_CAP`].

use core_model::{GateVerdict, StepId};

use crate::error::{BadReturn, Fault, LoadError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{fault_at, named, refusals, roster};
use crate::workflow::WorkflowDef;

/// A two-step design loop: `draft`, then a human gate that sends it back.
const DESIGN_PLAN: &str = "
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
    iteration_cap: 5
";

/// A Manifest with no Checks, which is all these steps need — none of them
/// names one, and the cross-file question is not what this file is about.
fn plain_manifest() -> Manifest {
    Manifest::parse(&named("armada.yml"), "version: 1\nid: armada\nchecks: {}\n")
        .expect("a manifest declaring no checks")
}

fn parsed(steps: &str) -> Result<WorkflowDef, LoadError> {
    WorkflowDef::parse(
        &named("workflows/design-plan.yml"),
        &format!(
            "version: 1\nworkflow_id: design_plan\nname: design_plan\nsteps:{steps}"
        ),
        &roster(),
    )
}

/// The shape `workflows.toml` calls Armada's only instantiated loop, loading
/// whole for the first time: `draft -> present`, with `request_changes` routing
/// back to `draft` and a cap of five on how often.
///
/// A return needs a verdict, which needs a Judge or a human gate, and both
/// exist: `human_always` is a carried gate that `fleet::gate` holds a step at,
/// and a panel runs from `judge_checks`.
#[test]
fn a_loop_carries_its_edge_and_its_cap_onto_the_step() {
    let def = parsed(DESIGN_PLAN).expect("the design loop");
    let present = &def.steps()[1];

    assert_eq!(
        present.verdict_routing().get(&GateVerdict::RequestChanges),
        Some(&StepId::new("draft".to_string()))
    );
    assert_eq!(present.iteration_cap(), Some(5));
}

/// **The edge lives on the step, not on the workflow**, which is what lets one
/// definition carry more than one loop. The step that does not emit a verdict
/// carries nothing, and that is what makes its only exit forward.
#[test]
fn a_step_that_emits_no_verdict_routes_nowhere() {
    let def = parsed(DESIGN_PLAN).expect("the design loop");
    assert!(def.steps()[0].verdict_routing().is_empty());
    assert_eq!(def.steps()[0].iteration_cap(), None);
}

/// **Absent is none, and none is not a number this parser invents.** The schema
/// defaults the cap from `default_gate_policy.iteration_cap`, that block is
/// still refused as deferred, and a ceiling invented here would be a bound on a
/// Job written where nobody reading the workflow would find it.
#[test]
fn a_loop_may_declare_an_edge_and_no_cap() {
    let def = parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
",
    )
    .expect("an uncapped loop");
    assert_eq!(def.steps()[1].iteration_cap(), None);
}

/// Zero is legal, for the reason `retry_limit: 0` is: a step saying its first
/// `request_changes` is its last is a sentence an author is entitled to write.
#[test]
fn a_cap_of_zero_is_a_sentence_an_author_may_write() {
    let def = parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
    iteration_cap: 0
",
    )
    .expect("a loop that goes round once");
    assert_eq!(def.steps()[1].iteration_cap(), Some(0));
}

/// A cap that is not a number is refused rather than read as absent. The file
/// meant to bound the loop, and reading it as unbounded would be the parser
/// deciding how long a Job may go round.
#[test]
fn a_cap_that_is_not_a_number_is_refused() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
    iteration_cap: five
",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[1].iteration_cap"),
        Fault::WrongType { .. }
    ));
}

/// **A cap with no edge bounds nothing.** The registry puts the two on one step
/// because a cap split from the count it bounds never fires, so a step carrying
/// only the cap is half a statement — the same shape as `declare_plan_at` with
/// no `evidence_scope`.
#[test]
fn a_cap_on_a_step_with_no_edge_is_refused() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
    iteration_cap: 5
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[0].iteration_cap"),
        &Fault::CapWithoutALoop
    );
}

/// **`approve` and `reject` have nowhere to be routed to.** One advances and
/// one ends the Job, so the map holds the one verdict that does neither, and a
/// file naming another is told what is read at that position rather than
/// silently routing on a word nothing emits.
#[test]
fn a_verdict_that_is_not_a_loop_return_is_refused() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
      approve: draft
",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[1].verdict_routing.approve"),
        Fault::Unknown { known } if *known == ["request_changes"]
    ));
}

/// `{}` is the key written and left blank, which is a different mistake from
/// never having written it — and it is the one that would otherwise satisfy the
/// wiring check with an edge that goes nowhere.
#[test]
fn an_empty_routing_map_is_refused() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing: {}
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[1].verdict_routing"),
        &Fault::Empty
    );
}

/// **An edge naming no step is a loop that cannot close**, and it is refused
/// where it is written for the reason an unresolvable artifact target is: the
/// only other place it would be found is a Job with a worktree, a Drone and a
/// person standing at a gate with nowhere to send the work back to.
#[test]
fn an_edge_that_names_no_step_is_refused() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: redraft
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[1].verdict_routing.request_changes"),
        &Fault::RoutesToNoSuchStep {
            value: "redraft".to_string(),
            declared: vec!["draft".to_string(), "present".to_string()],
        }
    );
}

/// **A step routing at itself is a retry wearing a loop's name.** The move
/// exists and is spelled `retry_limit`; the whole reason `iteration_count` and
/// `retry_count` are two counters is that a Drone which failed four times and a
/// plan asked for a fourth draft must not read alike, and folding them here
/// would be the parser making that conflation on the author's behalf.
#[test]
fn a_step_that_routes_at_itself_is_refused_as_a_retry() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: present
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[1].verdict_routing.request_changes"),
        &Fault::NotAReturn {
            value: "present".to_string(),
            why: BadReturn::Itself,
        }
    );
}

/// **A forward target is unreachable by construction, not unbuilt.** The edge a
/// return takes is `advanced -> running`, so it lands on a step that has already
/// run — and a step the Job has not reached has advanced nothing. There is no
/// later milestone in which this becomes legal, which is why it is refused here
/// rather than left to the step machine.
#[test]
fn a_step_that_routes_forward_is_refused_and_not_deferred() {
    let refused = refusals(parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: present
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[0].verdict_routing.request_changes"),
        &Fault::NotAReturn {
            value: "present".to_string(),
            why: BadReturn::Ahead,
        }
    );
}

/// The refusals are three and not one: a name that resolves to nothing is an
/// edit to the name, and a name that resolves to the wrong place is an edit to
/// the shape. Collapsing them would tell an author to look at the same line for
/// two different mistakes.
#[test]
fn a_target_that_is_strictly_earlier_is_the_only_one_that_loads() {
    let def = parsed(DESIGN_PLAN).expect("the design loop");
    assert_eq!(
        def.steps()[1]
            .verdict_routing()
            .get(&GateVerdict::RequestChanges),
        Some(&StepId::new("draft".to_string()))
    );
}

/// **Parsing is not resolving**, and the keys were carried to the first and no
/// further for one wave. This is the assertion that the pair reaches the record
/// a Job freezes — where `fleet` reads it, and where `store` writes it down.
#[test]
fn the_loop_reaches_the_resolved_step_a_job_freezes() {
    let def = parsed(DESIGN_PLAN).expect("the design loop");
    let manifest = plain_manifest();
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("no step names a check");

    let present = &resolved.steps()[1];
    assert_eq!(
        present.routes(GateVerdict::RequestChanges),
        Some(&StepId::new("draft"))
    );
    assert_eq!(present.iteration_cap(), 5);

    let draft = &resolved.steps()[0];
    assert!(
        !draft.closes_a_loop(),
        "the step the loop returns to closes none of its own"
    );
    assert_eq!(
        draft.iteration_cap(),
        0,
        "and its cap is the zero every step of every linear workflow carries"
    );
}

/// Where the file declares a route and no bound, the step takes the default
/// cap. A back edge with no bound at all would be a Job that never terminates,
/// and that guard is the passes cap now that nothing else declares a loop.
#[test]
fn a_route_with_no_cap_freezes_with_the_default_cap() {
    let def = parsed("
  - id: draft
    label: Draft
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: auto
  - id: present
    label: Present
    evidence: {submitted: {type: document}}
    delivers: false
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
",
    )
    .expect("a cap is not required to declare a route");
    let manifest = plain_manifest();
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("no step names a check");

    let present = &resolved.steps()[1];
    assert!(present.closes_a_loop());
    assert_eq!(present.iteration_cap(), crate::DEFAULT_ITERATION_CAP);
}
