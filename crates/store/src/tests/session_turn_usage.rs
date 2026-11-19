//! Usage kept a turn at a time: a turn is one row per session and message id, reporting it twice
//! changes nothing, and a read by session and time window returns the model, the four token fields
//! and the names of the tools the turn called.

use crate::session_turn_usage::{KeptCallCarrier, KeptCallResult, KeptOpenCall, KeptReading, KeptToolCall, KeptToolResult, KeptTurnSearches, KeptTurnTimes, KeptTurnUsage};
use crate::tests::{open, TempDir};

fn at(minute: u32) -> String {
    format!("2026-10-17T09:{minute:02}:00.000Z")
}

fn turn(message_id: &str, minute: u32, tools: &[&str]) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: message_id.into(),
        at: at(minute),
        model: "model-a".into(),
        input_tokens: 6,
        cache_creation_tokens: 1000,
        cache_creation_1h_tokens: 0,
        cache_read_tokens: 10_000,
        output_tokens: 90,
        tools: tools.iter().map(|one| one.to_string()).collect(),
        subagent: false,
        bash_use: None,
    }
}

#[test]
fn a_turn_is_kept_once_and_read_back_whole_across_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_turn_usage("s1", &turn("m1", 1, &["Bash", "Read"])).unwrap());
    assert!(!store.keep_turn_usage("s1", &turn("m1", 1, &["Bash", "Read"])).unwrap(), "the same turn again");
    assert!(store.keep_turn_usage("s2", &turn("m1", 1, &[])).unwrap(), "the same message id in another session");
    drop(store);

    let store = open(&dir);
    let read = store.turn_usage("s1", None, None).unwrap();
    assert_eq!(read, [turn("m1", 1, &["Bash", "Read"])]);
    assert_eq!(store.turn_usage("s2", None, None).unwrap(), [turn("m1", 1, &[])]);
    assert!(store.turn_usage("s3", None, None).unwrap().is_empty());
}

#[test]
fn a_read_takes_the_window_from_its_start_to_before_its_end_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (id, minute) in [("m3", 3), ("m1", 1), ("m2", 2), ("m4", 4)] {
        store.keep_turn_usage("s1", &turn(id, minute, &["Edit"])).unwrap();
    }
    let ids = |since: Option<&str>, until: Option<&str>| -> Vec<String> {
        store.turn_usage("s1", since, until).unwrap().into_iter().map(|one| one.message_id).collect()
    };
    assert_eq!(ids(None, None), ["m1", "m2", "m3", "m4"]);
    assert_eq!(ids(Some(&at(2)), Some(&at(4))), ["m2", "m3"]);
    assert_eq!(ids(Some(&at(3)), None), ["m3", "m4"]);
    assert_eq!(ids(None, Some(&at(2))), ["m1"]);
}

#[test]
fn a_turn_heard_again_with_more_output_replaces_the_partial_one_and_never_the_reverse() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let partial = KeptTurnUsage { output_tokens: 3, ..turn("m1", 1, &[]) };
    let whole = KeptTurnUsage { output_tokens: 90, ..turn("m1", 1, &["Bash"]) };
    assert!(store.keep_turn_usage("s1", &partial).unwrap());
    assert!(store.keep_turn_usage("s1", &whole).unwrap());
    assert!(!store.keep_turn_usage("s1", &partial).unwrap());
    assert_eq!(store.turn_usage("s1", None, None).unwrap(), [whole]);
}

#[test]
fn the_lines_of_one_message_heard_apart_join_their_calls_and_the_first_call_keeps_the_label() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = KeptTurnUsage { bash_use: Some("read_search".into()), ..turn("m1", 1, &["Bash"]) };
    let second = KeptTurnUsage { subagent: true, ..turn("m1", 1, &["Read"]) };
    assert!(store.keep_turn_usage("s1", &first).unwrap());
    assert!(store.keep_turn_usage("s1", &second).unwrap());
    assert!(!store.keep_turn_usage("s1", &first).unwrap(), "the first heard again is no change");
    let held = store.turn_usage("s1", None, None).unwrap();
    assert_eq!(held[0].tools, ["Bash", "Read"]);
    assert_eq!(held[0].bash_use.as_deref(), Some("read_search"));
    assert!(held[0].subagent);
}

