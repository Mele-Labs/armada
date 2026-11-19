//! What each tool call of a thread was, and how large its result was, for the waste that comes of what
//! the calls put into the context. `docs/concepts/session/where-the-tokens-went.md`, *Waste*.
//!
//! **Kept of a call's input**: a Read's file path and the lines it asked for, and the file an edit changed, to find
//! the same lines read twice with no edit between. A result is its size in bytes and nothing of its text.

/// A tool call, as written in its message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub tool_use_id: String,
    pub message_id: String,
    /// The stamp of the line that made the call.
    pub at: String,
    /// The thread the call was made in: empty for the session's own, a subagent's file name for a subagent's.
    pub thread: String,
    /// The compactions the thread had had when the call was made. A file read again after one is read
    /// into a context that no longer holds it.
    pub epoch: u32,
    pub tool: String,
    /// The file a Read touched; never set for any other tool.
    pub path: Option<String>,
    /// The lines a Read asked for, first and last, one-based. Both absent for the whole file, and `to_line` absent
    /// for from a line to the end. Never set for any other tool.
    pub from_line: Option<u64>,
    pub to_line: Option<u64>,
    /// The file an Edit, Write, MultiEdit or NotebookEdit changed; never set for any other tool.
    pub edited: Option<String>,
}

/// The lines a Read of `offset` and `limit` asked for.
pub(super) fn lines_of(offset: Option<u64>, limit: Option<u64>) -> (Option<u64>, Option<u64>) {
    match (offset, limit) {
        (None, None) => (None, None),
        (Some(from), None) => (Some(from.max(1)), None),
        (None, Some(count)) => (Some(1), Some(count.max(1))),
        (Some(from), Some(count)) => (Some(from.max(1)), Some(from.max(1) + count.max(1) - 1)),
    }
}

/// The size of a call's result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallResult {
    pub tool_use_id: String,
    pub bytes: u64,
}

/// The request that first carried a call's result: the first of the thread after the result was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCarried {
    pub tool_use_id: String,
    pub message_id: String,
}
