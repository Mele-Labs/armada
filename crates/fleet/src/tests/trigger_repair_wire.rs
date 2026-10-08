//! A failed Trigger's repair as Bridge reads it: each state a `job.trigger_changed`
//! carrying the repair, the held fix's files on `get_job`, and the act's refusals.

use std::sync::Arc;

use ipc::TriggerFiringState as Wire;
use testkit::{Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fitted_over;
use crate::tests::tmp::TempDir;
use crate::tests::triggering::{machine, manifest, to_the_delivering_step, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir, flag: &str) -> Fixture {
    let files = Arc::new(Files::default());
    files.say(vec![machine(
        "deploy.yml",
        "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\non_failure:\n  repair: true\n",
    )]);
    let drone = format!("touch {flag}");
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::running(
            "/bin/sh",
            &[
                "-c",
                &format!(
                    "IFS= read -r line; case \"$line\" in *\"REPAIR THE TRIGGER\"*) {drone};; *) sleep 30;; esac"
                ),
            ],
        ),
        FakeVcs::new().delivering(Delivering::default()),
    );
    fittings.starting().manifest = manifest(Some(&format!("test -f {flag}")));
    fittings.locating = Arc::new(files);
    Fleet::assembled(fittings)
}

async fn the_deploy(fleet: &Fixture, job: &ipc::JobId) -> ipc::JobTrigger {
    let detail = fleet.job_detail(job.clone()).await.unwrap();
    detail
        .triggers
        .into_iter()
        .find(|row| row.name == "deploy")
        .expect("the deploy Trigger is on the Job")
}

#[tokio::test]
async fn each_state_of_a_repair_is_an_event_and_the_held_fix_lists_its_files() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = a_fleet(&home, &flag);
    let mut heard = fleet.events().subscribe();
    let id = to_the_delivering_step(&fleet, &home).await;
    let job = ipc::JobId::from(&id);

    assert_eq!(the_deploy(&fleet, &job).await.state, Wire::Repairing);
    assert!(fleet.repair_next().await);

    let held = the_deploy(&fleet, &job).await;
    assert_eq!(held.state, Wire::FixReady);
    let repair = held.repair.expect("a repair");
    assert_eq!(repair.attempt, 1);
    assert_eq!(repair.files, ["src/log.rs"]);
    assert!(repair.branch.is_some());
    assert_eq!((repair.choice, repair.pull_request), (None, None));

    let seen = crate::tests::under_review::published(&mut heard).await;
    let moves: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            ipc::Event::JobTriggerChanged(changed) if changed.trigger.name == "deploy" => {
                Some(changed.trigger.state)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        [
            Wire::Running,
            Wire::Repairing,
            Wire::Repairing,
            Wire::Rerunning,
            Wire::FixReady
        ],
        "{seen:?}"
    );
}

#[tokio::test]
async fn the_act_is_refused_for_a_trigger_that_is_not_fix_ready() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = Arc::new(a_fleet(&home, &flag));
    let id = to_the_delivering_step(&fleet, &home).await;
    let job = ipc::JobId::from(&id);
    let ask = || ipc::ChooseTriggerFix {
        trigger: Some("deploy".into()),
        addition: None,
        choice: ipc::TriggerFixChoice::NewPr,
    };

    // Repairing, and no fix yet.
    let refused = Arc::clone(&fleet)
        .fix_chosen(job.clone(), ask())
        .await
        .expect_err("nothing to place");
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code.as_str(), "fleet.no_fix_waiting");

    // A Trigger the Job does not have is the same answer.
    let other = ipc::ChooseTriggerFix {
        trigger: Some("nothing".into()),
        ..ask()
    };
    let refused = Arc::clone(&fleet)
        .fix_chosen(job.clone(), other)
        .await
        .expect_err("none");
    assert_eq!(refused.error().code.as_str(), "fleet.no_fix_waiting");

    // Held, then placed once; the second ask finds nothing waiting.
    assert!(fleet.repair_next().await);
    let placed = Arc::clone(&fleet)
        .fix_chosen(job.clone(), ask())
        .await
        .expect("placed");
    assert_eq!(placed.state, Wire::Passed);
    assert!(placed.pull_request.is_some());
    let row = the_deploy(&fleet, &job).await;
    assert_eq!(
        row.repair.expect("a repair").choice,
        Some(ipc::TriggerFixChoice::NewPr)
    );
    let refused = Arc::clone(&fleet)
        .fix_chosen(job, ask())
        .await
        .expect_err("placed already");
    assert_eq!(refused.error().code.as_str(), "fleet.no_fix_waiting");
}
