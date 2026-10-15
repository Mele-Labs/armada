//! Autopilot through a Fleet: a sure pick starts a Job at `queued` unasked with its issue as its
//! source, an unsure one is set aside and the next issue is taken in the same pass, room and caps
//! hold the next pick, and Pause, Stop and the set-aside pile do what they say. #2092.

use std::sync::Arc;

use api::{Autopilots, Refusal};
use core_model::{Actor, JobStatus, Origin, TopLevelOrigin};
use ipc::{AutopilotCap, AutopilotPickState, AutopilotSettings, AutopilotState, ManifestId};
use testkit::{FakeHarness, FakeJudge, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const SURE_12: &str = "issue: 12\nworkflow: bug\nsure: yes\nbecause: a wrong label, one file";
const UNSURE_11: &str =
    "issue: 11\nworkflow: feature\nsure: no\nbecause: it may want a design call";
const SURE_13: &str = "issue: 13\nworkflow: bug\nsure: yes\nbecause: a plain crash";

fn url(number: u64) -> String {
    format!("https://forge.invalid/o/r/issues/{number}")
}

/// A Fleet whose tracker lists issues 11 to 13 and whose model answers `replies` in turn.
fn a_fleet(home: &TempDir, replies: &[&str]) -> (Fixture, Arc<FakeJudge>) {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    let model = Arc::new(FakeJudge::answering_in_turn(replies));
    fittings.judge = model.clone();
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::new(FakeTracker::listing(vec![
        FakeTracker::issue(13, "The reader crashes on an empty file"),
        FakeTracker::issue(12, "The Board labels a killed Job as failed"),
        FakeTracker::issue(11, "Should Helm keep one conversation per repository?"),
    ]))));
    (Fleet::assembled(fittings), model)
}

fn here(fleet: &Fixture) -> ManifestId {
    ManifestId::from(fleet.first().manifest().id())
}

fn settings(at_once: u32, job_cap: Option<u32>) -> AutopilotSettings {
    AutopilotSettings {
        instruction: "work through the open issues that don't need design decisions".to_string(),
        at_once,
        until: None,
        spend_cap_micros: None,
        job_cap,
    }
}

async fn started(fleet: &Fixture, at_once: u32, job_cap: Option<u32>) -> ipc::Autopilot {
    fleet
        .start_autopilot(here(fleet), settings(at_once, job_cap))
        .await
        .expect("it starts")
}

fn job_of(pick: &ipc::AutopilotPick) -> core_model::JobId {
    pick.job_id.as_ref().expect("a Job").to_domain()
}

#[tokio::test]
async fn a_sure_pick_starts_unasked_and_an_unsure_one_is_set_aside_in_the_same_pass() {
    let home = TempDir::new();
    let (fleet, _) = a_fleet(&home, &[UNSURE_11, SURE_12]);
    started(&fleet, 1, None).await;

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);

    let now = fleet.get_autopilot(here(&fleet)).await.expect("held");
    let states: Vec<(u64, AutopilotPickState)> = now
        .picks
        .iter()
        .map(|pick| (pick.issue_number, pick.state))
        .collect();
    assert_eq!(
        states,
        vec![
            (11, AutopilotPickState::SetAside),
            (12, AutopilotPickState::Started)
        ]
    );
    assert_eq!(now.picks[0].reason, "it may want a design call");
    let job = fleet.load(&job_of(&now.picks[1])).await.expect("the Job");
    assert_eq!(job.status(), JobStatus::Queued, "no approval asked");
    assert_eq!(job.origin(), Origin::from(TopLevelOrigin::AutopilotPicked));
    assert_eq!(job.workflow().id().as_str(), "bug");
    let events = fleet
        .store()
        .lock()
        .await
        .events_for(job.id())
        .expect("its log");
    assert_eq!(
        events.last().expect("a move").actor(),
        Actor::Fleet,
        "Fleet queued it, not a person"
    );
    let source = fleet
        .store()
        .lock()
        .await
        .issue_source(job.id())
        .expect("reads");
    assert_eq!(
        source.expect("its issue").url,
        url(12),
        "the write-back reads this"
    );
    assert_eq!(now.at_work, 1);
    assert_eq!(now.jobs_started, 1);
}

