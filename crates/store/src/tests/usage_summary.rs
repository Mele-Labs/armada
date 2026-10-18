//! What the kept turns add up to, summed in SQL: every split is a partition of the same total, the
//! window's end is exclusive, days and hour blocks are the caller's local time, the prior period is
//! the equal span before, and the Sessions past thirty are one entry.

use crate::session_turn_usage::KeptTurnUsage;
use crate::tests::{open, TempDir};

const NOW: &str = "2026-10-17T12:00:00.000Z";

/// Weighted tokens a unit holds: a million of output is five units.
#[allow(clippy::too_many_arguments)]
fn turn(session: &str, id: &str, at: &str, model: &str, tokens: [u64; 4], tools: &[&str], subagent: bool, bash: Option<&str>) -> (String, KeptTurnUsage) {
    (
        session.into(),
        KeptTurnUsage {
            message_id: id.into(),
            at: at.into(),
            model: model.into(),
            input_tokens: tokens[0],
            cache_creation_tokens: tokens[1],
            cache_read_tokens: tokens[2],
            output_tokens: tokens[3],
            tools: tools.iter().map(|one| one.to_string()).collect(),
            subagent,
            bash_use: bash.map(String::from),
        },
    )
}

fn seeded() -> Vec<(String, KeptTurnUsage)> {
    let mut rows = vec![
        // One unit is 200k output; local time is UTC+1, so a1 is Friday 16th 00:30 local.
        turn("s1", "a1", "2026-10-15T23:30:00.000Z", "model-a", [0, 0, 0, 200_000], &["Read"], false, None),
        turn("s1", "a2", "2026-10-16T10:00:00.000Z", "model-a", [0, 0, 0, 200_000], &["Bash"], false, Some("read_search")),
        turn("s1", "a3", "2026-10-16T10:05:00.000Z", "model-b", [100_000, 0, 0, 80_000], &[], false, None),
        turn("s1", "a4", "2026-10-16T12:00:00.000Z", "model-a", [0, 800_000, 0, 0], &["Bash"], true, Some("scripts")),
        turn("s1", "a5", "2026-10-16T13:00:00.000Z", "model-a", [0, 0, 250_000, 0], &["Grep"], false, None),
        turn("s2", "b1", "2026-10-15T08:00:00.000Z", "model-b", [0, 0, 10_000_000, 0], &["Edit"], false, None),
        // The prior period, [2026-10-03, 2026-10-10): two units, one from a subagent.
        turn("s3", "c1", "2026-10-05T08:00:00.000Z", "model-a", [0, 0, 0, 200_000], &[], false, None),
        turn("s3", "c2", "2026-10-05T09:00:00.000Z", "model-a", [0, 0, 0, 200_000], &[], true, None),
        // The end is exclusive, and long before the prior period is nobody's.
        turn("s4", "d1", "2026-10-17T00:00:00.000Z", "model-a", [0, 0, 0, 200_000], &[], false, None),
        turn("s4", "d2", "2026-09-01T00:00:00.000Z", "model-a", [0, 0, 0, 200_000], &[], false, None),
    ];
    for (n, label) in ["git", "build_test", "loops_waits", "forge", "other"].into_iter().enumerate() {
        rows.push(turn("s2", &format!("b-{n}"), "2026-10-15T08:30:00.000Z", "model-b", [0, 0, 0, 40_000], &["Bash"], false, Some(label)));
    }
    rows
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} is not {b}");
}

fn sum(rows: &[(String, f64)]) -> f64 {
    rows.iter().map(|(_, units)| units).sum()
}

fn of(rows: &[(String, f64)], name: &str) -> f64 {
    rows.iter().find(|(one, _)| one == name).map(|(_, units)| *units).unwrap_or(0.0)
}

