//! Who a line of a hosted session's stream belongs to: the subagent whose call it sits under, and the
//! call that started that subagent. `docs/concepts/session.md`, *The plan*, *Who touched which file*.
//!
//! **Read beside `transcript::read`, not inside it**, for `inside_a_subagent`'s reason: a Drone's
//! events do not carry them, and only a hosted session's Field wants to say whose edit it saw.
//! The stream marks every `assistant` and `user` line a subagent made with `parent_tool_use_id`, the
//! id of the `Agent` (once `Task`) call that started it, and that call's input names the type.

use serde::Deserialize;

/// What a subagent call that names no type asks for.
const DEFAULT_KIND: &str = "general-purpose";

/// The call that started a subagent, the type of subagent it asked for, and whether it runs in the
/// background, so the call returns at once rather than when the subagent ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubagentCall {
    pub call: String,
    pub kind: String,
    pub background: bool,
}

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    parent_tool_use_id: Option<String>,
    #[serde(default)]
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Content,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Blocks(Vec<Block>),
    Prose(#[allow(dead_code)] String),
}

impl Default for Content {
    fn default() -> Self {
        Content::Blocks(Vec::new())
    }
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    input: Input,
}

#[derive(Deserialize, Default)]
struct Input {
    #[serde(default)]
    subagent_type: Option<String>,
    #[serde(default)]
    run_in_background: Option<bool>,
}

/// The call whose subagent made this line. **`None` on the agent's own line**, and on every line
/// that is not a message.
pub fn subagent_parent(line: &str) -> Option<String> {
    // Every line of a turn passes here, and few carry the key set.
    if !line.contains("\"parent_tool_use_id\"") {
        return None;
    }
    let read: Line = ipc::decode("a transcript line", line.trim().as_bytes()).ok()?;
    matches!(read.kind.as_str(), "assistant" | "user").then_some(read.parent_tool_use_id).flatten()
}

/// The subagent calls an `assistant` line makes. **A call that names no type asks for the default
/// one**, `general-purpose`.
pub fn subagent_calls(line: &str) -> Vec<SubagentCall> {
    // Every line of a turn passes here, and few start a subagent.
    if !line.contains("\"Agent\"") && !line.contains("\"Task\"") {
        return Vec::new();
    }
    let Ok(read) = ipc::decode::<Line>("a transcript line", line.trim().as_bytes()) else {
        return Vec::new();
    };
    if read.kind != "assistant" {
        return Vec::new();
    }
    let Some(Message { content: Content::Blocks(blocks) }) = read.message else {
        return Vec::new();
    };
    blocks
        .into_iter()
        .filter(|block| block.kind == "tool_use" && matches!(block.name.as_str(), "Agent" | "Task"))
        .filter_map(|block| {
            let kind = block.input.subagent_type.as_deref().map(str::trim).filter(|kind| !kind.is_empty());
            let kind = kind.unwrap_or(DEFAULT_KIND).to_string();
            let background = block.input.run_in_background == Some(true);
            (!block.id.is_empty()).then_some(SubagentCall { call: block.id, kind, background })
        })
        .collect()
}
