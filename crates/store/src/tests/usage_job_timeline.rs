//! One Job's burn over its life: the bucket a span is cut into, the context line thinned, the compactions, and a
//! pause of each kind with the cold ones marked. Haiku weighs 1, so a million input tokens is a unit.

use crate::session_turn_usage::{KeptBackground, KeptReading, KeptTurnSpent, KeptTurnUsage};
use crate::tests::{open, TempDir};
use crate::usage_job_timeline::{bucket_seconds, thinned};
use crate::{JobTimelineRead, WaitedOn, MOST_CONTEXT_POINTS};

const NOW: &str = "2026-10-17T12:00:00.000Z";
const START: &str = "2026-10-17T10:00:00.000Z";

fn turn(id: &str, at: &str, tokens: [u64; 3], subagent: bool, tools: &[&str]) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: id.into(),
        at: at.into(),
        model: "haiku-4".into(),
        input_tokens: tokens[0],
        cache_creation_tokens: tokens[1],
        cache_creation_1h_tokens: 0,
        cache_read_tokens: tokens[2],
        output_tokens: 0,
        tools: tools.iter().map(|one| one.to_string()).collect(),
        subagent,
        bash_use: None,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} is not {b}");
}

/// Keeps `turn` for `job`, found cold where `cold`.
fn keep(store: &mut crate::Store, job: &str, turn: KeptTurnUsage, cold: bool) {
    let write = vec![(if cold { "cold_cache" } else { "own_words" }.to_string(), 1.0)];
    let reading = KeptReading {
        spent: vec![KeptTurnSpent {
            message_id: turn.message_id.clone(),
            cache_creation_1h_tokens: 0,
            shares: Some((vec![("own_words".to_string(), 1.0)], write)),
        }],
        ..Default::default()
    };
    store.keep_turns_read(job, &[turn], &reading, &format!("/{job}.jsonl"), 1).unwrap();
}

/// A compaction of `job` at `at`: 1M cache read, a tenth of a unit.
fn compact(store: &mut crate::Store, job: &str, at: &str) {
    let reading = KeptReading {
        background: vec![KeptBackground {
            id: format!("compaction-{at}"),
            at: at.into(),
            model: "haiku-4".into(),
            cache_read_tokens: 1_000_000,
            output_tokens: 0,
        }],
        ..Default::default()
    };
    store.keep_turns_read(job, &[], &reading, &format!("/{job}.jsonl"), 2).unwrap();
}

fn timeline(store: &crate::Store, job: &str) -> JobTimelineRead {
    store.usage_job_timeline(job, START, Some(NOW), NOW).unwrap()
}

#[test]
fn a_span_is_cut_into_the_smallest_bucket_that_keeps_it_to_120() {
    let hours = |hours: f64| bucket_seconds(hours * 3600.0);
    assert_eq!(bucket_seconds(0.0), 60, "a Job with no span still has a size");
    assert_eq!(hours(2.0), 60, "120 buckets of a minute");
    assert_eq!(bucket_seconds(7201.0), 300, "one second over, and the next size");
    assert_eq!((hours(10.0), hours(20.0), hours(30.0), hours(60.0), hours(120.0)), (300, 600, 900, 1800, 3600));
    assert_eq!(hours(1000.0), 3600, "no size is larger than an hour");
}

#[test]
fn the_context_line_is_thinned_to_400_keeping_the_largest_of_each_share_and_so_the_peak() {
    let few: Vec<(String, u64)> = (0..400).map(|at| (at.to_string(), at as u64)).collect();
    assert_eq!(thinned(few.clone()), few, "400 points are kept whole");

    let mut many: Vec<(String, u64)> = (0..1_000).map(|at| (format!("{at:04}"), 100 + (at % 7) as u64)).collect();
    many[613].1 = 9_000;
    let thin = thinned(many);
    assert_eq!(thin.len(), MOST_CONTEXT_POINTS);
    assert_eq!(thin.iter().map(|one| one.1).max(), Some(9_000), "the peak survives");
    assert!(thin.windows(2).all(|pair| pair[0].0 < pair[1].0), "still oldest first");
}

