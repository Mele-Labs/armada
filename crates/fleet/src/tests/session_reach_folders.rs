//! The owner's folder list, read by the guard before it draws a card: a blocked folder is refused
//! with no card, an allowed one runs, and anything else is put to him with the path it named.
//! `#2441`.

use ipc::{AskingToRun, CallKind, ReachesTo};

use crate::helm::{guarded_in_a_session, Because, Folders, Guarded, KindAllowed, Own};

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
    let folders = Folders { allow: &allow, block: &block, session: &session, home: HOME, ..Folders::default() };
    without_kind(guarded_in_a_session(call, &[SLOT], Own::default(), folders))
}

/// The guard's answer with the segment's kind left out, which the kind tests below read alone.
fn without_kind(guarded: Option<Guarded>) -> Option<Guarded> {
    guarded.map(|guarded| match guarded {
        Guarded::Because { because, reaching, to, .. } => Guarded::Because { because, reaching, to, kind: None },
        blocked => blocked,
    })
}

/// The kind of call a Bash line's card names, and the path it names.
fn kind_of(line: &str) -> (Option<CallKind>, Option<String>) {
    match guarded_in_a_session(&bash(line), &[SLOT], Own::default(), Folders { home: HOME, ..Folders::default() }) {
        Some(Guarded::Because { kind, reaching, .. }) => (kind, reaching),
        other => panic!("{line}: {other:?}"),
    }
}

fn kind(program: &str, script: Option<&str>) -> Option<CallKind> {
    Some(CallKind { program: program.to_string(), script: script.map(str::to_string) })
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
    Some(Guarded::Because { because, reaching: Some(at.to_string()), to: Some(to), kind: None })
}

#[test]
fn a_cards_kind_is_its_program_and_the_script_an_interpreter_runs() {
    let (node, at) = kind_of("node drive.mjs --out ~/Desktop/x");
    assert_eq!((node, at.as_deref()), (kind("node", Some("drive.mjs")), Some("/Users/user/Desktop/x")));
    assert_eq!(kind_of("python3 -u tools/x.py ~/y").0, kind("python3", Some("x.py")));
    assert_eq!(kind_of("cp ~/a ~/b").0, kind("cp", None));
    assert_eq!(kind_of("deno run --allow-all tools/x.ts ~/y").0, kind("deno", Some("x.ts")));
    // The kind is the segment's that named the path, not the line's first.
    assert_eq!(kind_of("cd /tmp && node drive.mjs ~/Desktop/x").0, kind("node", Some("drive.mjs")));
    // A Write names no program.
    let write = call("Write", r#"{"file_path":"/Users/user/notes.md","content":"x"}"#);
    let guarded = guarded_in_a_session(&write, &[SLOT], Own::default(), Folders { home: HOME, ..Folders::default() });
    assert!(matches!(guarded, Some(Guarded::Because { kind: None, .. })), "{guarded:?}");
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
            Some(Guarded::Because { because: Because::Destructive, reaching: None, to: None, kind: None }),
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

/// A home on disk holding one program, `~/bin/headless-shell`, for the guard to find there.
fn home_with_a_program(name: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let home = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(home.join("bin")).expect("a home");
    let program = home.join("bin/headless-shell");
    std::fs::write(&program, "#!/bin/sh\n").expect("a program");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("runnable");
    home.to_string_lossy().into_owned()
}

fn listed_at(call: &AskingToRun, home: &str) -> Option<Guarded> {
    without_kind(guarded_in_a_session(call, &[SLOT], Own::default(), Folders { home, ..Folders::default() }))
}

#[test]
fn a_program_handed_to_a_script_is_run_and_not_written() {
    // The walk driver, handed the browser it launches (26 Oct 2026).
    let home = home_with_a_program("program-handed");
    let drive = r#"cd /tmp/walkdrive; node drive.mjs \"http://127.0.0.1:5192/?walk=x&autoplay\" ~/bin/headless-shell /tmp/walkdrive 2>&1 | tail -8 | cut -c1-400"#;
    assert_eq!(listed_at(&bash(drive), &home), None);
}

#[test]
fn a_program_copied_over_or_redirected_into_is_still_written() {
    let home = home_with_a_program("program-written");
    let program = format!("{home}/bin/headless-shell");
    let write = asks_to(Because::ReachesOutside, &program, ReachesTo::Write);
    assert_eq!(listed_at(&bash("cp /tmp/other ~/bin/headless-shell"), &home), write);
    assert_eq!(listed_at(&bash("tee ~/bin/headless-shell < /tmp/other"), &home), write);
    assert_eq!(listed_at(&bash("node x.mjs >> ~/bin/headless-shell"), &home), write);
    assert_eq!(listed_at(&bash("node x.mjs > ~/bin/headless-shell"), &home), write);
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
        Some(Guarded::Because { because: Because::Destructive, reaching: None, to: None, kind: None })
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

/// The guard with these kinds allowed for the session, and the owner's blocked folders.
fn allowing(line: &str, kinds: &[KindAllowed], block: &[&str]) -> Option<Guarded> {
    let block: Vec<String> = block.iter().map(|one| one.to_string()).collect();
    let folders = Folders { block: &block, kinds, home: HOME, ..Folders::default() };
    without_kind(guarded_in_a_session(&bash(line), &[SLOT], Own::default(), folders))
}

fn allowed(program: &str, script: Option<&str>, folder: Option<&str>) -> KindAllowed {
    KindAllowed { kind: kind(program, script).expect("a kind"), folder: folder.map(str::to_string) }
}

#[test]
fn a_kind_allowed_for_the_session_lets_its_reach_run() {
    let script = [allowed("node", Some("drive.mjs"), None)];
    assert_eq!(allowing("node drive.mjs --browser ~/Library/x", &script, &[]), None);
    assert_eq!(
        allowing("node cleanup.mjs ~/code/other-repo", &script, &[]),
        asks_to(Because::ReachesOutside, "/Users/user/code/other-repo", ReachesTo::Write)
    );
    let program = [allowed("node", None, None)];
    assert_eq!(allowing("node cleanup.mjs ~/code/other-repo", &program, &[]), None);
    let desktop = [allowed("node", Some("drive.mjs"), Some("/Users/user/Desktop"))];
    assert_eq!(allowing("node drive.mjs --out ~/Desktop/shot.png", &desktop, &[]), None);
    assert_eq!(
        allowing("node drive.mjs --out ~/code/shot.png", &desktop, &[]),
        asks_to(Because::ReachesOutside, "/Users/user/code/shot.png", ReachesTo::Write)
    );
}

#[test]
fn a_kind_allowed_never_lets_a_block_a_removal_a_push_or_another_kind_through() {
    let script = [allowed("node", Some("drive.mjs"), None)];
    let desktop = "/Users/user/Desktop";
    assert_eq!(
        allowing("node drive.mjs --out ~/Desktop/shot.png", &script, &[desktop]),
        Some(Guarded::Blocked { folder: desktop.to_string() })
    );
    assert_eq!(
        allowing("node drive.mjs ~/x && rm ~/x", &script, &[]),
        Some(Guarded::Because { because: Because::Destructive, reaching: None, to: None, kind: None })
    );
    assert_eq!(
        allowing("node drive.mjs ~/x && git push", &script, &[]),
        Some(Guarded::Because { because: Because::PushesToShared, reaching: None, to: None, kind: None })
    );
    assert_eq!(
        allowing("node drive.mjs ~/x && cp ~/a ~/b", &script, &[]),
        asks_to(Because::ReachesOutside, "/Users/user/a", ReachesTo::Write)
    );
}
