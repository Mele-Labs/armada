//! The fold of a Job's time: values in, a split out, with no Fleet. A second goes to the first
//! bucket in the order that claims it, and the buckets sum to the whole.

use crate::job_time::{fold, spans_between, stints, Span, Times, Turn};
use crate::job_timing::turn_of;
use store::KeptTurnTimed;

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

#[test]
fn a_question_to_the_owner_is_his_time_and_not_a_tool_s() {
    let mut times = job(1000);
    times.turns = vec![Turn { tools_began: Some(100), tools_ended: Some(900), called_tools: true, asked_owner: true, ..turn(0, 100) }];
    let split = fold(&times).split;
    assert_eq!((split.tools, split.you, split.model), (0, 800, 100));
}

#[test]
fn work_still_wins_over_the_wait_on_a_question_so_a_check_running_through_it_is_a_check() {
    let mut times = job(1000);
    times.turns = vec![Turn { tools_began: Some(100), tools_ended: Some(900), called_tools: true, asked_owner: true, ..turn(0, 100) }];
    times.checks = vec![span(300, 400)];
    let split = fold(&times).split;
    assert_eq!((split.checks, split.you), (100, 700));
}

fn kept(tools: &[&str]) -> KeptTurnTimed {
    KeptTurnTimed {
        at: "2026-11-02T09:00:10.000Z".into(),
        tools: tools.iter().map(|name| name.to_string()).collect(),
        model_began: Some("2026-11-02T09:00:00.000Z".into()),
        tools_began: Some("2026-11-02T09:00:10.000Z".into()),
        tools_ended: Some("2026-11-02T09:30:10.000Z".into()),
    }
}

#[test]
fn a_turn_that_called_the_question_tool_is_read_as_waiting_on_the_owner_and_no_other_is() {
    assert!(turn_of(&kept(&["Read", "AskUserQuestion"])).unwrap().asked_owner);
    let shell = turn_of(&kept(&["Bash"])).unwrap();
    assert!(!shell.asked_owner && shell.called_tools);
    assert!(!turn_of(&kept(&[])).unwrap().called_tools);
}
