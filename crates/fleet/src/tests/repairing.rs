//! The claim: **a repair is an agent hosted beside a broken Job, with no session of its own, that Fleet
//! reads to its end** and stops at twenty minutes. The brief is `crate::repair::brief`'s tests; the
//! store is `store`'s. Here are the pure rules and the host against a stand-in process.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapters::HeadlessAgent;

use crate::drone::HostPaths;
use crate::repair::{report_of, verdict_of, Finish, RepairBreak, RepairEnded, RepairHost, Repairs, Report, REPAIR_LIMIT};
use crate::tests::tmp::TempDir;

/// The agent CLI as a repair meets it. It logs what it was started with and told, waits while a test
/// holds it (`hold` without `release`) or a brief names `never ends`, and answers with what a test left
/// in `answer`, or that it repaired. The waits end with the Fleet that started it.
const STAND_IN: &str = r##"#!/bin/sh
state='@STATE@'
printf '%s\n' "$*" >> "$state/argv.log"
pwd -P >> "$state/cwd.log"
IFS= read -r turn
printf '%s\n' "$turn" >> "$state/turns.log"
said() {
  printf '{"type":"assistant","message":{"content":[{"type":"text","text":"%s"}]}}\n' "$1"
}
printf '{"type":"system","subtype":"init","session_id":"repair-1","model":"stand-in","mcp_servers":[]}\n'
case "$turn" in
  *never\ ends*) sleep 60 ;;
esac
while [ -e "$state/hold" ] && [ ! -e "$state/release" ] && kill -0 $PPID 2>/dev/null; do sleep 0.05; done
if [ -f "$state/answer" ]; then answer=$(cat "$state/answer"); else answer='REPAIRED: it carries on'; fi
said "$answer"
printf '{"type":"result","subtype":"success","is_error":false,"num_turns":1,"total_cost_usd":0.01,"permission_denials":[]}\n'
"##;

/// Where a stand-in repair keeps what a test reads and leaves for it.
pub(super) fn state_in(home: &Path) -> PathBuf {
    home.join("repair-state")
}

/// A host whose agent is the stand-in, for a Fleet or for a host alone.
pub(super) fn stand_in(home: &Path) -> RepairHost {
    let state = state_in(home);
    std::fs::create_dir_all(&state).expect("a state directory");
    let script = home.join("repair-stand-in.sh");
    std::fs::write(&script, STAND_IN.replace("@STATE@", &state.to_string_lossy())).expect("the stand-in is written");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("the stand-in runs");
    let root = home.to_string_lossy().to_string();
    RepairHost::new(
        HeadlessAgent::at(script.to_string_lossy()),
        "stand-in",
        home.join("repair-mcp.json"),
        HostPaths { path: "/usr/bin:/bin", home: &root, user: "someone", width: checks_runner::CheckWidth::of(2) },
    )
}

/// The directory each repair was started in, one per line, as the process itself read it.
pub(super) fn directories(state: &Path) -> Vec<String> {
    std::fs::read_to_string(state.join("cwd.log")).unwrap_or_default().lines().map(str::to_string).collect()
}

/// What every repair was told on its one turn, one per line.
pub(super) fn turns(state: &Path) -> Vec<String> {
    std::fs::read_to_string(state.join("turns.log")).unwrap_or_default().lines().map(str::to_string).collect()
}

#[test]
fn what_broke_is_said_in_plain_words_and_kept_under_a_key() {
    let words: Vec<(&str, String)> = [
        RepairBreak::Quiet,
        RepairBreak::Hung,
        RepairBreak::Died,
        RepairBreak::Checkout,
        RepairBreak::RedPull { number: 3417 },
        RepairBreak::Stranded { slot: 15 },
    ]
    .into_iter()
    .map(|broke| (broke.key(), broke.words()))
    .collect();
    assert_eq!(
        words,
        [
            ("quiet", "it stopped twice with nothing asked".to_string()),
            ("hung", "a command hung".to_string()),
            ("died", "its process died twice".to_string()),
            ("checkout", "its checkout is broken".to_string()),
            ("red_pull", "checks failed on #3417".to_string()),
            ("stranded", "slot-15 was left with changes".to_string()),
        ]
    );
}

#[test]
fn only_a_break_with_an_agent_to_carry_on_wakes_its_job() {
    let woken: Vec<Option<&str>> = RepairBreak::ALL.iter().map(|broke| broke.woke_with()).collect();
    assert_eq!(
        woken,
        [
            Some("worked out why it stopped"),
            Some("stopped the command that hung"),
            Some("fixed what stopped its process"),
            Some("put its checkout right"),
            None,
            None
        ]
    );
}

