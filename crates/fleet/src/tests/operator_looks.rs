//! A message to the Operator is put to the call that reads, and a ruling is not: what Fleet writes for the
//! message to read, that it is gone when the message is answered, and that the other call is left alone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use api::Sessions;
use testkit::{FakeTracker, FakeTrackers};

use super::operator_acting::{asked, operator_on};
use super::session_host::{eventually, rig_fitted, Rig};
use crate::operator::{Answering, Calling, Looking};
use crate::CallFailed;

const ISSUE: &str = "The terminal scrollback is lost on redraw";

/// What a call that reads found in front of it when it was asked.
#[derive(Clone, Debug)]
struct Seen {
    question: String,
    folder: PathBuf,
    files: BTreeMap<String, String>,
    roots: Vec<String>,
}

/// A call that reads: it answers each reply in turn, and keeps the folder as it stood while it was out.
#[derive(Default)]
struct Reads {
    replies: Mutex<Vec<String>>,
    seen: Mutex<Vec<Seen>>,
}

impl Reads {
    fn saying(replies: &[&str]) -> Arc<Reads> {
        Arc::new(Reads { replies: Mutex::new(replies.iter().rev().map(|one| (*one).to_string()).collect()), seen: Mutex::new(Vec::new()) })
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("not poisoned").clone()
    }
}

impl Looking for Reads {
    fn look<'a>(&'a self, question: String, folder: &'a Path, roots: &'a [String]) -> Answering<'a> {
        let files = std::fs::read_dir(folder)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| (entry.file_name().to_string_lossy().to_string(), std::fs::read_to_string(entry.path()).unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default();
        self.seen.lock().expect("not poisoned").push(Seen { question, folder: folder.to_path_buf(), files, roots: roots.to_vec() });
        let reply = self.replies.lock().expect("not poisoned").pop();
        Box::pin(async move { reply.ok_or(CallFailed::SaidNothing) })
    }
}

/// The call that holds no tools: it keeps what it was asked and rules the first item the way it was offered.
#[derive(Default)]
struct Rules {
    asked: Mutex<Vec<String>>,
}

impl Rules {
    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl Calling for Rules {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        Box::pin(async { Ok("item: 1\nway: 1\nbecause: nothing else covers it".to_string()) })
    }
}

fn reply(said: &str) -> String {
    format!("cites: none\njob: none\nstart: none\norder: none\nsaid: {said}")
}

fn rig_listing() -> Rig {
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(5, ISSUE)]));
    rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker)))
}

async fn seated(rig: &Rig, rules: Arc<Rules>, reads: Arc<Reads>) {
    operator_on(rig).await;
    assert!(rig.fleet.operator_seated_looking(rules, reads));
}

/// Where every message's folder is made, which holds nothing while none is out.
fn desk_of(rig: &Rig) -> PathBuf {
    rig.fleet.scouts().host().runtime_dir().join("operator-desk")
}

fn left_in(desk: &Path) -> usize {
    std::fs::read_dir(desk).map_or(0, Iterator::count)
}

