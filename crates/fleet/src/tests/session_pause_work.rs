//! A pause halts the Job's work: its running Checks are stopped before the slot is parked, and its
//! watches wait and start again at the Resume.

use std::sync::Arc;

use api::{HostedSessions, Watches};
use ipc::{
    SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckState, SessionCheckStop, StartWatch,
    WatchStarts,
};

use super::planted::Held;
use super::session_host::{eventually, rig_fitted};
use super::session_pause::{holding_slot_one, pause, resume};

fn call(act: SessionCheckAct, cwd: &str, run: Option<u64>) -> SessionCheckCall {
    SessionCheckCall {
        act,
        cwd: cwd.into(),
        name: "components_test".into(),
        target: None,
        run,
        outcome: (act == SessionCheckAct::End).then_some(SessionCheckState::Failed),
        took_ms: None,
        log: None,
    }
}

#[tokio::test]
async fn a_pause_stops_the_jobs_running_checks_before_the_slot_is_parked() {
    let rig = rig_fitted(|_| {});
    let id = holding_slot_one(&rig).await;
    let slot = adapter_traits::slot_path(&rig.root, 1);
    let SessionCheckAnswer { run: Some(run), .. } =
        rig.fleet.session_check_reported(call(SessionCheckAct::Start, &slot, None)).await.expect("begins")
    else {
        panic!("the Job's run was not placed");
    };
    // The run's `armada check`: it asks at each chunk, and ends the Check when told to stop.
    let cli = {
        let fleet = Arc::clone(&rig.fleet);
        let slot = slot.clone();
        tokio::spawn(async move {
            loop {
                let heard = fleet.session_check_reported(call(SessionCheckAct::Chunk, &slot, Some(run))).await;
                if let Some(stop) = heard.ok().and_then(|answer| answer.stop) {
                    let _ = fleet.session_check_reported(call(SessionCheckAct::End, &slot, Some(run))).await;
                    return stop;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
    };

    rig.fleet.pause_session(pause(&id)).await.expect("paused");

    let running = rig.fleet.store().lock().await.running_session_checks(id.as_str()).unwrap();
    assert!(running.is_empty(), "a Check still runs in the parked slot: {running:?}");
    assert_eq!(cli.await.expect("the run heard"), SessionCheckStop::Force);
}

#[tokio::test]
async fn a_paused_jobs_watches_wait_and_start_again_at_the_resume() {
    let clock = Arc::new(Held::started());
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| fittings.clock = held);
    let id = holding_slot_one(&rig).await;
    // Where a watch of a Job with no slot reads, so a run for the paused Job shows as a reading.
    std::fs::write(std::path::Path::new(&rig.root).join("line.json"), "{\"state\":\"passed\",\"finished\":false}\n").unwrap();
    rig.fleet
        .start_watch(
            None,
            StartWatch {
                session_id: Some(id.clone()),
                name: "QA deploy".to_string(),
                starts: WatchStarts::Command {
                    command: vec!["/bin/cat".to_string(), "line.json".to_string()],
                    every: "30s".to_string(),
                    gives_up_after: None,
                },
                follows: None,
                set_aside: Vec::new(),
            },
        )
        .await
        .expect("started");
    rig.fleet.pause_session(pause(&id)).await.expect("paused");

    clock.on(31);
    rig.fleet.watches_run_when_due().await;
    eventually(|| async { rig.fleet.watches().running_now() == 0 }).await;
    let readings = |rig: &super::session_host::Rig| {
        let fleet = Arc::clone(&rig.fleet);
        let id = id.clone();
        async move { fleet.list_watches(None, Some(id), true).await.unwrap().watches.remove(0) }
    };
    let waiting = readings(&rig).await;
    assert!(waiting.readings.is_empty() && waiting.live, "a paused Job's watch ran: {:?}", waiting.readings);

    Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed");
    clock.on(62);
    rig.fleet.watches_run_when_due().await;
    eventually(|| async { rig.fleet.watches().running_now() == 0 }).await;
    assert_eq!(readings(&rig).await.readings.len(), 1, "it starts again");
}
