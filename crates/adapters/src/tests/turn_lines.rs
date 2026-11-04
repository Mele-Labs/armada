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
    let done = read_turns(&file, 0, true, u64::MAX, None).unwrap();
    let ids: Vec<&str> = done.turns.iter().map(|one| one.message_id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(done.turns[0].output_tokens, 9);
    assert_eq!(done.turns[0].tools, ["Bash"]);
    assert_eq!(done.turns[0].bash_use, Some(ReadSearch));
    assert_eq!(done.turns[1].tools, ["Read", "Bash"]);
    assert_eq!(done.turns[1].bash_use, None);
    assert!(done.turns.iter().all(|one| one.subagent));
    assert_eq!(done.next as usize, text.rfind('\n').unwrap() + 1, "stops after the last whole line");
    assert!(read_turns(&file, done.next, false, u64::MAX, None).unwrap().turns.is_empty());
    assert_eq!(read_turns(&file, 1 << 40, false, u64::MAX, None).unwrap().turns.len(), 2, "past the end starts again");
    std::fs::remove_dir_all(&dir).ok();
}

fn stamped(at: &str, line: &str) -> String {
    line.replacen("2026-10-17T09:00:00.000Z", at, 1)
}

fn result(at: &str, call: &str) -> String {
    format!(
        r#"{{"type":"user","timestamp":"{at}","message":{{"content":[{{"type":"tool_result","tool_use_id":"{call}","content":"an assistant said usage"}}]}}}}"#
    )
}

#[test]
fn a_turn_keeps_when_its_model_began_and_its_tools_ran_and_a_late_result_is_returned_alone() {
    let dir = std::env::temp_dir().join(format!("turn_times_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.jsonl");
    let call = |id: &str| format!(r#"{{"type":"tool_use","id":"{id}","name":"Bash","input":{{"command":"ls"}}}}"#);
    let first = [
        r#"{"type":"user","timestamp":"2026-10-17T09:00:01.000Z","message":{"content":"hi"}}"#.to_string(),
        stamped("2026-10-17T09:00:05.000Z", &line("a", 9, &format!("{},{}", call("t1"), call("t2")))),
        result("2026-10-17T09:00:07.000Z", "t1"),
        result("2026-10-17T09:00:09.000Z", "t2"),
        stamped("2026-10-17T09:00:12.000Z", &line("b", 4, &call("t3"))),
        String::new(),
    ]
    .join("\n");
    std::fs::write(&file, &first).unwrap();
    let one = read_turns(&file, 0, false, u64::MAX, None).unwrap();
    let a = &one.times[0];
    assert_eq!(a.model_began.as_deref(), Some("2026-10-17T09:00:01.000Z"));
    assert_eq!(a.tools_began.as_deref(), Some("2026-10-17T09:00:05.000Z"));
    assert_eq!(a.tools_ended.as_deref(), Some("2026-10-17T09:00:09.000Z"), "the last result, not the first");
    let b = &one.times[1];
    assert_eq!(b.model_began.as_deref(), Some("2026-10-17T09:00:09.000Z"), "the result before it");
    assert_eq!(b.tools_ended, None);
    assert_eq!(one.open.len(), 1);
    assert_eq!((one.open[0].tool_use_id.as_str(), one.open[0].message_id.as_str()), ("t3", "b"));
    assert_eq!(one.input_at.as_deref(), Some("2026-10-17T09:00:09.000Z"));

    let more = result("2026-10-17T09:00:20.000Z", "t3");
    let mut text = first.clone();
    text.push_str(&more);
    text.push('\n');
    std::fs::write(&file, &text).unwrap();
    let later = read_turns(&file, one.next, false, u64::MAX, one.input_at.clone()).unwrap();
    assert!(later.turns.is_empty());
    assert_eq!(later.late.len(), 1);
    assert_eq!((later.late[0].tool_use_id.as_str(), later.late[0].at.as_str()), ("t3", "2026-10-17T09:00:20.000Z"));
    std::fs::remove_dir_all(&dir).ok();
}
