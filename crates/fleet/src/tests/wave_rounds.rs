//! What an Epic's wave strip and graph read off the wire: each pass's line,
//! and each member's waits-on edges on its Board row (#1692).

use api::Queries;
use core_model::{Approach, NewTask, PlanChange};

use crate::tests::epic::{
    a_wave_of_one, asking, document, planning_a_wave, said, the_wave_ran, wrote,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// **Each pass reads its own plan's approach**, and a return from the roll-up
/// starts the next pass rather than overwriting the first. The second wave's
/// order rides each member's Board row, so the graph is drawn off the list
/// alone.
#[tokio::test]
async fn each_pass_reads_its_own_approach_and_a_member_s_row_carries_what_it_waits_on() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    a_wave_of_one(&fleet, &job, &home).await;
    the_wave_ran(&fleet).await;
    wrote(
        &home,
        &fleet.load(&job).await.expect("the Job"),
        ".armada/artifacts/roll-up.md",
        "# Wave 1\n\nOne Job, and it landed.\n",
    );
    submitted_by_the_one(&fleet, document("What the wave did."))
        .await
        .expect("the roll-up is reported");
    fleet.turn().await.expect("the human gate runs");
    fleet
        .request_changes(&job, &said("take the two this one made ready"))
        .await
        .expect("another wave is asked for");
    fleet.turn().await.expect("a Drone is put back on the plan");

    fleet
        .change_plan(
            &job,
            &PlanChange::Recorded {
                approach: Approach::new("Take the two the first wave made ready").expect("one"),
                tasks: vec![NewTask::new("Port the printer", "", &[], "").expect("a title")],
            },
        )
        .await
        .expect("the second pass records its own plan");
    let first = fleet
        .sub_dispatch(&job, &asking("port the printer"))
        .await
        .expect("proposed");
    let mut after = asking("wire the printer in");
    after.after = vec![first.as_str().to_string()];
    let second = fleet.sub_dispatch(&job, &after).await.expect("proposed");

    let detail = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Epic reads");
    assert_eq!(
        detail.wave_rounds,
        vec![
            ipc::WaveRound {
                pass: 1,
                approach: "Dispatch the one piece the split names".to_string(),
            },
            ipc::WaveRound {
                pass: 2,
                approach: "Take the two the first wave made ready".to_string(),
            },
        ],
        "each pass keeps the line it recorded"
    );

    let board = fleet.list_jobs(None).await.expect("listed");
    let row = |id: &core_model::JobId| {
        board
            .jobs
            .iter()
            .find(|row| row.id.as_str() == id.as_str())
            .expect("on the Board")
            .clone()
    };
    assert_eq!(row(&second).waits_on, vec![ipc::JobId::from(&first)]);
    assert!(row(&first).waits_on.is_empty(), "it may start at once");
    assert_eq!(row(&second).dispatched_pass, Some(2));
}

/// **A Job whose plan proposes nothing has no rounds**, and its open reads
/// nothing to say so.
#[tokio::test]
async fn a_job_whose_workflow_proposes_nothing_has_no_rounds() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    let child = a_wave_of_one(&fleet, &job, &home).await;
    let detail = fleet
        .get_job(ipc::JobId::from(&child))
        .await
        .expect("the member reads");
    assert!(detail.wave_rounds.is_empty());
}
