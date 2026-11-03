//! The fold of a Job's time: values in, a split out, with no Fleet. A second goes to the first
//! bucket in the order that claims it, and the buckets sum to the whole.

use crate::job_time::{fold, spans_between, stints, Span, Times, Turn};

fn span(from: i64, to: i64) -> Span {
    Span { from, to }
}

fn turn(model_began: i64, at: i64) -> Turn {
    Turn { at, model_began: Some(model_began), ..Default::default() }
}

fn job(ended: i64) -> Times {
    Times { began: 0, ended, ..Default::default() }
}

#[test]
fn every_millisecond_lands_in_one_bucket_so_the_buckets_sum_to_the_whole() {
    let mut times = job(1000);
    times.turns = vec![turn(100, 300), Turn { tools_began: Some(250), tools_ended: Some(500), called_tools: true, ..turn(520, 600) }];
    times.checks = vec![span(550, 700)];
    times.owner = vec![span(650, 900)];
    times.subagents = vec![span(0, 50)];
    let split = fold(&times).split;
    assert_eq!(split.total(), 1000);
    assert_eq!(split.subagents, 50);
    assert_eq!(split.checks, 150);
}

#[test]
fn work_wins_over_waiting_and_a_check_wins_over_a_tool() {
    let mut times = job(100);
    times.turns = vec![Turn { tools_began: Some(10), tools_ended: Some(60), called_tools: true, ..turn(0, 10) }];
    times.checks = vec![span(30, 50)];
    times.owner = vec![span(0, 100)];
    let split = fold(&times).split;
    assert_eq!((split.checks, split.tools, split.model, split.you, split.idle), (20, 30, 10, 40, 0));
}

#[test]
fn what_waits_ranks_you_then_the_pull_request_then_stopped_then_idle() {
    let mut times = job(100);
    times.owner = vec![span(0, 20)];
    times.pull_request = vec![span(10, 50)];
    times.stopped = vec![span(40, 80)];
    let split = fold(&times).split;
    assert_eq!((split.you, split.pull_request, split.stopped, split.idle), (20, 30, 30, 20));
}

#[test]
fn a_model_wait_does_not_run_across_a_restart() {
    let mut times = job(100);
    times.turns = vec![turn(10, 90)];
    times.restarts = vec![60];
    let split = fold(&times).split;
    assert_eq!((split.model, split.idle), (30, 70));
}

#[test]
fn a_turn_with_no_start_is_guessed_last_and_the_fold_says_so() {
    let mut times = job(10 * 60 * 1000);
    times.turns = vec![Turn { at: 8 * 60 * 1000, ..Default::default() }];
    times.owner = vec![span(0, 4 * 60 * 1000)];
    let folded = fold(&times);
    assert!(folded.model_starts_guessed);
    assert_eq!(folded.split.you, 4 * 60 * 1000);
    assert_eq!(folded.split.model, 4 * 60 * 1000, "the guess is the last five minutes less what the owner holds");
}

#[test]
fn tools_with_no_end_are_missing_except_on_the_last_turn() {
    let mut times = job(100);
    let called = Turn { called_tools: true, ..turn(0, 10) };
    times.turns = vec![called.clone(), Turn { at: 40, ..called.clone() }];
    assert!(fold(&times).tool_times_missing);
    times.turns = vec![turn(0, 10), called];
    assert!(!fold(&times).tool_times_missing);
}

#[test]
fn a_job_that_ends_before_it_began_has_nothing_and_a_claim_is_cut_to_the_job() {
    assert_eq!(fold(&Times { began: 50, ended: 40, ..Default::default() }).split.total(), 0);
    let mut times = job(100);
    times.pull_request = vec![span(-50, 500)];
    assert_eq!(fold(&times).split.pull_request, 100);
}

#[test]
fn an_open_closes_at_the_next_close_after_it_or_at_the_end() {
    assert_eq!(spans_between(&[10, 30], &[20, 50], 100), [span(10, 20), span(30, 50)]);
    assert_eq!(spans_between(&[60], &[20, 50], 100), [span(60, 100)]);
}

#[test]
fn a_stint_runs_from_standing_to_the_first_version_that_does_not() {
    let versions = [(10, true), (15, true), (20, false), (30, true)];
    assert_eq!(stints(&versions, 100), [span(10, 20), span(30, 100)]);
}