/// A Job of two hours: six main-thread requests, one subagent, one compaction, and four pauses, one of each kind.
fn a_job_with_every_pause(store: &mut crate::Store) {
    // 10:00:30 ended on a shell command, then 29 minutes and a cold cache.
    keep(store, "j1", turn("t1", "2026-10-17T10:00:30.000Z", [1_000_000, 0, 0], false, &["Read", "Bash"]), false);
    // 10:30 started a subagent, which ran at 10:35, and the next request came 20 minutes on, warm.
    keep(store, "j1", turn("t2", "2026-10-17T10:30:00.000Z", [0, 200_000, 800_000], false, &["Agent"]), true);
    keep(store, "j1", turn("s1", "2026-10-17T10:35:00.000Z", [500_000, 0, 0], true, &["Read"]), false);
    // 10:50 ended its turn with nothing pending, and a person answered 30 minutes later.
    keep(store, "j1", turn("t3", "2026-10-17T10:50:00.000Z", [100_000, 0, 0], false, &[]), false);
    compact(store, "j1", "2026-10-17T11:10:00.000Z");
    // 11:20 called a tool that is neither, and the next request was 20 minutes on.
    keep(store, "j1", turn("t4", "2026-10-17T11:20:00.000Z", [0, 0, 1_500_000], false, &["mcp__armada-fleet__waiting_for"]), false);
    keep(store, "j1", turn("t5", "2026-10-17T11:40:00.000Z", [200_000, 0, 0], false, &["Read"]), false);
    // A minute on is no pause.
    keep(store, "j1", turn("t6", "2026-10-17T11:41:00.000Z", [0, 0, 500_000], false, &[]), false);
}

#[test]
fn a_pause_is_a_gap_past_five_minutes_named_by_the_last_call_before_it_and_cold_by_the_request_after() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_every_pause(&mut store);

    let read = timeline(&store, "j1");
    let kinds: Vec<(&str, &str, WaitedOn, bool)> = read.pauses.iter().map(|one| (one.from.as_str(), one.to.as_str(), one.waited_on, one.cold)).collect();
    assert_eq!(
        kinds,
        [
            ("2026-10-17T10:00:30.000Z", "2026-10-17T10:30:00.000Z", WaitedOn::Command, true),
            ("2026-10-17T10:30:00.000Z", "2026-10-17T10:50:00.000Z", WaitedOn::Subagent, false),
            ("2026-10-17T10:50:00.000Z", "2026-10-17T11:20:00.000Z", WaitedOn::You, false),
            ("2026-10-17T11:20:00.000Z", "2026-10-17T11:40:00.000Z", WaitedOn::Other, false),
        ],
        "the subagent's request at 10:35 is no end of a pause, and a minute is no pause"
    );
    assert_eq!(read.cold_wakes, 1);
}

#[test]
fn a_task_call_is_a_subagent_and_a_gap_of_exactly_five_minutes_is_no_pause() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j1", turn("t1", "2026-10-17T10:00:00.000Z", [1, 0, 0], false, &["Task"]), false);
    keep(&mut store, "j1", turn("t2", "2026-10-17T10:05:00.000Z", [1, 0, 0], false, &["Task"]), false);
    keep(&mut store, "j1", turn("t3", "2026-10-17T10:10:01.000Z", [1, 0, 0], false, &[]), false);
    let pauses = timeline(&store, "j1").pauses;
    assert_eq!(pauses.len(), 1, "five minutes and a second is a pause, five minutes is not");
    assert_eq!((pauses[0].waited_on, pauses[0].from.as_str()), (WaitedOn::Subagent, "2026-10-17T10:05:00.000Z"));
}

