//! The claim, written before the code: **a step moment is a gate row's, and not the rail's. A closed
//! gate ends the groups of its stretch, closes its kind and starts the groups of the next stretch,
//! once each; the first stretch starts when the plan is first recorded.** `docs/concepts/rules.md`.

use std::path::Path;
use std::sync::Arc;

use api::Sessions;
use core_model::rule::Gate;
use ipc::{ApprovePlan, AttachmentReport, PlanGate, PlanGroupInput, SessionFact, SessionId, SessionReport};

use super::session_host::{rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running};

fn save(rig: &Rig, name: &str, when: &str) {
    let folder = Path::new(&rig.fleet.host().kit_home).join("rules");
    std::fs::create_dir_all(&folder).expect("a folder");
    let text = format!("version: 1\nrule_id: {name}\nname: {name}\nwhen: {{ {when} }}\ndo:\n  - tell_agent: {{ message: \"{name}\" }}\n");
    std::fs::write(folder.join(format!("{name}.yml")), text).expect("a rule file");
}

fn gated(id: &str, gate: PlanGate) -> PlanGroupInput {
    PlanGroupInput { gate, ..group(Some(id), id, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {id}"))]) }
}

/// Three stretches: `g1` alone, `g2` and `g3` together, and `g4` with no gate to end it.
fn groups() -> Vec<PlanGroupInput> {
    vec![
        gated("g1", PlanGate::PlanApproved),
        gated("g2", PlanGate::None),
        gated("g3", PlanGate::Merged),
        gated("g4", PlanGate::None),
    ]
}

const RULES: &[(&str, &str)] = &[
    ("starts_g1", "step_starts: g1"),
    ("starts_g2", "step_starts: g2"),
    ("starts_g3", "step_starts: g3"),
    ("starts_g4", "step_starts: g4"),
    ("ends_g1", "step_ends: g1"),
    ("ends_g2", "step_ends: g2"),
    ("ends_g3", "step_ends: g3"),
    ("ends_g4", "step_ends: g4"),
    ("plan_approved", "gate_closes: plan_approved"),
    ("checks_green", "gate_closes: checks_green"),
    ("merged", "gate_closes: merged"),
];

async fn ruled(rig: &Rig) -> SessionId {
    for (name, when) in RULES {
        save(rig, name, when);
    }
    running(rig).await
}

/// The rules that have planned an act, sorted, so a stretch's order within itself is not the claim.
async fn fired(rig: &Rig, id: &SessionId) -> Vec<String> {
    let kept = rig.fleet.store().lock().await.rule_acts_of(id.as_str()).expect("reads");
    let mut rules: Vec<String> = kept.into_iter().map(|one| one.rule_id).collect();
    rules.sort();
    rules
}

async fn merged(rig: &Rig, id: &SessionId) {
    let harness = read(rig, id).await.harness;
    let detail = [("state".to_string(), "merged".to_string())].into_iter().collect();
    let fact = SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: "3360".into(), detail } };
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("attached");
}

#[tokio::test]
async fn the_first_stretch_starts_when_the_plan_is_first_recorded_and_a_new_version_starts_nothing() {
    let rig = rig_placing(None);
    let id = ruled(&rig).await;
    recorded(&rig, plan(&id, groups())).await;
    assert_eq!(fired(&rig, &id).await, ["starts_g1"], "the groups up to the first gate");

    recorded(&rig, plan(&id, groups())).await;
    assert_eq!(fired(&rig, &id).await, ["starts_g1"], "a second version is no start");
}

#[tokio::test]
async fn a_closed_gate_ends_its_stretch_closes_its_kind_and_starts_the_next_once_each() {
    let rig = rig_placing(None);
    let id = ruled(&rig).await;
    recorded(&rig, plan(&id, groups())).await;

    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(
        fired(&rig, &id).await,
        ["ends_g1", "plan_approved", "starts_g1", "starts_g2", "starts_g3"],
        "g1 ends, its kind closes, and g2 and g3 start with the gate that ends them"
    );

    merged(&rig, &id).await;
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.len(), 1);
    assert_eq!(
        fired(&rig, &id).await,
        ["ends_g1", "ends_g2", "ends_g3", "merged", "plan_approved", "starts_g1", "starts_g2", "starts_g3", "starts_g4"],
        "g2 and g3 end together, the merge closes, g4 starts; a gate of another kind fired nothing"
    );

    let before = fired(&rig, &id).await;
    rig.fleet.gates_kept(id.as_str()).await;
    rig.fleet.rules_gates_closed(id.as_str(), &closed).await;
    assert_eq!(fired(&rig, &id).await, before, "a second sweep, and the same close heard again, plan nothing");
}

#[tokio::test]
async fn the_rails_step_moving_sets_off_no_step_moment() {
    let rig = rig_placing(None);
    let id = ruled(&rig).await;
    let mut row = read(&rig, &id).await;
    for step in [0, 1, 0] {
        row.workflow_step = Some(step);
        rig.fleet.rules_looked_at(&row).await;
    }
    assert!(fired(&rig, &id).await.is_empty());
}

#[test]
fn the_gate_words_a_rule_names_are_the_words_a_plan_names() {
    let words: Vec<&str> = PlanGate::ALL.iter().filter(|gate| !gate.is_none()).map(|gate| gate.word()).collect();
    assert_eq!(words, Gate::ALL.iter().map(|gate| gate.as_wire()).collect::<Vec<_>>());
}
