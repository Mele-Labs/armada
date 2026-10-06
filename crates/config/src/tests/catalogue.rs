//! The merge of a workflow's three sources, over text and nothing else.
//!
//! The carried half needs the adapter's roster, because the carried
//! definitions name its models, so it is `tests/carried.rs`. What is here is
//! the rule itself: the more specific place wins, whatever order the
//! definitions arrive in.

use std::path::Path;

use core_model::WorkflowSource;

use crate::catalogue::{Catalogue, WhyLeftOut, Written};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{named, roster};
use crate::workflow::WorkflowDef;

/// A one-step definition whose step is labelled `label`, gated on the Check
/// `gate` names or on nothing.
fn one_step(id: &str, label: &str, gate: Option<&str>) -> String {
    let checks = gate.map_or(String::new(), |check| {
        format!(
            "    evidence: {{submitted: {{type: diff}}}}\n    mechanical_checks:\n      \
             - {{ type: manifest_check, check: {check} }}\n"
        )
    });
    format!(
        "version: 1\nworkflow_id: {id}\nname: {id}\nsteps:\n  - id: only\n    \
         label: \"{label}\"\n    delivers: true\n    advance_gate: auto\n{checks}"
    )
}

fn kit(file: &str, text: String) -> Written {
    Written::in_kit(named(&format!("/kit/workflows/{file}")), text)
}

fn repository(file: &str, text: String) -> Written {
    Written::in_repository(named(&format!("/repo/.armada/workflows/{file}")), text)
}

fn a_manifest() -> Manifest {
    Manifest::parse(Path::new("/repo/armada.yml"), "version: 1\nid: fixture\n")
        .expect("a Manifest declaring nothing")
}

fn resolved(written: Vec<Written>) -> crate::catalogue::ResolvedCatalogue {
    Catalogue::of(written, &roster()).resolve(&a_manifest())
}

fn label_of(held: &crate::catalogue::ResolvedCatalogue, id: &str) -> (String, WorkflowSource) {
    let workflow = held
        .workflows()
        .values()
        .find(|workflow| workflow.id().as_str() == id)
        .expect("the id is held");
    (workflow.steps()[0].label().to_string(), workflow.source())
}

#[test]
fn the_repositorys_definition_wins_over_kits_in_either_order() {
    for flipped in [false, true] {
        let mut written = vec![
            kit("bug.yml", one_step("bug", "Kit's", None)),
            repository("bug.yml", one_step("bug", "The repository's", None)),
        ];
        if flipped {
            written.reverse();
        }
        assert_eq!(
            label_of(&resolved(written), "bug"),
            ("The repository's".to_string(), WorkflowSource::Repository),
            "arrival order flipped: {flipped}"
        );
    }
}

#[test]
fn an_id_only_one_place_declares_is_held_from_that_place() {
    let held = resolved(vec![
        kit("hotfix.yml", one_step("hotfix", "Hot", None)),
        repository("bug.yml", one_step("bug", "Bug", None)),
    ]);
    let sources: Vec<(&str, WorkflowSource)> = held
        .workflows()
        .iter()
        .map(|(id, workflow)| (id.as_str(), workflow.source()))
        .collect();
    assert_eq!(
        sources,
        [
            ("bug", WorkflowSource::Repository),
            ("hotfix", WorkflowSource::Kit)
        ]
    );
}

/// **Two Kit files naming one id are both left out, and named together**, and
/// what the next place down holds for the id runs instead.
#[test]
fn one_id_twice_in_kit_leaves_both_out_and_the_next_place_answers() {
    let held = resolved(vec![
        kit("first.yml", one_step("bug", "One", None)),
        kit("second.yml", one_step("bug", "Two", None)),
        repository("hotfix.yml", one_step("hotfix", "Hot", None)),
    ]);
    assert!(!held.workflows().keys().any(|id| id.as_str() == "bug"));
    let [left] = held.left_out() else {
        panic!("one sentence for the pair: {:?}", held.left_out());
    };
    assert!(matches!(left.why(), WhyLeftOut::Duplicated { also } if also.len() == 1));
    let said = left.to_string();
    assert!(
        said.starts_with("Kit's `bug` was left out, because ")
            && said.contains("first.yml")
            && said.contains("second.yml")
            && said.ends_with("; no `bug` runs here"),
        "{said}"
    );
}

/// **A definition that will not parse is left out and named**, in Kit and in
/// the repository alike, and the repository still gets its catalogue.
#[test]
fn a_definition_that_will_not_parse_is_left_out_and_named() {
    let broken = || "version: 1\nworkflow_id: broken\n".to_string();
    let held = resolved(vec![
        kit("broken.yml", broken()),
        repository("bug.yml", one_step("bug", "Bug", None)),
    ]);
    assert_eq!(held.workflows().len(), 1);
    let [left] = held.left_out() else {
        panic!("one left out: {:?}", held.left_out());
    };
    assert_eq!((left.id(), left.source()), (None, WorkflowSource::Kit));
    assert!(matches!(left.why(), WhyLeftOut::Unparsed(_)));
    let said = left.to_string();
    assert!(
        said.contains("from Kit") && said.contains("/kit/workflows/broken.yml"),
        "{said}"
    );
}

/// **One from Kit naming a Check this repository lacks is left out**, and the
/// next place down answers for its id — here, nobody, so the id is absent.
#[test]
fn a_kit_definition_naming_an_undeclared_check_is_left_out_and_named() {
    let held = resolved(vec![kit(
        "bug.yml",
        one_step("bug", "Kit's", Some("build")),
    )]);
    assert!(held.workflows().is_empty());
    let [left] = held.left_out() else {
        panic!("one left out: {:?}", held.left_out());
    };
    assert_eq!(left.id().map(|id| id.as_str()), Some("bug"));
    assert!(matches!(left.why(), WhyLeftOut::Unresolved(_)));
    let said = left.to_string();
    assert!(
        said.starts_with("Kit's `bug` was left out, because ") && said.contains("build"),
        "{said}"
    );
}

