//! A terminal session's thread, drawn from the transcript lines the CLI wrote.
//! **No process starts**: the lines are the ones spike 27 recorded.

use std::io::Write as _;

use ipc::{SessionRow, SessionVoice};

use crate::terminal_thread::{find, read_from};

fn line(kind: &str, uuid: &str, extra: &str, content: &str) -> String {
    format!(
        r#"{{"type":"{kind}","uuid":"{uuid}","timestamp":"2026-10-07T08:48:10.000Z",{extra}"message":{{"role":"x","content":{content}}}}}"#
    )
}

fn written(lines: &[String]) -> (crate::tests::repo::TempRepo, std::path::PathBuf) {
    let home = crate::tests::repo::TempRepo::empty();
    let dir = home.root().join(".claude/projects/-tmp-here");
    std::fs::create_dir_all(&dir).expect("a project directory");
    let file = dir.join("abc-123.jsonl");
    std::fs::write(&file, lines.join("\n") + "\n").expect("a transcript");
    (home, file)
}

/// **What the person sees is what was said**, whichever way it arrived: typed
/// in the terminal, or submitted by a mod as the person's own, which is how a
/// message from Bridge comes in.
#[test]
fn what_the_person_typed_and_what_a_mod_submitted_for_them_are_both_theirs() {
    let (_home, file) = written(&[
        line(
            "user",
            "u1",
            r#""origin":{"kind":"human"},"#,
            r#""Fix the build""#,
        ),
        line(
            "user",
            "u2",
            r#""origin":{"kind":"plugin","name":"armada","asUser":true},"#,
            r#"[{"type":"text","text":"And the lint"}]"#,
        ),
        line(
            "assistant",
            "a1",
            "",
            r#"[{"type":"text","text":"On it."}]"#,
        ),
    ]);
    let thread = read_from(&file, 0).expect("readable");
    let said: Vec<(String, bool, String)> = thread
        .rows
        .iter()
        .map(|row| match row {
            SessionRow::Message {
                id, from, text, ..
            } => (id.clone(), matches!(from, SessionVoice::You), text.clone()),
            other => panic!("not a message: {other:?}"),
        })
        .collect();
    assert_eq!(
        said,
        vec![
            ("u1".into(), true, "Fix the build".into()),
            ("u2".into(), true, "And the lint".into()),
            ("a1".into(), false, "On it.".into()),
        ]
    );
}

/// A tool call is one line, as a hosted thread draws it, and the reasoning, the
/// bookkeeping, a tool's answer and a subagent's own turns are not drawn.
#[test]
fn a_call_is_one_row_and_everything_that_is_not_the_conversation_is_left_out() {
    let (_home, file) = written(&[
        line(
            "assistant",
            "a1",
            "",
            r#"[{"type":"thinking","thinking":"","signature":"x"}]"#,
        ),
        line(
            "assistant",
            "a2",
            "",
            r#"[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]"#,
        ),
        line(
            "user",
            "u1",
            "",
            r#"[{"type":"tool_result","tool_use_id":"t1","content":"x"}]"#,
        ),
        line(
            "assistant",
            "a3",
            r#""isSidechain":true,"#,
            r#"[{"type":"text","text":"a subagent"}]"#,
        ),
        line(
            "user",
            "u2",
            r#""isMeta":true,"#,
            r#""caveat the CLI wrote""#,
        ),
        line(
            "user",
            "u3",
            r#""origin":{"kind":"peer"},"#,
            r#""from another session""#,
        ),
        r#"{"type":"queue-operation","operation":"enqueue"}"#.to_string(),
        "not json".to_string(),
    ]);
    let thread = read_from(&file, 0).expect("readable");
    assert_eq!(thread.rows.len(), 1, "{:?}", thread.rows);
    assert!(matches!(&thread.rows[0], SessionRow::Tool { id, text, .. } if id == "a2" && text == "Bash ls"));
}

