// What a session's turns cost, mirrored by hand from `crates/ipc/src/turn_usage.rs`. Names and
// counts only: a tool is its name, never its input.

/** One turn: a session reports each under the message id its harness gave it. */
export type TurnUsage = {
  message_id: string;
  at: string;
  model: string;
  input_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  output_tokens: number;
  /** The tools the turn called, by name, once for each call. Left out where none. */
  tools?: string[];
  /** A subagent answered the turn. Left out when false. */
  subagent?: boolean;
  /** What the first call was for, when it was a shell command. Left out otherwise. */
  bash_use?: BashUse;
};

/** What a shell command is for, from the first real command in it. A label, never the command. */
export type BashUse =
  | "read_search"
  | "scripts"
  | "git"
  | "build_test"
  | "loops_waits"
  | "forge"
  | "other";

/** `get_session_usage`: the turns in the window, oldest first. */
export type SessionTurnUsage = {
  turns: TurnUsage[];
};
