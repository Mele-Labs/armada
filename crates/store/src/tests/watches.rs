//! A watch survives a reopen whole, keeps its newest fifty readings, and is told of once.

use crate::tests::{open, TempDir};
use crate::{KeptWatch, KeptWatchPart, KeptWatchReading, KeptWatchSetAside, KeptWatchTaken, KeptWatchVerdict, READINGS_KEPT};

const AT: &str = "2026-10-22T10:00:00.000Z";

fn watch(id: &str, session: &str) -> KeptWatch {
    KeptWatch {
        id: id.into(),
        session_id: session.into(),
        name: "QA deploy".into(),
        command: vec!["scripts/deploy-state".into(), "abc 123".into()],
        recipe: Some("qa-deploy".into()),
        environment: Some("qa".into()),
        every_seconds: 60,
        gives_up_after_seconds: 7200,
        follows: Some(42),
        state: "waiting".into(),
        live: true,
        started_at: AT.into(),
        reported: Vec::new(),
        changed_at: AT.into(),
        ended_at: None,
        unsaid: None,
        in_terminal: None,
        regex: false,
    }
}

fn read(n: usize) -> KeptWatchReading {
    KeptWatchReading {
        at: format!("2026-10-22T10:{:02}:{:02}.000Z", n / 60, n % 60),
        taken: KeptWatchTaken::Read {
            state: "running".into(),
            finished: false,
            says: format!("reading {n}"),
            link: Some("https://ci.example.com/runs/1".into()),
            parts: vec![KeptWatchPart::new("build", "passed"), KeptWatchPart::new("deploy-api", "running")],
        },
        log: None,
    }
}

#[test]
fn a_watch_and_all_it_kept_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_watch(&watch("w-1", "s1")).unwrap();
    store.keep_watch(&watch("w-2", "s1")).unwrap();
    store.keep_watch(&watch("w-3", "s2")).unwrap();
    store.keep_watch_reading("w-1", &read(1)).unwrap();
    let broken = KeptWatchReading {
        at: AT.into(),
        taken: KeptWatchTaken::Broken { why: "It printed nothing.".into(), output: Some("boom".into()) },
        log: Some("/state/watch-logs/s1/w-1/reading.log".into()),
    };
    store.keep_watch_reading("w-1", &broken).unwrap();
    let aside = KeptWatchSetAside { part: "deploy-api".into(), sentence: "Another team's.".into(), at: AT.into() };
    store.keep_watch_set_aside("w-1", &aside).unwrap();
    let again = KeptWatchSetAside { sentence: "Flaky upstream.".into(), ..aside.clone() };
    store.keep_watch_set_aside("w-1", &again).unwrap();
    let verdict = KeptWatchVerdict {
        at: AT.into(),
        ours: false,
        sentence: "Flaky upstream.".into(),
        parts: vec!["deploy-api".into()],
        links: vec!["https://ci.example.test/runs/7".into()],
        logs: vec!["/scratch/deploy-api.log".into()],
        errors: vec!["Error: upstream answered 503".into()],
    };
    store.keep_watch_verdict("w-1", &verdict).unwrap();
    let mut ended = watch("w-1", "s1");
    ended.state = "failed".into();
    ended.live = false;
    ended.reported = vec!["build".into(), "deploy-api".into()];
    ended.unsaid = Some("failed:".into());
    store.keep_watch(&ended).unwrap();
    drop(store);

    let store = open(&dir);
    let one = store.watch_whole("w-1").unwrap().expect("kept");
    assert_eq!(one.watch, ended, "an update changes what moves and keeps the rest");
    assert_eq!(one.readings, vec![read(1), broken]);
    assert_eq!(one.set_aside, vec![again], "a part set aside again keeps the newer sentence");
    assert_eq!(one.verdicts, vec![verdict]);
    let ids = |all: Vec<crate::KeptWatchWhole>| all.into_iter().map(|one| one.watch.id).collect::<Vec<_>>();
    assert_eq!(ids(store.watches_of("s1").unwrap()), ["w-2", "w-1"], "the newest first");
    assert_eq!(ids(store.live_watches().unwrap()), ["w-2", "w-3"]);
    assert_eq!(ids(store.unsaid_watches().unwrap()), ["w-1"]);
    assert!(store.watch_whole("w-9").unwrap().is_none());
}

