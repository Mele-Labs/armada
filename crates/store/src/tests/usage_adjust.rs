//! What a setting would save on fixture turns, and what a change did: the subagent cache priced both ways,
//! the main thread's units past a point less the compactions it costs, and the waste before and since a
//! change. Haiku weighs 1, so a million input tokens is a unit.

use super::usage_waste::{close, compact, keep, turn};
use crate::session_turn_usage::{KeptReading, KeptToolCall};
use crate::tests::{open, TempDir};
use crate::WasteKind;

const NOW: &str = "2026-10-17T12:00:00.000Z";

/// A subagent's call in `thread`, which is how a request is known to be in it.
fn calling(store: &mut crate::Store, job: &str, thread: &str, requests: &[(&str, &str)]) {
    let calls = requests
        .iter()
        .map(|(message, at)| KeptToolCall {
            tool_use_id: format!("call-{message}"),
            message_id: message.to_string(),
            at: at.to_string(),
            thread: thread.into(),
            epoch: 0,
            tool: "Read".into(),
            ..Default::default()
        })
        .collect();
    store.keep_turns_read(job, &[], &KeptReading { calls, ..Default::default() }, &format!("/{job}-{thread}.jsonl"), 3).unwrap();
}

/// A subagent request of 100k written, found cold where `cold` and warm where not.
fn wrote(store: &mut crate::Store, id: &str, at: &str, cold: bool) {
    let part = if cold { "cold_cache" } else { "own_words" };
    keep(store, "j1", turn(id, at, [0, 100_000, 0], true), &[("own_words", 1.0)], &[(part, 1.0)]);
}

#[test]
fn an_hours_cache_prices_each_cold_rewrite_after_a_short_pause_as_a_read_and_every_other_write_dearer() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // A request ten minutes before the window, which only places the next in its thread.
    wrote(&mut store, "t-1", "2026-10-10T11:50:00.000Z", false);
    // 40 minutes after it: a cold rewrite under the hour.
    wrote(&mut store, "t0", "2026-10-10T12:30:00.000Z", true);
    // Twenty minutes on, again: a hit.
    wrote(&mut store, "t1", "2026-10-10T12:50:00.000Z", true);
    // Eighty minutes on: past the hour, so the hour would have gone cold too.
    wrote(&mut store, "t2", "2026-10-10T14:10:00.000Z", true);
    // Warm, so its writes only cost more.
    wrote(&mut store, "t3", "2026-10-10T14:15:00.000Z", false);
    // Cold, and made no call, so its thread and its gap are not known.
    wrote(&mut store, "u1", "2026-10-10T12:55:00.000Z", true);
    calling(
        &mut store,
        "j1",
        "agent-a",
        &[
            ("t-1", "2026-10-10T11:50:00.000Z"),
            ("t0", "2026-10-10T12:30:00.000Z"),
            ("t1", "2026-10-10T12:50:00.000Z"),
            ("t2", "2026-10-10T14:10:00.000Z"),
            ("t3", "2026-10-10T14:15:00.000Z"),
        ],
    );

    let read = store.subagent_cache_estimate("fortnight", NOW).unwrap().expect("priced");
    // Each hit: 100k written at 1.25 is 0.125 units, read at 0.1 is 0.01.
    close(read.saved, 2.0 * 0.115);
    assert_eq!(read.rewrites, 2);
    // The others pay 0.75 more on 100k: t-1, t2, t3 and u1.
    close(read.extra, 4.0 * 0.075);
    // A week starts after t-1, which still places t0 in its thread, so t0 is a hit there and t-1 pays nothing.
    let week = store.subagent_cache_estimate("week", NOW).unwrap().expect("priced");
    assert_eq!(week.rewrites, 2);
    close(week.extra, 3.0 * 0.075);
    assert!(store.subagent_cache_estimate("day", NOW).unwrap().is_none(), "no subagent made a request in the day");
    assert!(store.subagent_cache_estimate("month", NOW).unwrap().is_none(), "not a window");
}

