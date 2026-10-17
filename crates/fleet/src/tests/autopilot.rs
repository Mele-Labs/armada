//! Autopilot through a Fleet: a sure pick starts a Session unasked with its issue as its brief, an
//! unsure one is set aside and the next issue is taken in the same pass, room and caps count its
//! live Sessions and hold the next pick, and Pause, Stop and the set-aside pile do what they say.
//! #2092.

use std::sync::Arc;
use std::time::Duration;

use api::{Autopilots, HostedSessions, Refusal};
use ipc::{AutopilotCap, AutopilotPickState, AutopilotSettings, AutopilotState, ManifestId};
use testkit::{FakeHarness, FakeJudge, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use super::session_host::{eventually, Shared, StandIn};
use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

type Fixture = Arc<Fleet<FakeHarness, FakeVcs, FakeWorkProduct>>;

const SURE_12: &str = "issue: 12\nworkflow: bug\nsure: yes\nbecause: a wrong label, one file";
const UNSURE_11: &str =
    "issue: 11\nworkflow: feature\nsure: no\nbecause: it may want a design call";
const SURE_13: &str = "issue: 13\nworkflow: bug\nsure: yes\nbecause: a plain crash";

fn url(number: u64) -> String {
    format!("https://forge.invalid/o/r/issues/{number}")
}

/// A Fleet whose tracker lists issues 11 to 13, whose model answers `replies` in turn, and whose
/// Sessions run on a stand-in process.
fn a_fleet(home: &TempDir, replies: &[&str]) -> (Fixture, Arc<FakeJudge>, Arc<StandIn>) {
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
    let stand_in = Arc::new(StandIn::default());
    let fleet = Fleet::assembled(fittings)
        .hosting_sessions_on(Arc::new(Shared(Arc::clone(&stand_in))), Duration::from_secs(600));
    (Arc::new(fleet), model, stand_in)
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

fn session_of(pick: &ipc::AutopilotPick) -> String {
    pick.session_id.as_ref().expect("a Session").as_str().to_string()
}

/// The Session's ledger row.
async fn row_of(fleet: &Fixture, id: &str) -> store::KeptSession {
    fleet.store().lock().await.session(id).expect("reads").expect("a Session")
}

async fn closed(fleet: &Fixture, id: &str) {
    fleet
        .close_session(ipc::CloseSession { session_id: ipc::SessionId::carried(id) })
        .await
        .expect("closes");
}

#[tokio::test]
async fn a_sure_pick_starts_a_session_unasked_with_the_issue_as_its_brief() {
    let home = TempDir::new();
    let (fleet, _, stand_in) = a_fleet(&home, &[UNSURE_11, SURE_12]);
    started(&fleet, 1, None).await;

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);

    let now = fleet.get_autopilot(here(&fleet)).await.expect("held");
    let states: Vec<(&str, AutopilotPickState)> = now
        .picks
        .iter()
        .map(|pick| (pick.issue_reference.as_str(), pick.state))
        .collect();
    assert_eq!(
        states,
        vec![
            ("#11", AutopilotPickState::SetAside),
            ("#12", AutopilotPickState::Started)
        ]
    );
    assert_eq!(now.picks[0].reason, "it may want a design call");
    assert_eq!(now.picks[0].session_id, None, "a set-aside pick started nothing");
    let id = session_of(&now.picks[1]);
    let session = row_of(&fleet, &id).await;
    assert_eq!(session.state, store::SessionState::Live);
    assert_eq!(session.title.as_deref(), Some("The Board labels a killed Job as failed"));
    let hosting = fleet.store().lock().await.hosting(&id).expect("reads").expect("hosted");
    assert_eq!(hosting.workflow.as_deref(), Some("bug"), "the workflow the picker chose");
    assert_eq!(stand_in.starts().len(), 1, "its agent started on the brief, unasked");
    let sent = stand_in.process(0).sent.lock().unwrap().join("\n");
    assert!(sent.contains(&url(12)), "the issue's link is in the brief: {sent}");
    assert!(sent.contains("The Board labels a killed Job as failed"), "and its title: {sent}");
    assert!(sent.contains("a wrong label, one file"), "and the picker's reason: {sent}");
    assert_eq!(now.at_work, 1);
    assert_eq!(now.jobs_started, 1);
}

#[tokio::test]
async fn a_full_autopilot_asks_nothing_and_a_session_that_ends_or_waits_gives_its_place_back() {
    let home = TempDir::new();
    let (fleet, model, _) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let asked = model.asked().len();

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);
    assert_eq!(model.asked().len(), asked, "its one live Session fills it, so no call");

    let first = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    closed(&fleet, &session_of(&first)).await;
    assert_eq!(
        fleet.autopilot_pass().await.expect("a pass"),
        1,
        "the ended Session's place is free"
    );
    let now = fleet.get_autopilot(here(&fleet)).await.expect("held");
    assert_eq!(now.at_work, 1, "an ended Session holds none");
    assert_eq!(now.jobs_started, 2, "and still counts as started");
}

