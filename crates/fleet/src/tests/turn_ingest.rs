//! The claim: Fleet fills the per-turn usage table from the transcripts on disk, with nothing sent
//! to it. A fixture transcript, a main file and a subagent's, becomes usage rows exactly once
//! however often it is read, each row says whether a subagent answered, names the tools of the turn
//! and labels a turn whose first call was a shell command. A restart reads only what was appended,
//! and a file nobody touched for a week is left alone.
//!
//! **Here and not in `crates/acceptance`**, whose tests write no file: this claim is about files.
//! `docs/practices/acceptance-tests.md`.

use std::io::Write as _;
use std::time::{Duration, SystemTime};

use api::Sessions;
use ipc::{SessionFact, SessionId, SessionReport};
use store::KeptTurnUsage;
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

/// One line of a transcript: an assistant message, or a part of one.
fn assistant(id: &str, second: u32, output: u64, blocks: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-10-17T09:00:{second:02}.000Z","message":{{"id":"{id}","model":"model-a","usage":{{"input_tokens":3,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":{output}}},"content":[{blocks}]}}}}"#
    )
}

fn call(name: &str, command: Option<&str>) -> String {
    match command {
        Some(command) => format!(r#"{{"type":"tool_use","name":"{name}","input":{{"command":"{command}"}}}}"#),
        None => format!(r#"{{"type":"tool_use","name":"{name}","input":{{"file_path":"/x"}}}}"#),
    }
}

const THINKING: &str = r#"{"type":"thinking","thinking":""}"#;
const SAID: &str = r#"{"type":"text","text":"done"}"#;

fn write(path: &std::path::Path, lines: &[String]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

fn rows(store: &store::Store, session: &str) -> Vec<KeptTurnUsage> {
    store.turn_usage(session, None, None).unwrap()
}

#[tokio::test]
async fn transcripts_become_usage_rows_once_and_a_restart_reads_only_what_was_appended() {
    let home = TempDir::new();
    let projects = home.path().join(adapters::SESSIONS);
    let main = projects.join("-work-a/s1.jsonl");
    write(
        &main,
        &[
            r#"{"type":"user","message":{"content":"go"}}"#.to_string(),
            // m1 streams as two lines of one message: its thinking, then its one call.
            assistant("m1", 1, 20, THINKING),
            assistant("m1", 2, 20, &call("Bash", Some("cd /x && grep -r foo ."))),
            // m2 begins with a read, so a later shell call does not label it.
            assistant("m2", 3, 40, &call("Read", None)),
            assistant("m2", 4, 40, &call("Bash", Some("ls"))),
            assistant("m3", 5, 9, SAID),
            assistant("m4", 6, 12, &call("Bash", Some("python3 run.py"))),
        ],
    );
    // The same session under another project folder, smaller: counted once.
    write(&projects.join("-work-b/s1.jsonl"), &[assistant("m1", 1, 20, THINKING)]);
    let subagent = projects.join("-work-a/s1/subagents/agent-x.jsonl");
    write(&subagent, &[assistant("m5", 7, 30, &call("Bash", Some("cargo build")))]);
    // A transcript nobody has touched for a month is not read.
    let old = projects.join("-work-a/old.jsonl");
    write(&old, &[assistant("m9", 8, 5, SAID)]);
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(30 * 86_400))
        .unwrap();

    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let changed = fleet.turns_backfilled(false).await;
    assert_eq!(changed, 5);
    let once = rows(&*fleet.store().lock().await, "s1");
    let ids: Vec<&str> = once.iter().map(|one| one.message_id.as_str()).collect();
    assert_eq!(ids, ["m1", "m2", "m3", "m4", "m5"]);
    let by = |id: &str| once.iter().find(|one| one.message_id == id).unwrap();
    assert_eq!((by("m1").tools.as_slice(), by("m1").bash_use.as_deref()), (&["Bash".to_string()][..], Some("read_search")));
    assert_eq!((by("m2").tools.as_slice(), by("m2").bash_use.as_deref()), (&["Read".to_string(), "Bash".to_string()][..], None));
    assert_eq!((by("m3").tools.len(), by("m3").bash_use.as_deref()), (0, None));
    assert_eq!(by("m4").bash_use.as_deref(), Some("scripts"));
    assert_eq!((by("m5").subagent, by("m5").bash_use.as_deref()), (true, Some("build_test")));
    assert!(!by("m4").subagent);
    assert_eq!((by("m1").input_tokens, by("m1").cache_creation_tokens), (3, 100));
    assert_eq!((by("m1").cache_read_tokens, by("m1").output_tokens), (1000, 20));
    assert!(rows(&*fleet.store().lock().await, "old").is_empty());

    // Reading it again changes nothing, and so does reading it with every offset forgotten.
    assert_eq!(fleet.turns_backfilled(false).await, 0);
    assert_eq!(fleet.turns_backfilled(true).await, 0, "with every offset ignored");
    assert_eq!(rows(&*fleet.store().lock().await, "s1"), once);

    // A live session's file gains a turn: only that is read.
    let started = SessionFact::Started { cwd: "/work/a".into(), title: None, origin: ipc::SessionOrigin::Terminal, mod_version: None };
    let report = SessionReport { harness: "a_harness".into(), session_id: SessionId::carried("s1"), fact: started };
    fleet.report_session(report).await.unwrap();
    let mut file = std::fs::OpenOptions::new().append(true).open(&main).unwrap();
    writeln!(file, "{}", assistant("m6", 9, 15, SAID)).unwrap();
    assert_eq!(fleet.turns_followed().await, 1);
    assert_eq!(fleet.turns_followed().await, 0);
    assert_eq!(rows(&*fleet.store().lock().await, "s1").len(), 6);
    let offset = fleet.store().lock().await.transcript_offset(main.to_str().unwrap()).unwrap();
    assert_eq!(offset, Some(std::fs::metadata(&main).unwrap().len()));
}

#[tokio::test]
async fn the_usage_summary_is_served_and_refuses_a_bound_that_is_no_time() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let now = fleet.now().as_str().to_string();
    let turn = KeptTurnUsage {
        message_id: "m1".into(),
        at: now,
        model: "model-a".into(),
        input_tokens: 0,
        cache_creation_tokens: 0,
        cache_read_tokens: 0,
        output_tokens: 200_000,
        tools: vec!["Bash".into()],
        subagent: false,
        bash_use: Some("git".into()),
    };
    fleet.store().lock().await.keep_turn_usage("s1", &turn).unwrap();
    let summary = fleet
        .get_usage_summary(Some("2000-01-01T00:00:00Z".into()), Some("2999-01-01T00:00:00Z".into()), 0)
        .await
        .unwrap();
    assert!((summary.total.units - 1.0).abs() < 1e-9);
    assert_eq!(summary.sessions.len(), 1);
    assert_eq!(summary.total.by_bash_use[0].name, "git");
    assert!(fleet.get_usage_summary(Some("lately".into()), None, 0).await.is_err());
}
