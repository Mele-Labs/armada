//! A Job that runs one Check red five times in a row is told once, naming the test that keeps
//! failing, so its agent reads the failure before paying for another full compile.

use ipc::{SessionCheckAct, SessionCheckCall, SessionCheckState, SessionId, SessionRow};

use super::session_host::{rig_placing, Rig};
use crate::tests::slot_leases::{leased, By};

const FAILING: &str = "Summary [   1.204s] 3 tests run: 2 passed, 1 failed, 0 skipped\n        \
                       FAIL [   0.018s] (1/3) store tests::reads_the_last_row\n";

fn call(act: SessionCheckAct, slot: &str, name: &str) -> SessionCheckCall {
    SessionCheckCall {
        act,
        cwd: slot.into(),
        name: name.into(),
        target: None,
        run: None,
        outcome: None,
        took_ms: None,
        log: None,
    }
}

/// One run of `name`, begun and ended as `outcome`.
async fn ran(rig: &Rig, slot: &str, name: &str, outcome: SessionCheckState) {
    let run = rig
        .fleet
        .session_check_reported(call(SessionCheckAct::Start, slot, name))
        .await
        .expect("begins")
        .run
        .expect("the Job holds the slot");
    let mut end = call(SessionCheckAct::End, slot, name);
    end.run = Some(run);
    end.outcome = Some(outcome);
    end.took_ms = Some(90_000);
    end.log = Some(FAILING.into());
    rig.fleet.session_check_reported(end).await.expect("ends");
}

async fn reds(rig: &Rig, slot: &str, name: &str, count: usize) {
    for at in 0..count {
        // A run killed for its budget is as red as one that failed.
        let outcome = if at == 1 { SessionCheckState::TimedOut } else { SessionCheckState::Failed };
        ran(rig, slot, name, outcome).await;
    }
}

async fn told(rig: &Rig, id: &SessionId) -> Vec<String> {
    let rows = rig.fleet.rows_of(id.as_str()).await.expect("the thread");
    rows.into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice: Some(notice), text, .. } if notice == "red_streak" => Some(text),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn the_fifth_red_in_a_row_of_one_check_tells_the_job_once_and_a_new_streak_tells_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    let slot = leased(&rig.root, 2, "fleet/topic", By::Job(id.as_str()));

    reds(&rig, &slot, "test", 4).await;
    // Another Check's reds are not this one's streak.
    reds(&rig, &slot, "components_test", 1).await;
    assert!(told(&rig, &id).await.is_empty(), "four reds of one Check tell nothing");

    reds(&rig, &slot, "test", 1).await;
    assert_eq!(
        told(&rig, &id).await,
        ["`test` ended red five runs in a row, failing `tests::reads_the_last_row`. Each run is a full compile. \
          Read the failure and change the code before you run it again."],
    );

    reds(&rig, &slot, "test", 1).await;
    assert_eq!(told(&rig, &id).await.len(), 1, "the sixth red of the streak tells nothing");

    ran(&rig, &slot, "test", SessionCheckState::Passed).await;
    reds(&rig, &slot, "test", 4).await;
    assert_eq!(told(&rig, &id).await.len(), 1, "a pass ends the streak");
    reds(&rig, &slot, "test", 1).await;
    assert_eq!(told(&rig, &id).await.len(), 2, "five reds after a pass tell again");
}

/// Past three failing tests the word counts the rest; with none named it says where the log blames.
#[test]
fn the_word_names_three_tests_then_counts_the_rest() {
    let tests: Vec<String> = ["a", "b", "c", "d", "e"].map(String::from).into();
    assert_eq!(
        crate::session_checking::red_streak_said("test", &tests, None),
        "`test` ended red five runs in a row, failing `a`, `b`, `c` and 2 others. Each run is a full compile. \
         Read the failure and change the code before you run it again."
    );
    assert_eq!(
        crate::session_checking::red_streak_said("test", &tests[..2], None),
        "`test` ended red five runs in a row, failing `a` and `b`. Each run is a full compile. \
         Read the failure and change the code before you run it again."
    );
    assert_eq!(
        crate::session_checking::red_streak_said("lint", &[], Some("crates/store/src/reads.rs:88")),
        "`lint` ended red five runs in a row, failing at crates/store/src/reads.rs:88. Each run is a full compile. \
         Read the failure and change the code before you run it again."
    );
}
