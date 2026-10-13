// What fills a session's context window, mirrored by hand from `crates/ipc/src/context_breakdown.rs`.
// Names, paths and counts only: no message text, prompt or file contents.

/** What a row is. Branch on this, never on the row's name. */
export type ContextKind = "used" | "free" | "buffer" | "deferred";

/** One row, as `/context` lists it beside the grid. */
export type ContextCategory = { name: string; tokens: number; kind: ContextKind };

/** A memory file in the context: a CLAUDE.md, a rules file, an auto-memory entry. */
export type ContextMemoryFile = { path: string; tokens: number };

/** One MCP tool's schema in the context. */
export type ContextMcpTool = { name: string; server: string; tokens: number };

/** The latest breakdown a session's mod reported, carried on `SessionRecord.context_breakdown`. */
export type ContextBreakdown = {
  categories: ContextCategory[];
  memory_files: ContextMemoryFile[];
  /** The ten with the most tokens, largest first. */
  mcp_tools: ContextMcpTool[];
  /** Tokens in use, past `max_tokens` when over the window. */
  total_tokens: number;
  /** The window measured against. */
  max_tokens: number;
  /** The token count at which auto-compaction runs. Absent when it is off. */
  autocompact_threshold?: number;
};
