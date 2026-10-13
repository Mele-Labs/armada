//! A Job in trouble wakes its Commander once. `#2167`.
//!
//! **Real turns where a turn can be had cheaply**: a failed Check, a Judge's
//! refusal, a Drone that would not start, a spend that nears a cap. The Drone
//! that ended with nothing handed in and the drift are `Worked` values built
//! by hand, because `tests::resting` and `tests::scope` already hold that the
//! turn produces them and a real child per case would hold nothing more here.
//! The host is `tests::commander_conversation`'s scripted one.

use std::sync::Arc;
use std::time::Duration;

use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, RepoPath, StepId, Target,
};
use ipc::{CapRaise, RaisedBy};
use store::DroneSpend;
use testkit::{FakeHarness, FakeWorkProduct};

use crate::allowance::{Allowance, Micros, Nearly, Overspent};
use crate::commanding_wakes::{troubles, Trouble};
use crate::converging::{Stage, Wandering};
use crate::daemon::Fleet;
use crate::drone::Aftermath;
use crate::helm::Hosting;
use crate::scope::Drifting;
use crate::silence::{Quiet, Vigil};
use crate::tests::admitted::dispatched;
use crate::tests::commander_conversation::{
    hosted, said, settled, woke_rows, working, Hosted, Scripted,
};
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::turning::{Turned, Worked};

/// What a Fleet is fitted with, so a restart can build the same one again.
#[derive(Clone, Copy)]
enum Fitted {
    Plain,
    CheckFails,
    JudgeRefuses,
    DroneWontStart,
    GitRefuses,
    FiveDollars,
}

fn fleet(home: &TempDir, host: &Arc<Scripted>, how: Fitted) -> Arc<Hosted> {
    let (work, harness) = match how {
        Fitted::CheckFails => (FakeWorkProduct::untouched(), FakeHarness::that_listens()),
        Fitted::DroneWontStart => (
            FakeWorkProduct::changed(&["src/log.rs"]),
            FakeHarness::refusing("no agent binary on this machine"),
        ),
        _ => (
            FakeWorkProduct::changed(&["src/log.rs"]),
            FakeHarness::that_listens(),
        ),
    };
    let mut fitted = fitted_with(home, work, harness);
    match how {
        Fitted::JudgeRefuses => {
            fitted.starting().workflows = one(crate::tests::overruling::judged_then_summarised());
            fitted.judge = Arc::new(crate::tests::overruling::a_judge_that_refuses());
        }
        Fitted::FiveDollars => fitted.allowance = Allowance::of(Micros::dollars(5), 300),
        _ => {}
    }
    Arc::new(Fleet::assembled(fitted).hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>))
}

fn a_job_id() -> JobId {
    JobId::carried(core_model::Ulid::carried("01JOB00000000000000000001"))
}

fn the_implement_step() -> StepId {
    StepId::new("implement")
}

/// One turn that found these things on a Drone.
fn turned_with(worked: Worked) -> Turned {
    Turned {
        each: vec![worked],
        ..Turned::default()
    }
}

/// The rule every trouble answers to: **one wake with this row, none on the
/// next tick that sees the same fact, and none from a Fleet that has restarted.**
async fn wakes_once(
    home: &TempDir,
    host: &Arc<Scripted>,
    how: Fitted,
    fleet: &Arc<Hosted>,
    job: &JobId,
    turned: &Turned,
    row: &str,
) {
    fleet.woken(turned).await;
    settled(fleet, job).await;
    assert_eq!(woke_rows(&said(fleet, job).await), [row]);
    let carried = host.carried();
    assert_eq!(carried.len(), 1, "one wake is one turn");
    assert!(
        carried[0].turn.contains(&format!("Fleet woke you: {row}")),
        "{}",
        carried[0].turn
    );

    fleet.woken(turned).await;
    settled(fleet, job).await;
    assert_eq!(host.carried().len(), 1, "the next tick saw the same fact");

    let restarted = self::fleet(home, host, how);
    restarted.woken(turned).await;
    settled(&restarted, job).await;
    assert_eq!(host.carried().len(), 1, "a restarted Fleet saw it again");
}

// ------------------------------------------------ 1. nothing handed in

#[tokio::test]
async fn a_drone_reaped_with_nothing_handed_in_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::Plain);
    let job = working(&fleet, &home, "bound the reader").await;
    let mut worked = Worked::on(job.clone());
    worked.after = Some(Aftermath::JobMoves(Target::Escalated(
        EscalationTrigger::Silent,
    )));

    wakes_once(
        &home,
        &host,
        Fitted::Plain,
        &fleet,
        &job,
        &turned_with(worked),
        "The Drone on step implement ended with nothing handed in: stalled",
    )
    .await;
}

