//! The beads tracker: the `bd` commands it builds, the folder it runs them in, how it reads what
//! comes back, and the mark a write moves.
//!
//! **`bd` answers from a script; `jq` is the real one**, as for Jira, so the filters are what is
//! tried, over JSON in the shape `bd` 1.3.1 printed.

use std::collections::{BTreeMap, VecDeque};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use adapter_traits::{Backlog, BeadState, Tracker, TrackerChoice, Trackers};

use crate::beads_tracker::{main_checkout, mark_of, BeadsTracker};
use crate::jira_tracker::{Process, Run};
use crate::TrackersByProvider;

/// What `bd query --json` printed for two issues, the second closed and claimed.
const QUERIED: &str = r#"[
  {"id":"armada-3fz","title":"Sync the backlog","description":"First line\n\n  second","status":"open",
   "priority":1,"issue_type":"task","created_at":"2026-10-24T03:23:58Z","created_by":"Ana Ruiz",
   "updated_at":"2026-10-24T03:23:58Z","labels":["fleet","beads"]},
  {"id":"armada-1792806783819-955-760bc69c","title":"Imported","status":"in_progress","priority":2,
   "issue_type":"bug","assignee":"mac-a/7d2b2166","created_by":"Ana Ruiz","updated_at":"2026-10-25T09:00:00Z"}
]"#;

/// What `bd show --json` printed for one.
const SHOWN: &str = r#"[{"id":"armada-3fz","title":"Sync the backlog","description":"All of it.\nTwo lines.",
  "status":"in_progress","priority":0,"issue_type":"feature","assignee":"mac-b/0a1b2c3d","owner":"nick@example.com",
  "created_at":"2026-10-24T03:23:58Z","created_by":"Ana Ruiz","updated_at":"2026-10-24T03:24:44Z",
  "labels":["x"],"comments_omitted":true}]"#;

type Answer = (i32, &'static str, &'static str);

/// `bd` answers each call in turn from `answers`; `jq` is the machine's.
#[derive(Default)]
struct Bd {
    answers: Mutex<VecDeque<Answer>>,
    calls: Mutex<Vec<Vec<String>>>,
}

impl Bd {
    fn saying(answers: &[Answer]) -> Bd {
        Bd { answers: Mutex::new(answers.iter().copied().collect()), ..Bd::default() }
    }

    fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().expect("not poisoned").clone()
    }
}

impl Run for &Bd {
    fn run(&self, program: &str, args: &[String], stdin: Option<&[u8]>) -> std::io::Result<Output> {
        if program != "bd" {
            return Process.run(program, args, stdin);
        }
        self.calls.lock().expect("not poisoned").push(args.to_vec());
        let (code, stdout, stderr) = self.answers.lock().expect("not poisoned").pop_front().unwrap_or((0, "", ""));
        Ok(Output { status: ExitStatus::from_raw(code << 8), stdout: stdout.as_bytes().to_vec(), stderr: stderr.as_bytes().to_vec() })
    }
}

fn choosing(provider: &str, keys: &[(&str, &str)]) -> TrackerChoice {
    let keys: BTreeMap<String, String> = keys.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
    TrackerChoice::named(Some(provider), &keys)
}

/// A folder of its own under the system's temporary one, emptied.
fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("armada-beads-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("made");
    dir.canonicalize().expect("there")
}

/// A main checkout and a slot cut from it, as git lays out a linked worktree.
fn checkouts(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let at = folder(name);
    let (main, slot) = (at.join("armada"), at.join("armada/.armada/slots/slot-5"));
    let gitdir = main.join(".git/worktrees/slot-5");
    std::fs::create_dir_all(&gitdir).expect("made");
    std::fs::create_dir_all(&slot).expect("made");
    std::fs::write(gitdir.join("commondir"), "../..\n").expect("written");
    std::fs::write(slot.join(".git"), format!("gitdir: {}\n", gitdir.display())).expect("written");
    (main, slot)
}

#[test]
fn beads_is_a_provider_word_taking_no_keys_and_the_one_tracker_with_a_backlog() {
    let trackers = TrackersByProvider;
    assert_eq!(trackers.tracker(&choosing("beads", &[])).query_guide().language, "bd query");
    assert!(trackers.backlog(&choosing("beads", &[])).is_some());
    assert!(trackers.backlog(&choosing("Beads", &[])).is_some(), "the word is read in any case");

    let refused = trackers.tracker(&choosing("beads", &[("database", "elsewhere")]));
    assert!(refused.query_guide().guide.contains("takes no keys beyond `provider`"), "{:?}", refused.query_guide());
    assert!(trackers.backlog(&choosing("beads", &[("database", "elsewhere")])).is_none());
    for read_only in [choosing("jira", &[("site", "acme"), ("project", "ABC")]), TrackerChoice::of_the_forge()] {
        assert!(trackers.backlog(&read_only).is_none(), "{read_only:?} is never written to");
    }
}

