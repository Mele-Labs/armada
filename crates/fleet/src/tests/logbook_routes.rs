//! The Logbook over its routes: every state is read and can be filtered, an edit keeps what the
//! entry said, a lock holds, a forget is for good and a second one is refused, a taught entry stands
//! at once, and a bad section or a missing repository is refused. `docs/concepts/logbook.md`.

use api::{Logbook, Refusal};
use ipc::{
    EditLogbookEntry, LogbookEntry, LogbookKind, LogbookReplacedBy, LogbookSection, LogbookState, LockLogbookEntry,
    TeachLogbookEntry, TidyLogbook,
};
use store::{LogEntry, LogEvidence, LogHistory, LogHow, LogKind, LogRewrite, LogSection, LogState};

use super::session_host::{rig_fitted, rig_placing, Rig};
use crate::run_memory::places::key_of;

fn learnt(section: LogSection, key: &str, text: &str, state: LogState) -> LogEntry {
    LogEntry {
        id: None,
        section,
        repository: if section == LogSection::Repo { key.to_string() } else { String::new() },
        kind: LogKind::Gotcha,
        text: text.to_string(),
        paths: vec!["crates/api".to_string()],
        evidence: vec![
            LogEvidence { session_id: "s1".into(), cites: vec!["ran:1".into()], how: LogHow::First, hard: false, at: 1_000 },
            LogEvidence { session_id: "s2".into(), cites: vec!["ran:2".into(), "tool:3".into()], how: LogHow::Tripped, hard: false, at: 3_000 },
        ],
        first_seen: 1_000,
        last_confirmed: 3_000,
        last_served: None,
        state,
        locked: false,
        edited_by_person: false,
        tripped: 1,
        history: vec![LogHistory { text: "An older line.".into(), replaced_at: 2_000, why: LogRewrite::Tripped }],
    }
}

async fn seeded(rig: &Rig, section: LogSection, text: &str, state: LogState) -> String {
    let entry = learnt(section, &key_of(&rig.root), text, state);
    let id = rig.fleet.store().lock().await.save_logbook_entry(&entry).expect("kept").expect("not forgotten");
    id.to_string()
}

