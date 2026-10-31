//! Sort and Organize with the model answered by a closure: a sorter's answer is accepted or changes
//! nothing, stale finds exactly the old unlocked entries, merge proposals change nothing until
//! applied, and apply works. `docs/concepts/logbook.md`.

use ipc::OrganizeAction;
use store::{LogEntry, LogEvidence, LogHow, LogKind, LogSection, LogState, LogTopic, Store};

use crate::logbook::organizing::{apply, bugs, merges, stale};
use crate::logbook::sorting::pass as sort;
use crate::store_lock::StoreLock;
use crate::tests::tmp::TempDir;

const KEY: &str = "acme--shop";
const DAY: u64 = 24 * 60 * 60 * 1000;
const NOW: u64 = 100 * DAY;
const RUN: &str = "tests in apps/desktop: `pnpm exec vitest run src/rail.test.ts > /tmp/slot1-final.log`";
const RUN_AGAIN: &str = "tests in apps/desktop: `pnpm exec vitest run src/rail.test.ts --reporter=dot > /tmp/slot2-final.log`";
const MERGED: &str = "Run the desktop tests with `pnpm exec vitest run <file>` from apps/desktop.";

fn entry(text: &str, session: &str, confirmed: u64) -> LogEntry {
    LogEntry {
        id: None,
        section: LogSection::Repo,
        repository: KEY.to_string(),
        kind: LogKind::Command,
        title: None,
        topic: None,
        text: text.to_string(),
        paths: Vec::new(),
        evidence: vec![LogEvidence { session_id: session.to_string(), cites: vec![format!("ran:{session}")], how: LogHow::First, hard: false, at: confirmed }],
        first_seen: confirmed,
        last_confirmed: confirmed,
        last_served: None,
        state: LogState::Standing,
        locked: false,
        edited_by_person: false,
        tripped: 0,
        history: Vec::new(),
    }
}

fn kept(dir: &TempDir, entries: &[LogEntry]) -> (StoreLock, Vec<i64>) {
    let mut store = Store::open(&dir.path().join("armada.db")).expect("opens");
    let ids = entries.iter().map(|one| store.save_logbook_entry(one).expect("keeps").expect("not forgotten")).collect();
    (StoreLock::new(store), ids)
}

async fn held(store: &StoreLock) -> Vec<LogEntry> {
    store.lock().await.logbook_of(KEY).expect("reads")
}

fn topics(held: &[LogEntry]) -> Vec<Option<LogTopic>> {
    held.iter().map(|one| one.topic).collect()
}

#[tokio::test]
async fn the_sorter_places_every_untopiced_entry_including_a_locked_one_and_leaves_other_unplaced() {
    let dir = TempDir::new();
    let locked = LogEntry { locked: true, edited_by_person: true, ..entry("Open pull requests as drafts.", "s2", NOW) };
    let (store, ids) = kept(&dir, &[entry(RUN, "s1", NOW), locked, entry("Chrome is in /Applications.", "s3", NOW)]);
    let said = format!(r#"{{"topics":[{{"id":{},"topic":"testing"}},{{"id":{},"topic":"git"}},{{"id":{},"topic":"other"}}]}}"#, ids[0], ids[1], ids[2]);
    let placed = sort(&store, KEY, true, |_| async { Ok(said.clone()) }).await.expect("sorts");
    assert_eq!(placed, 2);
    assert_eq!(topics(&held(&store).await), vec![Some(LogTopic::Testing), Some(LogTopic::Git), None]);
    let again = sort(&store, KEY, true, |_| async { Ok(r#"{"topics":[{"id":3,"topic":"mac"}]}"#.to_string()) }).await.expect("sorts");
    assert_eq!(again, 1, "only the entry still without a topic is offered, and the others keep theirs");
    assert_eq!(held(&store).await[0].topic, Some(LogTopic::Testing));
}

#[tokio::test]
async fn a_sorter_answer_wrong_anywhere_changes_nothing() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry(RUN, "s1", NOW), entry("Chrome is in /Applications.", "s2", NOW)]);
    let bad = [
        "no json here".to_string(),
        format!(r#"{{"topics":[{{"id":{},"topic":"git"}},{{"id":999,"topic":"git"}}]}}"#, ids[0]),
        format!(r#"{{"topics":[{{"id":{},"topic":"git"}},{{"id":{},"topic":"weather"}}]}}"#, ids[0], ids[1]),
        format!(r#"{{"topics":[{{"id":{0},"topic":"git"}},{{"id":{0},"topic":"mac"}}]}}"#, ids[0]),
    ];
    for said in bad {
        assert!(sort(&store, KEY, true, |_| async { Ok(said.clone()) }).await.is_err(), "{said}");
        assert_eq!(topics(&held(&store).await), vec![None, None], "{said}");
    }
}

#[tokio::test]
async fn a_topic_survives_a_save_and_a_read() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[LogEntry { topic: Some(LogTopic::Slots), ..entry(RUN, "s1", NOW) }]);
    let again = store.lock().await.logbook_entry(ids[0]).expect("reads").expect("held");
    assert_eq!(again.topic, Some(LogTopic::Slots));
}

#[tokio::test]
async fn stale_finds_exactly_the_old_unlocked_standing_entries_nobody_tripped_lately() {
    let old = NOW - 46 * DAY;
    let tripped = LogEntry {
        evidence: vec![LogEvidence { session_id: "s9".into(), cites: Vec::new(), how: LogHow::Tripped, hard: false, at: NOW - 3 * DAY }],
        ..entry("Never push from a Drone.", "s9", old)
    };
    let entries: Vec<LogEntry> = [
        entry("Old standing.", "s1", old),
        entry("Recent standing.", "s2", NOW - 44 * DAY),
        LogEntry { locked: true, ..entry("Old but locked.", "s3", old) },
        LogEntry { state: LogState::Faded, ..entry("Old but archived.", "s4", old) },
        LogEntry { state: LogState::Candidate, ..entry("Old but not yet standing.", "s5", old) },
        tripped,
    ]
    .into_iter()
    .enumerate()
    .map(|(at, one)| LogEntry { id: Some(at as i64 + 1), ..one })
    .collect();
    let found = stale(&entries, NOW);
    assert_eq!(found.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), vec!["Old standing."]);
    assert_eq!(found[0].note, "Not confirmed in 46 days");
    assert_eq!((found[0].id.as_str(), found[0].entry_ids.clone()), ("1", vec!["1".to_string()]));
}

#[tokio::test]
async fn merge_proposals_change_nothing_until_they_are_applied() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry(RUN, "s1", NOW), entry(RUN_AGAIN, "s2", NOW)]);
    let before = held(&store).await;
    let said = format!(r#"{{"groups":[{{"keep":{},"text":"{MERGED}","absorb":[{}]}}]}}"#, ids[0], ids[1]);
    let proposals = merges(&before, |_| async { Ok(said.clone()) }).await.expect("proposes");
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].entry_ids, vec![ids[0].to_string(), ids[1].to_string()]);
    assert_eq!((proposals[0].text.as_str(), proposals[0].note.as_str()), (MERGED, ""));
    assert_eq!(held(&store).await, before, "a proposal is not a change");

    let (done, keys) = apply(&store, OrganizeAction::Merge, proposals, NOW).await.expect("applies");
    assert_eq!(done.applied, 1);
    assert_eq!(keys.into_iter().collect::<Vec<_>>(), vec![KEY.to_string()]);
    let entries = held(&store).await;
    assert_eq!((entries[0].text.as_str(), entries[1].state), (MERGED, LogState::Faded));
    assert_eq!(entries[0].evidence.len(), 2);
}

