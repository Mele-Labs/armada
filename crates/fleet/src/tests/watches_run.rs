//! A watch's command run in the Job's folder, its readings kept and counted, the Job told once per
//! change, and the four ends: passed, gave up, broken, and stopped with its Job or a Stop.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Watches};
use ipc::{
    CloseSession, SessionId, SessionRow, StartWatch, StopWatch, WatchReading, WatchStarts, WatchState, WatchSummary,
    WatchVerdict, WeighWatch,
};

use super::planted::Held;
use super::repositories::Planted;
use super::session_host::{eventually, rig_fitted, Rig};
use crate::watches::{interval, notice, owed};

fn rig_held() -> (Rig, Arc<Held>) {
    let clock = Arc::new(Held::started());
    let held = Arc::clone(&clock);
    (rig_fitted(move |fittings| fittings.clock = held), clock)
}

fn cat(job: &SessionId, file: &str, gives_up_after: Option<&str>) -> StartWatch {
    StartWatch {
        session_id: Some(job.clone()),
        name: "QA deploy".to_string(),
        starts: WatchStarts::Command {
            command: vec!["/bin/cat".to_string(), file.to_string()],
            every: "30s".to_string(),
            gives_up_after: gives_up_after.map(str::to_string),
        },
        follows: None,
        set_aside: Vec::new(),
    }
}

