//! The door a redirect into a healthy Drone came through. It moves nothing, so
//! the turn written into the Drone's transcript is the only place to keep it.

use std::time::Duration;

use crate::tests::redirect::{a_drone_that_answers, a_fleet_with, advice, started, Fixture};
use crate::tests::tmp::TempDir;

/// The `via` of the redirect turn in the Drone's transcript, once it is written.
async fn redirect_turn_door(fleet: &Fixture, home: &TempDir, job: &core_model::JobId) -> Option<ipc::Via> {
    let handle = fleet.load(job).await.unwrap().handle();
    let root = home.path().to_string_lossy().to_string();
    for _ in 0..400 {
        let (rows, _) = crate::transcript::history(&root, &handle).await;
        let turn = rows.iter().find_map(|row| match &row.saw {
            ipc::Saw::Instructed { occasion, via, .. } if occasion == "redirect" => Some(*via),
            _ => None,
        });
        if let Some(via) = turn {
            return via;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the redirect turn never reached the transcript");
}

async fn redirected_through(door: &str) -> Option<ipc::Via> {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    api::carrying(ipc::Via::from_wire(door), async {
        fleet
            .redirect(&job, &advice(), api::Steerer::Person)
            .await
            .expect("a Drone that is working can be told something")
    })
    .await;
    redirect_turn_door(&fleet, &home, &job).await
}

#[tokio::test]
async fn a_redirect_from_the_phone_keeps_the_phone_on_the_drones_turn() {
    assert_eq!(redirected_through("phone").await, ipc::Via::from_wire("phone"));
}

#[tokio::test]
async fn a_redirect_from_bridge_marks_no_phone_on_the_drones_turn() {
    assert_ne!(redirected_through("bridge").await, ipc::Via::from_wire("phone"));
}