#[tokio::test]
async fn a_run_that_ended_at_rest_with_nothing_handed_in_wakes_it_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::Plain);
    let job = working(&fleet, &home, "bound the reader").await;
    let mut worked = Worked::on(job.clone());
    worked.quiet = Some(Quiet {
        job: job.clone(),
        step: the_implement_step(),
        after: Duration::ZERO,
        said: Vigil::AtRest {
            found: EscalationTrigger::BlockedByPolicy,
        },
    });

    wakes_once(
        &home,
        &host,
        Fitted::Plain,
        &fleet,
        &job,
        &turned_with(worked),
        "The Drone on step implement ended with nothing handed in: blocked by policy",
    )
    .await;
}

#[tokio::test]
async fn a_drone_the_vigil_gave_up_on_wakes_it_once_too() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::Plain);
    let job = working(&fleet, &home, "bound the reader").await;
    let mut worked = Worked::on(job.clone());
    worked.quiet = Some(Quiet {
        job: job.clone(),
        step: the_implement_step(),
        after: Duration::from_secs(360),
        said: Vigil::Escalated {
            pokes: 2,
            found: EscalationTrigger::Stalled,
        },
    });

    wakes_once(
        &home,
        &host,
        Fitted::Plain,
        &fleet,
        &job,
        &turned_with(worked),
        "The Drone on step implement stopped answering with nothing handed in: stalled",
    )
    .await;
}

/// Reading only, over the values a turn leaves: **what is not trouble wakes
/// nothing**, so a poke, a gate that will rule, or a Job a person already
/// holds is not a reason to spend a model turn.
#[test]
fn what_is_not_trouble_reads_as_none() {
    let none = |worked: Worked| troubles(&worked);
    let mut poked = Worked::on(a_job_id());
    poked.quiet = Some(Quiet {
        job: a_job_id(),
        step: the_implement_step(),
        after: Duration::ZERO,
        said: Vigil::Poked { spent: 1 },
    });
    assert!(none(poked).is_empty());

    for after in [
        Aftermath::TheGateDecides,
        Aftermath::AlreadyStopped,
        Aftermath::JobMoves(Target::Escalated(EscalationTrigger::GateFailure)),
        Aftermath::JobMoves(Target::Escalated(EscalationTrigger::AskUnanswered)),
    ] {
        let mut worked = Worked::on(a_job_id());
        worked.after = Some(after);
        assert!(none(worked).is_empty());
    }

    let mut still_writing = Worked::on(a_job_id());
    still_writing.wandering = Some(Wandering {
        job: a_job_id(),
        step: the_implement_step(),
        stage: Stage::StillWriting { since: Vec::new() },
    });
    assert!(none(still_writing).is_empty(), "the Drone is given its chance");
    assert!(none(Worked::on(a_job_id())).is_empty());
}

#[test]
fn each_of_the_four_endings_reads_as_one_trouble() {
    for trigger in [
        EscalationTrigger::Silent,
        EscalationTrigger::Stalled,
        EscalationTrigger::BlockedByPolicy,
        EscalationTrigger::Interrupted,
    ] {
        let mut worked = Worked::on(a_job_id());
        worked.after = Some(Aftermath::JobMoves(Target::Escalated(trigger.clone())));
        assert_eq!(
            troubles(&worked),
            [Trouble::NothingHandedIn {
                step: None,
                found: trigger,
                ended: true
            }]
        );
    }
}

// ------------------------------------------- 2. a group red, retries spent

#[tokio::test]
async fn a_failed_check_with_its_retries_spent_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::CheckFails);
    let job = working(&fleet, &home, "change nothing").await;
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(crate::gate::Ruling::Failed { .. })),
        "{:?}",
        turned.ruled()
    );

    wakes_once(
        &home,
        &host,
        Fitted::CheckFails,
        &fleet,
        &job,
        &turned,
        "Step implement failed its Checks with its retries spent",
    )
    .await;
}

// ------------------------------------------------ 3. a Judge refusal

#[tokio::test]
async fn a_judge_refusal_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::JudgeRefuses);
    let job = working(&fleet, &home, "widen the bound instead of fixing it").await;
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(crate::gate::Ruling::Refused { .. })),
        "{:?}",
        turned.ruled()
    );

    wakes_once(
        &home,
        &host,
        Fitted::JudgeRefuses,
        &fleet,
        &job,
        &turned,
        "The Judge refused step implement",
    )
    .await;
}

