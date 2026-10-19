//! `record_plan`: a plan Fleet keeps is one an agent can act on, and one it refuses is refused with every
//! reason in a message that says what to call again. `docs/concepts/session.md`, *The plan*.

use crate::session_plan::MOST_GROUPS;
use crate::{PlanGroupInput, PlanMark, PlanTaskInput, RecordPlan};

fn group(id: Option<&str>, agent: &str, paths: &[&str], tasks: &[&str]) -> PlanGroupInput {
    PlanGroupInput {
        id: id.map(String::from),
        title: "Store".to_string(),
        agent: agent.to_string(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        tasks: tasks.iter().map(|one| PlanTaskInput { id: None, title: one.to_string() }).collect(),
    }
}

fn plan(groups: Vec<PlanGroupInput>) -> RecordPlan {
    RecordPlan { groups, sketch: None, walk: None, session_id: None }
}

#[test]
fn a_plan_is_trimmed_and_its_paths_are_made_plain() {
    let mut one = group(None, " rust-engineer ", &["./crates/store/", "crates/store", "docs"], &["  Add the tables  "]);
    one.title = "  Store  ".to_string();
    let checked = plan(vec![one]).checked().expect("a plan");
    let got = &checked.groups[0];
    assert_eq!((got.title.as_str(), got.agent.as_str()), ("Store", "rust-engineer"));
    assert_eq!(got.paths, ["crates/store", "docs"], "a repeat is one");
    assert_eq!(got.tasks[0].title, "Add the tables");
    assert!(checked.sketch.is_none());
}

#[test]
fn every_reason_is_in_one_refusal_that_names_the_tool_to_call_again() {
    let bad = plan(vec![
        group(Some("a b"), "rust engineer", &["/etc", "../x", "a\\b"], &[]),
        group(Some("g1"), "", &[], &["One"]),
        group(Some("g1"), "x", &[], &["Two"]),
    ]);
    let why = bad.checked().expect_err("refused");
    for part in ["groups[0] has the id `a b`", "`agent` is `rust engineer`", "path `/etc`", "path `../x`", "has no tasks", "groups[1] `agent` is ``", "repeats the id `g1`"] {
        assert!(why.contains(part), "{part}: {why}");
    }
    assert!(why.starts_with("the plan was not recorded: ") && why.ends_with("call record_plan again"), "{why}");
}

#[test]
fn an_empty_plan_and_one_past_the_limits_are_refused() {
    assert!(plan(Vec::new()).checked().unwrap_err().contains("`groups` is empty"));
    let many = (0..=MOST_GROUPS).map(|_| group(None, "x", &[], &["One"])).collect();
    assert!(plan(many).checked().unwrap_err().contains(&format!("at most {MOST_GROUPS}")));
}

#[test]
fn a_mark_is_tagged_by_its_kind_in_snake_case_and_leaves_out_what_it_has_not() {
    let rename = PlanMark::Rename { group: "g1".into(), task: None, from: "A".into(), to: "B".into() };
    let wire = crate::encode(&rename).unwrap();
    assert_eq!(wire, r#"{"kind":"rename","group":"g1","from":"A","to":"B"}"#);
    let back: PlanMark = crate::decode("a mark", br#"{"kind":"move_task","task":"t1","from":"g1","to":"g2"}"#).unwrap();
    assert_eq!(back, PlanMark::MoveTask { task: "t1".into(), from: "g1".into(), to: "g2".into() });
}
