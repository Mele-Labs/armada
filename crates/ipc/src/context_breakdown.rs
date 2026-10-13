//! What fills a session's context window, as `/context` shows it. `docs/concepts/session.md`.
//!
//! **Names, paths and counts only.** No message text, prompt or file contents are ever in it.

use serde::{Deserialize, Serialize};

/// What a row of the breakdown is. Bridge branches on this and never on the row's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    /// Content in the window.
    Used,
    /// The window left.
    Free,
    /// The compaction reserve.
    Buffer,
    /// Tool schemas loaded on demand, outside the window.
    Deferred,
}

/// One row, as `/context` lists it beside the grid: `System prompt`, `Messages`, `Free space`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCategory {
    pub name: String,
    pub tokens: u64,
    pub kind: ContextKind,
}

/// A memory file in the context: a CLAUDE.md, a rules file, an auto-memory entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextMemoryFile {
    pub path: String,
    pub tokens: u64,
}

/// One MCP tool's schema in the context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextMcpTool {
    pub name: String,
    pub server: String,
    pub tokens: u64,
}

/// The latest breakdown a session reported, estimated locally. Carried on `SessionRecord` and on the
/// `measured` fact, and kept per session in the ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBreakdown {
    pub categories: Vec<ContextCategory>,
    pub memory_files: Vec<ContextMemoryFile>,
    /// The ten with the most tokens, largest first.
    pub mcp_tools: Vec<ContextMcpTool>,
    /// Tokens in use, past `max_tokens` when over the window.
    pub total_tokens: u64,
    /// The window measured against.
    pub max_tokens: u64,
    /// The token count at which auto-compaction runs. Absent when it is off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autocompact_threshold: Option<u64>,
}
