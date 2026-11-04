//! A waiting Job's compaction is a row with the context before it, and is filled with the Job's
//! first main-thread turn after it once that turn is kept.

use crate::session_turn_usage::KeptTurnUsage;
use crate::tests::{open, TempDir};
use crate::waiting_compactions::NextRequest;

fn turn(id: &str, minute: u32, subagent: bool, tokens: (u64, u64, u64)) -> KeptTurnUsage {
    KeptTurnUsage {
        message_id: id.into(),
        at: format!("2026-11-04T09:{minute:02}:00.000Z"),
        model: "model-a".into(),
        input_tokens: tokens.0,
        cache_creation_tokens: tokens.1,
        cache_read_tokens: tokens.2,
        output_tokens: 1,
        tools: Vec::new(),
        subagent,
        bash_use: None,
    }
}

#[test]
fn a_compaction_keeps_the_context_before_and_is_filled_by_the_next_main_turn() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.last_main_context("s1").unwrap(), None);
    store.keep_turn_usage("s1", &turn("a", 0, false, (10, 2_000, 180_000))).unwrap();
    store.keep_turn_usage("s1", &turn("b", 1, true, (5, 900_000, 0))).unwrap();
    assert_eq!(
        store.last_main_context("s1").unwrap(),
        Some(("2026-11-04T09:00:00.000Z".into(), 182_010)),
        "a subagent's turn is not the Job's context"
    );

    store.keep_waiting_compaction("s1", "2026-11-04T09:50:00Z", 182_010).unwrap();
    store.waiting_compactions_measured("s1").unwrap();
    let kept = store.waiting_compactions("s1").unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].at.as_str(), kept[0].context_before, &kept[0].next), ("2026-11-04T09:50:00.000Z", 182_010, &None));

    store.keep_turn_usage("s1", &turn("c", 55, true, (1, 1, 1))).unwrap();
    store.keep_turn_usage("s1", &turn("d", 58, false, (3, 21_000, 12_000))).unwrap();
    store.keep_turn_usage("s1", &turn("e", 59, false, (3, 400, 33_000))).unwrap();
    store.waiting_compactions_measured("s1").unwrap();
    let next = store.waiting_compactions("s1").unwrap()[0].next.clone();
    assert_eq!(
        next,
        Some(NextRequest { at: "2026-11-04T09:58:00.000Z".into(), input: 3, cache_write: 21_000, cache_read: 12_000 })
    );
}
