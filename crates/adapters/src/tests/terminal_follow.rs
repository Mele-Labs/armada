//! A transcript kept drawn in memory is the transcript drawn from the start, however it grew.
//! **No process starts**: the lines are written by the test.

use crate::terminal_thread::{read_from, read_subagent, Followed};

fn user(uuid: &str, extra: &str, content: &str) -> String {
    format!(
        r#"{{"type":"user","uuid":"{uuid}","timestamp":"2026-10-09T08:00:00.000Z",{extra}"message":{{"role":"user","content":{content}}}}}"#
    )
}

fn assistant(uuid: &str, extra: &str, stop: &str, content: &str) -> String {
    format!(
        r#"{{"type":"assistant","uuid":"{uuid}","timestamp":"2026-10-09T08:00:01.000Z",{extra}"message":{{"role":"assistant","stop_reason":{stop},"content":{content}}}}}"#
    )
}

/// Every kind of line the thread treats differently, so a state that crossed lines would show.
fn corpus() -> Vec<String> {
    let human = r#""origin":{"kind":"human"},"#;
    let plugin = r#""origin":{"kind":"plugin","name":"armada","asUser":true},"#;
    vec![
        user("u1", human, r#""Fix the build, héllo 日本語 🙂""#),
        assistant(
            "a1",
            "",
            "null",
            r#"[{"type":"text","text":"On it."},{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a/b.rs"}},{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"ls"}}]"#,
        ),
        user("u2", "", r#"[{"type":"tool_result","tool_use_id":"t1","content":"fn main() {}"}]"#),
        "not json at all".to_string(),
        String::new(),
        user("u3", human, r#""<system-reminder>noise</system-reminder>""#),
        user("u4", human, r#""<command-name>/model</command-name><command-args>opus</command-args>""#),
        assistant("s1", r#""isSidechain":true,"#, "null", r#"[{"type":"text","text":"inside"}]"#),
        user("m1", r#""isMeta":true,"#, r#""meta""#),
        user("u5", plugin, r#"[{"type":"text","text":"And the lint"}]"#),
        user("c1", r#""isCompactSummary":true,"#, r#""The conversation so far: the build was fixed.""#),
        assistant("a2", "", "\"end_turn\"", r#"[{"type":"text","text":"Done."}]"#),
        user("u6", human, r#""after the compaction""#),
        assistant("a3", "", "null", r#"[{"type":"tool_use","id":"t3","name":"Edit","input":{"file_path":"/x"}}]"#),
    ]
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("follow-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A small deterministic generator, so a failing split can be run again.
struct Cuts(u64);

impl Cuts {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Grow `file` to `bytes` in pieces ending at arbitrary bytes, in the middle of lines and of a
/// multi-byte character included, and check after every piece that what is kept is what a read
/// from the start of the same bytes draws.
fn grown_in_pieces_matches(bytes: &[u8], file: &std::path::Path, open: fn(&std::path::Path) -> std::io::Result<Followed>) {
    let mut cuts = Cuts(7);
    for round in 0..120 {
        let mut ends: Vec<usize> = (0..1 + cuts.below(8)).map(|_| cuts.below(bytes.len() + 1)).collect();
        ends.sort_unstable();
        ends.push(bytes.len());
        std::fs::write(file, b"").unwrap();
        let mut kept = open(file).unwrap();
        for end in ends {
            std::fs::write(file, &bytes[..end]).unwrap();
            kept.catch_up().unwrap();
            let whole = read_from(file, 0).unwrap();
            assert_eq!(kept.rows(), whole.rows.as_slice(), "round {round}, cut at {end}");
            assert_eq!(kept.next(), whole.next, "round {round}, cut at {end}");
        }
    }
}

#[test]
fn a_thread_grown_in_pieces_draws_what_one_read_from_the_start_draws() {
    let dir = scratch("pieces");
    let bytes = (corpus().join("\n") + "\n").into_bytes();
    grown_in_pieces_matches(&bytes, &dir.join("s.jsonl"), Followed::thread);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_line_still_being_written_is_drawn_once_it_is_whole() {
    let dir = scratch("partial");
    let file = dir.join("s.jsonl");
    let first = user("u1", r#""origin":{"kind":"human"},"#, r#""one""#);
    let second = user("u2", r#""origin":{"kind":"human"},"#, r#""two""#);
    std::fs::write(&file, format!("{first}\n{}", &second[..40])).unwrap();
    let mut kept = Followed::thread(&file).unwrap();
    assert_eq!(kept.rows().len(), 1);
    assert_eq!(kept.next() as usize, first.len() + 1);
    std::fs::write(&file, format!("{first}\n{second}\n")).unwrap();
    kept.catch_up().unwrap();
    assert_eq!(kept.rows(), read_from(&file, 0).unwrap().rows.as_slice());
    assert_eq!(kept.rows().len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_shorter_file_and_a_replaced_one_are_drawn_again_from_the_start() {
    let dir = scratch("replaced");
    let file = dir.join("s.jsonl");
    let human = r#""origin":{"kind":"human"},"#;
    let old: Vec<String> = (0..6).map(|n| user(&format!("o{n}"), human, r#""old""#)).collect();
    std::fs::write(&file, old.join("\n") + "\n").unwrap();
    let mut kept = Followed::thread(&file).unwrap();
    assert_eq!(kept.rows().len(), 6);

    // Truncated and rewritten shorter.
    std::fs::write(&file, old[..2].join("\n") + "\n").unwrap();
    kept.catch_up().unwrap();
    assert_eq!(kept.rows(), read_from(&file, 0).unwrap().rows.as_slice());
    assert_eq!(kept.rows().len(), 2);

    // Another file at the same path, longer than what was drawn.
    let new: Vec<String> = (0..9).map(|n| user(&format!("n{n}"), human, r#""new""#)).collect();
    let elsewhere = dir.join("other.jsonl");
    std::fs::write(&elsewhere, new.join("\n") + "\n").unwrap();
    std::fs::rename(&elsewhere, &file).unwrap();
    kept.catch_up().unwrap();
    assert_eq!(kept.rows(), read_from(&file, 0).unwrap().rows.as_slice());
    assert_eq!(kept.rows().len(), 9);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A subagent's `done` crosses lines, so it is carried: a hand-back, work after it, an end of turn.
#[test]
fn a_subagent_grown_in_pieces_is_finished_where_one_read_from_the_start_says() {
    let dir = scratch("agent");
    let file = dir.join("agent-a1.jsonl");
    let side = r#""isSidechain":true,"#;
    let call = assistant("g1", side, "\"tool_use\"", r#"[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]"#);
    let back = assistant("g2", side, "null", r#"[{"type":"tool_use","id":"t2","name":"SubagentHandback","input":{"message":"Done: pushed."}}]"#);
    let end = assistant("g3", side, "\"end_turn\"", r#"[{"type":"text","text":"Nothing flaky."}]"#);
    let asked = user("g0", side, r#""Read the CI history""#);
    for lines in [
        vec![asked.clone(), call.clone(), back.clone()],
        vec![asked.clone(), call.clone(), back.clone(), call.clone()],
        vec![asked.clone(), call.clone(), end.clone()],
        vec![asked.clone(), back.clone(), call.clone(), end.clone(), call.clone(), back.clone()],
    ] {
        let bytes = (lines.join("\n") + "\n").into_bytes();
        let mut cuts = Cuts(11);
        for round in 0..60 {
            let mut ends: Vec<usize> = (0..1 + cuts.below(6)).map(|_| cuts.below(bytes.len() + 1)).collect();
            ends.sort_unstable();
            ends.push(bytes.len());
            std::fs::write(&file, b"").unwrap();
            let mut kept = Followed::subagent(&file).unwrap();
            for end in ends {
                std::fs::write(&file, &bytes[..end]).unwrap();
                kept.catch_up().unwrap();
                let whole = read_subagent(&file).unwrap();
                assert_eq!(kept.rows(), whole.rows.as_slice(), "round {round}, cut at {end}");
                assert_eq!(kept.finished(), (whole.finished, whole.report), "round {round}, cut at {end}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