#[test]
fn the_burn_is_in_buckets_of_the_main_thread_and_the_subagents_and_adds_up_to_the_jobs_summary() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_every_pause(&mut store);

    let read = timeline(&store, "j1");
    assert_eq!((read.started.as_str(), read.ended.as_deref(), read.bucket_seconds), (START, Some(NOW), 60));
    assert_eq!(read.buckets.len(), 120, "every bucket of the span, one with no burn too");
    assert_eq!(read.buckets[0].at, START);
    assert_eq!(read.buckets[119].at, "2026-10-17T11:59:00.000Z");
    close(read.buckets[0].main_units, 1.0);
    close(read.buckets[35].subagent_units, 0.5);
    close(read.buckets[35].main_units, 0.0);
    close(read.buckets[70].main_units, 0.1);

    // 1 + 0.33 + 0.5 + 0.1 + 0.1 (the compaction) + 0.15 + 0.2 + 0.05.
    close(read.total_units, 2.43);
    close(read.buckets.iter().map(|one| one.main_units + one.subagent_units).sum::<f64>(), read.total_units);
    close(read.subagent_share, 0.5 / 2.43);
    let summary = store.usage_summary(Some(START), Some(NOW), NOW, 0).unwrap().unwrap();
    close(read.total_units, summary.window.units);
}

#[test]
fn the_context_is_the_main_threads_at_each_request_and_the_compactions_are_stamps() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_every_pause(&mut store);

    let read = timeline(&store, "j1");
    let tokens: Vec<u64> = read.context.iter().map(|one| one.1).collect();
    assert_eq!(tokens, [1_000_000, 1_000_000, 100_000, 1_500_000, 200_000, 500_000], "the subagent's request is not on the line");
    assert_eq!(read.context[0].0, "2026-10-17T10:00:30.000Z");
    assert_eq!(read.peak_context, 1_500_000);
    assert_eq!(read.compactions, ["2026-10-17T11:10:00.000Z"]);
}

#[test]
fn the_week_is_every_jobs_burn_in_the_seven_days_ending_where_the_span_does() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j1", turn("t1", "2026-10-17T10:30:00.000Z", [1_000_000, 0, 0], false, &[]), false);
    // Another Job's burn three days before counts, and one eight days before does not.
    keep(&mut store, "j2", turn("a1", "2026-10-14T11:00:00.000Z", [2_000_000, 0, 0], false, &[]), false);
    keep(&mut store, "j2", turn("a2", "2026-10-09T11:00:00.000Z", [4_000_000, 0, 0], false, &[]), false);
    // And burn after the span's end is not in it.
    keep(&mut store, "j2", turn("a3", "2026-10-17T12:30:00.000Z", [8_000_000, 0, 0], false, &[]), false);

    let read = timeline(&store, "j1");
    close(read.total_units, 1.0);
    close(read.week_units, 3.0);

    // A Job that runs is read up to now, which is the same week here.
    let running = store.usage_job_timeline("j1", START, None, NOW).unwrap();
    assert_eq!(running.ended, None);
    close(running.week_units, 3.0);
}

#[test]
fn a_running_job_is_read_to_now_and_a_long_one_gets_a_larger_bucket() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j1", turn("t1", "2026-10-17T00:30:00.000Z", [1_000_000, 0, 0], false, &[]), false);

    // 14 hours: 300 seconds would make 168 buckets, and 600 make 84.
    let read = store.usage_job_timeline("j1", "2026-10-16T22:00:00.000Z", None, NOW).unwrap();
    assert_eq!((read.bucket_seconds, read.buckets.len(), read.ended.as_deref()), (600, 84, None));
    close(read.buckets[15].main_units, 1.0);
    assert_eq!(read.buckets[15].at, "2026-10-17T00:30:00.000Z");
}

#[test]
fn a_request_before_the_jobs_start_moves_the_start_back_and_a_job_with_no_turns_is_one_bucket() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j1", turn("t1", "2026-10-17T09:59:00.000Z", [1_000_000, 0, 0], false, &[]), false);
    let read = timeline(&store, "j1");
    assert_eq!(read.started, "2026-10-17T09:59:00.000Z");
    close(read.buckets.iter().map(|one| one.main_units).sum::<f64>(), 1.0);

    let none = timeline(&store, "nobody");
    assert_eq!((none.buckets.len(), none.context.len(), none.pauses.len(), none.peak_context, none.cold_wakes), (120, 0, 0, 0, 0));
    close(none.subagent_share, 0.0);
    let instant = store.usage_job_timeline("nobody", NOW, Some(NOW), NOW).unwrap();
    assert_eq!(instant.buckets.len(), 1, "a span of nothing is one bucket");
}
