//! What a hosted Job's process shares with the door: the tool that puts a call to a person, the
//! tools the door serves, and how a write is recognised in the stream. `#939`, `#1373`.

use adapter_traits::ToolUse;

/// The door's own tool the CLI puts every uncovered call to. **Not shown to the
/// model**, which spike 15 measured, so it adds nothing a Job can reach for.
pub(crate) fn asks_a_person() -> String {
    format!("mcp__{}__{}", ipc::door::SERVER, ipc::door::ASKS_A_PERSON)
}

/// The built-in tools that change the checkout a conversation runs in.
///
/// **A list to recognise a write by, not to refuse one.** A Job edits the
/// checkout when asked, so Fleet reads this to publish an event rather
/// than to stop the call.
pub const CHANGES_THE_CHECKOUT: &[&str] = &["Edit", "MultiEdit", "Write", "NotebookEdit"];

/// Whether a tool call changed the checkout the conversation is open in.
///
/// **`Bash` is not among them and cannot be**: a shell line may write a file or
/// read one, and nothing in the stream says which.
pub fn wrote_the_checkout(tool: &str) -> bool {
    CHANGES_THE_CHECKOUT.contains(&tool)
}

/// A file read, a shell command run or a call to a service outside the repository, out of the detail
/// [`crate::transcript`] composed for the call: the path, and the command as the call carried it.
/// **A `gh` command is the last, not a command run**, so it is entered once.
pub fn tool_use(tool: &str, detail: &str) -> Option<ToolUse> {
    let said = detail.trim();
    match tool {
        "Read" if !said.is_empty() => Some(ToolUse::Read(said.to_string())),
        "Bash" if !said.is_empty() => Some(match crate::outside::command_reach(said) {
            Some(reach) => ToolUse::Reached(reach),
            None => ToolUse::Ran(said.to_string()),
        }),
        _ => crate::outside::tool_reach(tool, said).map(ToolUse::Reached),
    }
}

/// The file a write call named, out of the detail [`crate::transcript`] composed
/// for it — a path, then `+added -removed` where the call said how much.
///
/// **Read here rather than where the event is published**, because the two ends
/// of that spelling are both this crate's and would drift apart in one move.
pub fn path_written(detail: &str) -> &str {
    let mut path = detail;
    for mark in ['-', '+'] {
        if let Some((head, size)) = path.rsplit_once(' ') {
            if size.starts_with(mark) && size[1..].chars().all(|c| c.is_ascii_digit()) {
                path = head;
            }
        }
    }
    path
}

/// Every tool the agent's door serves, as the CLI allows a whole server.
pub fn door_tools() -> String {
    format!("mcp__{}", ipc::door::SERVER)
}

/// The switch that gives a headless process the task tools a terminal has: without it the agent
/// can write no list, and a Session's build record never has pieces. `CLAUDE_CODE_ENABLE_TASKS`
/// does not do it — measured on the CLI, 2.1.300.
pub const TASK_TOOLS: &str = "CLAUDE_CODE_ENABLE_TODO_TOOLS";

/// Sent as a message, Claude Code reads this as its own command to shrink the context, not as words
/// for the agent.
pub const COMPACT_COMMAND: &str = "/compact";
