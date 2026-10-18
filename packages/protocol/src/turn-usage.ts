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
};

/** `get_session_usage`: the turns in the window, oldest first. */
export type SessionTurnUsage = {
  turns: TurnUsage[];
};
