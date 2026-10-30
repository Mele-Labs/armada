//! The ticket a plan's group works, filled where the agent left it blank and the group's words name
//! exactly one the Job holds. **Fleet reads it; the agent need not pass `issue`.**

use ipc::{PlanGroupInput, SessionId, SessionTag, TagKind};

use super::session_issues::{a_rig, said};
use super::session_plan::{group, plan, recorded};
use super::session_host::Rig;
use crate::plan_issues::name_the_tickets;

fn held(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|one| one.to_string()).collect()
}

fn named(groups: &mut [PlanGroupInput], keys: &[&str]) -> Vec<Option<String>> {
    name_the_tickets(groups, &held(keys));
    groups.iter().map(|one| one.issue.clone()).collect()
}

#[test]
fn a_group_whose_title_or_task_names_one_ticket_the_job_holds_works_it() {
    let mut groups = vec![
        group(None, "Start Job Workflow, armada-95v", "rust-engineer", &[], &[(None, "Rename it")]),
        group(None, "Changed Rule Shows Id", "rust-engineer", &[], &[(None, "Show the id"), (None, "Closes ARMADA-7GS.")]),
        group(None, "Forge issue", "rust-engineer", &[], &[(None, "Fix #42 first")]),
    ];
    assert_eq!(
        named(&mut groups, &["armada-95v", "armada-7gs", "42"]),
        [Some("armada-95v".into()), Some("armada-7gs".into()), Some("42".into())],
        "in a title, in a task in either case, and as the forge's number after a hash"
    );
}

#[test]
fn a_group_the_agent_gave_an_issue_keeps_it_whatever_its_words_name() {
    let mut groups = vec![group(None, "Fix armada-95v", "rust-engineer", &[], &[])];
    groups[0].issue = Some("armada-cwj".into());
    assert_eq!(named(&mut groups, &["armada-95v", "armada-cwj"]), [Some("armada-cwj".into())]);
}

#[test]
fn a_group_that_names_none_of_the_jobs_tickets_or_names_two_is_left_blank() {
    let mut groups = vec![
        group(None, "Tidy the plan", "rust-engineer", &[], &[(None, "Nothing to see")]),
        group(None, "armada-95v and armada-7gs", "rust-engineer", &[], &[]),
        group(None, "Title names one", "rust-engineer", &[], &[(None, "armada-95v"), (None, "armada-7gs")]),
        group(None, "A ticket the Job does not hold: armada-zzz", "rust-engineer", &[], &[]),
    ];
    assert_eq!(named(&mut groups, &["armada-95v", "armada-7gs"]), [None, None, None, None]);
}

#[test]
fn a_ticket_is_named_whole_and_never_by_the_start_of_a_longer_one() {
    let mut groups = vec![
        group(None, "armada-cwj2", "rust-engineer", &[], &[]),
        group(None, "armada-cwj-1", "rust-engineer", &[], &[]),
        group(None, "armada-cwj.1", "rust-engineer", &[], &[]),
        group(None, "xarmada-cwj", "rust-engineer", &[], &[]),
        group(None, "#420", "rust-engineer", &[], &[]),
        group(None, "42 items, not a ticket", "rust-engineer", &[], &[]),
        group(None, "Ends the sentence on armada-cwj.", "rust-engineer", &[], &[]),
        group(None, "Is #42", "rust-engineer", &[], &[]),
    ];
    assert_eq!(
        named(&mut groups, &["armada-cwj", "42"]),
        [None, None, None, None, None, None, Some("armada-cwj".into()), Some("42".into())]
    );
}

fn tag(key: &str) -> SessionTag {
    SessionTag { kind: TagKind::Issue, id: key.into(), title: key.into(), url: None }
}

async fn holding(rig: &Rig, keys: &[&str]) -> SessionId {
    let id = rig.start().await;
    said(rig, &id, "Work these", keys.iter().map(|key| tag(key)).collect()).await;
    id
}

#[tokio::test]
async fn recording_a_plan_fills_the_issue_of_a_group_that_names_a_ticket_the_job_holds_and_keeps_one_the_agent_passed() {
    let rig = a_rig();
    let id = holding(&rig, &["armada-95v", "armada-7gs"]).await;
    let mut passed = group(None, "Walk it", "rust-engineer", &[], &[(None, "About armada-95v")]);
    passed.issue = Some("armada-7gs".into());
    let groups = vec![
        group(None, "Start Job Workflow, armada-95v", "rust-engineer", &[], &[(None, "Rename it")]),
        passed,
        group(None, "Unrelated", "rust-engineer", &[], &[(None, "No ticket here")]),
    ];

    let kept = recorded(&rig, plan(&id, groups)).await.plan.expect("a plan");

    let issues: Vec<Option<&str>> = kept.groups.iter().map(|one| one.issue.as_deref()).collect();
    assert_eq!(issues, [Some("armada-95v"), Some("armada-7gs"), None]);
}