#[test]
fn a_repair_is_read_by_the_first_line_of_its_last_message_and_nothing_else() {
    let read = |text: &str| report_of(Some(text));
    assert_eq!(read("REPAIRED: it stopped the sleep"), Report::Repaired("it stopped the sleep".to_string()));
    assert_eq!(read("NOT REPAIRED: no such process\nREPAIRED: later"), Report::NotRepaired("no such process".to_string()));
    assert_eq!(read("REPAIRED:\nit did"), Report::Repaired(String::new()), "a marker with no sentence still counts");
    assert_eq!(read("  REPAIRED: leading space"), Report::Repaired("leading space".to_string()));
    assert_eq!(read("I fixed it.\nREPAIRED: yes"), Report::Unmarked, "the second line is not read");
    assert_eq!(read("repaired: lower case"), Report::Unmarked);
    assert_eq!(read(""), Report::Unmarked);
    assert_eq!(report_of(None), Report::Unmarked);
}

fn ended(said: Option<&str>, finish: Finish) -> RepairEnded {
    RepairEnded { said: said.map(str::to_string), finish }
}

#[test]
fn only_a_turn_that_ended_itself_and_said_repaired_counts_as_repaired() {
    let outcome = |ended: &RepairEnded| {
        let verdict = verdict_of(ended);
        (verdict.outcome, verdict.repaired, verdict.said)
    };
    assert_eq!(outcome(&ended(Some("REPAIRED: done"), Finish::Ended)), ("repaired", true, "done".to_string()));
    assert_eq!(outcome(&ended(Some("NOT REPAIRED: no log"), Finish::Ended)), ("not_repaired", false, "no log".to_string()));
    let unmarked = outcome(&ended(Some("All done."), Finish::Ended));
    assert_eq!((unmarked.0, unmarked.1), ("unmarked", false));
    assert_eq!(outcome(&ended(None, Finish::Ended)).0, "unmarked");
    // A marker on a turn that did not end itself is not trusted.
    assert_eq!(outcome(&ended(Some("REPAIRED: done"), Finish::TimedOut)).1, false);
    assert_eq!(outcome(&ended(Some("REPAIRED: done"), Finish::Stopped)), ("stopped", false, "the job moved on first".to_string()));
    assert_eq!(outcome(&ended(None, Finish::Failed("no binary".to_string()))), ("failed", false, "no binary".to_string()));
}

#[test]
fn a_repair_is_stopped_at_twenty_minutes_and_the_note_says_so() {
    assert_eq!(REPAIR_LIMIT, Duration::from_secs(20 * 60));
    let verdict = verdict_of(&ended(None, Finish::TimedOut));
    assert_eq!((verdict.outcome, verdict.repaired, verdict.said.as_str()), ("timed_out", false, "it ran past 20 minutes"));
}

fn directory(home: &TempDir) -> String {
    let checkout = home.path().join("checkout");
    std::fs::create_dir_all(&checkout).expect("a checkout");
    checkout.to_string_lossy().to_string()
}

#[tokio::test]
async fn a_repair_read_to_its_end_answers_what_it_said_last() {
    let home = TempDir::new();
    let host = Arc::new(stand_in(home.path()));
    std::fs::write(state_in(home.path()).join("answer"), "REPAIRED: it did\\nand more").expect("an answer");
    let running = Repairs::hosted_by(Arc::clone(&host)).listed("j", "r1", String::new(), String::new()).expect("listed");
    let ended = host.run(&directory(&home), "fix it", &running).await;
    assert_eq!(ended.finish, Finish::Ended);
    assert_eq!(report_of(ended.said.as_deref()), Report::Repaired("it did".to_string()));
    assert_eq!(turns(&state_in(home.path())).len(), 1, "one turn, told once");
}

#[tokio::test]
async fn a_repair_still_going_at_its_limit_is_interrupted_and_read_as_timed_out() {
    let home = TempDir::new();
    let host = Arc::new(stand_in(home.path()).stopping_after(Duration::from_millis(300), Duration::from_millis(300)));
    let running = Repairs::hosted_by(Arc::clone(&host)).listed("j", "r1", String::new(), String::new()).expect("listed");
    let started = std::time::Instant::now();
    let ended = host.run(&directory(&home), "never ends", &running).await;
    assert_eq!(ended.finish, Finish::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(20), "it did not wait out its sleep");
}

#[tokio::test]
async fn a_repair_told_to_stop_ends_as_stopped_and_one_is_listed_for_a_job_at_a_time() {
    let home = TempDir::new();
    let host = Arc::new(stand_in(home.path()));
    let repairs = Repairs::hosted_by(Arc::clone(&host));
    let running = repairs.listed("j", "r1", String::new(), String::new()).expect("listed");
    assert!(repairs.listed("j", "r1", String::new(), String::new()).is_none(), "one a Job");
    assert!(repairs.listed("other", "r2", String::new(), String::new()).is_some(), "another Job's is its own");
    assert!(repairs.running_any());
    let dir = directory(&home);
    let reading = tokio::spawn({
        let host = Arc::clone(&host);
        async move { host.run(&dir, "never ends", &running).await }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(repairs.stop("j"));
    let ended = tokio::time::timeout(Duration::from_secs(30), reading).await.expect("it ended").expect("joined");
    assert_eq!(ended.finish, Finish::Stopped);
    repairs.ended("j");
    assert!(!repairs.stop("j"), "none left to stop");
    assert!(repairs.listed("j", "r1", String::new(), String::new()).is_some());
}