fn code(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

async fn read(rig: &Rig, root: bool, section: Option<LogbookSection>, state: Option<LogbookState>) -> Vec<LogbookEntry> {
    let root = root.then(|| rig.root.clone());
    rig.fleet.get_logbook(root, section, state).await.expect("read").entries
}

fn teach(rig: &Rig, section: LogbookSection, repository: bool, text: &str) -> TeachLogbookEntry {
    TeachLogbookEntry {
        section,
        repository: repository.then(|| rig.root.clone()),
        kind: LogbookKind::Convention,
        text: text.to_string(),
        paths: Vec::new(),
        source: None,
    }
}

#[tokio::test]
async fn the_read_lists_every_state_and_narrows_by_state_and_section() {
    let rig = rig_placing(None);
    seeded(&rig, LogSection::Repo, "Use the focused test command.", LogState::Standing).await;
    seeded(&rig, LogSection::Repo, "Seen once.", LogState::Candidate).await;
    seeded(&rig, LogSection::Repo, "Not used for a month.", LogState::Faded).await;
    seeded(&rig, LogSection::Habits, "Walk before landing.", LogState::Standing).await;

    let all = read(&rig, true, None, None).await;
    let states = |state| all.iter().filter(|one| one.state == state && one.section == LogbookSection::Repo && one.evidence.len() == 3).count();
    assert_eq!((states(LogbookState::Standing), states(LogbookState::Candidate), states(LogbookState::Faded)), (1, 1, 1));

    let faded = read(&rig, true, None, Some(LogbookState::Faded)).await;
    assert_eq!(faded.len(), 1);
    assert_eq!(faded[0].text, "Not used for a month.");

    let habits = read(&rig, true, Some(LogbookSection::Habits), None).await;
    assert_eq!(habits.len(), 1);
    assert_eq!(habits[0].text, "Walk before landing.");
    assert_eq!(habits[0].repository, None);
}

#[tokio::test]
async fn habits_and_this_macs_entries_are_read_without_a_repository() {
    let rig = rig_placing(None);
    seeded(&rig, LogSection::Repo, "Belongs to the repository.", LogState::Standing).await;
    seeded(&rig, LogSection::Habits, "Walk before landing.", LogState::Standing).await;
    seeded(&rig, LogSection::Machine, "Chrome is not in /Applications.", LogState::Standing).await;

    let apart = read(&rig, false, None, None).await;
    assert!(apart.iter().all(|one| one.section != LogbookSection::Repo), "no repository entry without a repository");
    assert!(apart.iter().any(|one| one.text == "Walk before landing."));
    assert!(apart.iter().any(|one| one.text == "Chrome is not in /Applications."));

    let (status, said) = code(rig.fleet.get_logbook(Some("/nowhere".into()), None, None).await.expect_err("not served"));
    assert_eq!((status, said.as_str()), (404, "fleet.run_memory_repository_not_served"));
}

#[tokio::test]
async fn an_entry_reads_with_its_evidence_trip_and_earlier_text_in_the_mock_s_shape() {
    let rig = rig_placing(None);
    seeded(&rig, LogSection::Repo, "Use the focused test command.", LogState::Standing).await;
    let entry = read(&rig, true, Some(LogbookSection::Repo), None).await.into_iter().next().expect("one");

    assert_eq!(entry.repository.as_deref(), Some(rig.root.as_str()));
    assert_eq!(entry.confirmed_by, 2);
    assert_eq!(entry.evidence.len(), 3, "a row for each cite");
    assert_eq!(entry.evidence[0].job_id, "s1");
    assert_eq!(entry.evidence[0].cite, "ran:1");
    assert_eq!(entry.last_tripped.expect("tripped").as_str(), "1970-01-01T00:00:03.000Z");
    assert_eq!(entry.history.len(), 1);
    assert_eq!(entry.history[0].by, LogbookReplacedBy::Distil);
    assert!(!entry.edited && !entry.locked);
}

#[tokio::test]
async fn an_edit_sets_the_text_marks_it_edited_and_keeps_the_text_it_had() {
    let rig = rig_placing(None);
    let id = seeded(&rig, LogSection::Repo, "Use the focused test command.", LogState::Standing).await;

    let first = rig.fleet.edit_logbook_entry(id.clone(), EditLogbookEntry { text: "  Run one test, never the suite.  ".into() }).await.expect("edited");
    assert_eq!(first.text, "Run one test, never the suite.");
    assert!(first.edited);
    assert_eq!((first.history.len(), first.history[0].text.as_str(), first.history[0].by), (2, "Use the focused test command.", LogbookReplacedBy::Person));
    assert_eq!(first.history[1].text, "An older line.", "newest first");

    let same = rig.fleet.edit_logbook_entry(id.clone(), EditLogbookEntry { text: "Run one test, never the suite.".into() }).await.expect("edited");
    assert_eq!(same.history.len(), 2, "the same text adds no history");
}

#[tokio::test]
async fn an_edit_is_not_undone_by_the_file_that_still_holds_the_old_text() {
    let rig = rig_placing(None);
    let id = seeded(&rig, LogSection::Habits, "Walk before landing.", LogState::Standing).await;
    rig.fleet.logbook_written(&rig.root).await;

    rig.fleet.edit_logbook_entry(id, EditLogbookEntry { text: "Walk, then land.".into() }).await.expect("edited");
    let habits = read(&rig, false, Some(LogbookSection::Habits), None).await;
    assert_eq!(habits.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), ["Walk, then land."]);
}

