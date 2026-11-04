//! What the burn was spent on: every figure weighed by the model and the hour's writes as the summary
//! weighs it, a compaction's own request counted as background, each split adding up to the whole,
//! and a turn with no shares of its own shared as its Job's others are.

use crate::session_turn_usage::{KeptBackground, KeptReading, KeptTurnSpent, KeptTurnUsage};
use crate::tests::{open, TempDir};

const NOW: &str = "2026-10-17T12:00:00.000Z";

fn turn(id: &str, at: &str, model: &str, tokens: [u64; 4], subagent: bool) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: id.into(),
        at: at.into(),
        model: model.into(),
        input_tokens: tokens[0],
        cache_creation_tokens: tokens[1],
        cache_creation_1h_tokens: 0,
        cache_read_tokens: tokens[2],
        output_tokens: tokens[3],
        tools: vec!["Read".into()],
        subagent,
        bash_use: None,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} is not {b}");
}

fn on(split: &crate::SpentSplit, part: &str) -> f64 {
    split.on[crate::SPENT_ON.iter().position(|one| *one == part).unwrap()]
}

fn shares(pairs: &[(&str, f64)]) -> Vec<(String, f64)> {
    pairs.iter().map(|(part, share)| (part.to_string(), *share)).collect()
}

/// One Job's Opus turn with half its writes held an hour, its Haiku subagent's turn with no shares,
/// and a compaction on Sonnet; another Job's turn twelve days back.
fn seeded() -> (TempDir, crate::Store) {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let opus = turn("t1", "2026-10-17T10:00:00.000Z", "opus-5", [0, 1_000_000, 1_000_000, 100_000], false);
    let reading = KeptReading {
        spent: vec![KeptTurnSpent {
            message_id: "t1".into(),
            cache_creation_1h_tokens: 500_000,
            shares: Some((shares(&[("start_up", 0.5), ("reads", 0.5)]), shares(&[("own_words", 1.0)]))),
        }],
        background: vec![KeptBackground {
            id: "compaction-c1".into(),
            at: "2026-10-17T09:00:00.000Z".into(),
            model: "sonnet-4".into(),
            cache_read_tokens: 1_000_000,
            output_tokens: 200_000,
        }],
        ..Default::default()
    };
    store.keep_turns_read("j1", &[opus.clone()], &reading, "/j1.jsonl", 10).unwrap();
    // Heard again from the mod, and again in a read that did not see its request: it keeps both.
    store.keep_turn_usage("j1", &opus).unwrap();
    let again = KeptReading {
        spent: vec![KeptTurnSpent { message_id: "t1".into(), cache_creation_1h_tokens: 500_000, shares: None }],
        ..Default::default()
    };
    store.keep_turns_read("j1", &[opus], &again, "/j1.jsonl", 20).unwrap();
    store.keep_turn_usage("j1", &turn("t2", "2026-10-12T10:00:00.000Z", "haiku-4", [1_000_000, 0, 0, 0], true)).unwrap();
    store.keep_turn_usage("j2", &turn("t3", "2026-10-05T00:00:00.000Z", "other", [0, 0, 0, 200_000], false)).unwrap();
    (dir, store)
}

#[test]
fn burn_weighs_the_model_and_the_hours_writes_and_counts_a_compaction_as_a_turn() {
    let (_dir, store) = seeded();
    let read = store.usage_summary(Some("2026-10-16T12:00:00.000Z"), Some(NOW), NOW, 0).unwrap().unwrap();
    let all = &read.window;
    // Opus weighs 5: 0.5M writes at 1.25, 0.5M at 2, 1M read at 0.1 and 0.1M out at 5.
    close(all.units, 5.0 * 2.225 + 3.0 * 1.1);
    close(all.cache_creation_units, 5.0 * 0.625);
    close(all.cache_creation_1h_units, 5.0);
    close(all.input_units + all.cache_creation_units + all.cache_creation_1h_units + all.cache_read_units + all.output_units, all.units);
    let none = all.by_first_tool.iter().find(|(name, _)| name == "none").map(|(_, units)| *units);
    assert_eq!(none.map(|units| (units * 1e6).round()), Some(3.3e6), "the compaction called nothing");
    assert_eq!(store.transcript_carried("/j1.jsonl").unwrap(), None);
}

#[test]
fn each_split_adds_up_to_the_whole_and_the_subagents_are_the_summarys() {
    let (_dir, store) = seeded();
    let spent = store.usage_spent(None, NOW).unwrap().unwrap();
    let ids: Vec<&str> = spent.windows.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["day", "week", "fortnight"]);

    let day = &spent.windows[0];
    assert_eq!(day.since, "2026-10-16T12:00:00.000Z");
    close(on(&day.total, "start_up"), 0.25);
    close(on(&day.total, "reads"), 0.25);
    close(on(&day.total, "own_words"), 8.125);
    close(on(&day.total, "output"), 2.5);
    close(on(&day.total, "background"), 3.3);
    assert_eq!(day.total.by.map(|one| (one * 1e6).round() / 1e6), [11.125, 0.0, 3.3]);

    let week = &spent.windows[1];
    // The subagent's input had no shares, so it goes as the Job's own did: 8.625 of them.
    close(on(&week.total, "own_words"), 8.125 + 8.125 / 8.625);
    close(on(&week.total, "start_up"), 0.25 + 0.25 / 8.625);
    let summary = store.usage_summary(Some(&week.since), Some(&week.until), NOW, 0).unwrap().unwrap();
    close(week.total.by[1], summary.window.subagent_units);
    close(week.total.units(), summary.window.units);

    let fortnight = &spent.windows[2];
    let jobs: Vec<&str> = fortnight.jobs.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(jobs, ["j1", "j2"], "most first");
    for window in &spent.windows {
        close(window.total.by.iter().sum(), window.total.units());
        close(window.jobs.iter().map(|(_, one)| one.units()).sum(), window.total.units());
        for (_, job) in &window.jobs {
            close(job.by.iter().sum(), job.units());
        }
    }
    assert!(store.usage_spent(Some("lately"), NOW).unwrap().is_none());
}
