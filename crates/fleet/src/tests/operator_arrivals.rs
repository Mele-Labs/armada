//! What Fleet hands the Operator: only the live Jobs that share a path, an issue or uncommon words
//! with what arrived, with what they share, and no call at all where none does.

use std::sync::{Arc, Mutex};

use core_model::Timestamp;

use crate::operator::{Acting, Answering, Arrival, Arrived, Calling, Desk, Done, LiveJob, Logged, Operator, PassedOn, Ruled, Source, Way, FIRST_WRITE_WAIT};
use crate::CallFailed;

const REPO: &str = "01REPO";

struct Asked {
    replies: Mutex<Vec<String>>,
    asked: Mutex<Vec<String>>,
}

impl Calling for Asked {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let reply = self.replies.lock().expect("not poisoned").pop();
        Box::pin(async move { reply.ok_or(CallFailed::SaidNothing) })
    }
}

struct Desked {
    live: Vec<LiveJob>,
    log: Mutex<Vec<Logged>>,
}

impl Desk for Desked {
    fn now(&self) -> Timestamp {
        Timestamp::from_rfc3339("2026-11-04T10:00:00.000Z")
    }

    fn day_began(&self) -> Done<'_, Timestamp> {
        Box::pin(async move { Timestamp::from_rfc3339("2026-11-04T00:00:00.000Z") })
    }

    fn daily_calls(&self) -> u32 {
        100
    }

    fn live(&self) -> Done<'_, Vec<LiveJob>> {
        Box::pin(async move { self.live.clone() })
    }

    fn calls_since(&self, _since: Timestamp) -> Done<'_, u32> {
        Box::pin(async move { 0 })
    }

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move { self.log.lock().expect("not poisoned").push(row) })
    }
}

impl Acting for Desked {
    fn go_ahead(&self, _session: &str) {}
    fn hand_over(&self, _session: &str, _group: Option<&str>, _into: &str, _shared: &[String]) {}
    fn merge(&self, _ending: &str, _into: &str, _shared: &[String]) {}
    fn keep_in_touch(&self, _one: &str, _other: &str, _shared: &[String]) {}
}

