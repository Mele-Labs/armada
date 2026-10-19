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
//! | A trip is measured over the hour after it by the Job's active time, so an idle Job does not read as a win; under 5 turns is no reading | That Fleet fills it when the hour is covered, from the store. `fleet::tests::usage_watch` |
//! | `get_usage_rules` names the four rules with their on-off and limit, and each one's trips newest first, empty where it never tripped | That the route is served and the JSON is these names. `fleet::tests::usage_watch`, `api`'s route tests |

use core_model::Timestamp;
use fleet::usage_rules::KeptTurnUsage;
use fleet::usage_rules::{covered, hour_burn, measured_after, report, tripped, KeptTrip, Limits, Rule, RuleDial, Trip, Watch, COOLDOWN_MINUTES};

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
        "Context is 312k tokens. To carry on in a new job, ask the owner with the options \"Carry on in a new Job\" and \"Stay here\", and write your brief (where the work is, what is open, what you hold) in the description of \"Carry on in a new Job\". Start nothing yourself."
    );
}

#[test]
fn context_over_150k_counts_only_while_the_job_is_doing_more_than_20_turns_an_hour() {
    let busy: Vec<_> = (0..25).map(|minute| with_context(turn(minute), 160_000)).collect();
    assert_eq!(
        only(&busy).text,
        "Context is 160k tokens. To carry on in a new job, ask the owner with the options \"Carry on in a new Job\" and \"Stay here\", and write your brief (where the work is, what is open, what you hold) in the description of \"Carry on in a new Job\". Start nothing yourself."
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

/// Turns at the given minutes after the trip, each burning `output` tokens, so a unit is 200,000.
fn after(minutes: &[i64], output: u64) -> Vec<KeptTurnUsage> {
    minutes.iter().map(|minute| burning(*minute, output, false)).collect()
}

#[test]
fn the_hour_after_a_trip_is_a_burn_rate_over_the_jobs_active_time() {
    // Six turns a minute apart: five minutes active, 1.2 units, so 14.4 units an hour.
    let busy = measured_after(&after(&[1, 2, 3, 4, 5, 6], 40_000));
    assert_eq!(busy.turns, 6);
    assert!((busy.burn_per_hour.expect("a reading") - 14.4).abs() < 1e-9, "{busy:?}");

    // The same turns with 47 idle minutes in the middle: that gap counts as 5, so nine minutes
    // active and 8 units an hour. Wall clock would have made it 1.2 and a win.
    let idle = measured_after(&after(&[1, 2, 3, 50, 51, 52], 40_000));
    assert_eq!(idle.turns, 6);
    assert!((idle.burn_per_hour.expect("a reading") - 8.0).abs() < 1e-9, "{idle:?}");
}

#[test]
fn the_hour_before_a_trip_is_counted_the_same_way_and_stored_whatever_the_number_of_turns() {
    // Six turns with an idle gap in the middle: the same 8 units an hour as after.
    let idle = after(&[1, 2, 3, 50, 51, 52], 40_000);
    assert!((hour_burn(&idle) - 8.0).abs() < 1e-9);
    // Two turns three minutes apart: 0.4 units over three active minutes.
    assert!((hour_burn(&after(&[0, 3], 40_000)) - 8.0).abs() < 1e-9);
    // One turn has no gap to measure, and is stored as 0.
    assert_eq!(hour_burn(&after(&[0], 40_000)), 0.0);
    // A turn older than the hour before the newest is not in it.
    assert!((hour_burn(&after(&[0, 100, 101], 40_000)) - 24.0).abs() < 1e-9);
}

#[test]
fn fewer_than_5_turns_in_the_hour_is_a_trip_with_no_reading() {
    let few = measured_after(&after(&[1, 2, 3, 4], 40_000));
    assert_eq!((few.turns, few.burn_per_hour), (4, None));
    let none = measured_after(&[]);
    assert_eq!((none.turns, none.burn_per_hour), (0, None));
}

#[test]
fn the_hour_after_a_trip_is_covered_once_a_turn_lands_60_minutes_on() {
    let trip = Timestamp::from_rfc3339(at(10));
    assert!(!covered(&trip, &Timestamp::from_rfc3339(at(69))));
    assert!(covered(&trip, &Timestamp::from_rfc3339(at(70))));
}

fn kept(session: &str, rule: &str, minute: i64, after: Option<f64>) -> KeptTrip {
    KeptTrip {
        session_id: session.into(),
        rule: rule.into(),
        at: at(minute),
        figure: 312_000.0,
        burn_per_hour_before: 4.0,
        burn_per_hour_after: after,
        after_turns: after.map(|_| 6),
    }
}

#[test]
fn the_rules_report_names_each_rule_with_its_limit_and_its_trips_newest_first() {
    let dials = [
        RuleDial { on: true, limit: 300 },
        RuleDial { on: true, limit: 50 },
        RuleDial { on: false, limit: 70 },
        RuleDial { on: true, limit: 20 },
    ];
    let trips = [
        kept("s1", "context", 5, Some(2.5)),
        kept("s2", "context", 30, None),
        kept("s1", "subagents", 20, Some(1.0)),
    ];
    let asked = report("2026-10-11T00:00:00.000Z", "2026-10-18T12:00:00.000Z", &dials, &trips);
    let names: Vec<&str> = asked.rules.iter().map(|one| one.rule.as_str()).collect();
    assert_eq!(names, ["context", "subagents", "cache_hit", "opus_reading"]);
    let context = &asked.rules[0];
    assert_eq!((context.on, context.limit.value, context.limit.unit.as_str()), (true, 300, "thousand tokens"));
    let seen: Vec<(&str, &str, Option<f64>)> =
        context.trips.iter().map(|one| (one.session_id.as_str(), one.at.as_str(), one.burn_per_hour_after)).collect();
    assert_eq!(seen, [("s2", "2026-10-18T08:30:00.000Z", None), ("s1", "2026-10-18T08:05:00.000Z", Some(2.5))]);
    assert_eq!(context.trips[0].burn_per_hour_before, 4.0);
    assert_eq!(asked.rules[1].trips.len(), 1);
    assert_eq!((asked.rules[2].on, asked.rules[2].limit.value, asked.rules[2].limit.unit.as_str()), (false, 70, "percent"));
    assert!(asked.rules[2].trips.is_empty() && asked.rules[3].trips.is_empty());
    assert_eq!(asked.rules[3].limit.unit, "turns");

    // A reading that was not made is left out of the wire, not sent as null.
    let wire = ipc::encode(&context.trips[0]).expect("encodes");
    assert!(!wire.contains("burn_per_hour_after"), "{wire}");
    assert!(wire.contains("\"session_id\":\"s2\""), "{wire}");
}

#[test]
fn get_usage_rules_is_a_route_fleet_serves() {
    let served = api::SERVED.iter().find(|route| route.operation == "get_usage_rules").expect("served");
    assert_eq!((served.method, served.path), ("GET", "/usage/rules"));
}
