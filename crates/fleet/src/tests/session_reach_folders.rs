//! The owner's folder list, read by the guard before it draws a card: a blocked folder is refused
//! with no card, an allowed one runs, and anything else is put to him with the path it named.
//! `#2441`.

use ipc::{AskingToRun, ReachesTo};

use crate::helm::{guarded_in_a_session, Because, Folders, Guarded, Own};

const SLOT: &str = "/repo/.armada/slots/slot-3";
const HOME: &str = "/Users/user";

fn call(tool: &str, input: &str) -> AskingToRun {
    let body = format!(r#"{{"tool_name":"{tool}","input":{input}}}"#);
    ipc::decode("an ask", body.as_bytes()).expect("an ask")
}

fn listed(call: &AskingToRun, allow: &[&str], block: &[&str]) -> Option<Guarded> {
    listed_with(call, allow, block, &[])
}

/// [`listed`], and the folders this session alone was allowed.
fn listed_with(call: &AskingToRun, allow: &[&str], block: &[&str], session: &[&str]) -> Option<Guarded> {
    let owned = |list: &[&str]| -> Vec<String> { list.iter().map(|one| one.to_string()).collect() };
    let (allow, block, session) = (owned(allow), owned(block), owned(session));
    let folders = Folders { allow: &allow, block: &block, session: &session, home: HOME };
    guarded_in_a_session(call, &[SLOT], Own::default(), folders)
}

fn read(path: &str) -> AskingToRun {
    call("Read", &format!(r#"{{"file_path":"{path}"}}"#))
}

fn bash(line: &str) -> AskingToRun {
    call("Bash", &format!(r#"{{"command":"{line}"}}"#))
}

fn asks(because: Because, at: &str) -> Option<Guarded> {
    asks_to(because, at, ReachesTo::Read)
}

fn asks_to(because: Because, at: &str, to: ReachesTo) -> Option<Guarded> {
    Some(Guarded::Because { because, reaching: Some(at.to_string()), to: Some(to) })
}

#[test]
fn a_guarded_read_is_put_to_the_owner_with_the_path_it_named() {
    assert_eq!(listed(&read("~/Documents/x.md"), &[], &[]), asks(Because::ReachesOutside, "/Users/user/Documents/x.md"));
    assert_eq!(listed(&bash("cat ~/Documents/a/../b.md"), &[], &[]), asks(Because::ReachesOutside, "/Users/user/Documents/b.md"));
    assert_eq!(listed(&bash("ls $HOME/Desktop && cat ~/.ssh/config"), &[], &[]), asks(Because::ReachesOutside, "/Users/user/Desktop"));
}

#[test]
fn an_allowed_folder_runs_and_a_blocked_one_is_refused_with_no_card() {
    let documents = "/Users/user/Documents";
    assert_eq!(listed(&read("~/Documents/x.md"), &[documents], &[]), None);
    assert_eq!(listed(&bash("grep -rn todo ~/Documents/notes"), &[documents], &[]), None);
    assert_eq!(
        listed(&read("/Users/user/Documents/x.md"), &[], &[documents]),
        Some(Guarded::Blocked { folder: documents.to_string() })
    );
    // Another folder is still asked about.
    assert_eq!(listed(&read("~/Desktop/x.md"), &[documents], &[]), asks(Because::ReachesOutside, "/Users/user/Desktop/x.md"));
}

#[test]
fn the_deeper_entry_wins_and_a_block_wins_a_tie() {
    let (documents, private) = ("/Users/user/Documents", "/Users/user/Documents/private");
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
    assert_eq!(listed(&read("~/Desktop/a.md"), &["/Users/user/Desk"], &[]), asks(Because::ReachesOutside, "/Users/user/Desktop/a.md"));
}

#[test]
fn a_write_outside_the_slots_is_put_to_the_owner_and_the_list_lets_it_in() {
    let write = |path: &str| call("Write", &format!(r#"{{"file_path":"{path}","content":"x"}}"#));
    // The write tools take an absolute path.
    assert_eq!(listed(&write("/Users/user/notes-2441.md"), &[], &[]), asks_to(Because::ReachesOutside, "/Users/user/notes-2441.md", ReachesTo::Write));
    assert_eq!(listed(&write("/opt/armada-2441/new.md"), &[], &[]), asks_to(Because::ReachesOutside, "/opt/armada-2441/new.md", ReachesTo::Write));
    assert_eq!(listed(&write("/Users/user/notes-2441.md"), &[HOME], &[]), None);
    assert_eq!(listed(&bash("echo x > ~/notes.md"), &[], &[]), asks_to(Because::ReachesOutside, "/Users/user/notes.md", ReachesTo::Write));
    assert_eq!(listed(&bash("echo x > ~/notes.md"), &[HOME], &[]), None);
    // Inside the slot and scratch, nothing is asked.
    assert_eq!(listed(&write(&format!("{SLOT}/a.md")), &[], &[]), None);
    assert_eq!(listed(&write("/tmp/a.md"), &[], &[]), None);
}

#[test]
fn an_allowed_folder_never_lets_a_removal_or_a_refusal_with_no_folder_run() {
    let everything = ["/Users/user", "/repo"];
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
            Some(Guarded::Because { because: Because::Destructive, reaching: None, to: None }),
            "{line}"
        );
    }
}

#[test]
fn a_card_says_whether_the_call_reads_or_writes() {
    let edit = call("Edit", r#"{"file_path":"/Users/user/Documents/x.md","old_string":"a","new_string":"b"}"#);
    let documents = "/Users/user/Documents/x.md";
    assert_eq!(listed(&edit, &[], &[]), asks_to(Because::ReachesOutside, documents, ReachesTo::Write));
    assert_eq!(listed(&bash("cat ~/Documents/x.md"), &[], &[]), asks_to(Because::ReachesOutside, documents, ReachesTo::Read));
    assert_eq!(
        listed(&bash("echo x >> ~/Documents/x.md"), &[], &[]),
        asks_to(Because::ReachesOutside, documents, ReachesTo::Write)
    );
}

#[test]
fn a_script_it_runs_is_read_and_not_written() {
    // The owner's own line: a script named by a variable, run, with its output kept in scratch.
    let qa = r#"SHAS=\"57509e29127 $(git log --reverse --format=%h origin/master..origin/nm/spike-custom-margins-terms-service-inputs | tr '\\n' ' ')\"; echo \"$SHAS\" > /tmp/qa9-shas.txt; echo $SHAS; S=/Users/user/develop/fleet-marketplace/plugins/fleet/skills/deploy-qa/scripts/merge-to-qa.sh; $S --cherry-pick 9 nm/spike-custom-margins-terms-service-inputs --dry-run -- ${=SHAS} 2>&1 | tail -25"#;
    assert_eq!(listed(&bash(qa), &[], &[]), None);
    assert_eq!(listed(&bash("bash ~/bin/x.sh"), &[], &[]), None);
    assert_eq!(listed(&bash(". ~/bin/env.sh && source ~/.zshrc"), &[], &[]), None);
    // A script in a guarded folder is still a read put to the owner.
    assert_eq!(listed(&bash("~/Desktop/run.sh --go"), &[], &[]), asks(Because::ReachesOutside, "/Users/user/Desktop/run.sh"));
    assert_eq!(listed(&bash("zsh -x ~/Desktop/run.sh"), &[], &[]), asks(Because::ReachesOutside, "/Users/user/Desktop/run.sh"));
    // What a program is handed is still a write.
    let write = |at: &str| asks_to(Because::ReachesOutside, at, ReachesTo::Write);
    assert_eq!(listed(&bash("cp ~/notes.md ~/elsewhere.md"), &[], &[]), write("/Users/user/notes.md"));
    assert_eq!(listed(&bash("S=~/bin/x.sh; $S ~/out.txt"), &[], &[]), write("/Users/user/out.txt"));
}

#[test]
fn a_folder_allowed_for_the_session_runs_and_the_owners_block_still_wins() {
    let documents = "/Users/user/Documents";
    assert_eq!(listed_with(&read("~/Documents/x.md"), &[], &[], &[documents]), None);
    assert_eq!(listed_with(&bash("echo x > ~/Documents/x.md"), &[], &[], &[documents]), None);
    // Its own list holds only what it was allowed.
    assert_eq!(
        listed_with(&read("~/Desktop/x.md"), &[], &[], &[documents]),
        asks(Because::ReachesOutside, "/Users/user/Desktop/x.md")
    );
    assert_eq!(
        listed_with(&read("~/Documents/private/x.md"), &[], &["/Users/user/Documents/private"], &[documents]),
        Some(Guarded::Blocked { folder: "/Users/user/Documents/private".to_string() })
    );
    assert_eq!(
        listed_with(&read("~/Documents/x.md"), &[], &[HOME], &[documents]),
        Some(Guarded::Blocked { folder: HOME.to_string() })
    );
    // Never a removal.
    assert_eq!(
        listed_with(&bash("rm -rf ~/Documents/x"), &[], &[], &[documents]),
        Some(Guarded::Because { because: Because::Destructive, reaching: None, to: None })
    );
}

#[test]
fn a_write_through_a_search_path_names_its_first_folder() {
    let asked = listed(&bash(r#"cp shim \"$HOME/.rbenv/shims:$PATH\""#), &[], &[]);
    assert_eq!(asked, asks_to(Because::ReachesOutside, "/Users/user/.rbenv/shims", ReachesTo::Write));
}

#[test]
fn setting_a_variable_through_home_writes_nothing_and_runs() {
    let commit = r#"source ~/.nvm/nvm.sh >/dev/null && export PATH=\"$HOME/.rbenv/shims:$PATH\" && git commit -q -m \"x\" > /tmp/commit.log 2>&1"#;
    assert_eq!(listed(&bash(commit), &[], &[]), None);
    assert_eq!(listed(&bash("BUNDLE_PATH=~/.gems git status --short"), &[], &[]), None);
    // The value is still read where the line uses it.
    assert_eq!(
        listed(&bash("F=~/Documents/x.md; cat $F"), &[], &[]),
        asks(Because::ReachesOutside, "/Users/user/Documents/x.md")
    );
}
