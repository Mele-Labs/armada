//! Token watching's claim: **Fleet tells a Job once, with the number and one action, when its
//! usage crosses a rule's limit; staying over tells it nothing more; it is told again only after the
//! figure has fallen back under the limit and a cooldown has passed; and a Job under every limit
//! hears nothing.**
//!
//! The turns are held values, the clock is the turns' own stamps, and nothing is read from a file or
//! the store.
//!
//! | Proved | Not proved |
//! |---|---|
//! | Each of the four rules trips on its figure, with the words the Job is sent | That Fleet reads the turns from a transcript, delivers the words to a Job's thread and keeps the trip row. `fleet::tests::usage_watch` |
//! | A Job over a limit is due once, stays not due while over, and is due again only after it fell under and the cooldown passed | That the limits are settings.json keys. `config`'s settings tests |
//! | A rule switched off, and a Job under every limit, trip nothing | |

use core_model::Timestamp;
use fleet::usage_rules::KeptTurnUsage;
use fleet::usage_rules::{tripped, Limits, Rule, Trip, Watch, COOLDOWN_MINUTES};

/// The stamp `minute` minutes after 08:00 on the day of the case.
fn at(minute: i64) -> String {
    format!("2026-10-18T{:02}:{:02}:00.000Z", 8 + minute / 60, minute % 60)
}

/// A main-thread turn with nothing wrong with it: 91k of context, 99% of it read from the cache,
/// sonnet, an edit.
fn turn(minute: i64) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: format!("m{minute}"),
        at: at(minute),
        model: "model-sonnet-x".into(),
        input_tokens: 10,
        cache_creation_tokens: 1_000,
        cache_read_tokens: 90_000,
        output_tokens: 500,
        tools: vec!["Edit".into()],
        subagent: false,
        bash_use: None,
    }
}

fn with_context(mut turn: KeptTurnUsage, context: u64) -> KeptTurnUsage {
    turn.cache_creation_tokens = 0;
    turn.input_tokens = 0;
    turn.cache_read_tokens = context;
    turn
}

/// Only output tokens, so a unit is 200,000 of them.
fn burning(minute: i64, output: u64, subagent: bool) -> KeptTurnUsage {
    KeptTurnUsage {
        input_tokens: 0,
        cache_creation_tokens: 0,
        cache_read_tokens: 0,
        output_tokens: output,
        subagent,
        message_id: format!("b{minute}"),
        ..turn(minute)
    }
}

fn quiet(count: i64) -> Vec<KeptTurnUsage> {
    (0..count).map(|minute| turn(minute * 3)).collect()
}

fn trips(turns: &[KeptTurnUsage]) -> Vec<Trip> {
    tripped(&Limits::shipped(), turns)
}

fn only(turns: &[KeptTurnUsage]) -> Trip {
    let mut found = trips(turns);
    assert_eq!(found.len(), 1, "{found:#?}");
    found.remove(0)
}

#[test]
fn a_job_under_every_limit_hears_nothing() {
    assert!(trips(&quiet(40)).is_empty());
    assert!(trips(&[]).is_empty());
}

#[test]
fn context_over_300k_is_told_with_the_figure_and_one_action() {
    let mut turns = quiet(5);
    turns.push(with_context(turn(20), 312_000));
    let found = only(&turns);
    assert_eq!(found.rule, Rule::Context);
    assert_eq!(found.figure.round() as u64, 312_000);
    assert_eq!(
        found.text,
        "Context is 312k tokens. End this thread at the next boundary and carry on in a new one."
    );
}

#[test]
fn context_over_150k_counts_only_while_the_job_is_doing_more_than_20_turns_an_hour() {
    let busy: Vec<_> = (0..25).map(|minute| with_context(turn(minute), 160_000)).collect();
    assert_eq!(
        only(&busy).text,
        "Context is 160k tokens. End this thread at the next boundary and carry on in a new one."
    );
    let slow: Vec<_> = (0..25).map(|minute| with_context(turn(minute * 4), 160_000)).collect();
    assert!(trips(&slow).is_empty(), "16 turns in the last hour");
}