#[tokio::test]
async fn an_edit_is_refused_for_empty_text_an_unknown_id_a_kept_fact_and_forgotten_text() {
    let rig = rig_placing(None);
    let id = seeded(&rig, LogSection::Repo, "Use the focused test command.", LogState::Standing).await;
    let other = seeded(&rig, LogSection::Repo, "Second line.", LogState::Standing).await;
    let edit = |text: &str| EditLogbookEntry { text: text.to_string() };

    let empty = rig.fleet.edit_logbook_entry(id.clone(), edit("   ")).await.expect_err("empty");
    assert_eq!(code(empty), (422, "fleet.logbook_text_refused".to_string()));
    let long = rig.fleet.edit_logbook_entry(id.clone(), edit(&"a".repeat(401))).await.expect_err("long");
    assert_eq!(code(long), (422, "fleet.logbook_text_refused".to_string()));
    let unknown = rig.fleet.edit_logbook_entry("9999".into(), edit("Text.")).await.expect_err("unknown");
    assert_eq!(code(unknown), (404, "fleet.no_such_logbook_entry".to_string()));
    let folded = rig.fleet.edit_logbook_entry("runs-00000000000000ff".into(), edit("Text.")).await.expect_err("kept elsewhere");
    assert_eq!(code(folded), (422, "fleet.logbook_entry_not_kept".to_string()));

    rig.fleet.forget_logbook_entry(other).await.expect("forgot");
    let back = rig.fleet.edit_logbook_entry(id, edit("Second line.")).await.expect_err("forgotten text");
    assert_eq!(code(back), (422, "fleet.logbook_text_forgotten".to_string()));
}

#[tokio::test]
async fn a_pin_holds_and_lets_go() {
    let rig = rig_placing(None);
    let id = seeded(&rig, LogSection::Habits, "Walk before landing.", LogState::Standing).await;

    assert!(rig.fleet.lock_logbook_entry(id.clone(), LockLogbookEntry { locked: true }).await.expect("locked").locked);
    assert!(read(&rig, false, None, None).await.iter().any(|one| one.id == id && one.locked), "kept");
    assert!(!rig.fleet.lock_logbook_entry(id, LockLogbookEntry { locked: false }).await.expect("let go").locked);
}

#[tokio::test]
async fn a_forgotten_entry_is_gone_a_second_forget_is_refused_and_the_text_stays_on_the_deny_list() {
    let rig = rig_placing(None);
    let id = seeded(&rig, LogSection::Repo, "Use the focused test command.", LogState::Standing).await;

    let forgot = rig.fleet.forget_logbook_entry(id.clone()).await.expect("forgot");
    assert_eq!(forgot.id, id);
    assert!(read(&rig, true, Some(LogbookSection::Repo), None).await.is_empty());

    let again = rig.fleet.forget_logbook_entry(id.clone()).await.expect_err("already forgotten");
    assert_eq!(code(again), (404, "fleet.no_such_logbook_entry".to_string()));
    let locked = rig.fleet.lock_logbook_entry(id, LockLogbookEntry { locked: true }).await.expect_err("gone");
    assert_eq!(locked.status(), 404);

    let key = key_of(&rig.root);
    let denied = rig.fleet.store().lock().await.logbook_denied(LogSection::Repo, &key, "use the focused TEST command").expect("read");
    assert!(denied, "its words are on the deny list, and so is the text it had before");
    let saved = rig.fleet.store().lock().await.save_logbook_entry(&learnt(LogSection::Repo, &key, "Use the focused test command.", LogState::Candidate));
    assert_eq!(saved.expect("read"), None, "nothing brings it back");
}

