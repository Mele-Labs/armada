//! A red main handed to a Job: by a person's press, and by Fleet itself to the
//! Job whose pull request turned it red. The forge is scripted, as in
//! `main_ci`; the Jobs are the fixture's.

use std::time::Duration;

use testkit::FakeWorkProduct;

use super::main_ci::Fixture;
use super::main_ci::MANIFEST;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::fittings;
use crate::tests::daemon::manifest;
use crate::tests::tmp::TempDir;

/// The repository holds `bug`, the workflow a dispatched fix runs under.
pub(super) fn bug() -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: bug\nname: Bug\nsteps:\n  - id: fix\n    label: \"Fix it\"\n    \
                evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - { type: diff_nonempty }\n    \
                delivers: true\n    advance_gate: auto\n";
    let def = config::WorkflowDef::parse(
        std::path::Path::new("bug.yml"),
        text,
        &config::Roster::offering_nothing(),
    )
    .expect("the bug workflow parses");
    config::ResolvedWorkflow::resolve(&def, &manifest()).expect("it resolves")
}

pub(super) fn a_fleet_holding_bug(home: &TempDir, with_bug: bool) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.starting().manifest =
        config::Manifest::parse(std::path::Path::new("armada.yml"), MANIFEST).expect("parses");
    if with_bug {
        let mut held = fittings.starting().workflows.clone();
        held.insert(bug().id().clone(), bug());
        fittings.starting().workflows = held;
    }
    Fleet::assembled(fittings)
}

pub(super) async fn root(fleet: &Fixture) -> String {
    fleet.repositories().served()[0].root().to_string()
}

