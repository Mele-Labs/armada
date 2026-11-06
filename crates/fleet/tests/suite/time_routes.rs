//! The time reads, served: a Job's split beside its workflow's usual, a window's sums, and the hours
//! with the builds that ran in them, from a store with rows in it and from one with none. A figure
//! the store has none for stays absent on the wire and is never a zero.

use api::Sessions;
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport};
use store::{KeptBuildSplit, KeptJobTime, KeptReading, KeptTurnTimes, KeptTurnUsage, NewBuild};
use testkit::FakeWorkProduct;

use crate::daemon::a_fleet;
use fleet::tests::tmp::TempDir;

const MINUTE: i64 = 60_000;

async fn started(fleet: &impl Sessions, id: &str, title: Option<&str>) {
    let fact = SessionFact::Started { cwd: "/work/a".into(), title: title.map(str::to_string), origin: SessionOrigin::Terminal, mod_version: None };
    fleet.report_session(SessionReport { harness: "a_harness".into(), session_id: SessionId::carried(id), fact }).await.unwrap();
}

fn job(id: &str, workflow: &str, you_minutes: i64, builds: Vec<KeptBuildSplit>) -> KeptJobTime {
    KeptJobTime {
        session_id: id.into(),
        repository: "armada".into(),
        workflow: workflow.into(),
        began_at: "2026-11-03T09:00:00.000Z".into(),
        ended_at: "2026-11-03T12:00:00.000Z".into(),
        model_ms: 5 * MINUTE,
        tools_ms: 5 * MINUTE,
        you_ms: you_minutes * MINUTE,
        kept_at: "2026-11-03T12:00:01.000Z".into(),
        fold: 2,
        builds,
        ..Default::default()
    }
}

fn build(commit: &str, at: &str, source: Option<&str>, main: Option<&str>, branches: &[&str]) -> NewBuild {
    NewBuild {
        commit_id: commit.into(),
        started_at: at.into(),
        source: source.map(str::to_string),
        main_commit: main.map(str::to_string),
        branches: branches.iter().map(|one| one.to_string()).collect(),
        looked: source.is_some(),
    }
}

fn turn(id: &str, at: &str) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: id.into(),
        at: at.into(),
        model: "model-a".into(),
        input_tokens: 1_000_000,
        cache_creation_tokens: 0,
        cache_creation_1h_tokens: 0,
        cache_read_tokens: 0,
        output_tokens: 0,
        tools: Vec::new(),
        subagent: false,
        bash_use: None,
    }
}

#[tokio::test]
async fn a_jobs_split_is_served_beside_the_usual_of_its_workflow() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    for id in ["s1", "s2", "s3", "s4"] {
        started(&fleet, id, None).await;
    }
    {
        let mut store = fleet.store().lock().await;
        store.keep_job_time(&job("s1", "build", 10, vec![])).unwrap();
        store.keep_job_time(&job("s2", "build", 30, vec![])).unwrap();
        store.keep_job_time(&job("s4", "", 5, vec![])).unwrap();
    }

    let read = fleet.get_job_time(SessionId::carried("s1")).await.unwrap();
    let time = read.time.expect("s1 has a split");
    assert_eq!((read.session_id.as_str(), time.workflow.as_str()), ("s1", "build"));
    assert_eq!((time.spent.you, time.spent.model), (10 * MINUTE as u64, 5 * MINUTE as u64));
    let usual = time.usual.expect("two kept Jobs of the workflow");
    assert_eq!((usual.workflow.as_str(), usual.jobs, usual.split.you), ("build", 2, 20 * MINUTE as u64), "the median of 10 and 30");

    let loose = fleet.get_job_time(SessionId::carried("s4")).await.unwrap().time.expect("s4 has a split");
    assert!(loose.usual.is_none(), "a Job with no workflow is read against none");

    let unfinished = fleet.get_job_time(SessionId::carried("s3")).await.unwrap();
    assert!(unfinished.time.is_none(), "a Job known with no split answers empty, and does not fail");
    assert!(!ipc::encode(&unfinished).unwrap().contains("\"time\""), "an absent split leaves no key");
    assert!(fleet.get_job_time(SessionId::carried("nobody")).await.is_err(), "a Job Fleet does not know is refused");
}

#[tokio::test]
async fn a_windows_time_is_summed_with_its_slowest_jobs_and_each_workflows_usual() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    started(&fleet, "s1", Some("Add the audit table")).await;
    started(&fleet, "s2", None).await;
    {
        let mut store = fleet.store().lock().await;
        store.keep_job_time(&job("s1", "build", 10, vec![])).unwrap();
        store.keep_job_time(&job("s2", "build", 30, vec![])).unwrap();
    }
    let bounds = (Some("2026-11-03T00:00:00Z".to_string()), Some("2026-11-04T00:00:00Z".to_string()));

    let read = fleet.get_usage_time(bounds.0.clone(), bounds.1.clone(), None).await.unwrap();
    assert_eq!((read.jobs, read.total.you, read.total.model), (2, 40 * MINUTE as u64, 10 * MINUTE as u64));
    let slowest: Vec<(&str, Option<&str>)> = read.slowest.iter().map(|one| (one.session_id.as_str(), one.title.as_deref())).collect();
    assert_eq!(slowest, [("s2", None), ("s1", Some("Add the audit table"))], "longest first");
    assert_eq!(read.slowest[0].split.you, 30 * MINUTE as u64);
    assert_eq!((read.usual.len(), read.usual[0].workflow.as_str(), read.usual[0].jobs), (1, "build", 2));

    let one = fleet.get_usage_time(bounds.0, bounds.1, Some(1)).await.unwrap();
    assert_eq!((one.jobs, one.slowest.len()), (2, 1), "the count is of the window and the list is cut");
}

