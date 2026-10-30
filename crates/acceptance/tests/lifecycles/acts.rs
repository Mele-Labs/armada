//! What a rule plans is done, or asked about first. **Nothing is spawned and no Job is started**:
//! the autonomy the owner set, the rules and the readings are values, and the `fleet::lifecycles`
//! functions that say what Fleet does with a planned act are called on them. That the kept act
//! becomes one item in Needs you, survives a restart and is settled by an answer is `fleet`'s own
//! tests (`rule_asking`), which need a store this crate does not have.

use std::path::PathBuf;

use config::{Autonomies, PieceWritten, Workspaces};
use core_model::rule::{ActKind, AutonomyDefaults, Course, Occurrence, RanOnce};
use core_model::{HeldInstances, ManifestId, Moved, Observed, ReadingState, Ulid};
use fleet::lifecycles::holding::{read_instance, Transition};
use fleet::lifecycles::ruling::carried_here;
use fleet::lifecycles::saying::said;

use crate::bench::lifecycles::{
    a_job_holding_three_tickets, at, in_state, plan_step_start, rules_from, taken_on, the_jobs_copy, MOVES_TICKETS,
};

/// A rule for the first step that starts a Job, which asks until the owner says otherwise.
const STARTS_A_JOB: &str = "version: 1
rule_id: look_at_the_deploy
name: Look at the deploy
scope: my-store
when: { step_starts: plan }
do:
  - start_job: { prompt: Look at the deploy }
";

/// A rule that follows the web deploy once the pull request is approved, which acts without asking.
const FOLLOWS_THE_DEPLOY: &str = "version: 1
rule_id: follow_the_deploy
name: Follow the web deploy
scope: my-store
when: { enters: { lifecycle: pr, state: approved } }
do:
  - start_watch: { recipe: qa-deploy, values: { sha: abc }, deploy: web }
";

fn my_store() -> Workspaces {
    let file = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n";
    Workspaces::of([PieceWritten::in_repository(PathBuf::from("store-web/.armada/workspaces/my-store.yml"), file.into())])
}

fn defaults_of(files: Vec<PieceWritten>) -> AutonomyDefaults {
    Autonomies::applying_to(&ManifestId::carried(Ulid::carried("store-web")), &my_store(), files).defaults()
}

#[test]
fn a_planned_act_waits_as_asks_or_is_done_at_once_by_the_autonomy_set_for_it_and_the_repository_has_the_last_word() {
    let (rules, held) = (rules_from(&[STARTS_A_JOB]).frozen(), HeldInstances::new());
    let course = |defaults: &AutonomyDefaults| -> Vec<Course> {
        plan_step_start("plan", "10:00", &rules, defaults, &held, &mut RanOnce::new()).iter().map(|one| one.course()).collect()
    };
    assert_eq!(course(&AutonomyDefaults::new()), [Course::Asks], "a Job a rule starts asks until someone says otherwise");

    let workspace = PieceWritten::in_kit(PathBuf::from("kit/autonomy/team.yml"), "version: 1\nautonomy_id: team\nscope: my-store\ndefaults:\n  start_job: acts\n".into());
    assert_eq!(course(&defaults_of(vec![workspace.clone()])), [Course::Acts], "the Workspace's word makes it at once");

    let repository = PieceWritten::in_repository(
        PathBuf::from("store-web/.armada/autonomy/mine.yml"),
        "version: 1\nautonomy_id: mine\nscope: store-web\ndefaults:\n  start_job: asks\n".into(),
    );
    assert_eq!(course(&defaults_of(vec![workspace.clone(), repository])), [Course::Asks], "and the repository's laid over it asks again");

    let off = PieceWritten::in_repository(
        PathBuf::from("store-web/.armada/autonomy/off.yml"),
        "version: 1\nautonomy_id: off\nscope: store-web\ndefaults:\n  start_job: off\n".into(),
    );
    assert!(course(&defaults_of(vec![workspace, off])).is_empty(), "off plans nothing, so there is nothing to ask");
}

