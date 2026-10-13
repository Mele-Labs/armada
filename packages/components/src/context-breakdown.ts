// What fills a Session's context window, in the camelCase the view-model draws. Types only, so a file that
// has no DOM can name them: `sessions-wire.ts` maps the wire's `context_breakdown` onto it.

/**
 * What fills a Session's context window, as the harness reports it. `kind` says how a category counts:
 * `used` is in the window, `free` is room left, `buffer` is held back for autocompact, and `deferred`
 * is loaded on demand and counts for nothing until it is.
 */
export type ContextBreakdown = {
  categories: readonly { name: string; tokens: number; kind: "used" | "free" | "buffer" | "deferred" }[];
  memoryFiles: readonly { path: string; tokens: number }[];
  mcpTools: readonly { name: string; server: string; tokens: number }[];
  totalTokens: number;
  maxTokens: number;
  /** The count at which auto-compaction runs. Absent when it is off. */
  autocompactThreshold?: number;
};