#[tokio::test]
async fn a_full_autopilot_asks_nothing_and_a_job_waiting_on_a_person_gives_its_place_back() {
    let home = TempDir::new();
    let (fleet, model) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let asked = model.asked().len();

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);
    assert_eq!(model.asked().len(), asked, "no room, so no call");

    let first = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    let job = fleet.load(&job_of(&first)).await.expect("the Job");
    // Escalated is waiting on a person, like an asking Drone or a gate at review.
    fleet
        .move_job(
            &job,
            core_model::Target::Escalated(core_model::EscalationTrigger::DependencyFailed),
            Actor::Fleet,
        )
        .await
        .expect("it escalates");
    assert_eq!(
        fleet.autopilot_pass().await.expect("a pass"),
        1,
        "its place is free"
    );
}

#[tokio::test]
async fn the_first_cap_reached_pauses_it_and_says_which_and_the_started_job_carries_on() {
    let home = TempDir::new();
    let (fleet, model) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 2, Some(1)).await;

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);
    fleet.autopilot_pass().await.expect("a pass");

    let now = fleet.get_autopilot(here(&fleet)).await.expect("held");
    assert_eq!(now.state, AutopilotState::Paused, "paused, not ended");
    assert_eq!(now.ended_at, None);
    assert_eq!(now.capped_by, Some(AutopilotCap::Jobs));
    assert_eq!(model.asked().len(), 1, "the cap stopped the next pick");
    let job = fleet.load(&job_of(&now.picks[0])).await.expect("the Job");
    assert_eq!(
        job.status(),
        JobStatus::Queued,
        "and not the Job it started"
    );
}

/// The owner's *fix the gaps*, 12 Oct 2026: a cap pauses, Resume waits for it to be raised, and
/// a raised cap lets it carry on from where it was.
#[tokio::test]
async fn a_capped_autopilot_resumes_only_once_a_person_raises_the_cap() {
    let home = TempDir::new();
    let (fleet, model) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 2, Some(1)).await;
    fleet.autopilot_pass().await.expect("a pass");
    fleet.autopilot_pass().await.expect("a pass");
    assert_eq!(
        fleet.get_autopilot(here(&fleet)).await.expect("held").state,
        AutopilotState::Paused
    );

    let refused = fleet.resume_autopilot(here(&fleet)).await;
    let Err(Refusal::IllegalMove(why)) = refused else {
        panic!("refused while the cap holds, not {refused:?}");
    };
    assert_eq!(why.code, "fleet.autopilot_capped");
    assert!(why.message.contains("Job cap"), "{}", why.message);
    let still = fleet.get_autopilot(here(&fleet)).await.expect("held");
    assert_eq!(still.state, AutopilotState::Paused);
    assert_eq!(still.capped_by, Some(AutopilotCap::Jobs), "nothing moved");

    let raised = fleet
        .update_autopilot(here(&fleet), settings(2, Some(2)))
        .await
        .expect("the cap is raised");
    assert_eq!(
        raised.state,
        AutopilotState::Paused,
        "a change never resumes it"
    );
    assert_eq!(raised.job_cap, Some(2));
    assert_eq!(raised.capped_by, Some(AutopilotCap::Jobs));
    assert_eq!(raised.jobs_started, 1, "the run's picks are kept");

    let resumed = fleet.resume_autopilot(here(&fleet)).await.expect("resumes");
    assert_eq!(resumed.state, AutopilotState::Running);
    assert_eq!(resumed.capped_by, None);
    assert_eq!(
        fleet.autopilot_pass().await.expect("a pass"),
        1,
        "it picks again"
    );
    assert_eq!(model.asked().len(), 2);
    let now = fleet.get_autopilot(here(&fleet)).await.expect("held");
    assert_eq!(now.jobs_started, 2);
    fleet.autopilot_pass().await.expect("a pass");
    assert_eq!(
        fleet
            .get_autopilot(here(&fleet))
            .await
            .expect("held")
            .capped_by,
        Some(AutopilotCap::Jobs),
        "and pauses again at the raised cap"
    );
}