fn job(id: &str, manifest: &str, words: &str, paths: &[&str], issues: &[&str]) -> LiveJob {
    LiveJob {
        session_id: id.into(),
        title: format!("Job {id}"),
        manifest_id: manifest.into(),
        words: words.into(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        issues: issues.iter().map(|one| one.to_string()).collect(),
        running: Vec::new(),
        started_by: Vec::new(),
    }
}

fn arriving(words: &str, paths: &[&str], issues: &[&str]) -> Arrival {
    Arrival {
        id: "start:01NEW".into(),
        arrived: Arrived::Start,
        source: Source::Dispatch,
        session_id: "01NEW".into(),
        manifest_id: REPO.into(),
        subject: "Fix the pane".into(),
        words: words.into(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        issues: issues.iter().map(|one| one.to_string()).collect(),
        group: None,
        ways: Way::STARTING.to_vec(),
        answers: Vec::new(),
        rule: None,
        open: Vec::new(),
    }
}

fn operator(live: Vec<LiveJob>, replies: &[&str]) -> (Arc<Operator>, Arc<Asked>, Arc<Desked>) {
    let call = Arc::new(Asked { replies: Mutex::new(replies.iter().rev().map(|one| one.to_string()).collect()), asked: Mutex::new(Vec::new()) });
    let desk = Arc::new(Desked { live, log: Mutex::new(Vec::new()) });
    (Operator::new(call.clone(), desk.clone(), desk.clone()), call, desk)
}

#[tokio::test]
async fn the_call_is_shown_only_the_jobs_that_share_a_path_an_issue_or_words_and_what_they_share() {
    let live = vec![
        job("01NEW", REPO, "", &["apps/panes/terminal.tsx"], &[]),
        job("01FOLDER", REPO, "", &["apps/panes/"], &[]),
        job("01PATTERN", REPO, "", &["apps/pan*"], &[]),
        job("01ISSUE", REPO, "", &["crates/store/"], &["armada-a3f2"]),
        job("01WORDS", REPO, "Terminal scrollback is lost when the pane redraws", &[], &[]),
        job("01NOTHING", REPO, "Give the settings page a search box", &["crates/config/"], &["armada-b111"]),
        job("01ELSEWHERE", "01OTHERREPO", "Terminal scrollback is lost", &["apps/panes/terminal.tsx"], &["armada-a3f2"]),
    ];
    let reply = "item: 1\nway: 3\njob: 01NOTHING\nshared: crates/config/\nbecause: not offered";
    let (operator, call, _) = operator(live, &[reply]);

    let ruled = operator
        .ruled(arriving("The terminal scrollback is lost on redraw", &["apps/panes/terminal.tsx"], &["armada-a3f2"]), FIRST_WRITE_WAIT)
        .await;

    let asked = call.asked.lock().expect("not poisoned").clone();
    assert_eq!(asked.len(), 1);
    let question = &asked[0];
    for near in ["01FOLDER", "01PATTERN", "01ISSUE", "01WORDS"] {
        assert!(question.contains(&format!("Job {near}:")), "{near} shares something:\n{question}");
    }
    for far in ["01NOTHING", "01ELSEWHERE", "Job 01NEW:"] {
        assert!(!question.contains(far), "{far} shares nothing, is another repository's or is its own:\n{question}");
    }
    assert!(question.contains("With item 1: paths apps/panes/terminal.tsx"), "{question}");
    assert!(question.contains("With item 1: issues armada-a3f2"), "{question}");
    assert!(question.contains("With item 1: words redraw, scrollback, terminal"), "{question}");
    assert!(matches!(&ruled, Ruled::PassedOn(PassedOn::Refused(why)) if why.contains("01NOTHING")), "a Job not shown cannot be named: {ruled:?}");
}

#[tokio::test]
async fn a_live_job_is_shown_with_who_started_it_up_to_the_root_and_a_job_the_person_started_with_no_line() {
    let mut grandchild = job("01GRAND", REPO, "", &["apps/panes/terminal.tsx"], &[]);
    grandchild.started_by = vec![("01CHILD".into(), "Evaluate T3 Architecture".into()), ("01ROOT".into(), "Compare editors".into())];
    let mut child = job("01CHILD", REPO, "", &["apps/panes/terminal.tsx"], &[]);
    child.started_by = vec![("01ROOT".into(), "Compare editors".into())];
    let by_hand = job("01HAND", REPO, "", &["apps/panes/terminal.tsx"], &[]);
    let reply = "item: 1\nway: 3\njob: 01HAND\nshared: apps/panes/terminal.tsx\nbecause: not offered";
    let (operator, call, _) = operator(vec![grandchild, child, by_hand], &[reply]);

    operator.ruled(arriving("Fix the terminal pane", &["apps/panes/terminal.tsx"], &[]), FIRST_WRITE_WAIT).await;

    let asked = call.asked.lock().expect("not poisoned").clone();
    let question = &asked[0];
    assert!(
        question.contains("    Started by: Job 01CHILD \"Evaluate T3 Architecture\", which Job 01ROOT \"Compare editors\" started\n"),
        "the chain, nearest first:\n{question}"
    );
    assert!(question.contains("    Started by: Job 01ROOT \"Compare editors\"\n"), "a child names its starter alone:\n{question}");
    assert_eq!(question.matches("Started by:").count(), 2, "the person's own Job has no line:\n{question}");
}

#[tokio::test]
async fn work_no_live_job_shares_anything_with_is_a_new_job_without_a_call() {
    let live = vec![job("01OTHER", REPO, "Give the settings page a search box", &["crates/config/"], &[])];
    let (operator, call, desk) = operator(live, &[]);

    let ruled = operator.ruled(arriving("Fix the terminal pane", &["apps/panes/terminal.tsx"], &[]), FIRST_WRITE_WAIT).await;

    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::NewJob && ruling.other.is_none()), "{ruled:?}");
    assert!(call.asked.lock().expect("not poisoned").is_empty(), "no call is made");
    let log = desk.log.lock().expect("not poisoned").clone();
    assert_eq!((log.len(), log[0].call.as_deref()), (1, None));
    assert!(log[0].because.contains("no live Job shares"), "{}", log[0].because);
}

#[tokio::test]
async fn a_job_with_a_subagent_and_a_task_running_is_shown_with_what_they_are() {
    let running = vec![
        ipc::RunningWork { kind: ipc::RunningKind::Subagent, what: "Map the claims".into(), since: ipc::Instant::carried("2026-11-05T10:00:00.000Z") },
        ipc::RunningWork { kind: ipc::RunningKind::BackgroundTask, what: "dev server".into(), since: ipc::Instant::carried("2026-11-05T10:05:00.000Z") },
    ];
    let live = vec![LiveJob { running, ..job("01BUSY", REPO, "", &["apps/panes/"], &[]) }, job("01QUIET", REPO, "", &["apps/panes/"], &[])];
    let (operator, call, _) = operator(live, &["item: 1\nway: 1\nbecause: new"]);

    let _ = operator.ruled(arriving("Redraw the pane", &["apps/panes/terminal.tsx"], &[]), FIRST_WRITE_WAIT).await;

    let question = call.asked.lock().expect("not poisoned").clone().remove(0);
    assert!(question.contains("Still running past its turn: subagent \"Map the claims\", background task \"dev server\""), "{question}");
    assert_eq!(question.matches("Still running past its turn").count(), 1, "the quiet Job says nothing of it:\n{question}");
}
