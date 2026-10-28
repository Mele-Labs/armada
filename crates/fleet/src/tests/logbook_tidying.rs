//! Tidy's rules with the model answered by a closure: near-duplicates fold into one entry that keeps
//! every Session's evidence, a locked or a person's entry is never offered, and an answer that is
//! wrong anywhere changes nothing. `docs/concepts/logbook.md`.

use store::{LogEntry, LogEvidence, LogHow, LogKind, LogRewrite, LogSection, LogState, Store};

use crate::logbook::tidying::{due, pass};
use crate::store_lock::StoreLock;
use crate::tests::tmp::TempDir;

const KEY: &str = "acme--shop";
const RUN: &str = "tests in apps/desktop: `pnpm exec vitest run src/rail.test.ts > /tmp/slot1-final.log`";
const RUN_AGAIN: &str = "tests in apps/desktop: `pnpm exec vitest run src/rail.test.ts --reporter=dot > /tmp/slot2-final.log`";
const MERGED: &str = "Run the desktop tests with `pnpm exec vitest run <file>` from apps/desktop.";

fn entry(text: &str, session: &str) -> LogEntry {
    LogEntry {
        id: None,
        section: LogSection::Repo,
        repository: KEY.to_string(),
        kind: LogKind::Command,
        title: None,
        text: text.to_string(),
        paths: Vec::new(),
        evidence: vec![LogEvidence { session_id: session.to_string(), cites: vec![format!("ran:{session}")], how: LogHow::First, hard: false, at: 10 }],
        first_seen: 10,
        last_confirmed: 10,
        last_served: None,
        state: LogState::Candidate,
        locked: false,
        edited_by_person: false,
        tripped: 0,
        history: Vec::new(),
    }
}

/// A store holding `entries`, and the ids they were given in order.
fn kept(dir: &TempDir, entries: &[LogEntry]) -> (StoreLock, Vec<i64>) {
    let mut store = Store::open(&dir.path().join("armada.db")).expect("opens");
    let ids = entries.iter().map(|one| store.save_logbook_entry(one).expect("keeps").expect("not forgotten")).collect();
    (StoreLock::new(store), ids)
}

async fn held(store: &StoreLock) -> Vec<LogEntry> {
    store.lock().await.logbook_of(KEY).expect("reads")
}

fn group(keep: i64, text: &str, absorb: &[i64]) -> String {
    format!(r#"{{"keep":{keep},"text":"{text}","absorb":{absorb:?}}}"#)
}

fn says(groups: &[String]) -> String {
    format!(r#"{{"groups":[{}]}}"#, groups.join(","))
}

#[tokio::test]
async fn two_near_identical_commands_merge_into_one_that_keeps_both_sessions_evidence() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry(RUN, "s1"), entry(RUN_AGAIN, "s2")]);
    let said = says(&[group(ids[0], MERGED, &[ids[1]])]);
    let merged = pass(&store, KEY, |_| async { Ok(said.clone()) }, 5_000).await.expect("passes");
    assert_eq!(merged, 1);
    let entries = held(&store).await;
    let (kept, gone) = (&entries[0], &entries[1]);
    assert_eq!((kept.text.as_str(), kept.state), (MERGED, LogState::Standing), "two Sessions stand it");
    assert_eq!(kept.history.len(), 1);
    assert_eq!((kept.history[0].text.as_str(), kept.history[0].why), (RUN, LogRewrite::Merged));
    let sessions: Vec<&str> = kept.evidence.iter().map(|one| one.session_id.as_str()).collect();
    assert_eq!(sessions, vec!["s1", "s2"]);
    assert_eq!((gone.state, gone.text.as_str()), (LogState::Faded, RUN_AGAIN), "archived, kept and searchable");
}