/// A replaced definition never runs, so a Check it names that this repository
/// does not declare is not checked, and nothing is left out for it.
#[test]
fn a_replaced_definition_is_not_resolved() {
    let held = resolved(vec![
        kit("bug.yml", one_step("bug", "Kit's", Some("build"))),
        repository("bug.yml", one_step("bug", "Ungated", None)),
    ]);
    assert_eq!(label_of(&held, "bug").0, "Ungated");
    assert!(held.left_out().is_empty());
}

#[test]
fn a_definition_resolved_on_its_own_is_the_repositorys() {
    let def = WorkflowDef::parse(&named("bug.yml"), &one_step("bug", "Bug", None), &roster())
        .expect("it parses");
    let resolved = ResolvedWorkflow::resolve(&def, &a_manifest()).expect("it resolves");
    assert_eq!(resolved.source(), WorkflowSource::Repository);
    assert_eq!(resolved.frozen().source(), WorkflowSource::Repository);
}

/// **A repository's own bad file is left out and named, and the files beside
/// it stand**, at start and on every re-read.
#[test]
fn a_bad_repository_file_does_not_take_the_others_down() {
    let written = vec![
        repository("broken.yml", "workflow_id: [unclosed".to_string()),
        repository("needs.yml", one_step("needs", "Needs", Some("missing"))),
        repository("fine.yml", one_step("fine", "Fine", None)),
        kit("hotfix.yml", one_step("hotfix", "Hot", None)),
    ];
    let held = Catalogue::of(written, &roster()).resolve(&a_manifest());
    assert_eq!(
        label_of(&held, "fine"),
        ("Fine".to_string(), WorkflowSource::Repository)
    );
    assert_eq!(
        label_of(&held, "hotfix"),
        ("Hot".to_string(), WorkflowSource::Kit)
    );
    assert!(!held.workflows().keys().any(|id| id.as_str() == "needs"));
    let said: Vec<String> = held.left_out().iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said
        .iter()
        .any(|s| s.starts_with("the repository's `needs` was left out")));
}

/// A repository's own that will not resolve steps down, as Kit's does: the
/// next place down answers for its id, and the sentence says whose.
#[test]
fn a_repository_file_that_will_not_resolve_steps_down_to_kits() {
    let held = Catalogue::of(
        vec![
            kit("bug.yml", one_step("bug", "Kit's", None)),
            repository("bug.yml", one_step("bug", "Mine", Some("missing"))),
        ],
        &roster(),
    )
    .resolve(&a_manifest());
    assert_eq!(
        label_of(&held, "bug"),
        ("Kit's".to_string(), WorkflowSource::Kit)
    );
    let [left] = held.left_out() else {
        panic!("{:?}", held.left_out());
    };
    assert_eq!(left.instead(), Some(WorkflowSource::Kit));
}

#[test]
fn one_id_twice_in_the_repository_leaves_both_out() {
    let held = Catalogue::of(
        vec![
            repository("first.yml", one_step("shared", "One", None)),
            repository("second.yml", one_step("shared", "Two", None)),
            repository("other.yml", one_step("other", "Other", None)),
        ],
        &roster(),
    )
    .resolve(&a_manifest());
    assert!(!held.workflows().keys().any(|id| id.as_str() == "shared"));
    assert!(held.workflows().keys().any(|id| id.as_str() == "other"));
    assert!(matches!(
        held.left_out()[0].why(),
        WhyLeftOut::Duplicated { .. }
    ));
}

#[test]
fn a_definition_fits_where_it_parses_and_resolves_and_says_why_where_it_does_not() {
    let manifest = a_manifest();
    let at = Path::new("/repo/.armada/workflows/x.json");
    let id = crate::fit(at, &one_step("x", "X", None), &roster(), &manifest).expect("fits");
    assert_eq!(id.as_str(), "x");
    let unparsed = crate::fit(at, "workflow_id: [", &roster(), &manifest).expect_err("no");
    assert!(matches!(unparsed, crate::Unfit::Unparsed(_)));
    let unresolved = crate::fit(
        at,
        &one_step("x", "X", Some("missing")),
        &roster(),
        &manifest,
    )
    .expect_err("no");
    assert!(matches!(unresolved, crate::Unfit::Unresolved(_)));
    assert!(unresolved.to_string().contains("missing"), "{unresolved}");
}

/// **Every file that parsed is kept, with its text**, the one that runs and each
/// one a more specific place replaced, so a list can draw the dashed row and an
/// editor can open it.
#[test]
fn every_parsed_file_is_kept_and_says_what_replaced_it() {
    let held = resolved(vec![
        kit("bug.yml", one_step("bug", "Kit's", None)),
        repository("bug.yml", one_step("bug", "Mine", None)),
        kit("solo.yml", one_step("solo", "Solo", None)),
        kit("needs.yml", one_step("needs", "Needs", Some("missing"))),
    ]);
    let files: Vec<_> = held
        .files()
        .iter()
        .map(|file| (file.id().as_str(), file.source(), file.overridden_by()))
        .collect();
    assert_eq!(
        files,
        vec![
            ("bug", WorkflowSource::Repository, None),
            ("bug", WorkflowSource::Kit, Some(WorkflowSource::Repository)),
            ("solo", WorkflowSource::Kit, None),
        ],
        "a left-out file is on the left-out list and not here"
    );
    assert_eq!(held.files()[1].text(), one_step("bug", "Kit's", None));
}