fn line(rig: &Rig, state: &str, finished: bool, parts: &[(&str, &str)]) {
    let parts: Vec<String> = parts.iter().map(|(name, state)| format!(r#"{{"name":"{name}","state":"{state}"}}"#)).collect();
    let said = format!(
        "deploying\n{{\"state\":\"{state}\",\"finished\":{finished},\"says\":\"Deploy {state}.\",\"link\":\"https://ci.example.com/runs/7\",\"parts\":[{}]}}\n",
        parts.join(",")
    );
    std::fs::write(std::path::Path::new(&rig.root).join("line.json"), said).unwrap();
}

/// Run what is due, and wait for every command started to be kept.
async fn pass(rig: &Rig) {
    rig.fleet.watches_run_when_due().await;
    eventually(|| async { rig.fleet.watches().running_now() == 0 }).await;
}

async fn the_watch(rig: &Rig, job: &SessionId) -> WatchSummary {
    rig.fleet.list_watches(None, Some(job.clone())).await.unwrap().watches.remove(0)
}

async fn told(rig: &Rig, job: &SessionId, said: &str) -> usize {
    rig.rows(job)
        .await
        .iter()
        .filter(|row| matches!(row, SessionRow::Message { text, .. } if text.contains(said)))
        .count()
}

#[tokio::test]
async fn a_failed_part_tells_once_and_a_part_set_aside_lets_it_pass() {
    let (rig, clock) = rig_held();
    let job = rig.start().await;
    line(&rig, "failed", false, &[("build", "passed"), ("deploy-api", "failed")]);
    let id = rig.fleet.start_watch(None, cat(&job, "line.json", None)).await.unwrap().id;
    pass(&rig).await;

    let read = the_watch(&rig, &job).await;
    assert_eq!((read.state, read.live, read.readings.len()), (WatchState::Failed, true, 1));
    assert!(matches!(&read.last, Some(WatchReading::Read { parts, .. }) if parts.len() == 2), "{:?}", read.last);
    let failed = "QA deploy failed: deploy-api failed. https://ci.example.com/runs/7";
    eventually(|| async { told(&rig, &job, failed).await == 1 }).await;

    clock.on(31);
    pass(&rig).await;
    clock.on(61);
    rig.fleet.watches_told_when_due().await;
    assert_eq!(told(&rig, &job, failed).await, 1, "the same failure read again is not news");
    let again = the_watch(&rig, &job).await;
    assert!(
        matches!(&again.readings[..], [WatchReading::Read { count: 2, last_at: Some(_), .. }]),
        "both kept, answered as one: {:?}",
        again.readings
    );
    assert!(matches!(&again.last, Some(WatchReading::Read { count: 1, last_at: None, .. })), "the newest as taken");

    let not_ours = WatchVerdict::NotOurs { sentence: "Another team's service.".into(), parts: vec!["deploy-api".into()] };
    let weighed = rig
        .fleet
        .weigh_watch(None, WeighWatch { session_id: Some(job.clone()), watch: id.clone(), verdict: not_ours })
        .await
        .unwrap();
    assert_eq!((weighed.state, weighed.live), (WatchState::Passed, true), "passed, and not finished");
    assert_eq!(weighed.verdicts.len(), 1);
    assert_eq!(weighed.verdicts[0].sentence, "Another team's service.");
    assert_eq!(weighed.set_aside[0].part, "deploy-api");

    line(&rig, "failed", true, &[("build", "passed"), ("deploy-api", "failed")]);
    clock.on(31);
    pass(&rig).await;
    let ended = the_watch(&rig, &job).await;
    assert_eq!((ended.state, ended.live), (WatchState::Passed, false), "counted passed and finished");
    eventually(|| async { told(&rig, &job, "QA deploy passed.").await == 1 }).await;
}

#[tokio::test]
async fn three_broken_readings_stop_it_and_say_what_it_printed() {
    let (rig, clock) = rig_held();
    let job = rig.start().await;
    rig.fleet.start_watch(None, cat(&job, "missing.json", None)).await.unwrap();
    for _ in 0..3 {
        pass(&rig).await;
        clock.on(31);
    }
    let ended = the_watch(&rig, &job).await;
    assert_eq!((ended.state, ended.live), (WatchState::Broken, false));
    assert!(matches!(&ended.last, Some(WatchReading::Broken { why, output: Some(out), .. })
        if why == "It exited 1." && out.contains("missing.json")), "{:?}", ended.last);
    eventually(|| async { told(&rig, &job, "QA deploy stopped: its command failed three times.").await == 1 }).await;
}

#[tokio::test]
async fn it_gives_up_on_time_and_says_what_it_last_read() {
    let (rig, clock) = rig_held();
    let job = rig.start().await;
    line(&rig, "running", false, &[]);
    rig.fleet.start_watch(None, cat(&job, "line.json", Some("1m"))).await.unwrap();
    pass(&rig).await;
    clock.on(61);
    pass(&rig).await;
    let ended = the_watch(&rig, &job).await;
    assert_eq!((ended.state, ended.live), (WatchState::GaveUp, false));
    eventually(|| async { told(&rig, &job, "QA deploy gave up after 1m. It last said: Deploy running.").await == 1 }).await;
}

#[tokio::test]
async fn a_stop_ends_the_running_command_and_keeps_no_reading() {
    let (rig, _clock) = rig_held();
    let job = rig.start().await;
    let mut start = cat(&job, "-", None);
    start.starts = WatchStarts::Command { command: vec!["/bin/sleep".into(), "30".into()], every: "30s".into(), gives_up_after: None };
    let id = rig.fleet.start_watch(None, start).await.unwrap().id;
    rig.fleet.watches_run_when_due().await;
    assert!(rig.fleet.watches().is_running(&id));
    let stopped = rig.fleet.stop_watch(None, StopWatch { session_id: None, watch: id.clone() }).await.unwrap();
    assert_eq!(stopped.state, WatchState::Stopped);
    eventually(|| async { !rig.fleet.watches().is_running(&id) }).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(the_watch(&rig, &job).await.readings.is_empty(), "a run cut short by a Stop is kept nowhere");
}

#[tokio::test]
async fn a_job_that_ends_ends_its_watches() {
    let (rig, _clock) = rig_held();
    let job = rig.start().await;
    line(&rig, "running", false, &[]);
    rig.fleet.start_watch(None, cat(&job, "line.json", None)).await.unwrap();
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job.clone() }).await.expect("closed");
    let ended = the_watch(&rig, &job).await;
    assert_eq!((ended.state, ended.live), (WatchState::Stopped, false));
}