#[test]
fn subagents_running_most_of_the_last_hour_are_told_with_the_share() {
    let mut turns: Vec<_> = (0..5).map(|minute| burning(minute, 58_000, false)).collect();
    turns.extend((5..10).map(|minute| burning(minute, 142_000, true)));
    let found = only(&turns);
    assert_eq!(found.rule, Rule::Subagents);
    assert_eq!(
        found.text,
        "Subagents ran 71% of your last hour. Pass model sonnet on reading and searching agents and keep at most two running."
    );
    // The same share on an hour that burned under a unit says nothing.
    let small = [burning(0, 10_000, false), burning(1, 20_000, true), burning(2, 20_000, true)];
    assert!(trips(&small).is_empty());
}

#[test]
fn a_cache_hit_rate_under_70_percent_over_the_last_10_turns_is_told() {
    let mut turns = quiet(4);
    turns.extend((0..10).map(|minute| {
        let mut one = turn(20 + minute);
        one.cache_creation_tokens = 46_000;
        one.cache_read_tokens = 54_000;
        one.input_tokens = 0;
        one
    }));
    let found = only(&turns);
    assert_eq!(found.rule, Rule::CacheHit);
    assert_eq!(
        found.text,
        "Cache hit rate is 54% over your last 10 turns. Something in the prompt or tools changed each turn; check hooks and MCP servers."
    );
}

fn reading(minute: i64, opus: bool) -> KeptTurnUsage {
    let mut one = turn(minute);
    one.model = if opus { "model-opus-x".into() } else { "model-sonnet-x".into() };
    one.tools = vec!["Bash".into()];
    one.bash_use = Some("read_search".into());
    if minute % 2 == 1 {
        one.tools = vec!["Read".into()];
        one.bash_use = None;
    }
    one
}

#[test]
fn twenty_turns_of_reading_on_opus_are_told_and_an_edit_or_another_model_is_not() {
    let turns: Vec<_> = (0..20).map(|minute| reading(minute, true)).collect();
    let found = only(&turns);
    assert_eq!(found.rule, Rule::OpusReading);
    assert_eq!(
        found.text,
        "Your last 20 turns on opus only read and searched. Subagents for reading can run on sonnet."
    );

    let mut edited = turns.clone();
    edited[7].tools = vec!["Bash".into(), "Edit".into()];
    assert!(trips(&edited).is_empty(), "a turn that edited");
    let mut other = turns.clone();
    other[19].model = "model-sonnet-x".into();
    assert!(trips(&other).is_empty(), "a turn on another model");
    assert!(trips(&turns[1..]).is_empty(), "19 turns are not 20");
    let mut by_script = turns;
    by_script[3].tools = vec!["Bash".into()];
    by_script[3].bash_use = Some("scripts".into());
    assert!(trips(&by_script).is_empty(), "a shell command that was not reading");
}

#[test]
fn a_rule_switched_off_trips_nothing() {
    let turns = vec![with_context(turn(0), 900_000)];
    assert_eq!(trips(&turns).len(), 1);
    let off = Limits { context_tokens: None, ..Limits::shipped() };
    assert!(tripped(&off, &turns).is_empty());
}

#[test]
fn a_job_over_a_limit_is_told_once_and_again_only_after_falling_under_and_the_cooldown() {
    let watch = Watch::default();
    let job = "job-1";
    let stamp = |minute: i64| Timestamp::from_rfc3339(at(minute));
    let over = |minute: i64| watch.due(job, &trips(&[with_context(turn(minute), 312_000)]), &stamp(minute)).len();
    let under = |minute: i64| watch.due(job, &trips(&[turn(minute)]), &stamp(minute)).len();

    assert_eq!(over(0), 1, "crossing the limit");
    watch.told(job, Rule::Context, &stamp(0));
    for minute in [1, 2, 10, COOLDOWN_MINUTES + 5, 200] {
        assert_eq!(over(minute), 0, "still over at minute {minute}");
    }

    assert_eq!(under(210), 0);
    assert_eq!(over(211), 1, "fell under, and the cooldown passed long ago");
    watch.told(job, Rule::Context, &stamp(211));
    assert_eq!(under(212), 0);
    assert_eq!(over(213), 0, "fell under, but two minutes after the last telling");
    assert_eq!(under(214), 0);
    assert_eq!(over(211 + COOLDOWN_MINUTES), 1, "under since, and the cooldown has now passed");

    // Another Job is its own.
    assert_eq!(watch.due("job-2", &trips(&[with_context(turn(0), 312_000)]), &stamp(0)).len(), 1);
}
