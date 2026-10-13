//! A redirect marks the Job as told, and the row says when, so a surface can
//! tell a Job somebody has since spoken to from one that was left alone.

use crate::tests::redirect::{a_drone_that_answers, a_fleet_with, advice, refused};
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn a_job_nobody_has_told_has_no_instant_on_its_row() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let record = fleet.load(&job).await.expect("the Job");
    let row = fleet.summarised(&record).await.expect("a row");

    assert_eq!(row.last_told_at, None);
}

#[tokio::test]
async fn a_redirect_puts_the_instant_it_was_sent_on_the_row() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    fleet
        .redirect(&job, &advice(), api::Steerer::Person)
        .await
        .expect("a Drone to redirect");
    let record = fleet.load(&job).await.expect("the Job");
    let row = fleet.summarised(&record).await.expect("a row");

    assert!(row.last_told_at.is_some());
}
