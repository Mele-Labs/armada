//! The waste read, served: a kind carries Fleet's own words beside its figures, the window defaults to
//! the week, and a window that is none of the three is refused.

use api::Sessions;
use store::{KeptCallCarrier, KeptCallResult, KeptReading, KeptToolCall, KeptTurnSpent, KeptTurnUsage};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn the_waste_of_a_window_is_served_with_the_words_of_each_kind() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let at = fleet.now().as_str().to_string();
    {
        // One request that found the cache cold: 1M written at 1.25 on a model that weighs 1.
        let turn = KeptTurnUsage {
            message_id: "t1".into(),
            at,
            model: "haiku-4".into(),
            input_tokens: 0,
            cache_creation_tokens: 1_000_000,
            cache_creation_1h_tokens: 0,
            cache_read_tokens: 0,
            output_tokens: 0,
            tools: Vec::new(),
            subagent: false,
            bash_use: None,
        };
        let cold = vec![("cold_cache".to_string(), 1.0)];
        let shares = Some((vec![("own_words".to_string(), 1.0)], cold));
        let reading = KeptReading { spent: vec![KeptTurnSpent { message_id: "t1".into(), cache_creation_1h_tokens: 0, shares }], ..Default::default() };
        fleet.store().lock().await.keep_turns_read("s1", &[turn], &reading, "/s1.jsonl", 1).unwrap();
    }
    // The read ends at the clock, which has to be past the turn.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let read = fleet.get_usage_waste(None).await.unwrap();
    assert_eq!(read.window.id, "week", "left out, the window is the week");
    assert_eq!(read.until, read.window.until);
    assert!((read.total_units - 1.25).abs() < 1e-9 && (read.waste_units - 1.25).abs() < 1e-9, "{read:?}");
    let [cold] = &read.kinds[..] else { panic!("one kind: {:?}", read.kinds) };
    assert_eq!((cold.kind.as_str(), cold.title.as_str()), ("cold_cache", "Cold cache after a pause"));
    assert!(!cold.fix.is_empty() && !cold.fix.contains(" - "), "{}", cold.fix);
    assert_eq!((cold.occurrences, cold.jobs[0].session_id.as_str(), cold.jobs[0].peak_context), (1, "s1", None));

    let day = fleet.get_usage_waste(Some("day".into())).await.unwrap();
    assert_eq!((day.window.id.as_str(), day.kinds.len()), ("day", 1));
    assert!(fleet.get_usage_waste(Some("month".into())).await.is_err(), "a window that is none of the three is refused");
}

/// A file read again is served with its title, its fix and the file as the Job's cause, and a Job's kind with no
/// causes carries none on the wire.
#[tokio::test]
async fn a_repeat_read_is_served_with_its_words_and_the_file_it_read() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let at = fleet.now().as_str().to_string();
    {
        let turn = |id: &str| KeptTurnUsage {
            message_id: id.into(),
            at: at.clone(),
            model: "haiku-4".into(),
            input_tokens: 1_000_000,
            cache_creation_tokens: 0,
            cache_creation_1h_tokens: 0,
            cache_read_tokens: 0,
            output_tokens: 0,
            tools: Vec::new(),
            subagent: false,
            bash_use: None,
        };
        let call = |id: &str, message: &str, second: u32| KeptToolCall {
            tool_use_id: id.into(),
            message_id: message.into(),
            at: format!("2026-10-17T10:00:{second:02}.000Z"),
            thread: String::new(),
            epoch: 0,
            tool: "Read".into(),
            path: Some("/a/f.rs".into()),
            ..Default::default()
        };
        let reading = KeptReading {
            calls: vec![call("c1", "t1", 1), call("c2", "t2", 2)],
            call_results: vec![KeptCallResult { tool_use_id: "c1".into(), bytes: 4_000 }, KeptCallResult { tool_use_id: "c2".into(), bytes: 400_000 }],
            carriers: vec![
                KeptCallCarrier { tool_use_id: "c1".into(), message_id: "t2".into() },
                KeptCallCarrier { tool_use_id: "c2".into(), message_id: "t3".into() },
            ],
            ..Default::default()
        };
        fleet.store().lock().await.keep_turns_read("s1", &[turn("t1"), turn("t2"), turn("t3")], &reading, "/s1.jsonl", 1).unwrap();
    }
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let read = fleet.get_usage_waste(None).await.unwrap();
    let repeats = read.kinds.iter().find(|one| one.kind == "repeat_reads").expect("named");
    assert_eq!(repeats.title, "Same file read again");
    assert!(!repeats.fix.is_empty() && !repeats.fix.contains(" - "), "{}", repeats.fix);
    let causes = &repeats.jobs[0].causes;
    assert_eq!((causes.len(), causes[0].name.as_str()), (1, "/a/f.rs"));
    assert!((causes[0].units - repeats.units).abs() < 1e-9);
    assert!(read.kinds.iter().all(|one| one.kind != "big_results"), "a repeat over 20 KB is counted once, as a repeat");
    let context = read.kinds.iter().find(|one| one.kind == "main_context").expect("t3 holds 1M");
    assert!(context.jobs[0].causes.is_empty(), "a context kind names no cause");
}