#[test]
fn compacting_sooner_saves_the_units_past_the_point_less_what_each_extra_compaction_costs() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // A stretch to 1M held: its burn is 1 unit and four fifths of it is past 200k.
    keep(&mut store, "j1", turn("a", "2026-10-17T09:00:00.000Z", [1_000_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    // Its compaction reads 1M from the cache, 0.1 units.
    compact(&mut store, "j1", "2026-10-17T10:00:00.000Z");
    // The start after it, 150k all start-up: 0.15 units, under the point.
    keep(&mut store, "j1", turn("b", "2026-10-17T10:30:00.000Z", [150_000, 0, 0], false), &[("start_up", 1.0)], &[("start_up", 1.0)]);
    // Another Job's 400k, which is exactly the two hundred thousand past a half of it.
    keep(&mut store, "j2", turn("c", "2026-10-17T09:00:00.000Z", [400_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    // A subagent's request is not the main thread's, and a request at the point is under it.
    keep(&mut store, "j2", turn("s", "2026-10-17T09:05:00.000Z", [900_000, 0, 0], true), &[("reads", 1.0)], &[("reads", 1.0)]);
    keep(&mut store, "j2", turn("d", "2026-10-17T09:10:00.000Z", [200_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);

    let read = store.compaction_estimate("week", NOW, 200_000).unwrap().expect("priced");
    close(read.past, 0.8 + 0.2);
    assert_eq!(read.stretches, 2, "j1's first stretch and j2's, each compacted once more");
    close(read.each, 0.1 + 0.15);

    // Nothing was compacted in the window, so a compaction has no cost to price from.
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j2", turn("c", "2026-10-17T09:00:00.000Z", [400_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    assert!(store.compaction_estimate("week", NOW, 200_000).unwrap().is_none());
}

#[test]
fn a_change_reads_the_waste_it_cuts_before_it_and_since_once_a_day_has_passed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let at = |day: u32| format!("2026-10-{day:02}T10:00:00.000Z");
    let writes = |store: &mut crate::Store, id: &str, day: u32, cold: bool| {
        let part = if cold { "cold_cache" } else { "own_words" };
        keep(store, "j1", turn(id, &at(day), [0, 400_000, 0], false), &[("own_words", 1.0)], &[(part, 1.0)]);
    };
    // Each is 0.5 units. In the week before the change one of two is cold, and since it one of four.
    writes(&mut store, "b1", 10, true);
    writes(&mut store, "b2", 11, false);
    writes(&mut store, "s1", 14, false);
    writes(&mut store, "s2", 15, false);
    writes(&mut store, "s3", 16, true);
    writes(&mut store, "s4", 16, false);
    store.keep_settings_change("tokens.subagentCacheTtl", "2026-10-12T00:00:00.000Z", "5m", "1h").unwrap();
    // Three hours old: too new to read since.
    store.keep_settings_change("tokens.compactionWindow", "2026-10-17T09:00:00.000Z", "250000", "200000").unwrap();
    // A key nothing here names, and one past thirty days.
    store.keep_settings_change("bridge.theme", "2026-10-16T00:00:00.000Z", "dark", "light").unwrap();
    store.keep_settings_change("tokens.subagentCacheTtl", "2026-09-01T00:00:00.000Z", "1h", "5m").unwrap();

    let cuts = |key: &str| match key {
        "tokens.subagentCacheTtl" => Some(WasteKind::ColdCache),
        "tokens.compactionWindow" => Some(WasteKind::MainContext),
        _ => None,
    };
    let read = store.settings_changes_read(NOW, cuts).unwrap();
    let keys: Vec<&str> = read.iter().map(|one| one.key.as_str()).collect();
    assert_eq!(keys, ["tokens.compactionWindow", "tokens.subagentCacheTtl"], "newest first, and only what a kind is named for, of thirty days");

    let cache = &read[1];
    assert_eq!((cache.from.as_str(), cache.to.as_str(), cache.kind), ("5m", "1h", WasteKind::ColdCache));
    close(cache.before_share, 0.5);
    close(cache.since_share.expect("five days on"), 0.25);
    assert!((cache.since_days - 5.5).abs() < 1e-6, "{}", cache.since_days);

    let compaction = &read[0];
    assert!((compaction.since_days - 0.125).abs() < 1e-6, "{}", compaction.since_days);
    assert_eq!(compaction.since_share, None, "less than a day is nothing to read");
    close(compaction.before_share, 0.0);
}

#[test]
fn a_second_change_at_the_same_moment_keeps_the_later_value_and_only_six_are_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_settings_change("tokens.compactionWindow", "2026-10-17T09:00:00.000Z", "250000", "200000").unwrap();
    store.keep_settings_change("tokens.compactionWindow", "2026-10-17T09:00:00.000Z", "250000", "150000").unwrap();
    let read = store.settings_changes_read(NOW, |_| Some(WasteKind::MainContext)).unwrap();
    assert_eq!(read.iter().map(|one| one.to.as_str()).collect::<Vec<_>>(), ["150000"]);
    for hour in 1..=9 {
        store.keep_settings_change("tokens.compactionWindow", &format!("2026-10-16T{hour:02}:00:00.000Z"), "1", "2").unwrap();
    }
    assert_eq!(store.settings_changes_read(NOW, |_| Some(WasteKind::MainContext)).unwrap().len(), 6);
}