/// **A tail starts where the last read stopped**, and a line still being written
/// is not read half.
#[test]
fn a_read_resumes_at_the_last_whole_line() {
    let (_home, file) = written(&[line(
        "user",
        "u1",
        r#""origin":{"kind":"human"},"#,
        r#""one""#,
    )]);
    let first = read_from(&file, 0).expect("readable");
    assert_eq!(first.rows.len(), 1);
    let mut open = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .expect("open");
    let whole = line("user", "u2", r#""origin":{"kind":"human"},"#, r#""two""#);
    let (front, back) = whole.split_at(whole.len() - 4);
    write!(open, "{front}").expect("a half line");
    let half = read_from(&file, first.next).expect("readable");
    assert!(half.rows.is_empty());
    assert_eq!(half.next, first.next, "the half line is not consumed");
    writeln!(open, "{back}").expect("the rest");
    let rest = read_from(&file, first.next).expect("readable");
    assert!(matches!(&rest.rows[..], [SessionRow::Message { text, .. }] if text == "two"));
}

/// A session is found by its id wherever its project directory is, and an id
/// is never a path.
#[test]
fn a_transcript_is_found_by_id_under_any_project_directory() {
    let (home, _file) = written(&[]);
    let home = home.root_str();
    assert!(find(&home, "abc-123").is_some());
    assert!(find(&home, "nope").is_none());
    assert!(find(&home, "../abc-123").is_none());
}

/// **No markup reaches the thread.** A slash command is a row of its own, as
/// typed, and its output, the caveat before it, a reminder and another
/// session's hand-back are the CLI's and are not drawn.
#[test]
fn a_command_is_a_row_and_the_cli_s_own_markup_is_not_drawn() {
    let (_home, file) = written(&[
        line(
            "user",
            "c1",
            "",
            r#""<command-name>/reload-plugins</command-name>\n<command-message>reload-plugins</command-message>\n<command-args></command-args>""#,
        ),
        line(
            "user",
            "c2",
            r#""origin":{"kind":"human"},"#,
            r#""<command-message>model</command-message>\n<command-name>/model</command-name>\n<command-args>opus</command-args>""#,
        ),
        line("user", "o1", "", r#""<local-command-stdout>Reloaded: 3 plugins</local-command-stdout>""#),
        line("user", "o2", "", r#""<local-command-caveat>Caveat: run directly</local-command-caveat>""#),
        line("user", "o3", r#""origin":{"kind":"human"},"#, r#""<system-reminder>named</system-reminder>""#),
        line(
            "user",
            "o4",
            "",
            r#""Another Claude session sent a message:\n<agent-message from=\"x\">done</agent-message>""#,
        ),
        line("user", "o5", "", r#""<task-notification><task-id>1</task-id></task-notification>""#),
        line("user", "b1", "", r#""<bash-input>git status</bash-input>""#),
        line("user", "b2", "", r#""<bash-stdout>clean</bash-stdout><bash-stderr></bash-stderr>""#),
        line("user", "p1", r#""origin":{"kind":"human"},"#, r#""<system-reminder>x</system-reminder>Hello""#),
    ]);
    let thread = read_from(&file, 0).expect("readable");
    let drawn: Vec<(String, String)> = thread
        .rows
        .iter()
        .map(|row| match row {
            SessionRow::Command { id, text, .. } => (id.clone(), format!("command {text}")),
            SessionRow::Message { id, text, .. } => (id.clone(), format!("message {text}")),
            other => panic!("not drawn so: {other:?}"),
        })
        .collect();
    assert_eq!(
        drawn,
        vec![
            ("c1".into(), "command /reload-plugins".into()),
            ("c2".into(), "command /model opus".into()),
            ("b1".into(), "command ! git status".into()),
            ("p1".into(), "message Hello".into()),
        ]
    );
}

/// The summary the CLI writes where it compacted is a user line that is not the
/// person's, so it is a row of its own and never "You".
#[test]
fn a_compaction_summary_is_not_the_person() {
    let (_home, file) = written(&[
        line(
            "user",
            "k1",
            r#""isCompactSummary":true,"#,
            r#""This session is being continued from a previous conversation.""#,
        ),
        line("user", "k2", r#""origin":{"kind":"human"},"#, r#""Carry on""#),
    ]);
    let thread = read_from(&file, 0).expect("readable");
    assert!(matches!(
        &thread.rows[0],
        SessionRow::Compaction { id, text, .. } if id == "k1" && text.starts_with("This session")
    ));
    assert!(matches!(&thread.rows[1], SessionRow::Message { from: SessionVoice::You, .. }));
    assert_eq!(thread.rows.len(), 2);
}

/// A hand-back between agents is flagged `isMeta` and arrives with a peer's
/// origin; it is nobody's message in this thread.
#[test]
fn an_agent_to_agent_hand_back_is_not_drawn() {
    let (_home, file) = written(&[
        line(
            "user",
            "h1",
            r#""isMeta":true,"origin":{"kind":"peer","name":"x"},"#,
            r#""Another Claude session sent a message:\n<agent-message from=\"x\">report</agent-message>""#,
        ),
        line("user", "h2", r#""origin":{"kind":"peer","name":"x"},"#, r#""<agent-message from=\"x\">report</agent-message>""#),
    ]);
    assert!(read_from(&file, 0).expect("readable").rows.is_empty());
}

fn in_agent(kind: &str, uuid: &str, reason: &str, content: &str) -> String {
    format!(
        r#"{{"type":"{kind}","uuid":"{uuid}","isSidechain":true,"agentId":"x1","timestamp":"2026-10-07T08:48:10.000Z","message":{{"role":"x","stop_reason":{reason},"content":{content}}}}}"#
    )
}

fn agent_written(lines: &[String]) -> (crate::tests::repo::TempRepo, String) {
    let (home, file) = written(&[]);
    let dir = file.with_extension("").join("subagents");
    std::fs::create_dir_all(&dir).expect("a subagents directory");
    std::fs::write(dir.join("agent-x1.jsonl"), lines.join("\n") + "\n").expect("a transcript");
    let root = home.root().to_string_lossy().to_string();
    (home, root)
}

/// A subagent's own lines are all `isSidechain`, and they are the whole of its thread: its
/// messages and its calls are drawn, and it is not finished until a turn ends.
#[test]
fn a_running_subagent_draws_its_own_calls_and_a_finished_one_its_report() {
    let running = vec![
        in_agent("user", "s1", "null", r#""Read the CI history""#),
        in_agent("assistant", "s2", "\"tool_use\"", r#"[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a/b.rs"}}]"#),
    ];
    let (home, root) = agent_written(&running);
    let file = crate::terminal_thread::find_subagent(&root, "abc-123", "x1").expect("its file");
    let seen = crate::terminal_thread::read_subagent(&file).expect("readable");
    assert!(!seen.finished);
    assert_eq!(seen.report, None);
    assert!(matches!(seen.rows.as_slice(), [SessionRow::Message { .. }, SessionRow::Tool { text, .. }] if text.starts_with("Read")));

    let mut done = running.clone();
    done.push(in_agent("assistant", "s3", "\"end_turn\"", r#"[{"type":"text","text":"Nothing flaky."}]"#));
    std::fs::write(&file, done.join("\n") + "\n").unwrap();
    let seen = crate::terminal_thread::read_subagent(&file).expect("readable");
    assert!(seen.finished);
    assert_eq!(seen.report.as_deref(), Some("Nothing flaky."));
    assert_eq!(seen.rows.len(), 3);
    drop(home);
}

/// Only the one path is read: a name that could leave the directory finds nothing.
#[test]
fn a_subagent_name_cannot_leave_its_directory() {
    let (_home, root) = agent_written(&[]);
    assert!(crate::terminal_thread::find_subagent(&root, "abc-123", "../abc-123").is_none());
    assert!(crate::terminal_thread::find_subagent(&root, "abc-123", "nope").is_none());
}

/// A background subagent ends on its hand-back, never on `end_turn`, and the report is what it
/// handed back. Work after a hand-back (a resumed subagent) makes it running again.
#[test]
fn a_subagent_that_handed_back_is_finished_with_what_it_handed_back() {
    let dir = std::env::temp_dir().join(format!("handback-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("agent-a1.jsonl");
    let call = r#"{"type":"assistant","isSidechain":true,"message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#;
    let back = r#"{"type":"assistant","isSidechain":true,"message":{"role":"assistant","content":[{"type":"tool_use","id":"t2","name":"SubagentHandback","input":{"message":"Done: pushed."}}]}}"#;
    std::fs::write(&file, format!("{call}\n{back}\n")).unwrap();
    let read = crate::terminal_thread::read_subagent(&file).unwrap();
    assert!(read.finished);
    assert_eq!(read.report.as_deref(), Some("Done: pushed."));
    std::fs::write(&file, format!("{call}\n{back}\n{call}\n")).unwrap();
    assert!(!crate::terminal_thread::read_subagent(&file).unwrap().finished, "resumed after its hand-back");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A session that moves into a slot resumes in a project folder the CLI keys by the new
/// directory; the transcript has to be there, and be the same file.
#[test]
fn a_conversation_follows_its_session_into_another_directory() {
    let home = std::env::temp_dir().join(format!("follow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let a = home.join(".claude/projects/-work-armada");
    std::fs::create_dir_all(a.join("abc-123/subagents")).unwrap();
    std::fs::write(a.join("abc-123.jsonl"), "{}\n").unwrap();
    let root = home.to_string_lossy().to_string();
    crate::terminal_thread::bring_conversation_to(&root, "abc-123", "/work/armada/.armada/slots/slot-4").unwrap();
    let b = home.join(".claude/projects/-work-armada--armada-slots-slot-4");
    assert_eq!(std::fs::read_to_string(b.join("abc-123.jsonl")).unwrap(), "{}\n");
    assert!(b.join("abc-123/subagents").is_dir());
    let mut appended = std::fs::OpenOptions::new().append(true).open(b.join("abc-123.jsonl")).unwrap();
    std::io::Write::write_all(&mut appended, b"{\"more\":1}\n").unwrap();
    assert!(std::fs::read_to_string(a.join("abc-123.jsonl")).unwrap().contains("more"), "one file");
    crate::terminal_thread::bring_conversation_to(&root, "nope", "/elsewhere").unwrap();
    assert!(!home.join(".claude/projects/-elsewhere").exists());
    let _ = std::fs::remove_dir_all(&home);
}

fn subagent_file(lines: &[String]) -> (crate::tests::repo::TempRepo, String, std::path::PathBuf) {
    let (home, root) = agent_written(lines);
    let file = crate::terminal_thread::find_subagent(&root, "abc-123", "x1").expect("its file");
    (home, root, file)
}

/// The lean check says what `read_subagent` says, without drawing a row.
#[test]
fn the_lean_subagent_check_finishes_on_end_turn_or_hand_back_and_resumes_on_work() {
    let call = in_agent("assistant", "s2", "\"tool_use\"", r#"[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a"}}]"#);
    let ended = in_agent("assistant", "s3", "\"end_turn\"", r#"[{"type":"text","text":"Done."}]"#);
    let back = in_agent("assistant", "s4", "null", r#"[{"type":"tool_use","id":"t2","name":"SubagentHandback","input":{"message":"Done: pushed."}}]"#);
    let asked = in_agent("user", "s1", "null", r#""Read the CI history""#);
    let cases: [(&str, Vec<String>, bool); 5] = [
        ("end_turn", vec![asked.clone(), call.clone(), ended.clone()], true),
        ("hand-back", vec![asked.clone(), call.clone(), back.clone()], true),
        ("work after end_turn", vec![asked.clone(), ended.clone(), call.clone()], false),
        ("a user line after end_turn", vec![asked.clone(), ended.clone(), asked.clone()], true),
        ("nothing decided", vec![asked.clone()], false),
    ];
    for (name, lines, want) in cases {
        let (_home, root, file) = subagent_file(&lines);
        assert_eq!(crate::terminal_thread::subagent_ended(&root, "abc-123", "x1"), want, "{name}");
        assert_eq!(crate::terminal_thread::read_subagent(&file).unwrap().finished, want, "{name}, as read_subagent has it");
    }
    assert!(!crate::terminal_thread::subagent_ended("/nowhere", "abc-123", "x1"), "no file, not finished");
}

/// The deciding line can be further from the end than one read: the scan widens until it finds it.
#[test]
fn the_lean_check_reaches_back_past_a_long_tail() {
    let back = in_agent("assistant", "s4", "null", r#"[{"type":"tool_use","id":"t2","name":"SubagentHandback","input":{"message":"Done."}}]"#);
    let long = in_agent("user", "u", "null", &format!("\"{}\"", "x".repeat(150 * 1024)));
    let (_home, _root, file) = subagent_file(&[back.clone(), long.clone(), long.clone()]);
    assert!(crate::terminal_thread::finished_at_the_end(&file).unwrap());
    std::fs::write(&file, [long.clone(), long.clone()].join("\n") + "\n").unwrap();
    assert!(!crate::terminal_thread::finished_at_the_end(&file).unwrap(), "nothing to decide by");
}

/// An unchanged file (same length, same modified time) is not read again: the answer from the
/// first read stands even when the bytes under it differ.
#[test]
fn an_unchanged_subagent_file_is_served_from_the_cache() {
    let ended = in_agent("assistant", "s3", "\"end_turn\"", r#"[{"type":"text","text":"Done."}]"#);
    let (_home, root, file) = subagent_file(&[ended.clone()]);
    let before = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert!(crate::terminal_thread::subagent_ended(&root, "abc-123", "x1"));

    // Same length, no longer an ended turn, and the modified time put back.
    std::fs::write(&file, ended.replace("end_turn", "end_tur_") + "\n").unwrap();
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(before).unwrap();
    assert!(crate::terminal_thread::subagent_ended(&root, "abc-123", "x1"), "served from the cache");

    // The same bytes with a later modified time are read again.
    let later = before + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(later).unwrap();
    assert!(!crate::terminal_thread::subagent_ended(&root, "abc-123", "x1"), "a changed file is read again");
}