#[test]
fn a_window_sums_into_partitions_of_one_total_in_the_callers_local_time() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (session, row) in seeded() {
        store.keep_turn_usage(&session, &row).unwrap();
    }
    let read = store.usage_summary(Some("2026-10-10T00:00:00Z"), Some("2026-10-17T00:00:00Z"), NOW, 60).unwrap().unwrap();
    let all = &read.window;

    close(all.units, 5.525);
    for partition in [&all.by_model, &all.by_first_tool] {
        close(sum(partition), all.units);
    }
    close(all.input_units + all.cache_creation_units + all.cache_read_units + all.output_units, all.units);
    close(all.by_day.iter().map(|day| day.units).sum::<f64>(), all.units);
    close(all.hour_grid.iter().flatten().sum::<f64>(), all.units);

    close(of(&all.by_model, "model-a"), 3.025);
    close(of(&all.by_model, "model-b"), 2.5);
    close(all.input_units, 0.1);
    close(all.cache_creation_units, 1.0);
    close(all.cache_read_units, 1.025);
    close(all.output_units, 3.4);
    close(of(&all.by_first_tool, "Bash"), 3.0);
    close(of(&all.by_first_tool, "none"), 0.5);
    close(of(&all.by_first_tool, "Read"), 1.0);

    // Only turns whose first call was a shell command carry a label; each of the seven is there.
    for (label, units) in [("read_search", 1.0), ("scripts", 1.0), ("git", 0.2), ("build_test", 0.2), ("loops_waits", 0.2), ("forge", 0.2), ("other", 0.2)] {
        close(of(&all.by_bash_use, label), units);
    }
    close(sum(&all.by_bash_use), of(&all.by_first_tool, "Bash"));

    close(all.subagent_units, 1.0);
    close(all.above_200k_units, 2.025);
    close(all.above_300k_units, 2.0);
    assert_eq!(all.peak_context, 10_000_000);

    let days: Vec<&str> = all.by_day.iter().map(|day| day.day.as_str()).collect();
    assert_eq!(days, ["2026-10-15", "2026-10-16"], "a1 is the 16th at UTC+1");
    close(all.by_day[0].units, 2.0);
    close(all.by_day[1].units, 3.525);
    close(of(&all.by_day[1].by_model, "model-b"), 0.5);
    // 2026-10-16 is a Friday: row four with Monday first.
    close(all.hour_grid[4][0], 1.0);
    close(all.hour_grid[4][3], 1.5);
    close(all.hour_grid[4][4], 1.025);
    close(all.hour_grid[3][3], 2.0);

    let ids: Vec<&str> = read.sessions.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["s1", "s2"]);
    assert!(read.rest.is_none());
    let (_, first) = &read.sessions[0];
    close(first.units, 3.525);
    assert_eq!(first.peak_context, 800_000);
    close(first.subagent_units, 1.0);
    close(read.sessions.iter().map(|(_, one)| one.units).sum::<f64>(), all.units);
    close(of(&first.by_bash_use, "scripts"), 1.0);

    close(read.prior.units, 2.0);
    close(read.prior.subagent_units, 1.0);
    close(read.prior.above_200k_units, 0.0);
    assert_eq!(read.prior.last_day.0, "2026-10-16");
    close(read.prior.last_day.1, 3.525);
    assert_eq!(read.prior.day_before.0, "2026-10-15");
    close(read.prior.day_before.1, 2.0);
}

#[test]
fn the_window_defaults_to_a_week_back_from_now_and_a_bound_that_is_no_time_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (session, row) in seeded() {
        store.keep_turn_usage(&session, &row).unwrap();
    }
    let read = store.usage_summary(None, None, NOW, 0).unwrap().unwrap();
    assert_eq!(read.since, "2026-10-10T12:00:00.000Z");
    assert_eq!(read.until, NOW);
    close(read.window.units, 5.525 + 1.0);
    assert!(store.usage_summary(Some("lately"), None, NOW, 0).unwrap().is_none());
    let none = store.usage_summary(Some("2030-01-01T00:00:00Z"), Some("2030-01-02T00:00:00Z"), NOW, 0).unwrap().unwrap();
    close(none.window.units, 0.0);
    assert_eq!(none.window.hour_grid.len(), 7);
}

#[test]
fn sessions_past_the_thirty_largest_are_one_entry_that_keeps_the_total() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for n in 1..=32u64 {
        let (session, row) = turn(&format!("x{n:02}"), "m", "2026-10-16T10:00:00.000Z", "model-a", [0, 0, 0, n * 10_000], &[], false, None);
        store.keep_turn_usage(&session, &row).unwrap();
    }
    let read = store.usage_summary(Some("2026-10-10T00:00:00Z"), Some("2026-10-17T00:00:00Z"), NOW, 0).unwrap().unwrap();
    assert_eq!(read.sessions.len(), 30);
    assert_eq!(read.sessions[0].0, "x32", "largest first");
    let (rolled, rest) = read.rest.clone().expect("two are past the list");
    assert_eq!(rolled, 2);
    close(rest.units, (1.0 + 2.0) * 10_000.0 * 5.0 / 1e6);
    close(read.sessions.iter().map(|(_, one)| one.units).sum::<f64>() + rest.units, read.window.units);
}