#[tokio::test]
async fn a_message_reads_a_folder_holding_each_live_job_and_the_open_issues_and_the_folder_is_gone_after() {
    let rig = rig_listing();
    let rules = Arc::new(Rules::default());
    let reads = Reads::saying(&[&reply("It waits on you to land it.")]);
    seated(&rig, rules.clone(), reads.clone()).await;
    let job = asked(&rig, "Cap the retries").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The cap is five.\n\nWant me to land it?");
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { !fleet.operator_board().await.needs_you.is_empty() }
    })
    .await;
    {
        let mut store = rig.fleet.store().lock().await;
        let task = store::KeptTask {
            id: "t1".into(),
            text: "Wire the cap into the retry loop".into(),
            status: "pending".into(),
            first_seen_at: "2026-11-09T09:00:00.000Z".into(),
            completed_at: None,
            edited: false,
            made_by: None,
        };
        store.keep_session_build_tasks(job.as_str(), &[task]).expect("kept");
    }
    let item = rig.fleet.operator_board().await.needs_you.remove(0);
    let rulings = rules.asked().len();

    let said = rig.fleet.operator_talked("what is Cap the retries waiting on?", None).await.expect("answered");

    assert_eq!(said.said, "It waits on you to land it.");
    let seen = reads.seen();
    assert_eq!(seen.len(), 1);
    let seen = &seen[0];
    let mut names: Vec<String> = seen.files.keys().cloned().collect();
    names.sort();
    let mut wanted = vec!["issues.txt".to_string(), "tasks.txt".to_string(), format!("job-{}.txt", job.as_str())];
    wanted.sort();
    assert_eq!(names, wanted);
    let file = &seen.files[&format!("job-{}.txt", job.as_str())];
    assert!(file.contains(&format!("Job {}: \"{}\"", job.as_str(), item.title)), "{file}");
    assert!(file.contains("Wire the cap into the retry loop"), "{file}");
    assert!(file.contains(&format!("Item {}: {}", item.id, item.text)) && item.text.contains("Want me to land it?"), "needs you in full: {file}");
    assert!(file.contains("The person: Cap the retries"), "the thread: {file}");
    assert!(seen.files["issues.txt"].contains(&format!("#5 {ISSUE}")), "{}", seen.files["issues.txt"]);
    assert_eq!(seen.roots, vec![rig.fleet.repositories().hosted()[0].root().to_string()], "the checkouts it may read");
    assert!(seen.question.contains("Read, Grep and Glob") && seen.question.contains(&seen.folder.display().to_string()), "{}", seen.question);
    assert!(seen.question.contains("code at "), "{}", seen.question);

    assert!(!seen.folder.exists(), "the folder is gone once the message is answered");
    assert_eq!(left_in(&desk_of(&rig)), 0);
    assert_eq!(rules.asked().len(), rulings, "a message does not go to the call that rules");
}

#[tokio::test]
async fn a_ruling_goes_through_the_call_with_no_tools_and_never_through_the_one_that_reads() {
    let rig = rig_listing();
    let rules = Arc::new(Rules::default());
    let reads = Reads::saying(&[&reply("Nothing.")]);
    seated(&rig, rules.clone(), reads.clone()).await;
    let job = asked(&rig, "Keep the terminal scrollback when the pane redraws").await;
    let report = ipc::ReportBug { title: ISSUE.into(), what: "The pane redraws and the scrollback is gone.".into(), steps: Vec::new(), errors: Vec::new(), logs: Vec::new(), session_id: Some(job) };
    rig.fleet.report_bug(None, report).await.expect("reported");

    rig.fleet.sources_handed().await;
    eventually(|| async { !rules.asked().is_empty() }).await;

    assert!(rules.asked()[0].contains(ISSUE));
    assert!(reads.seen().is_empty(), "a ruling reads nothing");

    rig.fleet.operator_talked("anything?", None).await.expect("answered");
    assert_eq!(reads.seen().len(), 1);
}

#[tokio::test]
async fn a_folder_an_earlier_fleet_left_mid_message_is_gone_after_the_next_message() {
    let rig = rig_listing();
    let reads = Reads::saying(&[&reply("Nothing.")]);
    seated(&rig, Arc::new(Rules::default()), reads.clone()).await;
    let left = desk_of(&rig).join("talk-1-0");
    std::fs::create_dir_all(&left).expect("made");
    std::fs::write(left.join("issues.txt"), "left over").expect("written");

    rig.fleet.operator_talked("anything?", None).await.expect("answered");

    assert!(!left.exists(), "the left folder was swept");
    assert_eq!(left_in(&desk_of(&rig)), 0);
}

#[tokio::test]
async fn a_message_whose_call_failed_is_refused_and_leaves_no_folder_behind() {
    let rig = rig_listing();
    let reads = Reads::saying(&[]);
    seated(&rig, Arc::new(Rules::default()), reads.clone()).await;
    asked(&rig, "Cap the retries").await;

    let refused = rig.fleet.operator_talked("hello", None).await.expect_err("said nothing");

    assert!(format!("{refused:?}").contains("fleet.operator_said_nothing"));
    assert_eq!(reads.seen().len(), 1, "it was asked");
    assert_eq!(left_in(&desk_of(&rig)), 0);
    assert!(rig.fleet.operator_talk_wire().await.expect("read").lines.is_empty(), "nothing is kept");
}