#[tokio::test]
async fn a_session_that_replied_and_went_idle_gives_its_place_back_while_staying_open() {
    let home = TempDir::new();
    let (fleet, _, stand_in) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let first = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    stand_in.init(0);
    eventually(|| async { fleet.get_autopilot(here(&fleet)).await.expect("held").at_work == 1 })
        .await;

    stand_in.finishes(0, "It is fixed.");

    eventually(|| async { fleet.get_autopilot(here(&fleet)).await.expect("held").at_work == 0 })
        .await;
    assert_eq!(
        row_of(&fleet, &session_of(&first)).await.state,
        store::SessionState::Live,
        "it replied; it did not end"
    );
    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1, "its place is free");
}

#[tokio::test]
async fn an_issue_a_live_session_of_an_earlier_run_took_is_not_picked_again_after_stop_and_start()
{
    let home = TempDir::new();
    let (fleet, model, _) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let first = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    fleet.stop_autopilot(here(&fleet)).await.expect("stops");
    started(&fleet, 1, None).await;

    fleet.autopilot_pass().await.expect("a pass");

    let asked = model.asked();
    let second = asked.last().expect("the new run asked");
    assert!(
        !second.contains("The Board labels a killed Job as failed"),
        "the issue its live Session holds is not offered: {second}"
    );
    assert!(second.contains("#13") && second.contains("#11"), "the rest are: {second}");
    assert_eq!(
        row_of(&fleet, &session_of(&first)).await.state,
        store::SessionState::Live
    );
}

#[tokio::test]
async fn a_session_waiting_on_a_person_does_not_count_against_at_once() {
    let home = TempDir::new();
    let (fleet, _, _) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let first = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    let id = session_of(&first);
    let at = fleet.now().as_str().to_string();
    let manifest_id = row_of(&fleet, &id).await.manifest_id.expect("a repository");
    fleet
        .store()
        .lock()
        .await
        .attach(
            &store::KeptAttachment {
                holder: store::Holder::session(&id),
                kind: "unsent".to_string(),
                manifest_id,
                target: "a message nobody has sent".to_string(),
                state: store::AttachmentState::Standing,
                detail: Default::default(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");

    assert_eq!(fleet.get_autopilot(here(&fleet)).await.expect("held").at_work, 0);
    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1, "its place is free");
}

#[tokio::test]
async fn the_first_cap_reached_pauses_it_and_says_which_and_the_started_session_carries_on() {
    let home = TempDir::new();
    let (fleet, model, _) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 2, Some(1)).await;

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);
    fleet.autopilot_pass().await.expect("a pass");

    let now = fleet.get_autopilot(here(&fleet)).await.expect("held");
    assert_eq!(now.state, AutopilotState::Paused, "paused, not ended");
    assert_eq!(now.ended_at, None);
    assert_eq!(now.capped_by, Some(AutopilotCap::Jobs));
    assert_eq!(model.asked().len(), 1, "the cap stopped the next pick");
    let session = row_of(&fleet, &session_of(&now.picks[0])).await;
    assert_eq!(
        session.state,
        store::SessionState::Live,
        "and not the Session it started"
    );
}

