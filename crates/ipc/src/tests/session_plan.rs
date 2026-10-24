//! `record_plan`: a plan Fleet keeps is one an agent can act on, and one it refuses is refused with every
//! reason in a message that says what to call again. `docs/concepts/session.md`, *The plan*.

use crate::session_plan::MOST_GROUPS;
use crate::{IssueReference, PlanGroupInput, PlanMark, PlanTaskInput, RecordPlan};

fn group(id: Option<&str>, agent: &str, paths: &[&str], tasks: &[&str]) -> PlanGroupInput {
    PlanGroupInput {
        id: id.map(String::from),
        title: "Store".to_string(),
        agent: agent.to_string(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        issue: None,
        gives: None,
        tasks: tasks.iter().map(|one| PlanTaskInput { id: None, title: one.to_string() }).collect(),
    }
}

fn plan(groups: Vec<PlanGroupInput>) -> RecordPlan {
    RecordPlan { summary: "You get a plan page.".to_string(), groups, sketch: None, walk: None, session_id: None }
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

#[test]
fn a_group_names_its_issue_by_number_or_by_key_and_leaves_it_out_when_it_names_none() {
    let named: PlanGroupInput = crate::decode(
        "a group",
        br#"{"title":"Store","agent":"rust-engineer","issue":2214,"tasks":[{"title":"Add the tables"}]}"#,
    )
    .unwrap();
    assert_eq!(named.issue.as_deref(), Some("2214"), "a number, as callers before keys send it");
    assert!(crate::encode(&named).unwrap().contains(r#""issue":"2214""#));

    let keyed: PlanGroupInput = crate::decode(
        "a group",
        br##"{"title":"Store","agent":"rust-engineer","issue":"ABC-12","tasks":[{"title":"Add the tables"}]}"##,
    )
    .unwrap();
    let mut hashed = keyed.clone();
    hashed.issue = Some(" #7 ".into());
    let checked = plan(vec![named, keyed, hashed]).checked().unwrap();
    let issues: Vec<Option<&str>> = checked.groups.iter().map(|one| one.issue.as_deref()).collect();
    assert_eq!(issues, [Some("2214"), Some("ABC-12"), Some("7")], "checking keeps each, as the ledger holds it");

    let bare = group(None, "rust-engineer", &[], &["Add the tables"]);
    assert!(!crate::encode(&bare).unwrap().contains("issue"));
}

#[test]
fn a_group_naming_neither_a_number_nor_a_key_is_refused_by_its_place() {
    let mut bad = group(None, "x", &[], &["One"]);
    bad.issue = Some("abc-1".into());
    let why = plan(vec![group(None, "x", &[], &["One"]), bad]).checked().expect_err("refused");
    assert!(why.contains("groups[1] `issue` is `abc-1`"), "{why}");
}

#[test]
fn an_issue_is_digits_or_a_trackers_key_and_nothing_else() {
    for (written, held) in [
        ("7", "7"),
        ("#7", "7"),
        (" 0031 ", "31"),
        ("ABC-12", "ABC-12"),
        ("A_1B-3", "A_1B-3"),
        ("armada-3fz", "armada-3fz"),
        ("armada-3fz.1", "armada-3fz.1"),
        ("armada-1792806783819-955-760bc69c", "armada-1792806783819-955-760bc69c"),
    ] {
        assert_eq!(IssueReference::read(written).map(|one| one.as_str().to_string()), Some(held.to_string()), "{written}");
    }
    for written in [
        "", "#", "hello", "abc-1", "ABC-", "-12", "1ABC-2", "ABC-12-3", "ABC 12", "#ABC-12", "+7", "armada-", "armada-3fz.",
        "armada-3FZ", "1armada-3fz", "armada--3fz",
    ] {
        assert_eq!(IssueReference::read(written), None, "{written:?}");
    }
    assert_eq!(IssueReference::number(7).to_string(), "#7");
    assert_eq!(IssueReference::read("ABC-12").unwrap().to_string(), "ABC-12");
}

#[test]
fn a_plan_without_a_summary_is_refused_naming_the_field_and_one_with_it_is_kept_trimmed() {
    let one = || group(None, "rust-engineer", &[], &["Add the tables"]);
    for blank in ["", "  \n "] {
        let why = RecordPlan { summary: blank.to_string(), ..plan(vec![one()]) }.checked().expect_err("refused");
        assert!(why.contains("`summary` is blank"), "{why}");
    }
    let long = RecordPlan { summary: "x".repeat(601), ..plan(vec![one()]) }.checked().expect_err("refused");
    assert!(long.contains("`summary` is longer than 600 characters"), "{long}");

    let kept = RecordPlan { summary: "  You get a page.\nIt shows what is next.  ".to_string(), ..plan(vec![one()]) }
        .checked()
        .expect("a plan");
    assert_eq!(kept.summary, "You get a page.\nIt shows what is next.");
}

#[test]
fn a_group_gives_one_short_line_or_nothing() {
    let with = |gives: Option<&str>| {
        let mut one = group(None, "rust-engineer", &[], &["Add the tables"]);
        one.gives = gives.map(String::from);
        plan(vec![one]).checked()
    };
    assert_eq!(with(Some("  A page that shows what is next.  ")).unwrap().groups[0].gives.as_deref(), Some("A page that shows what is next."));
    assert_eq!(with(Some("   ")).unwrap().groups[0].gives, None, "a blank line is none");
    assert_eq!(with(None).unwrap().groups[0].gives, None);
    let why = with(Some(&"x".repeat(161))).expect_err("refused");
    assert!(why.contains("groups[0] `gives` is longer than 160 characters"), "{why}");
}
