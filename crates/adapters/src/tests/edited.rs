//! Which file an edit call named, read off the real stream's shape. Spike 022,
//! slice 5, answer 10: overlap between tasks running at once is read from
//! these, and a shell write is not among them.

use adapter_traits::AgentHarness;

use crate::harness::HeadlessAgent;

fn edited(line: &str) -> Vec<Option<String>> {
    let harness = HeadlessAgent::at("/usr/local/bin/agent");
    harness
        .read(line)
        .iter()
        .map(|event| harness.edited(event))
        .collect()
}

/// An edit and a write each name their file, without the size the row shows;
/// a read names one too and is not an edit.
#[test]
fn an_edit_and_a_write_name_their_file_and_a_read_does_not() {
    let read = edited(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"Read",
              "input":{"file_path":"/tmp/repo/src/settings.rs"}},
             {"type":"tool_use","id":"b","name":"Edit",
              "input":{"file_path":"/tmp/repo/reducer.rs",
                       "old_string":"one\ntwo","new_string":"a\nb\nc"}},
             {"type":"tool_use","id":"c","name":"Write",
              "input":{"file_path":"/tmp/repo/src/pending.rs",
                       "content":"pub struct Pending;\n"}}]}}"#,
    );
    assert_eq!(
        read,
        vec![
            None,
            Some("/tmp/repo/reducer.rs".to_string()),
            Some("/tmp/repo/src/pending.rs".to_string()),
        ]
    );
}

/// **A shell write goes unseen**: nothing in the call says it wrote anything,
/// and it is not guessed at.
#[test]
fn a_shell_write_is_not_an_edit_call() {
    let read = edited(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"Bash",
              "input":{"command":"sed -i '' 's/a/b/' /tmp/repo/reducer.rs"}}]}}"#,
    );
    assert_eq!(read, vec![None]);
}

/// A path longer than a row shows is read whole, not cut where the row was.
#[test]
fn a_long_path_is_read_whole() {
    let deep = format!("/tmp/repo/{}/leaf.rs", "nested/".repeat(40));
    let line = format!(
        r#"{{"type":"assistant","message":{{"content":[
             {{"type":"tool_use","id":"a","name":"Write",
              "input":{{"file_path":"{deep}","content":"x\n"}}}}]}}}}"#
    );
    assert_eq!(edited(&line), vec![Some(deep)]);
}