#[tokio::test]
async fn the_hours_carry_burn_and_medians_and_the_builds_that_ran_in_them() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    started(&fleet, "s1", None).await;
    {
        let mut store = fleet.store().lock().await;
        store.keep_build(&build("aaaaaaaaaaaaaaaa", "2026-11-03T09:00:00.000Z", Some("main"), Some("aaaaaaaaaaaaaaaa"), &[])).unwrap();
        store.keep_build(&build("bbbbbbbbbbbbbbbb", "2026-11-03T11:00:00.000Z", Some("preview"), Some("cccccccccccccccc"), &["armada/x"])).unwrap();
        // Git was not asked about this one, and no turn was taken on it.
        store.keep_build(&build("dddddddddddddddd", "2026-11-03T12:50:00.000Z", None, None, &[])).unwrap();
        let reading = KeptReading {
            times: vec![KeptTurnTimes { message_id: "t1".into(), model_began: Some("2026-11-03T10:05:00.000Z".into()), ..Default::default() }],
            ..Default::default()
        };
        let turns = [turn("t1", "2026-11-03T10:05:30.000Z"), turn("t2", "2026-11-03T12:30:00.000Z")];
        store.keep_turns_read("s1", &turns, &reading, "/transcript", 0).unwrap();
        store.keep_job_time(&job("s1", "build", 10, vec![KeptBuildSplit { build_id: 1, model_ms: 4_000, ..Default::default() }])).unwrap();
    }

    let read = fleet.get_usage_hours(Some("2026-11-03T10:00:00Z".into()), Some(3)).await.unwrap();

    assert_eq!((read.since.as_str(), read.until.as_str()), ("2026-11-03T10:00:00.000Z", "2026-11-03T13:00:00.000Z"));
    assert_eq!(read.hours.iter().map(|one| one.hour).collect::<Vec<_>>(), [0, 2], "the hour with nothing in it is left out");
    let (first, last) = (&read.hours[0], &read.hours[1]);
    assert_eq!((first.at.as_str(), first.model), ("2026-11-03T10:00:00.000Z", Some(30.0)));
    assert_eq!((first.burn[0].name.as_str(), first.burn[0].units), ("model-a", 1.0));
    assert_eq!((last.burn.len(), last.model, last.tools, last.checks, last.pull_request), (1, None, None, None, None), "no figure is not zero");

    let commits: Vec<&str> = read.builds.iter().map(|one| one.commit.as_str()).collect();
    assert_eq!(commits, ["aaaaaaa", "bbbbbbb", "ddddddd"], "seven characters, oldest first");
    let (main, preview, bare) = (&read.builds[0], &read.builds[1], &read.builds[2]);
    assert_eq!((main.source, main.main_commit.as_deref(), main.branches.is_empty()), (Some(ipc::BuildSource::Main), Some("aaaaaaa"), true));
    assert_eq!((preview.source, preview.main_commit.as_deref(), preview.branches.as_slice()), (Some(ipc::BuildSource::Preview), Some("ccccccc"), &["armada/x".to_string()][..]));
    assert_eq!(main.until.as_deref(), Some("2026-11-03T11:00:00.000Z"));
    assert!(bare.until.is_none());
    assert_eq!((main.turns, main.burn, main.model), (1, Some(1_000_000.0), Some(30.0)), "a million units a turn, from one million-token turn");
    assert_eq!(main.jobs, ["s1"]);
    assert_eq!(main.split.model, 4_000);
    assert_eq!((preview.turns, preview.burn, preview.model), (1, Some(1_000_000.0), None));
    assert_eq!((bare.source, bare.main_commit.as_deref(), bare.turns, bare.burn), (None, None, 0, None), "a build with no turn has no burn, not NaN");
    assert_eq!((bare.model, bare.tools, bare.checks, bare.pull_request), (None, None, None, None));
    let wire = ipc::encode(bare).unwrap();
    let figures = &wire[..wire.find("\"split\"").expect("a split")];
    assert!(!figures.contains("burn") && !figures.contains("model") && !figures.contains("null"), "{wire}");
}

#[tokio::test]
async fn a_store_with_nothing_in_it_answers_empty_and_a_bound_that_is_no_time_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));

    let hours = fleet.get_usage_hours(None, None).await.unwrap();
    assert!(hours.hours.is_empty() && hours.builds.is_empty());
    let time = fleet.get_usage_time(None, None, None).await.unwrap();
    assert_eq!((time.jobs, time.total.you, time.slowest.len(), time.usual.len()), (0, 0, 0, 0));

    assert!(fleet.get_usage_hours(Some("tomorrow".into()), None).await.is_err());
    assert!(fleet.get_usage_time(Some("lately".into()), None, None).await.is_err());
    assert!(fleet.get_usage_time(None, Some("lately".into()), None).await.is_err());
}
