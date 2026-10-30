// What a hosted session is doing, mirrored by hand from `crates/ipc/src/session_activity.rs`.
// Detected from its tool calls, never reported by the agent: only `todos` is the agent's own list.
// `account` is written by a small model from what the session did, in the person's words.

/** Where one item of the agent's own list stands. */
export type SessionTodoState = "done" | "doing" | "todo";

/** One item of the agent's own `TodoWrite` list. `task` is the id of the plan task Fleet matched it to. */
export type SessionTodo = { text: string; state: SessionTodoState; task?: string };

/** What one subagent of a hosted session is doing, by the gate's `agent_id` on its calls. */
export type SubagentActivity = {
  id: string;
  /** The subagent's latest call as one short action, as the session's own `now` is. */
  now: string;
};

/**
 * What a small model wrote of the turn so far, in the person's own words: no file names, paths, code
 * identifiers or tool names. Rewritten while the session works and kept until the person's next message.
 */
export type SessionAccount = {
  /** The id of the person's message the account answers, the last one he sent when it was written. */
  for: string;
  /** What the session has done for that message, oldest first. Absent is nothing yet. */
  done?: string[];
  /** What it is on now. */
  now?: string;
  /** What it does next. */
  next?: string;
  /** The id of the approved plan group it is on. Absent where the plan is not approved or none is named. */
  group?: string;
  /** When the account was written. */
  at: string;
};

/** The page the session is building, for a live view beside its account. */
export type SessionLive = {
  url: string;
  /** What the session called the page it showed, or the name of the server it started. */
  title: string;
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
  /** What the session has done, is on and does next, for the person's last message. Absent until written. */
  account?: SessionAccount;
  /** The newest page the session showed or served since the person's last message. */
  live?: SessionLive;
};
