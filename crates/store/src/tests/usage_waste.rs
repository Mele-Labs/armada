//! The waste the kept turns name: each kind on fixture turns, a request's burn handed out once among
//! the kinds, and the Jobs of a kind largest first. Haiku weighs 1, so a million input tokens is a unit.

use crate::session_turn_usage::{KeptBackground, KeptReading, KeptTurnSpent, KeptTurnUsage};
use crate::tests::{open, TempDir};
use crate::{UsageWasteRead, WasteKind};

const NOW: &str = "2026-10-17T12:00:00.000Z";

fn turn(id: &str, at: &str, tokens: [u64; 3], subagent: bool) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: id.into(),
        at: at.into(),
        model: "haiku-4".into(),
        input_tokens: tokens[0],
        cache_creation_tokens: tokens[1],
        cache_creation_1h_tokens: 0,
        cache_read_tokens: tokens[2],
        output_tokens: 0,
        tools: vec!["Read".into()],
        subagent,
        bash_use: None,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} is not {b}");
}

fn shares(pairs: &[(&str, f64)]) -> Vec<(String, f64)> {
    pairs.iter().map(|(part, share)| (part.to_string(), *share)).collect()
}

/// Keeps `turn` for `job`, its shares by `read` and `write`.
fn keep(store: &mut crate::Store, job: &str, turn: KeptTurnUsage, read: &[(&str, f64)], write: &[(&str, f64)]) {
    let reading = KeptReading {
        spent: vec![KeptTurnSpent { message_id: turn.message_id.clone(), cache_creation_1h_tokens: 0, shares: Some((shares(read), shares(write))) }],
        ..Default::default()
    };
    store.keep_turns_read(job, &[turn], &reading, &format!("/{job}.jsonl"), 1).unwrap();
}

/// A compaction of `job` at `at`: its own request, which the transcript does not hold.
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

fn waste(store: &crate::Store) -> UsageWasteRead {
    store.usage_waste("week", NOW).unwrap().unwrap()
}

fn kind(read: &UsageWasteRead, which: WasteKind) -> Option<&crate::WasteKindRead> {
    read.kinds.iter().find(|one| one.kind == which)
}

#[test]
fn a_turn_that_found_the_cache_cold_is_waste_by_its_cold_writes_only() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // 250k written at 1.25: the cold writes are 0.3125 units, all of them.
    keep(&mut store, "j1", turn("t1", "2026-10-17T10:00:00.000Z", [0, 250_000, 0], false), &[("own_words", 1.0)], &[("cold_cache", 1.0)]);
    // A warm turn that wrote as much is no waste.
    keep(&mut store, "j1", turn("t2", "2026-10-17T10:01:00.000Z", [0, 250_000, 0], false), &[("own_words", 1.0)], &[("own_words", 1.0)]);

    let read = waste(&store);
    let cold = kind(&read, WasteKind::ColdCache).expect("named");
    close(cold.units, 0.3125);
    assert_eq!(cold.occurrences, 1);
    assert_eq!((cold.jobs.len(), cold.jobs[0].session_id.as_str(), cold.jobs[0].occurrences), (1, "j1", 1));
    assert_eq!(cold.jobs[0].peak_context, None, "only the context kinds carry a peak");
    assert_eq!(read.kinds.len(), 1, "a kind with nothing is left out");
    close(read.total_units, 0.625);
    let summary = store.usage_summary(Some(&read.since), Some(&read.until), NOW, 0).unwrap().unwrap();
    close(read.total_units, summary.window.units);
}

#[test]
fn a_fresh_start_is_what_the_first_request_after_a_compaction_held_as_start_up() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // The Job's first start is the Job beginning, and no waste.
    keep(&mut store, "j1", turn("t1", "2026-10-17T08:00:00.000Z", [1_000_000, 0, 0], false), &[("start_up", 1.0)], &[("start_up", 1.0)]);
    compact(&mut store, "j1", "2026-10-17T09:00:00.000Z");
    // A subagent's request between is neither a start of the Job nor what stands between it and its next turn.
    keep(&mut store, "j1", turn("s1", "2026-10-17T09:10:00.000Z", [1_000_000, 0, 0], true), &[("start_up", 1.0)], &[("start_up", 1.0)]);
    // 1M read at 0.1 all start-up, and 1M input half start-up: 0.1 + 0.5.
    keep(&mut store, "j1", turn("t2", "2026-10-17T09:30:00.000Z", [1_000_000, 0, 1_000_000], false), &[("start_up", 1.0)], &[("start_up", 0.5), ("own_words", 0.5)]);
    // The turn after reads the start-up again, which is no start.
    keep(&mut store, "j1", turn("t3", "2026-10-17T10:00:00.000Z", [0, 0, 2_000_000], false), &[("start_up", 0.5), ("own_words", 0.5)], &[("own_words", 1.0)]);

    let starts = waste(&store);
    let starts = kind(&starts, WasteKind::FreshStarts).expect("named");
    close(starts.units, 0.6);
    assert_eq!((starts.occurrences, starts.jobs[0].occurrences), (1, 1));
}