#[test]
fn a_transcript_offset_is_kept_with_its_turns_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.transcript_offset("/a.jsonl").unwrap(), None);
    assert_eq!(store.keep_turns_read("s1", &[turn("m1", 1, &[])], &Default::default(), "/a.jsonl", 120).unwrap(), 1);
    assert_eq!(store.keep_turns_read("s1", &[turn("m1", 1, &[])], &Default::default(), "/a.jsonl", 300).unwrap(), 0);
    drop(store);
    assert_eq!(open(&dir).transcript_offset("/a.jsonl").unwrap(), Some(300));
}

fn reading(times: KeptTurnTimes, open: &[(&str, &str)]) -> KeptReading {
    KeptReading {
        times: vec![times],
        open: open
            .iter()
            .map(|(call, message)| KeptOpenCall { tool_use_id: (*call).into(), message_id: (*message).into() })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn a_result_read_after_its_call_ends_the_turn_s_tools_and_a_repeat_keeps_the_times() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = KeptTurnTimes {
        message_id: "m1".into(),
        model_began: Some(at(1)),
        tools_began: Some(at(2)),
        tools_ended: None,
    };
    store.keep_turns_read("s1", &[turn("m1", 2, &["Bash"])], &reading(first, &[("t1", "m1")]), "/a.jsonl", 10).unwrap();
    let late = KeptReading { late: vec![KeptToolResult { tool_use_id: "t1".into(), at: at(5) }], ..Default::default() };
    store.keep_turns_read("s1", &[], &late, "/a.jsonl", 20).unwrap();
    // The message heard again with its own calls and no result: the upsert must not drop the end.
    let again = KeptTurnTimes { message_id: "m1".into(), model_began: Some(at(1)), tools_began: Some(at(2)), tools_ended: None };
    store.keep_turns_read("s1", &[turn("m1", 2, &["Bash"])], &reading(again, &[]), "/a.jsonl", 30).unwrap();
    let timed = store.turns_timed("s1").unwrap();
    assert_eq!((timed[0].tools_began.as_deref(), timed[0].tools_ended.as_deref()), (Some(at(2).as_str()), Some(at(5).as_str())));
    assert_eq!(timed[0].model_began.as_deref(), Some(at(1).as_str()));
}

/// A Job's figure is what its turns up to and including its first edit counted, in the week of that edit, and a turn
/// read before searches were counted leaves the Job unmeasured rather than at zero.
#[test]
fn the_searches_before_a_first_edit_are_summed_by_week_and_a_turn_without_a_figure_is_unmeasured() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let searched = |counts: &[(&str, u32)]| KeptReading {
        searched: counts.iter().map(|(id, searches)| KeptTurnSearches { message_id: id.to_string(), searches: *searches }).collect(),
        ..Default::default()
    };
    let turns = [turn("m1", 1, &["Grep"]), turn("m2", 2, &["Edit"]), turn("m3", 3, &["Grep"])];
    store.keep_turns_read("s1", &turns, &searched(&[("m1", 2), ("m2", 1), ("m3", 5)]), "/s1.jsonl", 10).unwrap();
    store.keep_turn_usage("s2", &turn("m1", 1, &["Grep"])).unwrap();
    store.keep_turn_usage("s2", &turn("m2", 2, &["Write"])).unwrap();
    store.keep_turn_usage("s3", &turn("m1", 1, &["Read"])).unwrap();
    let mut early = turn("m1", 1, &["Edit"]);
    early.at = "2026-10-01T09:00:00.000Z".into();
    store.keep_turns_read("s4", &[early], &searched(&[("m1", 7)]), "/s4.jsonl", 10).unwrap();

    let now = "2026-10-18T00:00:00.000Z";
    let this_week = store.first_edits(now, 1).unwrap();
    let figures: Vec<(&str, &str, u32, bool)> =
        this_week.iter().map(|one| (one.session_id.as_str(), one.week.as_str(), one.searches, one.measured)).collect();
    assert_eq!(figures, [("s1", "2026-10-12", 3, true), ("s2", "2026-10-12", 0, false)], "s3 never edited; s4 edited in an earlier week");
    let four = store.first_edits(now, 4).unwrap();
    assert_eq!(four.iter().map(|one| one.session_id.as_str()).collect::<Vec<_>>(), ["s4", "s1", "s2"]);
    assert_eq!((four[0].week.as_str(), four[0].searches), ("2026-09-28", 7));
}

