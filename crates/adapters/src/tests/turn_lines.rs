//! A transcript read for what each turn cost. **No process starts**: the lines are written by the test.
//! The classifier is a table, because the table is its specification.

use ipc::BashUse::{self, *};

use crate::turn_lines::{classify_bash, read_turns, BackgroundTurn, Carry, Part, transcripts_of, transcripts_of_each};

/// Every live session's transcripts in one walk are the ones a walk per session finds: each
/// session's own and its subagents', from whichever project folder holds them, and no other's.
#[test]
fn one_walk_finds_each_live_sessions_transcripts_and_no_others() {
    let home = std::env::temp_dir().join(format!("armada-turn-walk-{}", std::process::id()));
    let projects = home.join(".claude/projects");
    for (folder, file) in [("a", "s1.jsonl"), ("b", "s2.jsonl"), ("b", "s2/subagents/agent-x.jsonl"), ("c", "s3.jsonl")] {
        let path = projects.join(folder).join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{}\n").unwrap();
    }
    let home = home.to_str().unwrap();

    let walked = transcripts_of_each(home, &["s1", "s2"]);
    let mut names: Vec<(String, bool)> = walked.iter().map(|file| (file.session_id.clone(), file.subagent)).collect();
    names.sort();
    assert_eq!(names, [("s1".into(), false), ("s2".into(), false), ("s2".into(), true)]);
    let mut one_by_one = [transcripts_of(home, "s1"), transcripts_of(home, "s2")].concat();
    one_by_one.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(walked, one_by_one);
    let _ = std::fs::remove_dir_all(std::path::Path::new(home));
}

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
    let done = read_turns(&file, 0, true, u64::MAX, None, Carry::default()).unwrap();
    let ids: Vec<&str> = done.turns.iter().map(|one| one.message_id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(done.turns[0].output_tokens, 9);
    assert_eq!(done.turns[0].tools, ["Bash"]);
    assert_eq!(done.turns[0].bash_use, Some(ReadSearch));
    assert_eq!(done.turns[1].tools, ["Read", "Bash"]);
    assert_eq!(done.turns[1].bash_use, None);
    assert!(done.turns.iter().all(|one| one.subagent));
    assert_eq!(done.next as usize, text.rfind('\n').unwrap() + 1, "stops after the last whole line");
    assert!(read_turns(&file, done.next, false, u64::MAX, None, Carry::default()).unwrap().turns.is_empty());
    assert_eq!(read_turns(&file, 1 << 40, false, u64::MAX, None, Carry::default()).unwrap().turns.len(), 2, "past the end starts again");
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
    let one = read_turns(&file, 0, false, u64::MAX, None, Carry::default()).unwrap();
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
    let later = read_turns(&file, one.next, false, u64::MAX, one.input_at.clone(), one.carry.clone()).unwrap();
    assert!(later.turns.is_empty());
    assert_eq!(later.late.len(), 1);
    assert_eq!((later.late[0].tool_use_id.as_str(), later.late[0].at.as_str()), ("t3", "2026-10-17T09:00:20.000Z"));
    std::fs::remove_dir_all(&dir).ok();
}

fn asked(id: &str, at: &str, cache_write: u64, cache_read: u64, hour: u64, blocks: &str) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"{at}","message":{{"id":"{id}","model":"m","usage":{{"input_tokens":0,"cache_creation_input_tokens":{cache_write},"cache_read_input_tokens":{cache_read},"output_tokens":5,"cache_creation":{{"ephemeral_1h_input_tokens":{hour},"ephemeral_5m_input_tokens":0}}}},"content":[{blocks}]}}}}"#
    )
}