#[test]
fn the_owner_is_asked_in_his_own_words_and_a_write_fleet_cannot_make_is_planned_and_raises_nothing() {
    let copy = the_jobs_copy();
    let (job_rules, ticket_rules) = (rules_from(&[STARTS_A_JOB]).frozen(), rules_from(&[MOVES_TICKETS]).frozen());
    let held = a_job_holding_three_tickets();

    let started = plan_step_start("plan", "10:00", &job_rules, &AutonomyDefaults::new(), &HeldInstances::new(), &mut RanOnce::new());
    assert_eq!(said(started[0].act(), started[0].on(), &copy), "Start a Job: Look at the deploy");

    let moves = plan_step_start("plan", "10:00", &ticket_rules, &AutonomyDefaults::new(), &held, &mut RanOnce::new());
    let lines: Vec<String> = moves.iter().map(|one| said(one.act(), one.on(), &copy)).collect();
    assert_eq!(lines, ["Move ARM-41 to In progress", "Move ARM-42 to In progress"], "the state goes by the label its lifecycle gives it");
    assert!(moves.iter().all(|one| one.course() == Course::Asks && !carried_here(one.act().kind())));

    // What Fleet has a door for, and what it plans and records and leaves.
    let done: Vec<ActKind> = ActKind::ALL.iter().copied().filter(|kind| carried_here(*kind)).collect();
    assert_eq!(done, [ActKind::Draft, ActKind::StartJob, ActKind::StartWatch, ActKind::TellAgent]);
}

#[test]
fn a_deploy_a_rules_watch_starts_is_taken_on_with_the_pull_request_and_the_watchs_readings_move_it() {
    let copy = the_jobs_copy();
    let (pull_requests, deploys) = (&copy[1], &copy[2]);
    let rules = rules_from(&[FOLLOWS_THE_DEPLOY]).frozen();
    let mut held = HeldInstances::new();
    held.take_on(taken_on(pull_requests, "812")).expect("held");

    let approved = Occurrence::entered(at(pull_requests, "812"), Moved { from: in_state("in_review"), to: in_state("approved") }, "10:00");
    let planned = core_model::rule::Engine::new(&rules, &AutonomyDefaults::new()).plan(&approved, &held, &Default::default(), &mut RanOnce::new());
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].course(), Course::Acts, "a watch is Fleet's own and starts without asking");
    assert_eq!(said(planned[0].act(), planned[0].on(), &copy), "Start the web deploy watch");
    let core_model::rule::Act::StartWatch { deploy: Some(deploy), .. } = planned[0].act() else { panic!("{:?}", planned[0].act()) };

    // What Fleet does once the watch starts: the Job takes the deploy on, linked to the pull request
    // that set the rule off, and each reading the watch keeps is handed to it.
    held.take_on(taken_on(deploys, deploy.as_str()).linked_to(at(pull_requests, "812"))).expect("held");
    assert_eq!(held.linked(&at(pull_requests, "812")).len(), 0, "the link runs from the deploy to the pull request");
    assert_eq!(held.linking(&at(pull_requests, "812")).len(), 1);

    let mut moves: Vec<Transition> = Vec::new();
    for (state, when) in [(ReadingState::Waiting, "10:01"), (ReadingState::Running, "10:02"), (ReadingState::Running, "10:03"), (ReadingState::Passed, "10:04")] {
        moves.extend(read_instance(&mut held, &copy, &at(deploys, "web"), &Observed::Watch(state), when));
    }
    let to: Vec<_> = moves.iter().map(|one| one.moved.to.clone()).collect();
    assert_eq!(to.last(), Some(&in_state("healthy")));
    assert!(to.contains(&in_state("rolling_out")), "{to:?}");
    assert_eq!(moves.iter().filter(|one| one.moved.to == in_state("rolling_out")).count(), 1, "a reading repeated moves nothing");
}
