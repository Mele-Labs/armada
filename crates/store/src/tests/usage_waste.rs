//! The waste the kept turns name: each kind on fixture turns, a request's burn handed out once among
//! the kinds, and the Jobs of a kind largest first. Haiku weighs 1, so a million input tokens is a unit.

use crate::session_turn_usage::{KeptBackground, KeptCallCarrier, KeptCallResult, KeptReading, KeptToolCall, KeptTurnSpent, KeptTurnUsage};
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

/// A call and the size of its result, carried by the request `carried`.
struct Made {
    call: KeptToolCall,
    bytes: u64,
    carried: &'static str,
}

/// A call of the main thread, made in `message` at second `second`, whose `bytes` of result `carried` carried.
fn made(id: &str, message: &str, second: u32, tool: &str, path: Option<&str>, bytes: u64, carried: &'static str) -> Made {
    Made {
        call: KeptToolCall {
            tool_use_id: id.into(),
            message_id: message.into(),
            at: format!("2026-10-17T10:00:{second:02}.000Z"),
            thread: String::new(),
            epoch: 0,
            tool: tool.into(),
            path: path.map(str::to_string),
            ..Default::default()
        },
        bytes,
        carried,
    }
}

fn keep_calls(store: &mut crate::Store, job: &str, list: Vec<Made>) {
    let reading = KeptReading {
        call_results: list.iter().map(|one| KeptCallResult { tool_use_id: one.call.tool_use_id.clone(), bytes: one.bytes }).collect(),
        carriers: list.iter().map(|one| KeptCallCarrier { tool_use_id: one.call.tool_use_id.clone(), message_id: one.carried.into() }).collect(),
        calls: list.into_iter().map(|one| one.call).collect(),
        ..Default::default()
    };
    store.keep_turns_read(job, &[], &reading, &format!("/{job}-calls.jsonl"), 3).unwrap();
}

/// A turn that wrote `input` tokens as plain input: at weight 1, a million is a unit.
fn wrote(store: &mut crate::Store, job: &str, id: &str, input: u64, subagent: bool) {
    keep(store, job, turn(id, &format!("2026-10-17T10:00:{:02}.500Z", id.trim_start_matches('m').parse::<u32>().unwrap()), [input, 0, 0], subagent), &[("reads", 1.0)], &[("reads", 1.0)]);
}

fn causes(job: &crate::WasteJobRead) -> Vec<(&str, f64)> {
    job.causes.iter().map(|(name, units)| (name.as_str(), *units)).collect()
}

#[test]
fn a_file_read_again_is_waste_by_the_repeat_results_tokens_as_a_share_of_the_request_that_carried_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // m1 calls the read, m2 carries it and reads the file again, m3 carries that and reads another file, which m4 carries.
    for (id, input) in [("m1", 10), ("m2", 100_000), ("m3", 100_000), ("m4", 100_000)] {
        wrote(&mut store, "j1", id, input, false);
    }
    keep_calls(
        &mut store,
        "j1",
        vec![
            made("t1", "m1", 1, "Read", Some("/f.rs"), 10_000, "m2"),
            made("t2", "m2", 2, "Read", Some("/f.rs"), 40_000, "m3"),
            made("t3", "m3", 3, "Read", Some("/g.rs"), 4_000, "m4"),
        ],
    );

    let read = waste(&store);
    let repeats = kind(&read, WasteKind::RepeatReads).expect("named");
    // 40,000 bytes are 10,000 tokens of m3's 100,000 input, which weighs 0.1 units.
    close(repeats.units, 0.01);
    assert_eq!((repeats.occurrences, repeats.jobs[0].occurrences), (1, 1));
    assert_eq!(causes(&repeats.jobs[0]).iter().map(|one| one.0).collect::<Vec<_>>(), ["/f.rs"]);
    close(causes(&repeats.jobs[0])[0].1, repeats.units);
    assert_eq!(repeats.jobs[0].peak_context, None);
    assert_eq!(read.kinds.len(), 1, "the first read of a file is no repeat, and nothing here is over 20 KB");
}