fn told(at: &str, text: &str, flags: &str) -> String {
    format!(r#"{{"type":"user","timestamp":"{at}",{flags}"message":{{"content":"{text}"}}}}"#)
}

fn shares(of: &[(Part, f64)]) -> Vec<(Part, u64)> {
    of.iter().map(|(part, share)| (*part, (share * 10_000.0).round() as u64)).collect()
}

/// Each turn's input is shared by what its context held: the cache read by what the request before
/// held, the writes by what was added since, and the writes of a turn that found the cache cold as
/// that. The opening text is what the first request held beyond what the transcript shows. A
/// compaction starts the estimate again, and its own request comes back as background. Read in two
/// pieces, the carry gives the same shares as one read.
#[test]
fn a_turns_input_is_shared_by_what_its_context_carried_and_a_compaction_is_background() {
    use Part::*;
    let dir = std::env::temp_dir().join(format!("turn_spent_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.jsonl");
    let read = format!(r#"{{"type":"tool_use","id":"t1","name":"Read","input":{{"file_path":"{}"}}}}"#, "f".repeat(36));
    let lines = [
        told("2026-10-17T09:00:00.000Z", &"x".repeat(400), ""),
        format!(r#"{{"type":"attachment","attachment":{{"type":"skill_listing","content":"{}"}}}}"#, "y".repeat(398)),
        asked("a1", "2026-10-17T09:00:01.000Z", 1_010, 0, 700, &read),
        format!(
            r#"{{"type":"user","timestamp":"2026-10-17T09:00:02.000Z","message":{{"content":[{{"type":"tool_result","tool_use_id":"t1","content":"{}"}}]}}}}"#,
            "z".repeat(800)
        ),
        asked("a2", "2026-10-17T09:00:03.000Z", 200, 1_010, 0, &format!(r#"{{"type":"text","text":"{}"}}"#, "w".repeat(40))),
        told("2026-10-17T09:10:00.000Z", &format!("<system-reminder>{}", "r".repeat(83)), ""),
        asked("a3", "2026-10-17T09:10:03.000Z", 2_000, 0, 0, ""),
        r#"{"type":"system","subtype":"compact_boundary","uuid":"c1","timestamp":"2026-10-17T09:20:00.000Z","compactMetadata":{"preTokens":5000}}"#.to_string(),
        told("2026-10-17T09:20:01.000Z", &"s".repeat(400), r#""isCompactSummary":true,"#),
        asked("a4", "2026-10-17T09:20:02.000Z", 600, 0, 0, ""),
    ];
    let text = lines.iter().map(|one| format!("{one}\n")).collect::<String>();
    std::fs::write(&file, &text).unwrap();

    let whole = read_turns(&file, 0, false, u64::MAX, None, Carry::default()).unwrap();
    let ids: Vec<&str> = whole.spent.iter().map(|one| one.message_id.as_str()).collect();
    assert_eq!(ids, ["a1", "a2", "a3", "a4"]);
    assert_eq!(whole.spent[0].cache_creation_1h_tokens, 700);
    let first = whole.spent[0].shares.clone().unwrap();
    assert_eq!(shares(&first.read), [(StartUp, 9010), (Messages, 990)], "910 of 1,010 is the opening text");
    assert_eq!(first.write, first.read);
    let second = whole.spent[1].shares.clone().unwrap();
    assert_eq!(second.read, first.read, "the cache read is what the request before held");
    assert_eq!(shares(&second.write), [(Reads, 9569), (OwnWords, 431)], "the writes are the call and its result");
    let cold = whole.spent[2].shares.clone().unwrap();
    assert_eq!(cold.write, [(ColdCache, 1.0)], "ten minutes on, a write larger than the read is a cold cache");
    assert_eq!(shares(&cold.read), [(StartUp, 7465), (Reads, 1641), (OwnWords, 74), (Messages, 820)]);
    let after = whole.spent[3].shares.clone().unwrap();
    assert_eq!(after.read, [(StartUp, 1.0)], "after a compaction the context is opening text again");
    assert_eq!(
        whole.background,
        [BackgroundTurn {
            id: "compaction-c1".into(),
            at: "2026-10-17T09:20:00.000Z".into(),
            model: "m".into(),
            cache_read_tokens: 5_000,
            output_tokens: 100,
        }]
    );

    let cut = lines[..5].iter().map(|one| one.len() as u64 + 1).sum();
    let head = read_turns(&file, 0, false, cut, None, Carry::default()).unwrap();
    assert_eq!(head.next, cut);
    let carried = Carry::resumed(head.carry.kept().as_deref());
    assert_eq!(carried, head.carry, "the carry is kept as text and read back whole");
    let tail = read_turns(&file, head.next, false, u64::MAX, head.input_at.clone(), carried).unwrap();
    assert_eq!([head.spent, tail.spent].concat(), whole.spent);
    assert_eq!(tail.background, whole.background);
    std::fs::remove_dir_all(&dir).ok();
}