#[test]
fn a_slot_runs_bd_in_the_main_checkout_it_was_cut_from() {
    let (main, slot) = checkouts("slot");
    assert_eq!(main_checkout(&slot), main);
    assert_eq!(main_checkout(&main), main, "the main checkout is its own");

    let bd = Bd::saying(&[(0, "[]", "")]);
    BeadsTracker::with_runner(&bd).open_issues(&slot.display().to_string()).expect("listed");
    assert_eq!(bd.calls()[0][..2], ["-C".to_string(), main.display().to_string()]);
}

#[test]
fn the_open_issues_are_read_through_bd_readonly_newest_edit_first() {
    let bd = Bd::saying(&[(0, QUERIED, "")]);
    let issues = BeadsTracker::with_runner(&bd).open_issues("/repo").expect("listed");

    assert_eq!(
        bd.calls()[0],
        ["-C", "/repo", "--readonly", "query", "status=open OR status=in_progress OR status=blocked", "-n", "0", "--json"]
    );
    let references: Vec<&str> = issues.iter().map(|one| one.reference.as_str()).collect();
    assert_eq!(references, ["armada-1792806783819-955-760bc69c", "armada-3fz"]);
    let (claimed, open) = (&issues[0], &issues[1]);
    assert_eq!(claimed.assignees[0].as_written(), "mac-a/7d2b2166");
    assert_eq!(open.title.as_written(), "Sync the backlog");
    assert_eq!(open.excerpt.as_written(), "First line second");
    assert_eq!(open.author.as_written(), "Ana Ruiz");
    assert!(open.assignees.is_empty());
    let labels: Vec<&str> = open.labels.iter().map(|one| one.as_written()).collect();
    assert_eq!(labels, ["fleet", "beads"]);
    let fields: Vec<(String, String)> =
        open.fields.iter().map(|one| (one.name.clone(), one.values[0].as_written().to_string())).collect();
    assert_eq!(fields, [("Status".into(), "open".into()), ("Priority".into(), "P1".into()), ("Type".into(), "task".into())]);
}

#[test]
fn an_issue_is_read_whole_and_one_bd_does_not_have_is_none() {
    let bd = Bd::saying(&[(0, SHOWN, ""), (1, "", "Error resolving armada-zzz: no issue found matching \"armada-zzz\"")]);
    let tracker = BeadsTracker::with_runner(&bd);

    let read = tracker.read_issue("/repo", "armada-3fz").expect("read").expect("there");
    assert_eq!(bd.calls()[0], ["-C", "/repo", "--readonly", "show", "armada-3fz", "--json"]);
    assert_eq!(read.body.as_written(), "All of it.\nTwo lines.");
    assert_eq!(read.assignees[0].as_written(), "mac-b/0a1b2c3d");
    assert_eq!(read.opened_at, "2026-10-24T03:23:58Z");
    assert!(!read.closed);

    assert_eq!(tracker.read_issue("/repo", "armada-zzz").expect("answered"), None);
}

#[test]
fn a_claim_names_its_actor_a_sync_never_adopts_a_remote_and_each_refusal_is_bds_own_words() {
    let bd = Bd::saying(&[
        (0, "✓ Updated issue: armada-3fz", ""),
        (1, "", "Error updating armada-3fz: issue already claimed by mac-a/7d2b2166"),
        (0, "", ""),
        (3, "", "sync: retries exhausted"),
    ]);
    let tracker = BeadsTracker::with_runner(&bd);

    tracker.claim("/repo", "armada-3fz", "mac-b/0a1b2c3d").expect("claimed");
    let held = tracker.claim("/repo", "armada-3fz", "mac-b/0a1b2c3d").expect_err("held");
    assert!(held.said.contains("already claimed by mac-a/7d2b2166"), "{}", held.said);
    tracker.sync("/repo").expect("synced");
    assert!(tracker.sync("/repo").expect_err("not synced").said.contains("retries exhausted"));

    let calls = bd.calls();
    assert_eq!(calls[0], ["-C", "/repo", "update", "armada-3fz", "--claim", "--actor", "mac-b/0a1b2c3d"]);
    assert_eq!(calls[2], ["-C", "/repo", "sync", "--no-adopt"]);
}

#[test]
fn a_child_is_filed_with_its_parent_and_answers_the_id_bd_printed() {
    let bd = Bd::saying(&[(0, "armada-3fz.1\n", ""), (1, "", "Error: parent armada-zzz not found"), (0, "", "")]);
    let tracker = BeadsTracker::with_runner(&bd);

    let made = tracker.create_child("/repo", "armada-3fz", "Store - add tables").expect("filed");
    assert_eq!(made, "armada-3fz.1");
    assert!(tracker.create_child("/repo", "armada-zzz", "x").expect_err("no parent").said.contains("not found"));
    let empty = tracker.create_child("/repo", "armada-3fz", "y").expect_err("printed nothing");
    assert!(empty.said.contains("no issue id"), "{}", empty.said);
    assert_eq!(bd.calls()[0], ["-C", "/repo", "create", "--title", "Store - add tables", "--parent", "armada-3fz", "--silent"]);
}

