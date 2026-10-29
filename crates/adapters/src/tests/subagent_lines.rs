//! Who a line of a hosted session's stream belongs to: the subagent whose call it sits under, and the
//! type of subagent that call asked for. `docs/concepts/session.md`, *Who touched which file*.

use crate::{subagent_calls, subagent_parent, SubagentCall};

const SPAWNING: &str = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"on it"},{"type":"tool_use","id":"toolu_1","name":"Agent","input":{"subagent_type":"rust-engineer","description":"store"}},{"type":"tool_use","id":"toolu_2","name":"Task","input":{"description":"docs"}},{"type":"tool_use","id":"toolu_3","name":"Edit","input":{"file_path":"a.rs"}}]}}"#;
const INSIDE: &str = r#"{"type":"assistant","parent_tool_use_id":"toolu_1","message":{"content":[{"type":"tool_use","id":"toolu_9","name":"Write","input":{"file_path":"b.rs"}}]}}"#;
const OWN: &str = r#"{"type":"assistant","parent_tool_use_id":null,"message":{"content":[{"type":"tool_use","id":"toolu_4","name":"Write","input":{"file_path":"c.rs"}}]}}"#;

#[test]
fn an_agent_or_task_call_names_its_type_and_one_that_names_none_is_general_purpose() {
    let calls = subagent_calls(SPAWNING);
    assert_eq!(
        calls,
        [
            SubagentCall { call: "toolu_1".into(), kind: "rust-engineer".into(), background: false },
            SubagentCall { call: "toolu_2".into(), kind: "general-purpose".into(), background: false },
        ],
        "an Edit starts nothing"
    );
}

#[test]
fn a_line_a_subagent_made_names_the_call_that_started_it_and_the_agents_own_does_not() {
    assert_eq!(subagent_parent(INSIDE).as_deref(), Some("toolu_1"));
    assert_eq!(subagent_parent(OWN), None);
    assert_eq!(subagent_parent(SPAWNING), None);
    assert_eq!(subagent_parent("not json {\"parent_tool_use_id\""), None);
    assert!(subagent_calls(INSIDE).is_empty());
    assert!(subagent_calls("not json \"Agent\"").is_empty());
}