fn call(id: &str, message: &str, tool: &str, path: Option<&str>) -> KeptToolCall {
    KeptToolCall {
        tool_use_id: id.into(),
        message_id: message.into(),
        at: at(1),
        thread: String::new(),
        epoch: 0,
        tool: tool.into(),
        path: path.map(str::to_string),
        ..Default::default()
    }
}

type CallRow = (String, String, String, Option<String>, Option<i64>, Option<String>);

fn call_rows(store: &crate::Store, session: &str) -> Vec<CallRow> {
    let mut statement = store
        .conn
        .prepare("SELECT tool_use_id, message_id, tool, path, result_bytes, carried_by FROM session_tool_calls WHERE session_id = ?1 ORDER BY tool_use_id")
        .unwrap();
    let rows = statement.query_map([session], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)));
    rows.unwrap().map(Result::unwrap).collect()
}

/// A call is its tool, a Read's path and its result's size, and the table has no column that could hold text.
#[test]
fn a_call_keeps_its_tool_a_reads_path_and_its_results_size_and_no_text() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let columns: Vec<String> = store
        .conn
        .prepare("SELECT name FROM pragma_table_info('session_tool_calls') ORDER BY cid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        columns,
        [
            "session_id", "tool_use_id", "message_id", "at", "thread", "epoch", "tool", "path", "from_line", "to_line", "edited",
            "result_bytes", "carried_by"
        ],
        "a session, a call, a tool, a path, its lines, a file edited, a size and who carried it"
    );

    let calls = KeptReading {
        calls: vec![call("t1", "m1", "Read", Some("/a/f.rs")), call("t2", "m1", "Bash", None)],
        call_results: vec![KeptCallResult { tool_use_id: "t1".into(), bytes: 30_000 }],
        carriers: vec![KeptCallCarrier { tool_use_id: "t1".into(), message_id: "m2".into() }],
        ..Default::default()
    };
    store.keep_turns_read("s1", &[], &calls, "/s1.jsonl", 10).unwrap();
    let expected = |bytes: Option<i64>, carried: Option<&str>| -> Vec<CallRow> {
        vec![
            ("t1".into(), "m1".into(), "Read".into(), Some("/a/f.rs".into()), bytes, carried.map(str::to_string)),
            ("t2".into(), "m1".into(), "Bash".into(), None, None, None),
        ]
    };
    assert_eq!(call_rows(&store, "s1"), expected(Some(30_000), Some("m2")));

    // Read again from the top, the calls come with no result in sight: the size and the carrier stay.
    let again = KeptReading { calls: calls.calls.clone(), ..Default::default() };
    store.keep_turns_read("s1", &[], &again, "/s1.jsonl", 20).unwrap();
    assert_eq!(call_rows(&store, "s1"), expected(Some(30_000), Some("m2")), "a call heard again keeps its size and carrier");
    assert!(call_rows(&store, "s2").is_empty());
}

#[test]
fn a_transcript_read_before_calls_were_kept_is_marked_so_it_is_read_again_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.conn.execute("INSERT INTO transcript_offsets (path, read_to, carried, searched) VALUES ('/old.jsonl', 5, '{}', 1)", []).unwrap();
    assert!(!store.transcript_tooled("/old.jsonl").unwrap());
    assert!(!store.transcript_tooled("/never.jsonl").unwrap());
    store.keep_turns_read("s1", &[], &Default::default(), "/old.jsonl", 9).unwrap();
    assert!(store.transcript_tooled("/old.jsonl").unwrap());
}