#[test]
fn context_past_300k_is_waste_by_the_part_past_it_and_a_subagents_is_its_own_kind() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // 600k held, 0.51 units: half is past the limit. A turn at the limit itself is under it.
    keep(&mut store, "j1", turn("t1", "2026-10-17T10:00:00.000Z", [500_000, 0, 100_000], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    keep(&mut store, "j1", turn("t0", "2026-10-17T10:05:00.000Z", [300_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    // 400k held, 0.4 units: a quarter is past it.
    keep(&mut store, "j2", turn("t2", "2026-10-17T10:00:00.000Z", [400_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    // 500k held by a subagent, 0.5 units: two fifths.
    keep(&mut store, "j1", turn("s1", "2026-10-17T10:10:00.000Z", [500_000, 0, 0], true), &[("reads", 1.0)], &[("reads", 1.0)]);

    let read = waste(&store);
    let ids: Vec<&str> = read.kinds.iter().map(|one| one.kind.id()).collect();
    assert_eq!(ids, ["main_context", "subagent_context"], "largest first");
    let main = kind(&read, WasteKind::MainContext).expect("named");
    close(main.units, 0.255 + 0.1);
    assert_eq!(main.occurrences, 2);
    let jobs: Vec<(&str, Option<u64>)> = main.jobs.iter().map(|job| (job.session_id.as_str(), job.peak_context)).collect();
    assert_eq!(jobs, [("j1", Some(600_000)), ("j2", Some(400_000))], "largest first, each with the most it held");
    let subagents = kind(&read, WasteKind::SubagentContext).expect("named");
    close(subagents.units, 0.2);
    assert_eq!((subagents.occurrences, subagents.jobs[0].peak_context), (1, Some(500_000)));
}

#[test]
fn no_unit_is_in_two_kinds() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // 400k written at 1.25 is 0.5 units, half of them cold: the cold writes are 0.25, and of the 0.25 left
    // a quarter is past the context limit.
    keep(&mut store, "j1", turn("t1", "2026-10-17T10:00:00.000Z", [0, 400_000, 0], false), &[("reads", 1.0)], &[("cold_cache", 0.5), ("reads", 0.5)]);
    // A fresh start that is also past the limit: the start-up takes the whole 0.4, and none is left for the context.
    compact(&mut store, "j2", "2026-10-17T07:00:00.000Z");
    keep(&mut store, "j2", turn("t2", "2026-10-17T08:00:00.000Z", [400_000, 0, 0], false), &[("start_up", 1.0)], &[("start_up", 1.0)]);

    let read = waste(&store);
    close(kind(&read, WasteKind::ColdCache).expect("named").units, 0.25);
    close(kind(&read, WasteKind::FreshStarts).expect("named").units, 0.4);
    let context = kind(&read, WasteKind::MainContext).expect("named");
    close(context.units, 0.25 * 0.25);
    assert_eq!(context.jobs.iter().map(|job| job.session_id.as_str()).collect::<Vec<_>>(), ["j1"]);
    let whole: f64 = read.kinds.iter().map(|one| one.units).sum();
    assert!(whole <= 0.5 + 0.4 + 1e-9, "the kinds hold no more than the burn of the turns they read: {whole}");
}

#[test]
fn a_window_with_no_waste_has_no_kinds_and_a_window_that_is_not_one_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j1", turn("t1", "2026-10-17T10:00:00.000Z", [100_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);
    // Twelve days back: in the fortnight and not in the week.
    keep(&mut store, "j1", turn("t2", "2026-10-05T10:00:00.000Z", [500_000, 0, 0], false), &[("reads", 1.0)], &[("reads", 1.0)]);

    let week = waste(&store);
    assert!(week.kinds.is_empty());
    close(week.total_units, 0.1);
    let fortnight = store.usage_waste("fortnight", NOW).unwrap().unwrap();
    assert_eq!(fortnight.window, "fortnight");
    close(fortnight.kinds[0].units, 0.2);
    assert!(store.usage_waste("month", NOW).unwrap().is_none());
    assert!(store.usage_waste("week", "lately").unwrap().is_none());
}
