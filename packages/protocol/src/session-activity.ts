// What a hosted session is doing, mirrored by hand from `crates/ipc/src/session_activity.rs`.
// Detected from its tool calls, never reported by the agent: only `todos` is the agent's own list.

/** Where one item of the agent's own list stands. */
export type SessionTodoState = "done" | "doing" | "todo";

/** One item of the agent's own `TodoWrite` list. */
export type SessionTodo = { text: string; state: SessionTodoState };

/**
 * Carried on `SessionRecord.activity`. Absent from the row until the first tool call, on a session a
 * terminal runs, and once the session ends. Memory only: a Fleet that restarts starts it empty.
 */
export type SessionActivity = {
  /** The latest tool call as one short action, such as `Editing gate.rs` or `Running cargo check`. */
  now?: string;
  /** The agent's own list as its last `TodoWrite` left it. Absent is no list. */
  todos?: SessionTodo[];
};