#[tokio::test]
async fn a_locked_or_a_persons_entry_is_never_offered_merged_or_rewritten() {
    let dir = TempDir::new();
    let locked = LogEntry { locked: true, ..entry("Run the rail tests with `pnpm exec vitest run src/rail.test.ts`.", "s3") };
    let theirs = LogEntry { edited_by_person: true, ..entry("Run rail tests with `pnpm exec vitest run src/rail.test.ts` please.", "s4") };
    let (store, ids) = kept(&dir, &[entry(RUN, "s1"), entry(RUN_AGAIN, "s2"), locked, theirs]);
    let before = held(&store).await;
    let questions = std::sync::Mutex::new(Vec::new());
    for said in [says(&[group(ids[0], MERGED, &[ids[2]])]), says(&[group(ids[3], MERGED, &[ids[1]])])] {
        let ask = |question: String| {
            questions.lock().expect("held").push(question);
            let said = said.clone();
            async move { Ok(said) }
        };
        assert!(pass(&store, KEY, ask, 5_000).await.is_err(), "it names an entry that was not offered");
    }
    assert_eq!(held(&store).await, before);
    let asked = questions.lock().expect("held").join("\n");
    assert!(asked.contains("slot1-final") && !asked.contains("rail.test.ts` please") && !asked.contains("Run the rail tests"));
}

#[tokio::test]
async fn a_bad_or_partial_answer_changes_nothing() {
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry(RUN, "s1"), entry(RUN_AGAIN, "s2"), entry("Chrome is not in /Applications.", "s3"), entry("Lease the slot before writing.", "s4")]);
    let before = held(&store).await;
    let bad = [
        "no json here".to_string(),
        says(&[group(ids[0], MERGED, &[999])]),
        // The first group alone is sound; the entry it shares with the second makes the answer wrong.
        says(&[group(ids[0], MERGED, &[ids[1]]), group(ids[2], "Chrome is missing.", &[ids[1]])]),
        says(&[group(ids[0], MERGED, &[ids[0]])]),
        says(&[group(ids[0], MERGED, &[ids[1]]), group(ids[2], "Chrome is missing.", &[])]),
    ];
    for said in bad {
        let result = pass(&store, KEY, |_| async { Ok(said.clone()) }, 5_000).await;
        assert!(result.is_err(), "{said}");
        assert_eq!(held(&store).await, before, "{said}");
    }
}

#[tokio::test]
async fn a_repository_is_tidied_at_most_once_a_week_and_only_where_entries_changed() {
    const DAY: u64 = 24 * 60 * 60 * 1000;
    let dir = TempDir::new();
    let (store, ids) = kept(&dir, &[entry(RUN, "s1"), entry(RUN_AGAIN, "s2")]);
    assert!(due(&store, KEY, 20).await.expect("reads"), "never tidied, and two entries to fold");
    pass(&store, KEY, |_| async { Ok(says(&[])) }, 1_000).await.expect("passes");
    assert!(!due(&store, KEY, 8 * DAY).await.expect("reads"), "nothing changed since");

    let mut again = store.lock().await.logbook_entry(ids[0]).expect("reads").expect("held");
    again.last_confirmed = 2 * DAY;
    store.lock().await.save_logbook_entry(&again).expect("keeps");
    assert!(!due(&store, KEY, 3 * DAY).await.expect("reads"), "changed, but tidied a day ago");
    assert!(due(&store, KEY, 8 * DAY).await.expect("reads"), "changed, and a week has passed");
}

#[tokio::test]
async fn the_entry_a_merge_keeps_keeps_its_title() {
    let dir = TempDir::new();
    let titled = LogEntry { title: Some("Run desktop tests".into()), ..entry(RUN, "s1") };
    let (store, ids) = kept(&dir, &[titled, entry(RUN_AGAIN, "s2")]);
    let said = says(&[group(ids[0], MERGED, &[ids[1]])]);
    pass(&store, KEY, |_| async { Ok(said.clone()) }, 5_000).await.expect("passes");
    let entries = held(&store).await;
    assert_eq!((entries[0].text.as_str(), entries[0].title.as_deref()), (MERGED, Some("Run desktop tests")));
}