#[tokio::test]
async fn a_stopped_autopilot_takes_no_new_settings() {
    let home = TempDir::new();
    let (fleet, _) = a_fleet(&home, &[]);
    started(&fleet, 1, None).await;
    fleet.stop_autopilot(here(&fleet)).await.expect("stops");

    let refused = fleet
        .update_autopilot(here(&fleet), settings(1, Some(3)))
        .await;

    assert!(
        matches!(&refused, Err(Refusal::IllegalMove(why)) if why.code == "fleet.autopilot_stopped"),
        "{refused:?}"
    );
    let blank = AutopilotSettings {
        instruction: " ".to_string(),
        ..settings(1, None)
    };
    started(&fleet, 1, None).await;
    assert!(matches!(
        fleet.update_autopilot(here(&fleet), blank).await,
        Err(Refusal::Unacceptable(_))
    ));
}

#[tokio::test]
async fn pause_picks_nothing_resume_carries_on_and_stop_withdraws_what_never_started() {
    let home = TempDir::new();
    let (fleet, model) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.pause_autopilot(here(&fleet)).await.expect("pauses");

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);
    assert!(model.asked().is_empty(), "paused, so nothing is asked");

    fleet.resume_autopilot(here(&fleet)).await.expect("resumes");
    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);

    let stopped = fleet.stop_autopilot(here(&fleet)).await.expect("stops");
    assert_eq!(stopped.state, AutopilotState::Stopped);
    assert_eq!(stopped.picks[0].state, AutopilotPickState::Withdrawn);
    let job = fleet
        .load(&job_of(&stopped.picks[0]))
        .await
        .expect("the Job");
    assert_eq!(job.status(), JobStatus::Killed, "it never had a Drone");
    assert!(matches!(
        fleet.resume_autopilot(here(&fleet)).await,
        Err(Refusal::IllegalMove(_))
    ));
}

#[tokio::test]
async fn a_set_aside_pick_a_person_dispatches_waits_at_the_gate_and_one_dismissed_leaves_the_pile()
{
    let home = TempDir::new();
    let (fleet, _) = a_fleet(
        &home,
        &[
            UNSURE_11,
            "issue: none\nbecause: the rest are not what was asked",
            "workflow: feature\ntitle: One Helm conversation per repository",
        ],
    );
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let aside = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    assert_eq!(aside.state, AutopilotPickState::SetAside);

    let now = fleet
        .dispatch_set_aside(here(&fleet), aside.pick_id.clone())
        .await
        .expect("dispatched");

    assert_eq!(now.picks[0].state, AutopilotPickState::Started);
    let job = fleet.load(&job_of(&now.picks[0])).await.expect("the Job");
    assert_eq!(
        job.status(),
        JobStatus::AwaitingApproval,
        "a person chose it, so the gate holds"
    );
    assert_eq!(job.origin(), Origin::Manual);
    assert!(matches!(
        fleet.dismiss_set_aside(here(&fleet), aside.pick_id).await,
        Err(Refusal::IllegalMove(_))
    ));
}

#[tokio::test]
async fn a_picker_that_finds_nothing_is_not_asked_again_until_the_list_changes() {
    let home = TempDir::new();
    let (fleet, model) = a_fleet(
        &home,
        &["issue: none\nbecause: nothing here is what was asked"],
    );
    started(&fleet, 1, None).await;

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);
    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);

    assert_eq!(model.asked().len(), 1);
}

#[tokio::test]
async fn a_start_over_a_running_one_or_with_no_room_is_refused() {
    let home = TempDir::new();
    let (fleet, _) = a_fleet(&home, &[]);

    assert!(matches!(
        fleet.start_autopilot(here(&fleet), settings(0, None)).await,
        Err(Refusal::Unacceptable(_))
    ));
    started(&fleet, 1, None).await;
    assert!(matches!(
        fleet.start_autopilot(here(&fleet), settings(1, None)).await,
        Err(Refusal::IllegalMove(_))
    ));
}