const TOKEN: &str = "t0ken-0f-the-0wner-1234";

#[tokio::test]
async fn a_recipes_requests_environment_is_given_and_its_secret_kept_nowhere() {
    let planted = Arc::new(Planted::nothing());
    planted.will_offer_recipes(vec![config::WatchRecipeWritten::in_repository(
        std::path::PathBuf::from(".armada/watches/qa.yml"),
        "name: qa\ncommand: [/bin/sh, watch.sh]\nevery: 30s\nenvironment: staging\n".to_string(),
    )]);
    let rig = rig_fitted(move |fittings| fittings.locating = planted);
    let environment = ipc::RequestEnvironmentWrite {
        name: "staging".to_string(),
        variables: vec![ipc::VariableWrite::Secret { name: "token".to_string(), value: Some(TOKEN.to_string()) }],
    };
    let save = ipc::SaveRequestEnvironment { repository: Some(rig.root.clone()), session_id: None, environment };
    api::Requests::save_request_environment(&*rig.fleet, None, save).await.expect("kept");
    std::fs::write(
        std::path::Path::new(&rig.root).join("watch.sh"),
        "printf '{\"state\":\"running\",\"finished\":false,\"says\":\"%s %s\"}\\n' \"$token\" \"$ARMADA_SPAWNED_BY\"\n",
    )
    .unwrap();
    let job = rig.start().await;
    let start = StartWatch {
        session_id: Some(job.clone()),
        name: "QA deploy".into(),
        starts: WatchStarts::Recipe { recipe: "qa".into(), values: Default::default() },
        follows: None,
        set_aside: Vec::new(),
    };
    assert_eq!(rig.fleet.start_watch(None, start).await.expect("started").environment.as_deref(), Some("staging"));
    pass(&rig).await;
    let read = the_watch(&rig, &job).await;
    let Some(WatchReading::Read { says, .. }) = &read.last else { panic!("{:?}", read.last) };
    assert!(says.starts_with("{{token}} "), "the value is given and taken back out: {says}");
    assert!(!format!("{read:?}").contains(TOKEN));
    if let Some(mark) = crate::orphans::own_mark() {
        assert!(says.ends_with(&mark), "the crash sweep's mark: {says}");
    }
}

#[test]
fn a_watch_slows_when_nothing_changes() {
    let minutes = |m: u64| Duration::from_secs(m * 60);
    assert_eq!(interval(30, minutes(29)), Duration::from_secs(30));
    assert_eq!(interval(30, minutes(30)), minutes(2));
    assert_eq!(interval(30, minutes(60)), minutes(5));
    assert_eq!(interval(600, minutes(90)), minutes(10), "a slower ask stands");
}

#[test]
fn waiting_and_running_owe_nothing() {
    for state in [core_model::ReadingState::Waiting, core_model::ReadingState::Running] {
        let mut watch = core_model::Watch::new("QA".into(), vec!["x".into()], 60, 60);
        watch.read(core_model::Reading { state, finished: false, says: "on".into(), link: None, parts: Vec::new() });
        assert_eq!(owed(&watch), None);
        let kept = store::KeptWatchWhole {
            watch: store::KeptWatch {
                id: "w-1".into(),
                session_id: "s".into(),
                name: "QA".into(),
                command: vec!["x".into()],
                recipe: None,
                environment: None,
                every_seconds: 60,
                gives_up_after_seconds: 60,
                follows: None,
                state: watch.state.as_wire().into(),
                live: true,
                started_at: String::new(),
                reported: Vec::new(),
                changed_at: String::new(),
                ended_at: None,
                unsaid: None,
                in_terminal: None,
                regex: false,
            },
            readings: Vec::new(),
            set_aside: Vec::new(),
            verdicts: Vec::new(),
        };
        assert_eq!(notice(&kept, &watch), None);
    }
}
