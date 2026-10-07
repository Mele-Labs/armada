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
