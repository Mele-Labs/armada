//! A Session's rail and its judges, kept and read back: what the rail did survives a reopen, a concern
//! is numbered after the last and settles once, and only settled concerns are trimmed.
//! `docs/concepts/session.md`, *The rail*, *The judges*.

use crate::tests::{open, TempDir};
use crate::{KeptCite, KeptConcern, KeptRail, KeptReading, KeptStarted, SETTLED_KEPT};

fn concern(text: &str, step: Option<(u32, &str)>) -> KeptConcern {
    KeptConcern {
        n: 0,
        text: text.to_string(),
        since: "2026-10-16T09:00:00.000Z".to_string(),
        from_step: step.map(|(at, label)| (at, label.to_string())),
        cites: vec![
            KeptCite { region: "diff".into(), path: Some("crates/store/src/lib.rs".into()), from_line: 12, to_line: 14 },
            KeptCite { region: "plan".into(), path: None, from_line: 3, to_line: 3 },
        ],
        expected: Some("A diff touching dispatch-panel".into()),
        produced: Some("No file has changed yet".into()),
        consequence: None,
        settled: None,
    }
}

#[test]
fn what_the_rail_did_survives_a_reopen_and_a_session_with_none_reads_as_default() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_rail("s1").unwrap(), KeptRail::default());
    let rail = KeptRail { step: Some(2), nudged: vec![0, 1], judged: vec![1], stalled: vec![2], visits: Vec::new() };
    store.keep_session_rail("s1", &rail).unwrap();
    store.keep_session_rail("s2", &KeptRail { step: None, ..KeptRail::default() }).unwrap();
    drop(store);
    let store = open(&dir);
    assert_eq!(store.session_rail("s1").unwrap(), rail);
    assert_eq!(store.session_rail("s2").unwrap(), KeptRail::default());
}

#[test]
fn the_steps_a_session_began_are_kept_in_order_and_a_step_it_went_back_to_appears_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let visits = vec![(0, "2026-10-27T10:00:00Z".to_string()), (1, "2026-10-27T10:05:00Z".to_string()), (0, "2026-10-27T10:09:00Z".to_string())];
    store.keep_session_rail("s1", &KeptRail { step: Some(0), visits: visits.clone(), ..KeptRail::default() }).unwrap();
    drop(store);
    assert_eq!(open(&dir).session_rail("s1").unwrap().visits, visits);
}

#[test]
fn how_a_session_started_is_kept_once_and_a_later_start_does_not_rewrite_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_started("s1").unwrap(), None);
    let first = KeptStarted { workflow: Some("Bug".into()), delivery: Some("open_pr".into()), model: Some("opus".into()), effort: None, mode: "auto".into() };
    store.keep_session_started("s1", &first).unwrap();
    store.keep_session_started("s1", &KeptStarted { model: Some("sonnet".into()), mode: "plan".into(), ..first.clone() }).unwrap();
    drop(store);
    assert_eq!(open(&dir).session_started("s1").unwrap(), Some(first));
}

#[test]
fn a_concern_is_numbered_after_the_last_keeps_its_cites_and_settles_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.keep_session_concern("s1", &concern("one", None)).unwrap();
    let second = store.keep_session_concern("s1", &concern("two", Some((1, "Implement")))).unwrap();
    let other = store.keep_session_concern("s2", &concern("elsewhere", None)).unwrap();
    assert_eq!((first, second, other), (1, 2, 1));

    let held = store.session_concerns("s1").unwrap();
    assert_eq!(held.len(), 2);
    assert_eq!(held[1].from_step, Some((1, "Implement".to_string())));
    assert_eq!(held[1].cites, concern("", None).cites, "the path and the lines come back as they went in");
    assert_eq!(held[0].settled, None);
    assert_eq!(
        (held[1].expected.as_deref(), held[1].produced.as_deref(), held[1].consequence.as_deref()),
        (Some("A diff touching dispatch-panel"), Some("No file has changed yet"), None),
        "the parts come back as they went in, and a part left out stays absent"
    );

    assert!(store.settle_session_concern("s1", 1, "sent", "2026-10-16T09:05:00.000Z").unwrap());
    assert!(!store.settle_session_concern("s1", 1, "dismissed", "2026-10-16T09:06:00.000Z").unwrap(), "settled once");
    assert_eq!(
        store.session_concerns("s1").unwrap()[0].settled,
        Some(("sent".to_string(), "2026-10-16T09:05:00.000Z".to_string()))
    );
    assert_eq!(store.session_concerns("s2").unwrap()[0].settled, None, "another Session's is alone");
}

#[test]
fn only_settled_concerns_are_trimmed_and_a_waiting_one_is_never_lost() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_session_concern("s1", &concern("waits for good", None)).unwrap();
    for n in 0..(SETTLED_KEPT + 5) {
        let at = store.keep_session_concern("s1", &concern(&format!("c{n}"), None)).unwrap();
        store.settle_session_concern("s1", at, "dismissed", "2026-10-16T09:05:00.000Z").unwrap();
    }
    // One more, so the trim runs after the last settle.
    store.keep_session_concern("s1", &concern("last", None)).unwrap();
    let held = store.session_concerns("s1").unwrap();
    assert!(held.iter().any(|one| one.text == "waits for good"), "a waiting concern is never trimmed");
    assert!(held.iter().filter(|one| one.settled.is_some()).count() <= SETTLED_KEPT);
    assert!(!held.iter().any(|one| one.text == "c0"), "the oldest settled went");
}

#[test]
fn the_last_reading_is_replaced_and_zero_is_a_reading() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_judge_reading("s1").unwrap(), None);
    store.keep_session_judge_reading("s1", &KeptReading { at: "2026-10-16T09:00:00.000Z".into(), concerns: 2 }).unwrap();
    store.keep_session_judge_reading("s1", &KeptReading { at: "2026-10-16T09:10:00.000Z".into(), concerns: 0 }).unwrap();
    assert_eq!(
        store.session_judge_reading("s1").unwrap(),
        Some(KeptReading { at: "2026-10-16T09:10:00.000Z".into(), concerns: 0 })
    );
}
