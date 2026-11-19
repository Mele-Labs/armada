//! What each tool call of a thread was, and how large its result was, for the waste that comes of what
//! the calls put into the context. `docs/concepts/session/where-the-tokens-went.md`, *Waste*.
//!
//! **A Read's file path is the one input kept**, to find the same file read twice. A result is its size
//! in bytes and nothing of its text.

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
