//! A running Job names nothing it waits on, until a step is between states.
//!
//! Beside `issues`, which drives the same real child under a pushed clock.

use std::sync::Arc;

use api::Queries;
use core_model::StepState;
use ipc::JobWaiting;

use crate::job_waiting::waiting_of;
use crate::silence::Liveness;
use crate::tests::planted::Held;
use crate::tests::silence::{a_drone_that_goes_quiet, one_step, started, watching, QUIET_AFTER};
use crate::tests::tmp::TempDir;

/// A Job with a Drone on it is working, and a step between states is not.
#[tokio::test]
async fn a_running_job_with_a_drone_names_nothing_until_a_step_is_between_states() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(
        &home,
        one_step(),
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;
    let mut detail = fleet.get_job(ipc::JobId::from(&job)).await.expect("reads");
    assert_eq!(detail.waiting, []);

    let step_id = detail.steps[0].step_id.clone();
    for between in [StepState::NotStarted, StepState::Retrying] {
        detail.steps[0].state = between.into();
        assert_eq!(
            waiting_of(&detail, Vec::new()),
            [JobWaiting::Transition {
                step_id: step_id.clone()
            }],
            "{between:?}"
        );
    }
    for held in [StepState::Running, StepState::AwaitingHuman] {
        detail.steps[0].state = held.into();
        assert_eq!(waiting_of(&detail, Vec::new()), [], "{held:?}");
    }
}