#[test]
fn only_the_newest_fifty_readings_are_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_watch(&watch("w-1", "s1")).unwrap();
    for n in 1..=55 {
        store.keep_watch_reading("w-1", &read(n)).unwrap();
    }
    let readings = store.watch_whole("w-1").unwrap().unwrap().readings;
    assert_eq!(readings.len() as i64, READINGS_KEPT);
    assert_eq!(readings.first(), Some(&read(6)));
    assert_eq!(readings.last(), Some(&read(55)));
}

#[test]
fn a_notice_is_kept_once_across_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_watch_notice("w-1", "failed:deploy-api", AT).unwrap());
    assert!(!store.keep_watch_notice("w-1", "failed:deploy-api", AT).unwrap());
    drop(store);
    let store = open(&dir);
    assert!(store.watch_notice_kept("w-1", "failed:deploy-api").unwrap());
    assert!(!store.watch_notice_kept("w-1", "passed").unwrap());
}

#[test]
fn a_terminal_watch_keeps_what_it_waits_for() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut kept = watch("w-terminal", "job-1");
    kept.command = Vec::new();
    kept.in_terminal = Some(String::from("Listening on \\d+"));
    kept.regex = true;
    store.keep_watch(&kept).expect("kept");
    drop(store);
    let whole = open(&dir).watch_whole("w-terminal").expect("reads").expect("there");
    assert_eq!(whole.watch, kept);
}

#[test]
fn a_deleted_watch_takes_everything_kept_of_it_and_nothing_else() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for id in ["w-1", "w-2"] {
        store.keep_watch(&watch(id, "s1")).unwrap();
        store.keep_watch_reading(id, &read(1)).unwrap();
        let aside = KeptWatchSetAside { part: "deploy-api".into(), sentence: "Another team's.".into(), at: AT.into() };
        store.keep_watch_set_aside(id, &aside).unwrap();
        let verdict = KeptWatchVerdict { at: AT.into(), ours: true, sentence: "Mine.".into(), parts: Vec::new(), links: Vec::new(), logs: Vec::new(), errors: Vec::new() };
        store.keep_watch_verdict(id, &verdict).unwrap();
        store.keep_watch_notice(id, "failed:", AT).unwrap();
    }
    assert!(store.delete_watch("w-1").unwrap());
    assert!(!store.delete_watch("w-1").unwrap(), "nothing holds it now");
    assert!(store.watch_whole("w-1").unwrap().is_none());
    assert!(!store.watch_notice_kept("w-1", "failed:").unwrap());
    let readings: i64 =
        store.conn.query_row("SELECT COUNT(*) FROM watch_readings WHERE watch_id = 'w-1'", [], |row| row.get(0)).unwrap();
    assert_eq!(readings, 0);

    let kept = store.watch_whole("w-2").unwrap().expect("the other is kept");
    assert_eq!((kept.readings.len(), kept.set_aside.len(), kept.verdicts.len()), (1, 1, 1));
    assert!(store.watch_notice_kept("w-2", "failed:").unwrap());
}

#[test]
fn a_jobs_ended_watches_are_read_by_their_end() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let ended = |id: &str, session: &str, state: &str| KeptWatch { state: state.into(), live: false, ..watch(id, session) };
    store.keep_watch(&ended("w-1", "s1", "not_ours")).unwrap();
    store.keep_watch(&ended("w-2", "s1", "passed")).unwrap();
    store.keep_watch(&KeptWatch { state: "not_ours".into(), ..watch("w-3", "s1") }).unwrap();
    store.keep_watch(&ended("w-4", "s2", "not_ours")).unwrap();
    store.keep_watch(&ended("w-5", "s1", "not_ours")).unwrap();
    let ids: Vec<String> = store.ended_watches_in("s1", "not_ours").unwrap().into_iter().map(|one| one.watch.id).collect();
    assert_eq!(ids, ["w-1", "w-5"], "ended ones of that Job in that end, the oldest first");
}

#[test]
fn a_part_kept_before_parts_carried_evidence_still_reads() {
    let parts = crate::watches::read_parts(r#"[["build","passed"],["provision","failed","https://ci.example.test/7","Error: no account",null]]"#);
    assert_eq!(parts[0], KeptWatchPart::new("build", "passed"));
    assert_eq!(parts[1].link.as_deref(), Some("https://ci.example.test/7"));
    assert_eq!((parts[1].error.as_deref(), parts[1].tail.as_deref()), (Some("Error: no account"), None));
}