#[tokio::test]
async fn a_merge_over_an_entry_locked_since_is_skipped_and_a_wrong_merge_answer_proposes_nothing() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry(RUN, "s1", NOW), entry(RUN_AGAIN, "s2", NOW)]);
    let entries = held(&store).await;
    let wrong = format!(r#"{{"groups":[{{"keep":{},"text":"{MERGED}","absorb":[999]}}]}}"#, ids[0]);
    assert!(merges(&entries, |_| async { Ok(wrong.clone()) }).await.is_err());

    let said = format!(r#"{{"groups":[{{"keep":{},"text":"{MERGED}","absorb":[{}]}}]}}"#, ids[0], ids[1]);
    let proposals = merges(&entries, |_| async { Ok(said.clone()) }).await.expect("proposes");
    store.lock().await.lock_logbook_entry(ids[1], true).expect("locks");
    let (done, _) = apply(&store, OrganizeAction::Merge, proposals, NOW).await.expect("applies");
    assert_eq!(done.applied, 0);
    let now = held(&store).await;
    assert_eq!((now[0].text.as_str(), now[1].state), (RUN, LogState::Standing));
}

#[tokio::test]
async fn applying_stale_archives_the_named_entries_and_skips_one_confirmed_since() {
    let dir = TempDir::new();
    let old = NOW - 60 * DAY;
    let (store, ids) = kept(&dir, &[entry("Old one.", "s1", old), entry("Old two.", "s2", old)]);
    let proposals = stale(&held(&store).await, NOW);
    assert_eq!(proposals.len(), 2);
    let mut again = store.lock().await.logbook_entry(ids[1]).expect("reads").expect("held");
    again.last_confirmed = NOW - DAY;
    store.lock().await.save_logbook_entry(&again).expect("keeps");
    let (done, _) = apply(&store, OrganizeAction::Stale, proposals, NOW).await.expect("applies");
    assert_eq!(done.applied, 1);
    let states: Vec<LogState> = held(&store).await.iter().map(|one| one.state).collect();
    assert_eq!(states, vec![LogState::Faded, LogState::Standing]);
}

#[tokio::test]
async fn bugs_come_back_with_a_fault_each_and_reporting_writes_nothing() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry("Run `armada check test` by full name: a short name ran nothing.", "s1", NOW), entry(RUN, "s2", NOW)]);
    let before = held(&store).await;
    let said = format!(r#"{{"faults":[{{"id":{},"fault":"The test runner matches nothing on a short name"}}]}}"#, ids[0]);
    let found = bugs(&before, |_| async { Ok(said.clone()) }).await.expect("finds");
    assert_eq!((found.len(), found[0].note.as_str()), (1, "The test runner matches nothing on a short name"));
    let wrong = format!(r#"{{"faults":[{{"id":{},"fault":"one"}},{{"id":999,"fault":"two"}}]}}"#, ids[0]);
    assert!(bugs(&before, |_| async { Ok(wrong.clone()) }).await.is_err());
    let (done, _) = apply(&store, OrganizeAction::Bugs, found.clone(), NOW).await.expect("reports");
    assert_eq!((done.applied, done.reported), (0, found));
    assert_eq!(held(&store).await, before);
}
