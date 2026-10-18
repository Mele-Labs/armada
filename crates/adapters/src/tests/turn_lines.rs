//! A transcript read for what each turn cost. **No process starts**: the lines are written by the test.
//! The classifier is a table, because the table is its specification.

use ipc::BashUse::{self, *};

use crate::turn_lines::{classify_bash, read_turns};

#[test]
fn a_command_is_labelled_by_the_first_real_command_in_it() {
    let table: &[(&str, BashUse)] = &[
        ("grep -rn foo src", ReadSearch),
        ("cd /a/b && rg foo", ReadSearch),
        ("cd /a && cd b && cat x | head", ReadSearch),
        ("cd /a; ls -la", ReadSearch),
        ("/usr/bin/find . -name x", ReadSearch),
        ("sed -n 1,5p f", ReadSearch),
        ("jq . f.json", ReadSearch),
        ("git grep foo", ReadSearch),
        ("git show HEAD~1", ReadSearch),
        ("git -C /x log --oneline", ReadSearch),
        ("git diff main", ReadSearch),
        ("git ls-files", ReadSearch),
        ("git status", Git),
        ("git -c a=b commit -m x", Git),
        ("git", Git),
        ("python3 run.py", Scripts),
        ("python -c 'x'", Scripts),
        ("node build.js", Scripts),
        ("FOO=1 python3 x.py", Scripts),
        ("pnpm test", BuildTest),
        ("npx vitest", BuildTest),
        ("cargo check -p x", BuildTest),
        ("taskpolicy -b cargo build", BuildTest),
        ("time make", BuildTest),
        ("timeout 5 sleep 9", BuildTest),
        ("armada check test a b", BuildTest),
        ("armada --json check test", BuildTest),
        ("armada sessions list", LoopsWaits),
        ("armada", LoopsWaits),
        ("for f in *; do echo $f; done", LoopsWaits),
        ("while true; do sleep 1; done", LoopsWaits),
        ("until x; do y; done", LoopsWaits),
        ("sleep 30", LoopsWaits),
        ("curl -s http://x", LoopsWaits),
        ("gh pr view 12", Forge),
        ("cd x && gh api /y", Forge),
        ("mkdir -p x", Other),
        ("echo hi", Other),
        ("cd x", Other),
        ("", Other),
    ];
    for (command, label) in table {
        assert_eq!(classify_bash(command), *label, "{command}");
    }
}

fn line(id: &str, output: u64, blocks: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-10-17T09:00:00.000Z","message":{{"id":"{id}","model":"m","usage":{{"input_tokens":1,"cache_creation_input_tokens":2,"cache_read_input_tokens":3,"output_tokens":{output}}},"content":[{blocks}]}}}}"#
    )
}

#[test]
fn the_lines_of_a_message_join_and_a_line_still_being_written_waits() {
    let dir = std::env::temp_dir().join(format!("turn_lines_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.jsonl");
    let bash = r#"{"type":"tool_use","name":"Bash","input":{"command":"ls"}}"#;
    let read = r#"{"type":"tool_use","name":"Read","input":{"file_path":"/x"}}"#;
    let text = format!(
        "{}\n{}\n{}\n{}\n{}",
        r#"{"type":"user","message":{"content":"hi"}}"#,
        line("a", 5, r#"{"type":"thinking","thinking":""}"#),
        line("a", 9, bash),
        line("b", 4, &format!("{read},{bash}")),
        &line("c", 1, "")[..40],
    );
    std::fs::write(&file, &text).unwrap();
    let done = read_turns(&file, 0, true, u64::MAX).unwrap();
    let ids: Vec<&str> = done.turns.iter().map(|one| one.message_id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(done.turns[0].output_tokens, 9);
    assert_eq!(done.turns[0].tools, ["Bash"]);
    assert_eq!(done.turns[0].bash_use, Some(ReadSearch));
    assert_eq!(done.turns[1].tools, ["Read", "Bash"]);
    assert_eq!(done.turns[1].bash_use, None);
    assert!(done.turns.iter().all(|one| one.subagent));
    assert_eq!(done.next as usize, text.rfind('\n').unwrap() + 1, "stops after the last whole line");
    assert!(read_turns(&file, done.next, false, u64::MAX).unwrap().turns.is_empty());
    assert_eq!(read_turns(&file, 1 << 40, false, u64::MAX).unwrap().turns.len(), 2, "past the end starts again");
    std::fs::remove_dir_all(&dir).ok();
}