#[test]
fn a_result_over_20_kb_is_waste_by_the_part_past_it_and_a_repeat_is_not_counted_twice() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, input) in [("m1", 10), ("m2", 100_000), ("m3", 100_000), ("m4", 100_000)] {
        wrote(&mut store, "j1", id, input, false);
    }
    keep_calls(
        &mut store,
        "j1",
        vec![
            // Over the limit by 19,520 bytes, so 4,880 tokens of m2's 100,000.
            made("t1", "m1", 1, "Read", Some("/f.rs"), 40_000, "m2"),
            // The same file again, also over: all of it is a repeat, and none of it is a big result.
            made("t2", "m2", 2, "Read", Some("/f.rs"), 40_000, "m3"),
            // At the limit exactly is under it; a Fleet act's reply is named by its tool.
            made("t3", "m3", 3, "mcp__armada-fleet__waiting_for", None, crate::BIG_RESULT_BYTES, "m4"),
            made("t4", "m3", 4, "mcp__armada-fleet__waiting_for", None, crate::BIG_RESULT_BYTES + 4_000, "m4"),
        ],
    );

    let read = waste(&store);
    let bigs = kind(&read, WasteKind::BigResults).expect("named");
    close(bigs.units, 0.1 * 4_880.0 / 100_000.0 + 0.1 * 1_000.0 / 100_000.0);
    assert_eq!(bigs.occurrences, 2, "the repeat is not a big result, and a result at the limit is not one");
    let named = causes(&bigs.jobs[0]);
    assert_eq!(named.iter().map(|one| one.0).collect::<Vec<_>>(), ["Read", "mcp__armada-fleet__waiting_for"], "a Fleet act's reply is named by its tool");
    close(named[0].1, 0.1 * 4_880.0 / 100_000.0);
    close(named[1].1, 0.1 * 1_000.0 / 100_000.0);
    close(kind(&read, WasteKind::RepeatReads).expect("named").units, 0.1 * 10_000.0 / 100_000.0);
}

#[test]
fn a_compaction_or_another_thread_means_a_file_read_again_is_no_repeat() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, input) in [("m1", 10), ("m2", 100_000), ("m3", 100_000), ("m4", 100_000)] {
        wrote(&mut store, "j1", id, input, false);
    }
    // The second read is after a compaction, and the third is a subagent's own thread.
    let mut after = made("t2", "m2", 2, "Read", Some("/f.rs"), 4_000, "m3");
    after.call.epoch = 1;
    let mut agent = made("t3", "m3", 3, "Read", Some("/f.rs"), 4_000, "m4");
    agent.call.thread = "agent-x".into();
    keep_calls(&mut store, "j1", vec![made("t1", "m1", 1, "Read", Some("/f.rs"), 4_000, "m2"), after, agent]);
    assert!(kind(&waste(&store), WasteKind::RepeatReads).is_none(), "each read is the first its context held");

    // Read again with no compaction between, it is one.
    let mut same = made("t4", "m3", 4, "Read", Some("/f.rs"), 4_000, "m4");
    same.call.epoch = 1;
    keep_calls(&mut store, "j1", vec![same]);
    let read = waste(&store);
    let repeats = kind(&read, WasteKind::RepeatReads).expect("named");
    close(repeats.units, 0.1 * 1_000.0 / 100_000.0);
    assert_eq!(repeats.occurrences, 1);
}

