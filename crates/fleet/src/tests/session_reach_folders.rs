//! The owner's folder list, read by the guard before it draws a card: a blocked folder is refused
//! with no card, an allowed one runs, and anything else is put to him with the path it named.
//! `#2441`.

use ipc::AskingToRun;

use crate::helm::{guarded_in_a_session, Because, Folders, Guarded, Own};

const SLOT: &str = "/repo/.armada/slots/slot-3";
const HOME: &str = "/Users/someone";

fn call(tool: &str, input: &str) -> AskingToRun {
    let body = format!(r#"{{"tool_name":"{tool}","input":{input}}}"#);
    ipc::decode("an ask", body.as_bytes()).expect("an ask")
}

fn listed(call: &AskingToRun, allow: &[&str], block: &[&str]) -> Option<Guarded> {
    let allow: Vec<String> = allow.iter().map(|one| one.to_string()).collect();
    let block: Vec<String> = block.iter().map(|one| one.to_string()).collect();
    let folders = Folders { allow: &allow, block: &block, home: HOME };
    guarded_in_a_session(call, &[SLOT], Own::default(), folders)
}

fn read(path: &str) -> AskingToRun {
    call("Read", &format!(r#"{{"file_path":"{path}"}}"#))
}

fn bash(line: &str) -> AskingToRun {
    call("Bash", &format!(r#"{{"command":"{line}"}}"#))
}

fn asks(because: Because, at: &str) -> Option<Guarded> {
    Some(Guarded::Because { because, reaching: Some(at.to_string()) })
}

#[test]
fn a_guarded_read_is_put_to_the_owner_with_the_path_it_named() {
    assert_eq!(listed(&read("~/Documents/x.md"), &[], &[]), asks(Because::ReachesOutside, "/Users/someone/Documents/x.md"));
    assert_eq!(listed(&bash("cat ~/Documents/a/../b.md"), &[], &[]), asks(Because::ReachesOutside, "/Users/someone/Documents/b.md"));
    assert_eq!(listed(&bash("ls $HOME/Desktop && cat ~/.ssh/config"), &[], &[]), asks(Because::ReachesOutside, "/Users/someone/Desktop"));
}

#[test]
fn an_allowed_folder_runs_and_a_blocked_one_is_refused_with_no_card() {
    let documents = "/Users/someone/Documents";
    assert_eq!(listed(&read("~/Documents/x.md"), &[documents], &[]), None);
    assert_eq!(listed(&bash("grep -rn todo ~/Documents/notes"), &[documents], &[]), None);
    assert_eq!(
        listed(&read("/Users/someone/Documents/x.md"), &[], &[documents]),
        Some(Guarded::Blocked { folder: documents.to_string() })
    );
    // Another folder is still asked about.
    assert_eq!(listed(&read("~/Desktop/x.md"), &[documents], &[]), asks(Because::ReachesOutside, "/Users/someone/Desktop/x.md"));
}

#[test]
fn the_deeper_entry_wins_and_a_block_wins_a_tie() {
    let (documents, private) = ("/Users/someone/Documents", "/Users/someone/Documents/private");
    assert_eq!(
        listed(&read("~/Documents/private/a.md"), &[documents], &[private]),
        Some(Guarded::Blocked { folder: private.to_string() })
    );
    assert_eq!(listed(&read("~/Documents/a.md"), &[documents], &[private]), None);
    assert_eq!(listed(&read("~/Documents/private/a.md"), &[private], &[documents]), None);
    assert_eq!(
        listed(&read("~/Documents/a.md"), &[documents], &[documents]),
        Some(Guarded::Blocked { folder: documents.to_string() })
    );
    // A folder is matched a part at a time: `Desk` does not hold `Desktop`.
    assert_eq!(listed(&read("~/Desktop/a.md"), &["/Users/someone/Desk"], &[]), asks(Because::ReachesOutside, "/Users/someone/Desktop/a.md"));
}

#[test]
fn a_write_outside_the_slots_is_put_to_the_owner_and_the_list_lets_it_in() {
    let write = |path: &str| call("Write", &format!(r#"{{"file_path":"{path}","content":"x"}}"#));
    // The write tools take an absolute path.
    assert_eq!(listed(&write("/Users/someone/notes-2441.md"), &[], &[]), asks(Because::ReachesOutside, "/Users/someone/notes-2441.md"));
    assert_eq!(listed(&write("/opt/armada-2441/new.md"), &[], &[]), asks(Because::ReachesOutside, "/opt/armada-2441/new.md"));
    assert_eq!(listed(&write("/Users/someone/notes-2441.md"), &[HOME], &[]), None);
    assert_eq!(listed(&bash("echo x > ~/notes.md"), &[], &[]), asks(Because::ReachesOutside, "/Users/someone/notes.md"));
    assert_eq!(listed(&bash("echo x > ~/notes.md"), &[HOME], &[]), None);
    // Inside the slot and scratch, nothing is asked.
    assert_eq!(listed(&write(&format!("{SLOT}/a.md")), &[], &[]), None);
    assert_eq!(listed(&write("/tmp/a.md"), &[], &[]), None);
}

#[test]
fn an_allowed_folder_never_lets_a_removal_or_a_refusal_with_no_folder_run() {
    let everything = ["/Users/someone", "/repo"];
    for line in [
        "rm -rf ~/Documents/x",
        "mv ~/Documents/a ~/Documents/b",
        "chmod -R 777 ~/.ssh",
        "rm -rf /repo/crates",
        "sudo ls",
        "kill -9 -1",
    ] {
        assert_eq!(
            listed(&bash(line), &everything, &[]),
            Some(Guarded::Because { because: Because::Destructive, reaching: None }),
            "{line}"
        );
    }
}