#[test]
fn a_state_is_an_update_and_a_close_is_forced_past_a_blocker_with_its_reason() {
    let bd = Bd::saying(&[]);
    let tracker = BeadsTracker::with_runner(&bd);

    tracker.put_state("/repo", "armada-3fz.1", BeadState::InProgress, "").expect("started");
    tracker.put_state("/repo", "armada-3fz.1", BeadState::Open, "").expect("reopened");
    tracker.put_state("/repo", "armada-3fz.1", BeadState::Closed, "done in the plan").expect("closed");
    tracker.retitle("/repo", "armada-3fz.1", "Store - add the tables").expect("renamed");
    tracker.block("/repo", "armada-3fz.2", "armada-3fz.1").expect("ordered");
    tracker.unblock("/repo", "armada-3fz.2", "armada-3fz.1").expect("unordered");

    let calls = bd.calls();
    assert_eq!(calls[0], ["-C", "/repo", "update", "armada-3fz.1", "--status", "in_progress"]);
    assert_eq!(calls[1], ["-C", "/repo", "update", "armada-3fz.1", "--status", "open"]);
    assert_eq!(calls[2], ["-C", "/repo", "close", "armada-3fz.1", "--reason", "done in the plan", "--force"]);
    assert_eq!(calls[3], ["-C", "/repo", "update", "armada-3fz.1", "--title", "Store - add the tables"]);
    assert_eq!(calls[4], ["-C", "/repo", "dep", "add", "armada-3fz.2", "armada-3fz.1"]);
    assert_eq!(calls[5], ["-C", "/repo", "dep", "remove", "armada-3fz.2", "armada-3fz.1"]);
}

#[test]
fn closing_a_tree_reads_the_children_at_every_depth_and_closes_the_open_ones_deepest_first() {
    let children_of_issue = r#"[{"id":"armada-3fz.1","status":"open"},{"id":"armada-3fz.2","status":"closed"}]"#;
    let children_of_first = r#"[{"id":"armada-3fz.1.1","status":"in_progress"}]"#;
    let bd = Bd::saying(&[
        (0, children_of_issue, ""),
        (0, children_of_first, ""),
        (0, "[]", ""),
        (0, "[]", ""),
        (0, r#"[{"id":"armada-3fz","status":"open"}]"#, ""),
        (0, "", ""),
    ]);
    let tracker = BeadsTracker::with_runner(&bd);

    tracker.close_tree("/repo", "armada-3fz", "merged in https://forge.invalid/pull/5").expect("closed");

    let calls = bd.calls();
    assert_eq!(calls[0], ["-C", "/repo", "--readonly", "children", "armada-3fz", "--json"]);
    assert_eq!(calls[1], ["-C", "/repo", "--readonly", "children", "armada-3fz.1", "--json"]);
    assert_eq!(
        calls.last().expect("a close"),
        &["-C", "/repo", "close", "armada-3fz.1.1", "armada-3fz.1", "armada-3fz", "--reason", "merged in https://forge.invalid/pull/5", "--force"]
    );
    assert_eq!(calls.iter().filter(|one| one.contains(&"close".to_string())).count(), 1, "one close for all of them");
}

#[test]
fn a_tree_already_closed_is_left_as_it_was() {
    let bd = Bd::saying(&[(0, "[]", ""), (0, r#"[{"id":"armada-3fz","status":"closed"}]"#, "")]);
    let tracker = BeadsTracker::with_runner(&bd);

    tracker.close_tree("/repo", "armada-3fz", "merged").expect("nothing to close");
    assert!(bd.calls().iter().all(|one| !one.contains(&"close".to_string())), "{:?}", bd.calls());
}

#[test]
fn the_mark_moves_when_a_stores_root_moves_and_not_when_its_files_are_only_touched() {
    let (main, slot) = checkouts("mark");
    let tracker = BeadsTracker::over();
    let in_slot = slot.display().to_string();
    assert_eq!(tracker.mark(&in_slot), None, "no database yet");

    let noms = main.join(".beads/embeddeddolt/armada/.dolt/noms");
    std::fs::create_dir_all(&noms).expect("made");
    std::fs::create_dir_all(main.join(".beads/backup")).expect("made");
    let manifest = |root: &str| format!("5:__DOLT__:lock:{root}:00000000000000000000000000000000:vvvv:3419");
    std::fs::write(noms.join("manifest"), manifest("first")).expect("written");
    let first = tracker.mark(&in_slot).expect("a mark");

    std::fs::write(noms.join("manifest"), manifest("first")).expect("written again, as a read leaves it");
    std::fs::write(main.join(".beads/backup/manifest"), "a backup is not the copy").expect("written");
    assert_eq!(tracker.mark(&in_slot), Some(first.clone()));

    std::fs::write(noms.join("manifest"), manifest("second")).expect("written");
    assert_ne!(tracker.mark(&in_slot), Some(first));
    assert_eq!(mark_of(Path::new("/nowhere/.beads")), None);
}