#[test]
fn a_job_names_its_three_largest_causes_and_only_the_result_kinds_name_any() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // A cold turn, to show a kind that is not a result kind names no cause.
    keep(&mut store, "j1", turn("t0", "2026-10-17T09:00:00.000Z", [0, 250_000, 0], false), &[("own_words", 1.0)], &[("cold_cache", 1.0)]);
    let mut list = Vec::new();
    for (at, input) in [("m1", 10), ("m2", 1_000_000), ("m3", 1_000_000), ("m4", 1_000_000), ("m5", 1_000_000), ("m6", 1_000_000)] {
        wrote(&mut store, "j1", at, input, false);
    }
    // Four files, each read twice; the second read of each carries a result of a different size.
    for (n, size) in [(1u32, 4_000u64), (2, 8_000), (3, 12_000), (4, 16_000)] {
        list.push(made(&format!("a{n}"), "m1", n, "Read", Some(&format!("/f{n}.rs")), 100, "m2"));
        list.push(made(&format!("b{n}"), "m2", n + 10, "Read", Some(&format!("/f{n}.rs")), size, "m3"));
    }
    keep_calls(&mut store, "j1", list);

    let read = waste(&store);
    let repeats = kind(&read, WasteKind::RepeatReads).expect("named");
    let named = causes(&repeats.jobs[0]);
    assert_eq!(named.iter().map(|one| one.0).collect::<Vec<_>>(), ["/f4.rs", "/f3.rs", "/f2.rs"], "three, largest first");
    close(named[0].1, 1.0 * 4_000.0 / 1_000_000.0);
    close(repeats.units, 1.0 * (1_000.0 + 2_000.0 + 3_000.0 + 4_000.0) / 1_000_000.0);
    assert!(kind(&read, WasteKind::ColdCache).expect("named").jobs[0].causes.is_empty());
}

/// One request that is cold, a fresh start, past 300k and carries a repeat and a big result gives each kind its part
/// of the same burn in turn, and a subagent's request fills the sixth kind.
#[test]
fn all_six_kinds_take_one_requests_burn_in_order_and_no_unit_is_in_two() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    keep(&mut store, "j1", turn("m1", "2026-10-17T06:00:00.000Z", [10, 0, 0], false), &[("start_up", 1.0)], &[("start_up", 1.0)]);
    compact(&mut store, "j1", "2026-10-17T07:00:00.000Z");
    // 400k written at 1.25 is 0.5 units: 0.2 of the writes are cold, 0.3 start-up. The turn carries a 100,000 byte repeat
    // and a 60,000 byte result, which are 25,000 and 9,880 tokens of the 400,000.
    keep(
        &mut store,
        "j1",
        turn("m2", "2026-10-17T08:00:00.000Z", [0, 400_000, 0], false),
        &[("start_up", 1.0)],
        &[("cold_cache", 0.2), ("start_up", 0.3), ("reads", 0.5)],
    );
    keep_calls(
        &mut store,
        "j1",
        vec![
            made("t0", "m1", 0, "Read", Some("/f.rs"), 100, "m1"),
            made("t1", "m1", 1, "Read", Some("/f.rs"), 100_000, "m2"),
            made("t2", "m1", 2, "Bash", None, 60_000, "m2"),
        ],
    );
    keep(&mut store, "j2", turn("s1", "2026-10-17T08:30:00.000Z", [500_000, 0, 0], true), &[("reads", 1.0)], &[("reads", 1.0)]);

    let read = waste(&store);
    let ids: std::collections::BTreeSet<&str> = read.kinds.iter().map(|one| one.kind.id()).collect();
    assert_eq!(
        ids.into_iter().collect::<Vec<_>>(),
        ["big_results", "cold_cache", "fresh_starts", "main_context", "repeat_reads", "subagent_context"],
        "every kind is named"
    );
    let units = |which| kind(&read, which).expect("named").units;
    let (cold, start) = (0.5 * 0.2, 0.5 * 0.3);
    let repeat = 0.5 * 25_000.0 / 400_000.0;
    let big = 0.5 * 9_880.0 / 400_000.0;
    close(units(WasteKind::ColdCache), cold);
    close(units(WasteKind::FreshStarts), start);
    close(units(WasteKind::RepeatReads), repeat);
    close(units(WasteKind::BigResults), big);
    close(units(WasteKind::MainContext), (0.5 - cold - start - repeat - big) * 100_000.0 / 400_000.0);
    close(units(WasteKind::SubagentContext), 0.5 * 200_000.0 / 500_000.0);
    let whole: f64 = read.kinds.iter().map(|one| one.units).sum();
    assert!(whole <= read.total_units + 1e-9, "no more than the burn: {whole} of {}", read.total_units);
    let of_one_request = whole - units(WasteKind::SubagentContext);
    assert!(of_one_request < 0.5, "the five kinds of one request hold less than its burn of 0.5: {of_one_request}");
}