// ------------------------------------------------ 4. past the plan

#[tokio::test]
async fn work_outside_the_plan_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::Plain);
    let job = working(&fleet, &home, "write the plan").await;
    let mut worked = Worked::on(job.clone());
    worked.drifting = Some(Drifting {
        job: job.clone(),
        step: the_implement_step(),
        paths: vec![RepoPath::new("crates/fleet/src/gate.rs")],
    });

    wakes_once(
        &home,
        &host,
        Fitted::Plain,
        &fleet,
        &job,
        &turned_with(worked),
        "The Drone on step implement is working outside the plan",
    )
    .await;
}

// ------------------------------------- 5. a broken worktree or environment

#[tokio::test]
async fn a_drone_that_would_not_start_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::DroneWontStart);
    let job = fleet.propose(a_proposal("a Job the machine will not start")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.expect_err("it would not start");

    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(
        turned.broken,
        [(job.id().clone(), EscalationTrigger::WouldNotStart)]
    );

    wakes_once(
        &home,
        &host,
        Fitted::DroneWontStart,
        &fleet,
        job.id(),
        &turned,
        "No Drone could run: the Drone would not start",
    )
    .await;
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Escalated,
        "the wake does not move the Job; the Commander decides"
    );
}

/// **A Commander that cannot start leaves the Job as it was**, with a line
/// naming why, once and not on every tick.
#[tokio::test]
async fn a_worktree_git_would_not_make_leaves_the_job_escalated_with_one_line_why() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::GitRefuses);
    let job = fleet.propose(a_proposal("a Job with nowhere to work")).await.unwrap();
    fleet.vcs().refuse_next("a full disk");
    dispatched(&fleet, job.id()).await.expect_err("no worktree");

    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(
        turned.broken,
        [(job.id().clone(), EscalationTrigger::NoWorktree)]
    );
    fleet.woken(&turned).await;
    fleet.woken(&turned).await;

    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &job.handle(),
    ))
    .expect("the Job's log");
    assert_eq!(
        log.matches("the Job's Commander could not be woken").count(),
        1,
        "{log}"
    );
    assert!(log.contains("no worktree"), "{log}");
    assert!(host.carried().is_empty());
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Escalated
    );
}

// ------------------------------------------------ 6. a cap nearly spent

const SPEND_ON: &str = "01DRONE00000000000000001";

async fn spend(fleet: &Arc<Hosted>, job: &JobId, cost: u64, turns: u64) {
    fleet
        .record_spend(
            job,
            &core_model::DroneId::carried(core_model::Ulid::carried(SPEND_ON)),
            &DroneSpend {
                cost_micros: Some(cost),
                turns,
                ran_ms: 1_000,
            },
        )
        .await
        .expect("the spend is written");
}

#[tokio::test]
async fn a_cost_cap_four_fifths_spent_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::FiveDollars);
    let job = working(&fleet, &home, "bound the reader").await;
    spend(&fleet, &job, 4_250_000, 20).await;
    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(turned.spent, [job.clone()]);

    wakes_once(
        &home,
        &host,
        Fitted::FiveDollars,
        &fleet,
        &job,
        &turned,
        "The cost cap is nearly spent: $4.25 of $5.00",
    )
    .await;
}

#[tokio::test]
async fn a_turn_cap_four_fifths_spent_wakes_the_commander_once() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::FiveDollars);
    let job = working(&fleet, &home, "bound the reader").await;
    spend(&fleet, &job, 100_000, 245).await;
    let turned = fleet.turn().await.expect("a turn");

    wakes_once(
        &home,
        &host,
        Fitted::FiveDollars,
        &fleet,
        &job,
        &turned,
        "The turn cap is nearly spent: 245 of 300 turns",
    )
    .await;
}

#[tokio::test]
async fn a_job_under_four_fifths_of_both_caps_wakes_nothing() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::FiveDollars);
    let job = working(&fleet, &home, "bound the reader").await;
    spend(&fleet, &job, 3_990_000, 239).await;
    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(turned.spent, [job.clone()], "the spend was noticed");

    fleet.woken(&turned).await;
    settled(&fleet, &job).await;
    assert!(host.carried().is_empty());
}