/// The owner's *fix the gaps*, 12 Oct 2026: a cap pauses, Resume waits for it to be raised, and
/// a raised cap lets it carry on from where it was.
#[tokio::test]
async fn a_capped_autopilot_resumes_only_once_a_person_raises_the_cap() {
    let home = TempDir::new();
    let (fleet, model, _) = a_fleet(&home, &[SURE_12, SURE_13]);
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
    let (fleet, _, _) = a_fleet(&home, &[]);
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
async fn pause_picks_nothing_resume_carries_on_and_stop_leaves_a_session_that_ran_to_finish() {
    let home = TempDir::new();
    let (fleet, model, _) = a_fleet(&home, &[SURE_12, SURE_13]);
    started(&fleet, 1, None).await;
    fleet.pause_autopilot(here(&fleet)).await.expect("pauses");

    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 0);
    assert!(model.asked().is_empty(), "paused, so nothing is asked");

    fleet.resume_autopilot(here(&fleet)).await.expect("resumes");
    assert_eq!(fleet.autopilot_pass().await.expect("a pass"), 1);

    let stopped = fleet.stop_autopilot(here(&fleet)).await.expect("stops");
    assert_eq!(stopped.state, AutopilotState::Stopped);
    assert_eq!(
        stopped.picks[0].state,
        AutopilotPickState::Started,
        "its agent had started, so nothing is withdrawn"
    );
    let session = row_of(&fleet, &session_of(&stopped.picks[0])).await;
    assert_eq!(session.state, store::SessionState::Live, "and it finishes on its own");
    assert!(matches!(
        fleet.resume_autopilot(here(&fleet)).await,
        Err(Refusal::IllegalMove(_))
    ));
}

#[tokio::test]
async fn a_set_aside_pick_a_person_dispatches_starts_a_session_and_one_dismissed_leaves_the_pile()
{
    let home = TempDir::new();
    let (fleet, _, _) = a_fleet(
        &home,
        &[
            UNSURE_11,
            "issue: none\nbecause: the rest are not what was asked",
        ],
    );
    started(&fleet, 1, None).await;
    fleet.autopilot_pass().await.expect("a pass");
    let aside = fleet.get_autopilot(here(&fleet)).await.expect("held").picks[0].clone();
    assert_eq!(aside.state, AutopilotPickState::SetAside);

    let now = Arc::clone(&fleet)
        .dispatch_set_aside(here(&fleet), aside.pick_id.clone())
        .await
        .expect("dispatched");

    assert_eq!(now.picks[0].state, AutopilotPickState::Started);
    let session = row_of(&fleet, &session_of(&now.picks[0])).await;
    assert_eq!(session.state, store::SessionState::Live);
    assert_eq!(
        session.title.as_deref(),
        Some("Should Helm keep one conversation per repository?")
    );
    assert!(matches!(
        fleet.dismiss_set_aside(here(&fleet), aside.pick_id).await,
        Err(Refusal::IllegalMove(_))
    ));
}

#[tokio::test]
async fn a_picker_that_finds_nothing_is_not_asked_again_until_the_list_changes() {
    let home = TempDir::new();
    let (fleet, model, _) = a_fleet(
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
    let (fleet, _, _) = a_fleet(&home, &[]);

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
    let (fleet, _, _) = a_fleet(
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
    let (fleet, model, _) = a_fleet(
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
    let fleet: Fixture = Arc::new(Fleet::assembled(fittings).hosting_sessions_on(
        Arc::new(Shared(Arc::new(StandIn::default()))),
        Duration::from_secs(600),
    ));
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
    let fleet: Fixture = Arc::new(Fleet::assembled(fittings).hosting_sessions_on(
        Arc::new(Shared(Arc::new(StandIn::default()))),
        Duration::from_secs(600),
    ));
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