#[tokio::test]
async fn a_taught_entry_stands_at_once_marked_the_person_s_and_teaching_it_again_makes_no_second() {
    let rig = rig_placing(None);
    let taught = rig.fleet.teach_logbook_entry(teach(&rig, LogbookSection::Repo, true, "Never run the whole suite.")).await.expect("taught");
    assert_eq!(taught.state, LogbookState::Standing);
    assert!(taught.edited && taught.evidence.is_empty());
    assert_eq!(taught.repository.as_deref(), Some(rig.root.as_str()));
    assert_eq!(taught.kind, LogbookKind::Convention);

    let again = rig.fleet.teach_logbook_entry(teach(&rig, LogbookSection::Repo, true, "never run the whole suite")).await.expect("taught");
    assert_eq!(again.id, taught.id);
    assert_eq!(read(&rig, true, Some(LogbookSection::Repo), None).await.len(), 1);

    let habit = rig.fleet.teach_logbook_entry(teach(&rig, LogbookSection::Habits, true, "Walk before landing.")).await.expect("taught");
    assert_eq!((habit.section, habit.repository), (LogbookSection::Habits, None), "habits name no repository");
}

#[tokio::test]
async fn a_taught_entry_with_a_source_shows_it_as_its_one_evidence_row() {
    let rig = rig_placing(None);
    let source = ipc::TeachSource { session_id: "job-9".into(), cite: "message:4".into() };
    let from = TeachLogbookEntry { source: Some(source), ..teach(&rig, LogbookSection::Habits, false, "Walk before landing.") };
    let taught = rig.fleet.teach_logbook_entry(from).await.expect("taught");
    assert_eq!(taught.evidence.len(), 1);
    assert_eq!((taught.evidence[0].job_id.as_str(), taught.evidence[0].cite.as_str()), ("job-9", "message:4"));
    let read = read(&rig, false, Some(LogbookSection::Habits), None).await;
    assert_eq!(read[0].evidence, taught.evidence, "kept, not only answered");
}

#[tokio::test]
async fn teaching_forgotten_text_brings_it_back() {
    let rig = rig_placing(None);
    let id = seeded(&rig, LogSection::Habits, "Walk before landing.", LogState::Standing).await;
    rig.fleet.forget_logbook_entry(id).await.expect("forgot");

    let taught = rig.fleet.teach_logbook_entry(teach(&rig, LogbookSection::Habits, false, "Walk before landing.")).await.expect("taught");
    assert_eq!(taught.state, LogbookState::Standing);
    assert_eq!(read(&rig, false, Some(LogbookSection::Habits), None).await.len(), 1);
}

#[tokio::test]
async fn a_repository_entry_without_a_repository_or_with_empty_text_is_refused() {
    let rig = rig_placing(None);
    let none = rig.fleet.teach_logbook_entry(teach(&rig, LogbookSection::Repo, false, "Text.")).await.expect_err("no repository");
    assert_eq!(code(none), (422, "fleet.logbook_repository_missing".to_string()));
    let empty = rig.fleet.teach_logbook_entry(teach(&rig, LogbookSection::Habits, false, " ")).await.expect_err("empty");
    assert_eq!(code(empty), (422, "fleet.logbook_text_refused".to_string()));
    let elsewhere = TeachLogbookEntry { repository: Some("/nowhere".into()), ..teach(&rig, LogbookSection::Repo, false, "Text.") };
    assert_eq!(rig.fleet.teach_logbook_entry(elsewhere).await.expect_err("not served").status(), 404);
}