#[test]
fn four_fifths_is_exact_and_a_cap_of_nothing_is_not_a_share() {
    let near = |spent: store::Spend, cap: Allowance| cap.nearly_spent_by(&spent);
    let spent = |cost_micros, turns| store::Spend {
        cost_micros,
        turns,
        ..store::Spend::default()
    };
    let cap = Allowance::of(Micros::dollars(5), 300);

    assert!(near(spent(3_999_999, 239), cap).is_empty());
    assert_eq!(
        near(spent(4_000_000, 0), cap),
        [Nearly {
            cap: Overspent::Cost,
            used: 4_000_000,
            of: 5_000_000
        }]
    );
    assert_eq!(
        near(spent(0, 240), cap),
        [Nearly {
            cap: Overspent::Turns,
            used: 240,
            of: 300
        }]
    );
    assert_eq!(near(spent(5_000_000, 300), cap).len(), 2, "both, and past");
    assert!(near(spent(1, 1), Allowance::of(Micros::of(0), 0)).is_empty());
    assert!(
        near(spent(u64::MAX, u64::MAX), Allowance::of(Micros::of(u64::MAX), u64::MAX)).len() == 2,
        "no overflow at the top"
    );
}

// ------------------------------------------------ the loop bound

/// **The Commander raises the cap three times and the Job has not moved.** Each
/// raise is a new cap level, so each is a fresh fact; the fourth is the loop.
#[tokio::test]
async fn a_fourth_wake_on_one_problem_with_the_job_unmoved_is_not_made() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::FiveDollars);
    let job = working(&fleet, &home, "bound the reader").await;

    for cap in [5_000_000, 5_100_000, 5_200_000, 5_300_000] {
        raise_to(&fleet, &job, cap).await;
        spend(&fleet, &job, 4_400_000, 20).await;
        let turned = fleet.turn().await.expect("a turn");
        fleet.woken(&turned).await;
        settled(&fleet, &job).await;
    }

    assert_eq!(
        woke_rows(&said(&fleet, &job).await),
        [
            "The cost cap is nearly spent: $4.40 of $5.00",
            "The cost cap is nearly spent: $4.40 of $5.10",
            "The cost cap is nearly spent: $4.40 of $5.20",
        ],
        "three wakes, and the fourth cap level wakes nothing"
    );
    assert_eq!(host.carried().len(), 3);
    let log = log_of(&home, &fleet, &job).await;
    assert_eq!(
        log.matches("woken three times for one problem").count(),
        1,
        "{log}"
    );
    assert!(log.contains("$5.30"), "the line names the problem: {log}");
}

/// **A Job that moved between two wakes is not in a loop.**
#[tokio::test]
async fn a_job_that_moved_between_wakes_starts_the_count_again() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = fleet(&home, &host, Fitted::FiveDollars);
    let job = working(&fleet, &home, "bound the reader").await;

    for (n, cap) in [5_000_000, 5_100_000, 5_200_000, 5_300_000, 5_400_000]
        .into_iter()
        .enumerate()
    {
        if n == 2 {
            let record = fleet.load(&job).await.unwrap();
            fleet
                .move_job(
                    &record,
                    Target::Escalated(EscalationTrigger::Stalled),
                    Actor::Fleet,
                )
                .await
                .expect("the Job moved");
        }
        raise_to(&fleet, &job, cap).await;
        spend(&fleet, &job, 4_400_000, 20).await;
        let turned = fleet.turn().await.expect("a turn");
        fleet.woken(&turned).await;
        settled(&fleet, &job).await;
    }

    assert_eq!(
        woke_rows(&said(&fleet, &job).await).len(),
        5,
        "all five, because the count began again when the Job moved"
    );
    assert!(!log_of(&home, &fleet, &job).await.contains("woken three times"));
}

async fn raise_to(fleet: &Arc<Hosted>, job: &JobId, micros: u64) {
    if micros > Micros::dollars(5).count() {
        fleet
            .raise_cost_cap(
                job,
                &CapRaise {
                    cost_cap_micros: micros,
                },
                RaisedBy::Person,
            )
            .await
            .expect("raised");
    }
}

async fn log_of(home: &TempDir, fleet: &Arc<Hosted>, job: &JobId) -> String {
    let record = fleet.load(job).await.unwrap();
    std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &record.handle(),
    ))
    .expect("the Job's log")
}

// ------------------------------------------------ nothing else wakes it

#[tokio::test]
async fn a_turn_with_nothing_in_it_wakes_nothing() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;
    let turned = fleet.turn().await.expect("a turn");
    fleet.woken(&turned).await;
    settled(&fleet, &job).await;
    assert!(host.carried().is_empty());
}