/// `call` as a Read of the lines `from` to `to`.
fn of_lines(mut call: Made, from: Option<u64>, to: Option<u64>) -> Made {
    call.call.from_line = from;
    call.call.to_line = to;
    call
}

fn repeats_of(store: &crate::Store) -> (f64, u32) {
    kind(&waste(store), WasteKind::RepeatReads).map_or((0.0, 0), |one| (one.units, one.occurrences))
}

/// Each read of 4,000 bytes is 1,000 of its carrier's 100,000 tokens, which weigh 0.1 units.
const ONE_REPEAT: f64 = 0.1 * 1_000.0 / 100_000.0;

#[test]
fn lines_that_do_not_overlap_are_no_repeat_and_lines_that_do_are() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, input) in [("m1", 10), ("m2", 100_000), ("m3", 100_000), ("m4", 100_000), ("m5", 100_000)] {
        wrote(&mut store, "j1", id, input, false);
    }
    // Lines 1 to 100, then 101 to 200: the second starts where the first ended.
    keep_calls(
        &mut store,
        "j1",
        vec![
            of_lines(made("t1", "m1", 1, "Read", Some("/f.rs"), 4_000, "m2"), Some(1), Some(100)),
            of_lines(made("t2", "m2", 2, "Read", Some("/f.rs"), 4_000, "m3"), Some(101), Some(200)),
        ],
    );
    assert_eq!(repeats_of(&store), (0.0, 0), "two slices of one file are no repeat");

    // 150 to 300 runs into 101 to 200.
    keep_calls(&mut store, "j1", vec![of_lines(made("t3", "m3", 3, "Read", Some("/f.rs"), 4_000, "m4"), Some(150), Some(300))]);
    let (units, occurrences) = repeats_of(&store);
    close(units, ONE_REPEAT);
    assert_eq!(occurrences, 1);

    // The whole file runs into every slice, and from a line to the end reaches 300.
    keep_calls(&mut store, "j1", vec![of_lines(made("t4", "m4", 4, "Read", Some("/f.rs"), 4_000, "m5"), None, None)]);
    let (units, occurrences) = repeats_of(&store);
    close(units, 2.0 * ONE_REPEAT);
    assert_eq!(occurrences, 2);
    let open_ended = of_lines(made("t5", "m4", 5, "Read", Some("/f.rs"), 4_000, "m5"), Some(250), None);
    keep_calls(&mut store, "j1", vec![open_ended]);
    assert_eq!(repeats_of(&store).1, 3);
}

#[test]
fn a_read_after_the_thread_edited_the_file_is_no_repeat_and_another_threads_edit_does_not_count() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, input) in [("m1", 10), ("m2", 100_000), ("m3", 100_000), ("m4", 100_000)] {
        wrote(&mut store, "j1", id, input, false);
    }
    let edit = |id: &str, message: &str, second: u32, thread: &str, path: &str, carried: &'static str| {
        let mut call = made(id, message, second, "Edit", None, 10, carried);
        call.call.edited = Some(path.into());
        call.call.thread = thread.into();
        call
    };
    keep_calls(
        &mut store,
        "j1",
        vec![
            made("t1", "m1", 10, "Read", Some("/f.rs"), 4_000, "m2"),
            edit("x1", "m2", 11, "", "/f.rs", "m3"),
            made("t2", "m2", 12, "Read", Some("/f.rs"), 4_000, "m3"),
        ],
    );
    assert_eq!(repeats_of(&store), (0.0, 0), "the file was changed between the two reads");

    // An edit of another file, and an edit in another thread, leave the next read a repeat of the last.
    keep_calls(
        &mut store,
        "j1",
        vec![
            edit("x2", "m3", 13, "", "/g.rs", "m4"),
            edit("x3", "m3", 14, "agent-x", "/f.rs", "m4"),
            made("t3", "m3", 15, "Read", Some("/f.rs"), 4_000, "m4"),
        ],
    );
    let (units, occurrences) = repeats_of(&store);
    close(units, ONE_REPEAT);
    assert_eq!(occurrences, 1);
}

mod again;
