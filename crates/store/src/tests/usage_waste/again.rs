//! A result is read from the cache again by every later request of its thread, until the next compaction, and the
//! later requests pay for it at a tenth. Haiku weighs 1, so a million input tokens is a unit.

use super::*;

/// A turn that wrote 10 tokens and read `cache_read` from the cache, at second `id` of the minute.
fn read_cache(store: &mut crate::Store, job: &str, id: &str, cache_read: u64, subagent: bool) {
    let at = format!("2026-10-17T10:00:{:02}.500Z", id.trim_start_matches('m').parse::<u32>().unwrap());
    keep(store, job, turn(id, &at, [10, 0, cache_read], subagent), &[("reads", 1.0)], &[("reads", 1.0)]);
}

/// A file read twice, the second read's 10,000 tokens carried by m3, whose 100,000 input they were a tenth of.
fn read_twice(store: &mut crate::Store, thread: &str, subagent: bool) {
    for (id, input) in [("m1", 10), ("m2", 100_000), ("m3", 100_000)] {
        wrote(store, "j1", id, input, subagent);
    }
    let in_thread = |mut one: Made| {
        one.call.thread = thread.into();
        one
    };
    keep_calls(
        store,
        "j1",
        vec![
            in_thread(made("t1", "m1", 1, "Read", Some("/f.rs"), 100, "m2")),
            in_thread(made("t2", "m2", 2, "Read", Some("/f.rs"), 40_000, "m3")),
        ],
    );
}

#[test]
fn a_repeat_read_is_paid_for_again_by_each_later_request_until_the_next_compaction() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    read_twice(&mut store, "", false);
    // m4 reads the 10,000 tokens at a tenth of a unit a million, out of a context under the 300k limit. m5 reads only 1,000 tokens in all, so it can have read
    // no more than that. The compaction empties the context, so m6 reads nothing of the result.
    read_cache(&mut store, "j1", "m4", 250_000, false);
    read_cache(&mut store, "j1", "m5", 1_000, false);
    compact(&mut store, "j1", "2026-10-17T10:00:05.700Z");
    read_cache(&mut store, "j1", "m6", 250_000, false);

    let read = waste(&store);
    let repeats = kind(&read, WasteKind::RepeatReads).expect("named");
    close(repeats.units, 0.1 * 10_000.0 / 100_000.0 + 10_000.0 * 0.1 / 1e6 + 1_000.0 * 0.1 / 1e6);
    assert_eq!((repeats.occurrences, repeats.jobs[0].occurrences), (1, 1), "the later requests are not more repeat reads");
    let named = causes(&repeats.jobs[0]);
    assert_eq!(named.iter().map(|one| one.0).collect::<Vec<_>>(), ["/f.rs"]);
    close(named[0].1, repeats.units);
    assert_eq!(read.kinds.iter().map(|one| one.kind.id()).collect::<Vec<_>>(), ["repeat_reads"], "nothing is in two kinds");
}

#[test]
fn a_big_result_is_paid_for_again_by_the_bytes_past_20_kb_only() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, input) in [("m1", 10), ("m2", 100_000)] {
        wrote(&mut store, "j1", id, input, false);
    }
    keep_calls(&mut store, "j1", vec![made("t1", "m1", 1, "Bash", None, 60_000, "m2")]);
    read_cache(&mut store, "j1", "m3", 1_000_000, false);

    // 60,000 bytes are 39,520 past the limit, 9,880 tokens.
    let read = waste(&store);
    let bigs = kind(&read, WasteKind::BigResults).expect("named");
    close(bigs.units, 0.1 * 9_880.0 / 100_000.0 + 9_880.0 * 0.1 / 1e6);
    assert_eq!(bigs.occurrences, 1);
    close(causes(&bigs.jobs[0])[0].1, bigs.units);
}

#[test]
fn a_subagents_result_is_read_again_by_its_own_later_requests_and_not_by_the_main_thread() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    read_twice(&mut store, "agent-x", true);
    // m4 is the subagent's, and made a call, which is how it is known to be in the thread. m5 is the main thread's.
    read_cache(&mut store, "j1", "m4", 1_000_000, true);
    read_cache(&mut store, "j1", "m5", 1_000_000, false);
    let mut call = made("t3", "m4", 3, "Bash", None, 10, "m9");
    call.call.thread = "agent-x".into();
    keep_calls(&mut store, "j1", vec![call]);

    let repeats = kind(&waste(&store), WasteKind::RepeatReads).expect("named").units;
    close(repeats, 0.1 * 10_000.0 / 100_000.0 + 10_000.0 * 0.1 / 1e6);
}
