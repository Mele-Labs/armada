//! The claim, written before the code: **a plan's groups say which pull request they land in, and the
//! pull request Fleet has seen is read onto every group that shares the label.**
//! `docs/concepts/session/the-plan.md`.

use ipc::{PlanGroupInput, PlanLandsInPr, PlanLandsInState, PlanTaskInput};
use store::KeptPlan;

use crate::plan_record::{bind_lands_in, recorded, wire, PlanPull};

const NOW: &str = "2026-10-29T10:00:00.000Z";

fn group(title: &str, lands_in: Option<&str>) -> PlanGroupInput {
    PlanGroupInput {
        id: None,
        title: title.into(),
        agent: "rust-engineer".into(),
        paths: Vec::new(),
        issue: None,
        gives: None,
        lands_in: lands_in.map(String::from),
        own_job: false,
        tasks: vec![PlanTaskInput { id: None, title: format!("Do {title}") }],
    }
}

fn kept(groups: &[PlanGroupInput]) -> KeptPlan {
    recorded(None, groups, None, None, false, "You get it.".into(), NOW)
}

fn pull(number: u64, from_task: Option<&str>) -> PlanPull {
    PlanPull {
        number,
        title: format!("Pull {number}"),
        state: PlanLandsInState::Open,
        from_task: from_task.map(String::from),
        handed: None,
    }
}

fn read(plan: &KeptPlan, pulls: &[PlanPull]) -> Vec<(String, Option<u64>)> {
    let mut wired = wire(plan);
    bind_lands_in(&mut wired, plan, pulls);
    wired.groups.iter().map(|one| (one.lands_in.label.clone(), one.lands_in.pull_request.as_ref().map(|pr| pr.number))).collect()
}

#[test]
fn a_group_that_names_no_pull_request_lands_in_pr_1() {
    let plan = kept(&[group("Store", None), group("Fleet", None)]);
    let wired = wire(&plan);
    assert!(wired.groups.iter().all(|one| one.lands_in.label == "PR 1" && one.lands_in.pull_request.is_none()));
}

#[test]
fn a_label_is_kept_when_the_plan_is_recorded_again() {
    let groups = [group("Store", Some("PR 1")), group("Fleet", Some("PR 2"))];
    let first = kept(&groups);
    let second = recorded(Some(&first), &groups, None, None, false, "You get it.".into(), NOW);
    assert_eq!(second.version, 2);
    assert_eq!(second.groups[1].lands_in.as_deref(), Some("PR 2"));
    assert_eq!(wire(&second).groups[1].lands_in.label, "PR 2");
}

#[test]
fn every_group_sharing_a_label_reads_the_pull_request_bound_to_it() {
    let plan = kept(&[group("Store", Some("PR 1")), group("Fleet", Some("PR 2")), group("Bridge", Some("PR 2"))]);
    let task = plan.groups[1].tasks[0].id.clone();
    let mut wired = wire(&plan);
    bind_lands_in(&mut wired, &plan, &[pull(7, Some(&task))]);
    assert_eq!(
        wired.groups[2].lands_in.pull_request,
        Some(PlanLandsInPr { number: 7, title: "Pull 7".into(), state: PlanLandsInState::Open })
    );
    let numbers: Vec<_> = wired.groups.iter().map(|one| one.lands_in.pull_request.as_ref().map(|pr| pr.number)).collect();
    assert_eq!(numbers, [None, Some(7), Some(7)]);
}

#[test]
fn a_pull_request_that_followed_no_task_binds_the_first_label_nothing_has_bound() {
    let plan = kept(&[group("Store", Some("PR 1")), group("Fleet", Some("PR 2")), group("Bridge", Some("PR 3"))]);
    let task = plan.groups[0].tasks[0].id.clone();
    assert_eq!(read(&plan, &[pull(5, None)]).iter().map(|one| one.1).collect::<Vec<_>>(), [Some(5), None, None]);
    let both = read(&plan, &[pull(4, Some(&task)), pull(5, None)]);
    assert_eq!(both.iter().map(|one| one.1).collect::<Vec<_>>(), [Some(4), Some(5), None]);
}

#[test]
fn a_group_handed_to_a_job_reads_that_jobs_pull_request() {
    let plan = kept(&[group("Store", Some("PR 1")), group("Fleet", Some("PR 2"))]);
    let handed = PlanPull { handed: Some(plan.groups[1].id.clone()), ..pull(9, None) };
    assert_eq!(read(&plan, &[handed]).iter().map(|one| one.1).collect::<Vec<_>>(), [None, Some(9)]);
}

#[test]
fn a_group_that_goes_to_its_own_job_says_so_on_the_wire_until_the_plan_is_recorded_again_without_it() {
    let own = PlanGroupInput { own_job: true, ..group("Fleet", Some("PR 2")) };
    let groups = [group("Store", None), own];
    let first = kept(&groups);
    let marks = |plan: &KeptPlan| wire(plan).groups.iter().map(|one| one.own_job).collect::<Vec<_>>();
    assert_eq!(marks(&first), [false, true]);
    let second = recorded(Some(&first), &groups, None, None, false, "You get it.".into(), NOW);
    assert_eq!(marks(&second), [false, true], "recording it again with the mark keeps it");
    let plain = [group("Store", None), group("Fleet", Some("PR 2"))];
    let third = recorded(Some(&second), &plain, None, None, false, "You get it.".into(), NOW);
    assert_eq!(marks(&third), [false, false], "the new recording's, as the label is");
}
