// What a hosted session is doing, mirrored by hand from `crates/ipc/src/session_activity.rs`.
// Detected from its tool calls, never reported by the agent: only `todos` is the agent's own list.

/** Where one item of the agent's own list stands. */
export type SessionTodoState = "done" | "doing" | "todo";

/** One item of the agent's own `TodoWrite` list. */
export type SessionTodo = { text: string; state: SessionTodoState };

/** What one subagent of a hosted session is doing, by the gate's `agent_id` on its calls. */
export type SubagentActivity = {
  id: string;
  /** The subagent's latest call as one short action, as the session's own `now` is. */
  now: string;
};

/**
 * Carried on `SessionRecord.activity`. Absent from the row until the first tool call, on a session a
 * terminal runs, and once the session ends. Memory only: a Fleet that restarts starts it empty.
 */
export type SessionActivity = {
  /** The latest tool call as one short action, such as `Editing gate.rs` or `Running cargo check`. */
  now?: string;
  /**
   * The first line of the agent's latest text this turn, at most 160 characters. Absent until it has
   * said something, and again when a turn begins.
   */
  said?: string;
  /** The agent's own list as its last `TodoWrite` left it. Absent is no list. */
  todos?: SessionTodo[];
  /** What each subagent at work is doing, by its own calls. An entry goes when the subagent ends and when a turn begins. Absent is none. */
  subagents?: SubagentActivity[];
};