/// Wait until no tidy is running, and say what the Logbook then reports of it.
async fn settled(rig: &Rig) -> ipc::LogbookTidy {
    for _ in 0..200 {
        let tidy = rig.fleet.get_logbook(None, None, None).await.expect("read").tidy;
        if !tidy.running {
            return tidy;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("the tidy never finished");
}

#[tokio::test]
async fn tidy_answers_at_once_while_the_pass_runs_and_a_second_call_starts_no_second_pass() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::{Notify, Semaphore};
    let rig = rig_fitted(|_| {});
    seeded(&rig, LogSection::Repo, "Run the focused test command.", LogState::Standing).await;
    seeded(&rig, LogSection::Repo, "Run the focused test command with a log.", LogState::Standing).await;
    let (gate, asked, calls) = (Arc::new(Semaphore::new(0)), Arc::new(Notify::new()), Arc::new(AtomicUsize::new(0)));
    let holding = {
        let (gate, asked, calls) = (Arc::clone(&gate), Arc::clone(&asked), Arc::clone(&calls));
        move |_question: String| {
            let (gate, asked, calls) = (Arc::clone(&gate), Arc::clone(&asked), Arc::clone(&calls));
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                asked.notify_one();
                gate.acquire().await.expect("open").forget();
                Ok(r#"{"groups":[{"keep":1,"text":"Run the focused test command, logging to a file.","absorb":[2]}]}"#.to_string())
            }
        }
    };
    let wait = std::time::Duration::from_secs(5);

    rig.fleet.logbook_tidy_started_with(Some(rig.root.clone()), holding.clone());
    tokio::time::timeout(wait, asked.notified()).await.expect("the pass asked");
    assert!(rig.fleet.get_logbook(None, None, None).await.expect("read").tidy.running);

    rig.fleet.logbook_tidy_started_with(Some(rig.root.clone()), holding);
    gate.add_permits(1);
    let tidy = settled(&rig).await;
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a second pass was started");
    assert_eq!(tidy.last.map(|last| last.merged), Some(1));
}

#[tokio::test]
async fn a_failed_pass_is_kept_as_the_last_one_and_says_it_failed() {
    let rig = rig_fitted(|_| {});
    seeded(&rig, LogSection::Repo, "Run the focused test command.", LogState::Standing).await;
    seeded(&rig, LogSection::Repo, "Run the focused test command with a log.", LogState::Standing).await;
    rig.fleet.logbook_tidy_started_with(Some(rig.root.clone()), |_question: String| async { Err("the call failed".to_string()) });
    let last = settled(&rig).await.last.expect("a failed pass is remembered");
    assert_eq!((last.merged, last.failed), (0, true));
}

#[tokio::test]
async fn the_logbook_says_whether_a_tidy_is_running_and_what_the_last_one_merged() {
    let said = r#"{"groups":[{"keep":1,"text":"Run the focused test command, logging to a file.","absorb":[2]}]}"#;
    let rig = rig_fitted(|fittings| fittings.judge = std::sync::Arc::new(testkit::FakeJudge::saying(said)));
    seeded(&rig, LogSection::Repo, "Run the focused test command.", LogState::Standing).await;
    seeded(&rig, LogSection::Repo, "Run the focused test command with a log.", LogState::Standing).await;
    let before = rig.fleet.get_logbook(None, None, None).await.expect("read").tidy;
    assert_eq!((before.running, before.last), (false, None));

    assert!(rig.fleet.clone().tidy_logbook(TidyLogbook { repository: Some(rig.root.clone()) }).await.expect("started").running);
    let after = settled(&rig).await;
    let last = after.last.expect("the pass is remembered");
    assert_eq!((last.merged, last.failed), (1, false));
    assert!(!last.at.as_str().is_empty());
}

#[tokio::test]
async fn tidy_logbook_merges_near_duplicates_in_the_background() {
    // A fresh store numbers its entries 1 and 2, so the judge can name them before they exist.
    let said = r#"{"groups":[{"keep":1,"text":"Run the focused test command, logging to a file.","absorb":[2]}]}"#;
    let rig = rig_fitted(|fittings| fittings.judge = std::sync::Arc::new(testkit::FakeJudge::saying(said)));
    let first = seeded(&rig, LogSection::Repo, "Run the focused test command.", LogState::Standing).await;
    let second = seeded(&rig, LogSection::Repo, "Run the focused test command with a log.", LogState::Standing).await;
    assert_eq!((first.as_str(), second.as_str()), ("1", "2"));

    let started = rig.fleet.clone().tidy_logbook(TidyLogbook { repository: Some(rig.root.clone()) }).await.expect("started");
    assert!(started.running);
    assert_eq!(settled(&rig).await.last.map(|last| last.merged), Some(1));

    let kept = read(&rig, true, Some(LogbookSection::Repo), Some(LogbookState::Standing)).await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].history.first().map(|earlier| earlier.by), Some(LogbookReplacedBy::Merged));
}