#[tokio::test]
async fn one_sentence_is_proposed_and_nothing_starts() {
    let home = TempDir::new();
    let (fleet, _) = a_fleet(
        &home,
        &[
            "instruction: fix the flaky tests\ninstruction_from: fix the flaky tests\nat_once: 2\n\
           at_once_from: two at a time\nuntil: none\nuntil_from: none\nspend: 30\n\
           spend_from: $30 max\njobs: none",
        ],
    );

    let proposed = fleet
        .propose_autopilot(
            here(&fleet),
            ipc::ProposeAutopilot {
                sentence: "fix the flaky tests, two at a time, $30 max".to_string(),
                local_now: None,
            },
        )
        .await
        .expect("read");

    assert_eq!(proposed.settings.at_once, 2);
    assert_eq!(proposed.settings.spend_cap_micros, Some(30_000_000));
    assert_eq!(proposed.quoted.at_once.as_deref(), Some("two at a time"));
    assert_eq!(proposed.quoted.spend_cap_micros.as_deref(), Some("$30 max"));
    assert_eq!(proposed.quoted.until, None, "the sentence named no time");
    assert_eq!(proposed.quoted.job_cap, None, "no `jobs_from` line at all");
    assert!(fleet
        .list_autopilots()
        .await
        .expect("listed")
        .autopilots
        .is_empty());
}

#[tokio::test]
async fn a_dismissed_pick_leaves_the_pile_and_is_not_offered_again() {
    let home = TempDir::new();
    let (fleet, model) = a_fleet(
        &home,
        &[
            UNSURE_11,
            "issue: none\nbecause: the rest are not what was asked",
        ],
    );
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let aside = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();

    let now = fleet
        .dismiss_set_aside(here(&fleet), aside.pick_id)
        .await
        .expect("dismissed");

    assert_eq!(now.picks[0].state, AutopilotPickState::Dismissed);
    let asked = model.asked();
    assert!(
        !asked.last().expect("asked twice").contains("#11 "),
        "the next call was not offered it"
    );
}

#[tokio::test]
async fn a_pass_that_could_not_pick_says_why_on_the_autopilot_until_one_goes_through() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(FakeJudge::answering_in_turn(&[SURE_12]));
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(
        12,
        "A wrong label",
    )]));
    tracker.will_not_list("nobody is signed in to the forge");
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker.clone()));
    let fleet: Fixture = Fleet::assembled(fittings);
    started(&fleet, 1, None).await;

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);
    let fault = fleet.get_autopilot(here(&fleet)).await.expect("held").fault;
    assert!(fault.is_some_and(|why| why.contains("nobody is signed in")));

    tracker.answers_again();
    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);
    assert_eq!(
        fleet.get_autopilot(here(&fleet)).await.expect("held").fault,
        None
    );
}

#[tokio::test]
async fn a_picker_that_fails_rests_longer_each_time_and_keeps_its_fault_meanwhile() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    let model = Arc::new(FakeJudge::that_fails("the network"));
    fittings.judge = model.clone();
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::new(FakeTracker::listing(vec![FakeTracker::issue(12, "A wrong label")]))));
    let clock = Arc::new(crate::tests::planted::Held::started());
    fittings.clock = clock.clone();
    let fleet: Fixture = Fleet::assembled(fittings);
    started(&fleet, 1, None).await;

    fleet.autopilot_pass().await.expect("a pass");
    fleet.autopilot_pass().await.expect("a pass");
    assert_eq!(model.asked().len(), 1);
    assert!(fleet.get_autopilot(here(&fleet)).await.expect("held").fault.is_some());

    clock.on(30);
    fleet.autopilot_pass().await.expect("a pass");
    clock.on(30);
    fleet.autopilot_pass().await.expect("a pass");
    assert_eq!(model.asked().len(), 2, "the second rest is a minute");

    clock.on(30);
    fleet.autopilot_pass().await.expect("a pass");
    assert_eq!(model.asked().len(), 3);
}
