//! A Job's retro is handed where its time went only when a bucket ran far over the usual for its
//! workflow, read off the Jobs Fleet has kept time for.

use store::KeptJobTime;

use super::build_notices::a_rig;

const MINUTE: i64 = 60_000;

fn job(id: &str, workflow: &str, you_minutes: i64) -> KeptJobTime {
    KeptJobTime {
        session_id: id.into(),
        repository: "armada".into(),
        workflow: workflow.into(),
        began_at: "2026-11-03T09:00:00.000Z".into(),
        ended_at: "2026-11-03T12:00:00.000Z".into(),
        model_ms: 5 * MINUTE,
        tools_ms: 5 * MINUTE,
        checks_ms: 0,
        subagents_ms: 0,
        slot_ms: 0,
        you_ms: you_minutes * MINUTE,
        pull_request_ms: 0,
        stopped_ms: 0,
        idle_ms: 0,
        tool_times_missing: false,
        model_starts_guessed: false,
        timeline_cut: false,
        kept_at: "2026-11-03T12:00:01.000Z".into(),
        fold: 2,
        builds: Vec::new(),
        waits: Vec::new(),
    }
}

#[tokio::test]
async fn a_job_is_read_against_the_median_of_every_job_kept_of_its_workflow() {
    let rig = a_rig();
    {
        let mut store = rig.fleet.store().lock().await;
        for (id, workflow, you) in [("a", "build", 10), ("b", "build", 20), ("c", "build", 90), ("z", "review", 500)] {
            store.keep_job_time(&job(id, workflow, you)).unwrap();
        }
    }

    let timing = rig.fleet.timing_of("c").await.unwrap().expect("c has a usual");

    assert_eq!(timing.workflow, "build");
    assert_eq!(timing.jobs, 3);
    assert_eq!(timing.spent.you, 90 * MINUTE as u64);
    assert_eq!(timing.usual.you, 20 * MINUTE as u64, "the median of 10, 20 and 90, and not review's 500");
    assert_eq!(timing.usual.model, 5 * MINUTE as u64);
}

#[tokio::test]
async fn a_job_with_no_time_kept_or_no_workflow_has_nothing_to_be_read_against() {
    let rig = a_rig();
    rig.fleet.store().lock().await.keep_job_time(&job("loose", "", 300)).unwrap();

    assert!(rig.fleet.timing_of("loose").await.unwrap().is_none(), "a Job with no workflow has no usual");
    assert!(rig.fleet.timing_of("never-timed").await.unwrap().is_none());
}
